use super::{Fixture, NAMESPACE};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    fs, thread,
    time::{Duration, Instant},
};

impl Fixture {
    pub(super) fn enable_autoscaling(&self) -> Result<Value> {
        let profile = self.directory.path().join(".envs/.prod/k3s");
        let mut hpa: Value =
            serde_yaml_ng::from_str(include_str!("../../../deploy/k3s/autoscaling.yaml"))?;
        hpa["metadata"]["namespace"] = NAMESPACE.into();
        hpa["spec"]["minReplicas"] = 3.into();
        hpa["spec"]["maxReplicas"] = 3.into();
        fs::write(
            profile.join("autoscaling.yaml"),
            serde_yaml_ng::to_string(&hpa)?,
        )?;
        self.apply(hpa)?;
        self.wait_autoscaling(true)?;
        let hpa = self.json(&["get", "hpa", "hirune-api"])?;
        let pods = self.api_pods()?;
        let path = profile.join("kustomization.yaml");
        let mut kustomization: Value = serde_yaml_ng::from_str(&fs::read_to_string(&path)?)?;
        kustomization["resources"]
            .as_array_mut()
            .context("resources missing")?
            .push("autoscaling.yaml".into());
        fs::write(&path, serde_yaml_ng::to_string(&kustomization)?)?;
        self.probe("start-traffic")?;
        self.deploy("auto")?;
        self.assert_autoscaling(&hpa, true)?;
        ensure!(
            self.api_pods()?.iter().all(|pod| pods
                .iter()
                .any(|old| old["metadata"]["uid"] == pod["metadata"]["uid"])),
            "HPA ownership transfer replaced API Pods"
        );
        let api = self.json(&["get", "deployment", "hirune-api"])?;
        let previous: Value = serde_json::from_str(
            api["metadata"]["annotations"]["kubectl.kubernetes.io/last-applied-configuration"]
                .as_str()
                .context("last-applied missing")?,
        )?;
        ensure!(
            previous["spec"]["replicas"].is_null(),
            "apply still owns autoscaled replicas"
        );
        println!("{}", self.probe("stop-traffic")?.trim());
        // Drain and cold-start recovery also exercise omitted rendered replicas.
        kustomization["patches"]
            .as_array_mut()
            .context("patches missing")?
            .push(json!({
            "target":{"kind":"Deployment","name":"hirune-api"},
            "patch":"- op: remove\n  path: /spec/replicas\n"}));
        fs::write(path, serde_yaml_ng::to_string(&kustomization)?)?;
        let mut invalid = self.deployment()?;
        invalid
            .resources
            .iter_mut()
            .find(|r| r["kind"] == "HorizontalPodAutoscaler")
            .context("HPA missing")?["spec"]["minReplicas"] = 0.into();
        let error = crate::kubernetes::deploy(&invalid, crate::kubernetes::Mode::Auto)
            .err()
            .context("unsafe zero-minimum HPA accepted")?;
        ensure!(
            error.to_string().contains("minReplicas"),
            "unexpected HPA validation error: {error}"
        );
        self.assert_autoscaling(&hpa, true)?;
        ensure!(
            !crate::kubernetes_maintenance::active(
                crate::kubernetes_maintenance::state(&invalid.target)?.as_ref()
            ),
            "unsafe HPA changed maintenance"
        );
        println!(
            "Real metrics-backed HPA scaled API to three; rolling preserved capacity and transferred replica ownership without replacing Pods."
        );
        Ok(hpa)
    }

    fn assert_autoscaling(&self, original: &Value, active: bool) -> Result<()> {
        let hpa = self.json(&["get", "hpa", "hirune-api"])?;
        ensure!(
            hpa["metadata"]["uid"] == original["metadata"]["uid"]
                && hpa["spec"] == original["spec"],
            "deployment changed HPA identity or settings"
        );
        let api = self.json(&["get", "deployment", "hirune-api"])?;
        ensure!(
            api["spec"]["replicas"] == if active { 3 } else { 0 },
            "unexpected HPA target replica count"
        );
        if !active {
            ensure!(
                self.api_pods()?.is_empty(),
                "API Pods remained during migration maintenance"
            );
            ensure!(
                api["metadata"]["annotations"]["hirune.axum-kit/resume-replicas"] == "3",
                "saved HPA replica count was lost"
            );
        }
        Ok(())
    }

    fn wait_autoscaling(&self, active: bool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let hpa = self.json(&["get", "hpa", "hirune-api"])?;
            let api = self.json(&["get", "deployment", "hirune-api"])?;
            let observed = hpa["status"]["conditions"]
                .as_array()
                .is_some_and(|conditions| {
                    conditions.iter().any(|c| {
                        c["type"] == "ScalingActive"
                            && c["status"] == if active { "True" } else { "False" }
                            && (active || c["reason"] == "ScalingDisabled")
                    })
                });
            if observed
                && api["spec"]["replicas"] == if active { 3 } else { 0 }
                && (!active || api["status"]["availableReplicas"] == 3)
            {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "real HPA controller did not reach expected state"
            );
            thread::sleep(Duration::from_secs(2));
        }
    }

    pub(super) fn check_autoscaled_migration_recovery(&self, autoscaler: &Value) -> Result<()> {
        let mut failing = self.deployment()?;
        let job = failing
            .resources
            .iter_mut()
            .find(|r| r["kind"] == "Job" && r["metadata"]["name"] == "hirune-migration")
            .context("migration missing")?;
        job["spec"]["template"]["spec"]["containers"][0]["command"] =
            json!(["sh", "-c", "migration && exit 1"]);
        let error = crate::kubernetes::deploy(&failing, crate::kubernetes::Mode::Drain)
            .err()
            .context("HPA migration failure was ignored")?;
        ensure!(
            error.to_string().contains("hirune-migration failed"),
            "unexpected HPA migration failure: {error}"
        );
        self.wait_autoscaling(false)?;
        let deadline = Instant::now() + Duration::from_secs(35);
        while Instant::now() < deadline {
            self.assert_autoscaling(autoscaler, false)?;
            thread::sleep(Duration::from_secs(2));
        }
        println!("{}", self.probe("maintenance")?.trim());
        self.deploy("auto")?;
        self.wait_autoscaling(true)?;
        self.assert_autoscaling(autoscaler, true)?;
        println!(
            "HPA stayed disabled across controller reconciliations after migration failure; automatic retry restored three replicas with the same HPA UID and settings."
        );
        Ok(())
    }

    pub(super) fn check_cold_start(&self, autoscaler: &Value) -> Result<()> {
        // The persistent test client must not join the runtime deletion selector.
        crate::run(&mut self.kubectl(&[
            "label",
            "pod/trusted-probe",
            "app.kubernetes.io/part-of-",
        ]))?;
        let stopped =
            crate::kubernetes::stop(&self.deployment()?.target, crate::kubernetes::DEPLOYMENTS);
        let repeated = stopped.and_then(|()| {
            self.assert_autoscaling(autoscaler, false)?;
            crate::kubernetes::stop(&self.deployment()?.target, crate::kubernetes::DEPLOYMENTS)
        });
        let restored = crate::run(&mut self.kubectl(&[
            "label",
            "pod/trusted-probe",
            "app.kubernetes.io/part-of=hirune-kit",
        ]));
        repeated?;
        restored?;
        self.deploy("auto")?;
        self.wait_autoscaling(true)?;
        self.assert_autoscaling(autoscaler, true)?;
        // The live controller must still restore the minimum after release recovery.
        crate::run(&mut self.kubectl(&["scale", "deployment/hirune-api", "--replicas=2"]))?;
        self.wait_autoscaling(true)?;
        self.assert_autoscaling(autoscaler, true)?;
        println!("{}", self.probe("retained-authority")?.trim());
        println!(
            "Automatic cold-start recovery preserved HPA identity and settings after repeated down; the controller resumed scaling."
        );
        Ok(())
    }
}
