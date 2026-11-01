use crate::kubernetes_environment::Target;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    thread,
    time::{Duration, Instant},
};

pub const CONFIG_MAP: &str = "hirune-gateway-maintenance";
const GATEWAY_SELECTOR: &str =
    "app.kubernetes.io/part-of=hirune-kit,app.kubernetes.io/component=gateway";

pub fn state(target: &Target) -> Result<Option<Value>> {
    let result = target
        .kubectl(&[
            "get",
            "configmap",
            CONFIG_MAP,
            "--ignore-not-found",
            "-o",
            "json",
        ])
        .output()?;
    ensure!(
        result.status.success(),
        "cannot inspect gateway maintenance state"
    );
    if result.stdout.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&result.stdout)?))
}

pub fn active(state: Option<&Value>) -> bool {
    state.is_some_and(|value| value["data"]["state"] != "off\n")
}

pub fn initialize(target: &Target, enabled: bool) -> Result<()> {
    if state(target)?.is_none() {
        crate::kubernetes::apply(
            target,
            vec![json!({"apiVersion":"v1","kind":"ConfigMap",
            "metadata":{"name":CONFIG_MAP,"namespace":target.namespace},
            "data":{"state":if enabled {"on\n"} else {"off\n"}}})],
        )?;
    }
    Ok(())
}

pub fn request(target: &Target, enabled: bool) -> Result<()> {
    let patch = json!({"data":{"state":if enabled {"on\n"} else {"off\n"}}}).to_string();
    crate::run(&mut target.kubectl(&[
        "patch",
        "configmap",
        CONFIG_MAP,
        "--type=merge",
        "--patch",
        &patch,
    ]))
}

fn gateways(target: &Target) -> Result<Vec<String>> {
    let pods = target.json(&["get", "pods", "--selector", GATEWAY_SELECTOR])?;
    pods["items"]
        .as_array()
        .context("gateway Pod list missing")?
        .iter()
        .filter(|pod| {
            !matches!(
                pod["status"]["phase"].as_str(),
                Some("Succeeded" | "Failed")
            )
        })
        .map(|pod| {
            pod["metadata"]["name"]
                .as_str()
                .map(str::to_owned)
                .context("gateway Pod name missing")
        })
        .collect()
}

fn response(target: &Target, pod: &str, path: &str) -> Result<(u16, Value)> {
    // APISIX ships Bash and coreutils. HTTP/1.0 keeps the response unchunked.
    let output = target.kubectl(&["exec", pod, "-c", "apisix", "--", "timeout", "5", "bash", "-c",
        "exec 3<>/dev/tcp/127.0.0.1/9080; printf 'GET %s HTTP/1.0\\r\\nHost: localhost\\r\\nConnection: close\\r\\n\\r\\n' \"$1\" >&3; cat <&3",
        "gateway-probe", path]).output()?;
    ensure!(output.status.success(), "gateway HTTP probe failed");
    let text = std::str::from_utf8(&output.stdout)?;
    let (headers, body) = text
        .split_once("\r\n\r\n")
        .context("invalid gateway HTTP response")?;
    let status = headers
        .split_whitespace()
        .nth(1)
        .context("gateway status missing")?
        .parse()?;
    Ok((
        status,
        if body.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(body)?
        },
    ))
}

pub fn verify_backend(target: &Target) -> Result<()> {
    let pods = gateways(target)?;
    ensure!(!pods.is_empty(), "no gateway Pods to verify");
    for pod in &pods {
        ensure!(
            response(target, pod, "/health/ready")?.0 == 204,
            "backend readiness failed through gateway; keep maintenance enabled"
        );
    }
    ensure!(
        gateways(target)? == pods,
        "gateway Pods changed during backend verification"
    );
    Ok(())
}

fn observed(target: &Target, enabled: bool) -> Result<bool> {
    let pods = gateways(target)?;
    if pods.is_empty() {
        return Ok(false);
    }
    for pod in &pods {
        let Ok((200, health)) = response(target, pod, "/health/gateway") else {
            return Ok(false);
        };
        if health["maintenance"] != enabled {
            return Ok(false);
        }
        if enabled {
            let Ok((503, body)) = response(target, pod, "/api/__maintenance_probe__") else {
                return Ok(false);
            };
            if body["error"]["code"] != "maintenance" {
                return Ok(false);
            }
        } else if !response(target, pod, "/health/ready").is_ok_and(|(status, _)| status == 204) {
            return Ok(false);
        }
    }
    // Include Pods that appeared while probes were running.
    Ok(gateways(target)? == pods)
}

fn wait(target: &Target, enabled: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        if observed(target, enabled)? {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "maintenance propagation was not verified on every gateway"
        );
        thread::sleep(Duration::from_secs(2));
    }
}

pub fn set(target: &Target, enabled: bool) -> Result<()> {
    if !enabled {
        verify_backend(target)?;
    }
    let result = request(target, enabled).and_then(|()| wait(target, enabled));
    if let Err(error) = result {
        if !enabled && let Err(recovery) = request(target, true).and_then(|()| wait(target, true)) {
            return Err(error.context(format!(
                "could not restore and verify gateway maintenance: {recovery}"
            )));
        }
        return Err(error);
    }
    Ok(())
}

pub fn command(target: &Target, action: &str) -> Result<()> {
    match action {
        "status" => {
            let state = state(target)?;
            let label = match state.as_ref() {
                None => "uninitialized",
                Some(value) if active(Some(value)) => "on",
                Some(_) => "off",
            };
            println!("Gateway maintenance: {label}");
        }
        "on" | "off" => {
            initialize(target, true)?;
            set(target, action == "on")?;
        }
        _ => bail!("maintenance action must be on, off, or status"),
    }
    Ok(())
}
