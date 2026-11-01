use crate::kubernetes_environment::Target;
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{
    thread,
    time::{Duration, Instant},
};

fn terminal(job: &Value, condition: &str) -> bool {
    job["status"]["conditions"]
        .as_array()
        .is_some_and(|conditions| {
            conditions
                .iter()
                .any(|value| value["type"] == condition && value["status"] == "True")
        })
}

pub fn inspect(target: &Target, name: &str) -> Result<Option<Value>> {
    let output = target
        .kubectl(&["get", "job", name, "--ignore-not-found", "-o", "json"])
        .output()?;
    ensure!(output.status.success(), "cannot inspect deployment Job");
    if output.stdout.is_empty() {
        return Ok(None);
    }
    let job: Value = serde_json::from_slice(&output.stdout)?;
    ensure!(
        terminal(&job, "Complete") || terminal(&job, "Failed"),
        "{name} Job is unfinished; inspect it before another deployment"
    );
    Ok(Some(job))
}

pub fn run(target: &Target, job: Value, timeout: Duration) -> Result<Value> {
    let name = job["metadata"]["name"]
        .as_str()
        .context("Job name missing")?
        .to_owned();
    if inspect(target, &name)?.is_some() {
        crate::run(&mut target.kubectl(&["delete", "job", &name, "--wait=true"]))?;
    }
    crate::kubernetes::apply(target, vec![job])?;
    let created = target.json(&["get", "job", &name])?;
    let uid = created["metadata"]["uid"]
        .as_str()
        .context("Job identity missing")?;
    let deadline = Instant::now() + timeout;
    loop {
        let current = target.json(&["get", "job", &name])?;
        ensure!(
            current["metadata"]["uid"] == uid,
            "deployment Job changed; serialize releases"
        );
        ensure!(
            !terminal(&current, "Failed") && !terminal(&current, "FailureTarget"),
            "{name} failed; inspect the selected Job logs"
        );
        if terminal(&current, "Complete") {
            return Ok(current);
        }
        ensure!(
            Instant::now() < deadline,
            "{name} did not complete before its deadline"
        );
        thread::sleep(Duration::from_secs(2));
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationReport {
    pending_migrations: u64,
}

pub fn pending_count(target: &Target, job: &Value) -> Result<u64> {
    let uid = job["metadata"]["uid"]
        .as_str()
        .context("check Job identity missing")?;
    let pods = target.json(&[
        "get",
        "pods",
        "-l",
        &format!("batch.kubernetes.io/controller-uid={uid}"),
    ])?;
    let reports: Vec<_> = pods["items"]
        .as_array()
        .context("check Pod list missing")?
        .iter()
        .filter(|pod| {
            pod["status"]["phase"] == "Succeeded"
                && pod["metadata"]["ownerReferences"]
                    .as_array()
                    .is_some_and(|owners| {
                        owners
                            .iter()
                            .any(|owner| owner["uid"] == uid && owner["controller"] == true)
                    })
        })
        .filter_map(|pod| pod["status"]["containerStatuses"].as_array())
        .flatten()
        .filter(|container| {
            container["name"] == "migration-check"
                && container["state"]["terminated"]["exitCode"] == 0
        })
        .filter_map(|container| container["state"]["terminated"]["message"].as_str())
        .collect();
    ensure!(
        reports.len() == 1,
        "migration check must return exactly one successful report"
    );
    let report: MigrationReport =
        serde_json::from_str(reports[0]).context("invalid migration check report")?;
    Ok(report.pending_migrations)
}
