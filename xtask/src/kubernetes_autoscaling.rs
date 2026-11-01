use crate::kubernetes_environment::Target;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    thread,
    time::{Duration, Instant},
};

const RESUME_REPLICAS: &str = "hirune.axum-kit/resume-replicas";
const LAST_APPLIED: &str = "kubectl.kubernetes.io/last-applied-configuration";

struct Autoscaler {
    name: String,
    minimum: u64,
    maximum: u64,
}

pub struct Autoscaling {
    targets: BTreeMap<String, Autoscaler>,
}

impl Autoscaling {
    pub fn validate(resources: &[Value]) -> Result<()> {
        Self::from_resources(resources, crate::kubernetes::DEPLOYMENTS).map(|_| ())
    }

    pub fn inspect(target: &Target, resources: &[Value], deployments: &[&str]) -> Result<Self> {
        let live = target.json(&["get", "hpa"])?;
        let mut live = live["items"]
            .as_array()
            .context("autoscaler list missing")?
            .clone();
        for resource in &mut live {
            resource["kind"] = "HorizontalPodAutoscaler".into();
        }
        // Validate both configurations before any maintenance or replica changes.
        Self::from_resources(&live, deployments)?;
        Self::from_resources(resources, deployments)?;
        let mut effective = BTreeMap::new();
        for resource in live.iter().chain(resources.iter()) {
            if resource["kind"] == "HorizontalPodAutoscaler" {
                let name = resource["metadata"]["name"]
                    .as_str()
                    .context("autoscaler name missing")?;
                effective.insert(name, resource.clone());
            }
        }
        Self::from_resources(&effective.into_values().collect::<Vec<_>>(), deployments)
    }

    fn from_resources(resources: &[Value], deployments: &[&str]) -> Result<Self> {
        let mut targets = BTreeMap::new();
        for resource in resources {
            let spec = &resource["spec"];
            let reference = &spec["scaleTargetRef"];
            let Some(deployment) = reference["name"].as_str() else {
                continue;
            };
            if resource["kind"] != "HorizontalPodAutoscaler"
                || reference["apiVersion"] != "apps/v1"
                || reference["kind"] != "Deployment"
                || !deployments.contains(&deployment)
            {
                continue;
            }
            let minimum = if spec["minReplicas"].is_null() {
                1
            } else {
                spec["minReplicas"]
                    .as_u64()
                    .context("invalid HPA minReplicas")?
            };
            let maximum = spec["maxReplicas"]
                .as_u64()
                .context("invalid HPA maxReplicas")?;
            ensure!(
                minimum > 0,
                "HPA minReplicas must be positive for native maintenance deactivation"
            );
            ensure!(maximum >= minimum, "HPA maxReplicas is below minReplicas");
            ensure!(
                deployment != "hirune-edge-valkey",
                "Valkey cannot use independent autoscaled replicas"
            );
            let scaler = Autoscaler {
                name: resource["metadata"]["name"]
                    .as_str()
                    .context("autoscaler name missing")?
                    .into(),
                minimum,
                maximum,
            };
            ensure!(
                targets.insert(deployment.into(), scaler).is_none(),
                "multiple HPAs target one Deployment"
            );
        }
        Ok(Self { targets })
    }

    pub fn apply(&self, target: &Target, mut resources: Vec<Value>) -> Result<()> {
        let mut resume = Vec::new();
        for resource in &mut resources {
            if resource["kind"] != "Deployment" {
                continue;
            }
            let name = resource["metadata"]["name"]
                .as_str()
                .context("Deployment name missing")?;
            let Some(scaler) = self.targets.get(name) else {
                continue;
            };
            let existing = deployment(target, name)?;
            if let Some(existing) = &existing
                && let Some(annotation) = existing["metadata"]["annotations"][LAST_APPLIED].as_str()
            {
                let mut previous: Value =
                    serde_json::from_str(annotation).context("invalid last-applied Deployment")?;
                // Transfer client-side apply ownership without resetting the live scale to one.
                if previous["spec"]
                    .as_object_mut()
                    .context("last-applied spec missing")?
                    .remove("replicas")
                    .is_some()
                {
                    patch(
                        target,
                        name,
                        json!({"metadata":{"annotations":{LAST_APPLIED:previous.to_string()}}}),
                    )?;
                }
            }
            if existing
                .as_ref()
                .is_none_or(|value| value["spec"]["replicas"] == 0)
            {
                resume.push((name.to_owned(), scaler.resume_replicas(existing.as_ref())?));
            }
            resource["spec"]
                .as_object_mut()
                .context("Deployment spec missing")?
                .remove("replicas");
        }
        crate::kubernetes::apply(target, resources)?;
        for (name, count) in resume {
            crate::run(&mut target.kubectl(&[
                "scale",
                "deployment",
                &name,
                &format!("--replicas={count}"),
            ]))?;
        }
        Ok(())
    }

    pub fn remember(&self, target: &Target) -> Result<()> {
        for name in self.targets.keys() {
            if let Some(existing) = deployment(target, name)? {
                let replicas = existing["spec"]["replicas"]
                    .as_u64()
                    .context("Deployment replicas missing")?;
                if replicas > 0 {
                    patch(
                        target,
                        name,
                        json!({"metadata":{"annotations":{RESUME_REPLICAS:replicas.to_string()}}}),
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn wait_stopped(&self, target: &Target) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let mut stopped = true;
            for (name, scaler) in &self.targets {
                let Some(existing) = deployment(target, name)? else {
                    continue;
                };
                ensure!(
                    existing["spec"]["replicas"] == 0,
                    "HPA target resumed before migration"
                );
                let hpa = target.json(&["get", "hpa", &scaler.name])?;
                ensure!(
                    hpa["spec"]["minReplicas"].as_u64().unwrap_or(1) > 0,
                    "HPA changed during drain"
                );
                stopped &= hpa["status"]["conditions"]
                    .as_array()
                    .is_some_and(|conditions| {
                        conditions.iter().any(|condition| {
                            condition["type"] == "ScalingActive"
                                && condition["status"] == "False"
                                && condition["reason"] == "ScalingDisabled"
                        })
                    });
            }
            if stopped {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "HPA maintenance deactivation was not observed"
            );
            thread::sleep(Duration::from_secs(2));
        }
    }
}

impl Autoscaler {
    fn resume_replicas(&self, deployment: Option<&Value>) -> Result<u64> {
        let saved =
            deployment.and_then(|value| value["metadata"]["annotations"].get(RESUME_REPLICAS));
        let count = match saved {
            Some(value) => value
                .as_str()
                .context("invalid saved replica count")?
                .parse::<u64>()
                .context("invalid saved replica count")?,
            None => self.minimum,
        };
        ensure!(count > 0, "saved replica count must be positive");
        Ok(count.clamp(self.minimum, self.maximum))
    }
}

fn deployment(target: &Target, name: &str) -> Result<Option<Value>> {
    let output = target
        .kubectl(&[
            "get",
            "deployment",
            name,
            "--ignore-not-found",
            "-o",
            "json",
        ])
        .output()?;
    ensure!(
        output.status.success(),
        "cannot inspect autoscaled Deployment"
    );
    if output.stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::from_slice(&output.stdout)?))
    }
}

fn patch(target: &Target, name: &str, value: Value) -> Result<()> {
    use std::{io::Write, process::Stdio};
    // Last-applied configuration may contain sensitive environment values.
    let mut child = target
        .kubectl(&[
            "patch",
            "deployment",
            name,
            "--type=merge",
            "--patch-file=/dev/stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .context("kubectl stdin missing")?
        .write_all(&serde_json::to_vec(&value)?)?;
    ensure!(
        child.wait_with_output()?.status.success(),
        "autoscaling metadata update failed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_maintenance_requires_positive_minimum_only_for_selected_targets() {
        let mut hpa = json!({"kind":"HorizontalPodAutoscaler","metadata":{"name":"api"},
            "spec":{"scaleTargetRef":{"apiVersion":"apps/v1","kind":"Deployment","name":"hirune-api"},"minReplicas":0,"maxReplicas":4}});
        assert!(Autoscaling::from_resources(&[hpa.clone()], &["hirune-api"]).is_err());
        assert!(
            Autoscaling::from_resources(&[hpa.clone()], &["hirune-worker"])
                .unwrap()
                .targets
                .is_empty()
        );
        hpa["spec"].as_object_mut().unwrap().remove("minReplicas");
        assert_eq!(
            Autoscaling::from_resources(&[hpa.clone()], &["hirune-api"])
                .unwrap()
                .targets["hirune-api"]
                .minimum,
            1
        );
        assert!(Autoscaling::from_resources(&[hpa.clone(), hpa], &["hirune-api"]).is_err());
    }

    #[test]
    fn recovery_uses_durable_positive_counts_with_current_hpa_bounds() {
        let scaler = Autoscaler {
            name: "api".into(),
            minimum: 2,
            maximum: 4,
        };
        assert_eq!(scaler.resume_replicas(None).unwrap(), 2);
        for (saved, expected) in [("1", 2), ("3", 3), ("8", 4)] {
            assert_eq!(
                scaler
                    .resume_replicas(Some(
                        &json!({"metadata":{"annotations":{RESUME_REPLICAS:saved}}})
                    ))
                    .unwrap(),
                expected
            );
        }
        for saved in ["0", "invalid"] {
            assert!(
                scaler
                    .resume_replicas(Some(
                        &json!({"metadata":{"annotations":{RESUME_REPLICAS:saved}}})
                    ))
                    .is_err()
            );
        }
    }
}
