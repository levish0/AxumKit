use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy)]
pub enum Environment {
    Dev,
    Prod,
}

impl Environment {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "dev" => Ok(Self::Dev),
            "prod" => Ok(Self::Prod),
            _ => bail!("deployment environment must be dev or prod"),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Prod => "prod",
        }
    }
}

pub struct Target {
    pub context: String,
    pub namespace: String,
    pub kubeconfig: Option<PathBuf>,
}

impl Target {
    pub fn load(directory: &Path) -> Result<Self> {
        let entries: BTreeMap<_, _> = dotenvy::from_path_iter(directory.join("target.env"))
            .context("copy the dev or prod example into .envs/.<environment>/k3s first")?
            .collect::<std::result::Result<_, _>>()?;
        ensure!(
            entries.len() == 2
                && entries.contains_key("KUBE_CONTEXT")
                && entries.contains_key("KUBE_NAMESPACE"),
            "target.env must contain only KUBE_CONTEXT and KUBE_NAMESPACE"
        );
        let context = entries["KUBE_CONTEXT"].clone();
        let namespace = entries["KUBE_NAMESPACE"].clone();
        ensure!(
            !context.trim().is_empty() && !context.starts_with('-'),
            "KUBE_CONTEXT must select an explicit context"
        );
        ensure!(
            !namespace.is_empty()
                && namespace.len() <= 63
                && namespace
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
                && !namespace.starts_with('-')
                && !namespace.ends_with('-'),
            "KUBE_NAMESPACE must be a Kubernetes namespace name"
        );
        Ok(Self {
            context,
            namespace,
            kubeconfig: None,
        })
    }

    pub fn kubectl(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new("kubectl");
        if let Some(path) = &self.kubeconfig {
            command.arg("--kubeconfig").arg(path);
        }
        command
            .args(["--context", &self.context, "--namespace", &self.namespace])
            .args(arguments);
        command
    }

    pub fn json(&self, arguments: &[&str]) -> Result<Value> {
        let output = self.kubectl(arguments).arg("--output=json").output()?;
        ensure!(
            output.status.success(),
            "kubectl read failed for selected target"
        );
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}

pub struct Deployment {
    pub target: Target,
    pub resources: Vec<Value>,
}

impl Deployment {
    pub fn load(root: &Path, environment: Environment) -> Result<Self> {
        let directory = root.join(format!(".envs/.{}/k3s", environment.name()));
        let target = Target::load(&directory)?;
        let output = Command::new("kubectl")
            .arg("kustomize")
            .arg(&directory)
            .output()?;
        // Kustomize diagnostics can contain Secret source data. Keep it out of logs.
        ensure!(
            output.status.success(),
            "cannot render the selected Kubernetes environment"
        );
        let resources = serde_yaml_ng::Deserializer::from_slice(&output.stdout)
            .map(|document| {
                Value::deserialize(document).map_err(|_| anyhow::anyhow!("invalid Kubernetes YAML"))
            })
            .collect::<Result<Vec<_>>>()?;
        let deployment = Self { target, resources };
        deployment.validate()?;
        Ok(deployment)
    }

    pub fn validate(&self) -> Result<()> {
        crate::kubernetes_autoscaling::Autoscaling::validate(&self.resources)?;
        let namespaces: Vec<_> = self
            .resources
            .iter()
            .filter(|r| r["kind"] == "Namespace")
            .collect();
        ensure!(
            namespaces.len() == 1,
            "deployment must define exactly one namespace"
        );
        ensure!(
            namespaces[0]["metadata"]["name"] == self.target.namespace,
            "rendered namespace does not match KUBE_NAMESPACE"
        );
        for resource in &self.resources {
            if resource["kind"] != "Namespace" {
                ensure!(
                    resource["metadata"]["namespace"] == self.target.namespace,
                    "resource is outside the selected namespace"
                );
            }
        }
        for (kind, name) in [
            ("Job", "hirune-migration"),
            ("Job", "hirune-migration-check"),
            ("ConfigMap", "hirune-gateway-maintenance"),
        ]
        .into_iter()
        .chain(
            crate::kubernetes::DEPLOYMENTS
                .iter()
                .map(|name| ("Deployment", *name)),
        ) {
            ensure!(
                self.resources
                    .iter()
                    .filter(|r| r["kind"] == kind && r["metadata"]["name"] == name)
                    .count()
                    == 1,
                "deployment requires exactly one {kind}/{name}"
            );
        }
        let counter = self
            .resources
            .iter()
            .find(|r| r["kind"] == "Deployment" && r["metadata"]["name"] == "hirune-edge-valkey")
            .context("edge counter Deployment missing")?;
        ensure!(
            counter["spec"]["replicas"] == 1 && counter["spec"]["strategy"]["type"] == "Recreate",
            "the disposable shared Valkey requires one replica and Recreate"
        );
        ensure!(
            !self
                .resources
                .iter()
                .any(|r| r["kind"] == "HorizontalPodAutoscaler"
                    && r["spec"]["scaleTargetRef"]["name"] == "hirune-edge-valkey"),
            "independent Valkey replicas cannot share one counter Service"
        );
        let settings = self
            .resources
            .iter()
            .find(|r| {
                r["kind"] == "ConfigMap"
                    && r["metadata"]["name"]
                        .as_str()
                        .is_some_and(|name| name.starts_with("hirune-edge-settings-"))
            })
            .context("gateway settings missing")?;
        ensure!(
            settings["data"]["EDGE_RATE_LIMIT_REQUESTS"]
                .as_str()
                .and_then(|value| value.parse::<u32>().ok())
                .is_some_and(|value| value > 0),
            "EDGE_RATE_LIMIT_REQUESTS must be a positive integer"
        );
        Ok(())
    }
}

pub fn copy_directory(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}
