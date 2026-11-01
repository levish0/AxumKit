use crate::kubernetes_autoscaling::Autoscaling;
use crate::kubernetes_environment::{Deployment, Environment, Target};
use crate::{kubernetes_jobs as jobs, kubernetes_maintenance as maintenance};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::time::Duration;
use std::{io::Write, path::Path, process::Stdio};

pub const APPLICATIONS: &[&str] = &["hirune-api", "hirune-worker", "hirune-maintenance"];
pub const DEPLOYMENTS: &[&str] = &[
    "hirune-api",
    "hirune-worker",
    "hirune-maintenance",
    "hirune-apisix",
    "hirune-cloudflared",
    "hirune-edge-valkey",
];

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Auto,
    Rolling,
    Drain,
}

impl Mode {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "rolling" => Ok(Self::Rolling),
            "drain" => Ok(Self::Drain),
            _ => bail!("deployment mode must be auto, rolling, or drain"),
        }
    }

    fn resolve(self, pending: u64, active: bool, running: bool) -> Result<Self> {
        let offline = pending > 0 || active || !running;
        match self {
            Self::Auto => Ok(if offline { Self::Drain } else { Self::Rolling }),
            Self::Rolling if offline => bail!(
                "rolling requires no pending migrations, no active maintenance, and running applications; use auto or drain"
            ),
            mode => Ok(mode),
        }
    }
}

pub fn from_arguments() -> Result<()> {
    let arguments: Vec<_> = std::env::args().skip(2).collect();
    let action = arguments
        .first()
        .context("select check, up, down, status, logs, manage, cleanup, or maintenance")?;
    let environment =
        Environment::parse(arguments.get(1).context("select dev or prod explicitly")?)?;
    let extra: Vec<_> = arguments.iter().skip(2).map(String::as_str).collect();
    if matches!(action.as_str(), "check" | "up") {
        let deployment = Deployment::load(Path::new("."), environment)?;
        println!(
            "Target: context={}, namespace={}",
            deployment.target.context, deployment.target.namespace
        );
        match action.as_str() {
            "check" => {
                ensure!(extra.is_empty(), "check takes only an environment");
                println!(
                    "Kubernetes environment rendered and namespace validation passed. Secret values are not displayed."
                );
                Ok(())
            }
            _ => {
                ensure!(
                    extra.len() <= 1,
                    "up takes an environment and optional auto, rolling, or drain mode"
                );
                deploy(
                    &deployment,
                    Mode::parse(extra.first().copied().unwrap_or("auto"))?,
                )
            }
        }
    } else {
        let deployment = Deployment::load(Path::new("."), environment)?;
        let target = deployment.target;
        println!(
            "Target: context={}, namespace={}",
            target.context, target.namespace
        );
        match action.as_str() {
            "maintenance" => {
                ensure!(
                    extra.len() <= 1,
                    "maintenance takes an environment and optional on, off, or status"
                );
                maintenance::command(&target, extra.first().copied().unwrap_or("status"))
            }
            "down" => {
                ensure!(extra.is_empty(), "down takes only an environment");
                stop(&target, DEPLOYMENTS)
            }
            "status" => {
                ensure!(extra.is_empty(), "status takes only an environment");
                crate::run(&mut target.kubectl(&["get", "deployment,pod,service,job,hpa,pdb"]))
            }
            "logs" => {
                ensure!(
                    extra.len() == 1 && DEPLOYMENTS.contains(&extra[0]),
                    "logs requires a hirune Deployment name"
                );
                crate::run(&mut target.kubectl(&[
                    "logs",
                    "--follow",
                    &format!("deployment/{}", extra[0]),
                    "--all-pods=true",
                    "--all-containers=true",
                ]))
            }
            "manage" => {
                ensure!(!extra.is_empty(), "manage requires a management command");
                crate::run(
                    target
                        .kubectl(&["exec", "deployment/hirune-api", "--", "manage"])
                        .args(extra),
                )
            }
            "cleanup" => {
                ensure!(extra.is_empty(), "cleanup takes only an environment");
                crate::run(&mut target.kubectl(&[
                    "exec",
                    "deployment/hirune-maintenance",
                    "--",
                    "manage",
                    "cleanup",
                ]))
            }
            _ => bail!("unknown Kubernetes deployment action"),
        }
    }
}

pub fn apply(target: &Target, resources: Vec<Value>) -> Result<()> {
    if resources.is_empty() {
        return Ok(());
    }
    // Render once. Send Secret values only on stdin, without printing YAML.
    let mut child = target
        .kubectl(&["apply", "-f", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .context("kubectl stdin missing")?
        .write_all(&serde_json::to_vec(
            &json!({"apiVersion":"v1","kind":"List","items":resources}),
        )?)?;
    ensure!(
        child.wait_with_output()?.status.success(),
        "resource apply failed for selected target"
    );
    Ok(())
}

pub fn deploy(deployment: &Deployment, mode: Mode) -> Result<()> {
    deployment.validate()?;
    let target = &deployment.target;
    let select = |kind: &str| {
        deployment
            .resources
            .iter()
            .filter(|r| r["kind"] == kind)
            .cloned()
            .collect()
    };
    apply(target, select("Namespace"))?;
    jobs::inspect(target, "hirune-migration")?;
    jobs::inspect(target, "hirune-migration-check")?;
    let check = resource(deployment, "Job", "hirune-migration-check")?.clone();
    let secret = &check["spec"]["template"]["spec"]["containers"][0]["env"][0]["valueFrom"]["secretKeyRef"]
        ["name"];
    let credentials: Vec<_> = deployment
        .resources
        .iter()
        .filter(|r| r["kind"] == "Secret" && &r["metadata"]["name"] == secret)
        .cloned()
        .collect();
    ensure!(
        credentials.len() == 1,
        "migration check credentials missing"
    );
    apply(target, credentials)?;
    let check = jobs::run(target, check, Duration::from_secs(270))?;
    let pending = jobs::pending_count(target, &check)?;
    let state = maintenance::state(target)?;
    let applications = target.json(&[
        "get",
        "deployments",
        "-l",
        "app.kubernetes.io/part-of=hirune-kit",
    ])?;
    let running = APPLICATIONS.iter().all(|name| {
        applications["items"].as_array().is_some_and(|items| {
            items.iter().any(|item| {
                item["metadata"]["name"] == *name
                    && item["spec"]["replicas"]
                        .as_u64()
                        .is_some_and(|count| count > 0)
            })
        })
    });
    let mode = mode.resolve(pending, maintenance::active(state.as_ref()), running)?;
    println!(
        "Migration check: {pending} pending; deployment mode: {}.",
        if mode == Mode::Drain {
            "drain"
        } else {
            "rolling"
        }
    );
    let autoscaling = Autoscaling::inspect(target, &deployment.resources, DEPLOYMENTS)?;
    apply(
        target,
        deployment
            .resources
            .iter()
            .filter(|r| {
                !matches!(r["kind"].as_str(), Some("Namespace" | "Job" | "Deployment"))
                    && r["metadata"]["name"] != maintenance::CONFIG_MAP
            })
            .cloned()
            .collect(),
    )?;
    maintenance::initialize(target, mode == Mode::Drain)?;
    if mode == Mode::Drain {
        maintenance::request(target, true)?;
    }
    // Upgrade old gateways to the maintenance-aware configuration before stopping apps.
    autoscaling.apply(
        target,
        vec![resource(deployment, "Deployment", "hirune-apisix")?.clone()],
    )?;
    rollout(target, "hirune-apisix")?;
    if mode == Mode::Drain {
        maintenance::set(target, true)?;
        stop(target, APPLICATIONS)?;
    }
    jobs::run(
        target,
        resource(deployment, "Job", "hirune-migration")?.clone(),
        Duration::from_secs(630),
    )?;
    autoscaling.apply(
        target,
        deployment
            .resources
            .iter()
            .filter(|r| r["kind"] == "Deployment" && r["metadata"]["name"] == "hirune-edge-valkey")
            .cloned()
            .collect(),
    )?;
    crate::run(&mut target.kubectl(&[
        "rollout",
        "status",
        "deployment/hirune-edge-valkey",
        "--timeout=300s",
    ]))?;
    autoscaling.apply(
        target,
        deployment
            .resources
            .iter()
            .filter(|r| r["kind"] == "Deployment" && r["metadata"]["name"] != "hirune-edge-valkey")
            .cloned()
            .collect(),
    )?;
    for name in DEPLOYMENTS {
        rollout(target, name)?;
    }
    if mode == Mode::Drain {
        maintenance::set(target, false)?;
    }
    Ok(())
}

fn resource<'a>(deployment: &'a Deployment, kind: &str, name: &str) -> Result<&'a Value> {
    deployment
        .resources
        .iter()
        .find(|r| r["kind"] == kind && r["metadata"]["name"] == name)
        .context("required deployment resource missing")
}

fn rollout(target: &Target, name: &str) -> Result<()> {
    crate::run(&mut target.kubectl(&[
        "rollout",
        "status",
        &format!("deployment/{name}"),
        "--timeout=300s",
    ]))
}

pub(crate) fn stop(target: &Target, deployments: &[&str]) -> Result<()> {
    let autoscaling = Autoscaling::inspect(target, &[], deployments)?;
    autoscaling.remember(target)?;
    for name in deployments {
        let existing = target
            .kubectl(&[
                "get",
                "deployment",
                name,
                "--ignore-not-found",
                "-o",
                "name",
            ])
            .output()?;
        ensure!(
            existing.status.success(),
            "cannot inspect application Deployment"
        );
        if !existing.stdout.is_empty() {
            crate::run(&mut target.kubectl(&["scale", "deployment", name, "--replicas=0"]))?;
        }
    }
    let selector = if deployments == APPLICATIONS {
        "app.kubernetes.io/part-of=hirune-kit,app.kubernetes.io/component in (api,worker,maintenance)"
    } else {
        "app.kubernetes.io/part-of=hirune-kit,app.kubernetes.io/component in (api,worker,maintenance,gateway,tunnel,edge-counter)"
    };
    crate::run(&mut target.kubectl(&[
        "wait",
        "--for=delete",
        "pod",
        "--selector",
        selector,
        "--timeout=300s",
    ]))?;
    autoscaling.wait_stopped(target)
}

#[cfg(test)]
mod tests {
    use super::Mode;

    #[test]
    fn automatic_releases_drain_for_schema_changes_recovery_and_cold_starts() {
        for (pending, active, running) in [(1, false, true), (0, true, true), (0, false, false)] {
            assert!(matches!(
                Mode::Auto.resolve(pending, active, running).unwrap(),
                Mode::Drain
            ));
            assert!(Mode::Rolling.resolve(pending, active, running).is_err());
        }
        assert!(matches!(
            Mode::Auto.resolve(0, false, true).unwrap(),
            Mode::Rolling
        ));
        assert!(matches!(
            Mode::Drain.resolve(0, false, true).unwrap(),
            Mode::Drain
        ));
    }
}
