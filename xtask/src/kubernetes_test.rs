use crate::kubernetes_environment::{Deployment, Environment, Target, copy_directory};
use anyhow::{Context, Result, ensure};
use base64::Engine;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    net::TcpListener,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;
use uuid::Uuid;

mod autoscaling;
mod database;
mod services;
mod workflows;

const NAMESPACE: &str = "hirune-prod";
const NODE_IMAGE: &str = "rancher/k3s:v1.37.1-k3s1";
const IMAGES: &[&str] = &[
    "apache/apisix:3.19.0-debian",
    "valkey/valkey:9.1.2-alpine",
    "postgres:18-alpine",
    "axllent/mailpit:v1.31.5",
    "python:3.14-alpine",
    "chrislusf/seaweedfs:4.48",
    "getmeili/meilisearch:v1.55.0",
    "caddy:2.11.7-alpine",
    "grafana/k6:2.2.0",
    "rancher/mirrored-pause:3.10.2",
    "rancher/klipper-helm:v0.13.3-build20260727",
];

pub(crate) fn configure_api_spread(profile: &Path) -> Result<()> {
    fs::write(
        profile.join("api-spread.yaml"),
        include_str!("../../deploy/k3s/verification/api-spread.yaml"),
    )?;
    let mut kustomization = fs::OpenOptions::new()
        .append(true)
        .open(profile.join("kustomization.yaml"))?;
    writeln!(
        kustomization,
        "\npatches:\n  - path: api-spread.yaml\n    target:\n      kind: Deployment\n      name: hirune-api"
    )?;
    Ok(())
}

pub fn verify() -> Result<()> {
    workflows::profile()?;
    let image = std::env::var("K3S_TEST_IMAGE")
        .context("set K3S_TEST_IMAGE to the canonical release application image")?;
    ensure!(
        Command::new("docker")
            .args(["image", "inspect", &image])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success(),
        "build the canonical release image before Kubernetes acceptance: {image} is not local"
    );
    let fixture = Fixture {
        cluster: format!("kit-{}", &Uuid::now_v7().simple().to_string()[..12]),
        directory: tempfile::tempdir()?,
    };
    let result = fixture.verify(&image);
    if result.is_err() {
        let _ = crate::run(&mut fixture.kubectl(&["get", "nodes", "-o", "wide"]));
        let _ = crate::run(&mut fixture.kubectl(&["get", "pods", "-o", "wide"]));
        let _ = crate::run(&mut fixture.kubectl(&[
            "get",
            "events",
            "--field-selector",
            "type=Warning",
        ]));
        for role in ["server", "agent"] {
            let node = format!("k3d-{}-{role}-0", fixture.cluster);
            if let Ok(logs) = Command::new("docker")
                .args(["logs", "--tail", "2000", &node])
                .output()
            {
                let logs = format!(
                    "{}{}",
                    String::from_utf8_lossy(&logs.stdout),
                    String::from_utf8_lossy(&logs.stderr)
                );
                let errors: Vec<_> = logs
                    .lines()
                    .filter(|line| {
                        line.contains("Aborting sync.") || line.contains("Extension physdev")
                    })
                    .collect();
                for line in errors.iter().rev().take(3) {
                    eprintln!("{node}: {line}");
                }
            }
        }
    }
    result?;
    println!(
        "Two-node k3s acceptance passed: deployed file/content/notification/search workflows and k6, retained attachments across replacement, two API replicas, live rolling updates, shared sessions and quota, two gateways sharing Valkey, counter failure and recovery, environment isolation, and private ingress policies. Real Cloudflare Tunnel was not connected."
    );
    Ok(())
}

struct Fixture {
    cluster: String,
    directory: TempDir,
}

impl Fixture {
    fn kubectl(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new("kubectl");
        command
            .arg("--kubeconfig")
            .arg(self.directory.path().join("kubeconfig"))
            .args([
                "--context",
                &format!("k3d-{}", self.cluster),
                "--namespace",
                NAMESPACE,
            ])
            .args(arguments);
        command
    }

    fn read(&self, arguments: &[&str]) -> Result<String> {
        let output = self.kubectl(arguments).output()?;
        ensure!(
            output.status.success(),
            "k3s fixture command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(String::from_utf8(output.stdout)?)
    }

    fn json(&self, arguments: &[&str]) -> Result<Value> {
        let mut command = self.kubectl(arguments);
        let output = command.arg("-o").arg("json").output()?;
        ensure!(output.status.success(), "k3s fixture read failed");
        Ok(serde_json::from_slice(&output.stdout)?)
    }

    fn apply(&self, value: Value) -> Result<()> {
        // Secret values go through stdin and never appear in arguments or output.
        let mut child = self
            .kubectl(&["apply", "-f", "-"])
            .stdin(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .context("kubectl stdin missing")?
            .write_all(&serde_json::to_vec(&value)?)?;
        ensure!(child.wait()?.success(), "fixture resource apply failed");
        Ok(())
    }

    fn probe(&self, operation: &str) -> Result<String> {
        self.read(&[
            "exec",
            "trusted-probe",
            "--",
            "python",
            "/checks/probe.py",
            operation,
        ])
    }

    fn deployment(&self) -> Result<Deployment> {
        let mut deployment = Deployment::load(self.directory.path(), Environment::Prod)?;
        deployment.target.kubeconfig = Some(self.directory.path().join("kubeconfig"));
        Ok(deployment)
    }

    fn deploy(&self, mode: &str) -> Result<()> {
        crate::kubernetes::deploy(&self.deployment()?, crate::kubernetes::Mode::parse(mode)?)
    }

    fn verify(&self, image: &str) -> Result<()> {
        for image in std::iter::once(NODE_IMAGE).chain(IMAGES.iter().copied()) {
            crate::run(Command::new("docker").args(["pull", image]))?;
        }
        let api_address = TcpListener::bind("127.0.0.1:0")?.local_addr()?.to_string();
        crate::run(Command::new("k3d").args([
            "cluster",
            "create",
            &self.cluster,
            "--image",
            NODE_IMAGE,
            "--servers",
            "1",
            "--agents",
            "1",
            "--no-lb",
            "--api-port",
            &api_address,
            "--kubeconfig-update-default=false",
            "--kubeconfig-switch-context=false",
            "--k3s-arg",
            "--disable=traefik,servicelb,local-storage@server:0",
            "--k3s-arg",
            "--snapshotter=native@all",
            "--k3s-arg",
            "--flannel-backend=host-gw@server:0",
            "--volume",
            "/dev/null:/dev/kmsg@all",
            "--wait",
            "--timeout",
            "180s",
        ]))?;
        let config = Command::new("k3d")
            .args(["kubeconfig", "get", &self.cluster])
            .output()?;
        ensure!(config.status.success(), "cannot get isolated kubeconfig");
        fs::write(self.directory.path().join("kubeconfig"), &config.stdout)?;
        for image in std::iter::once(image).chain(IMAGES.iter().copied()) {
            self.import_image(image)?;
        }
        // Import the distribution's exact system images for nodes that cannot pull.
        for name in ["coredns", "metrics-server"] {
            crate::run(&mut self.kubectl(&[
                "wait",
                "--for=create",
                &format!("deployment/{name}"),
                "-n",
                "kube-system",
                "--timeout=180s",
            ]))?;
            let system = self.json(&["get", "deployment", name, "-n", "kube-system"])?;
            let system_image = system["spec"]["template"]["spec"]["containers"][0]["image"]
                .as_str()
                .context("system image missing")?;
            crate::run(Command::new("docker").args(["pull", system_image]))?;
            self.import_image(system_image)?;
        }
        crate::run(&mut self.kubectl(&[
            "wait",
            "--for=condition=Ready",
            "node",
            "--all",
            "--timeout=180s",
        ]))?;
        crate::run(&mut self.kubectl(&[
            "wait",
            "--for=condition=DiskPressure=false",
            "node",
            "--all",
            "--timeout=180s",
        ]))?;
        ensure!(
            self.json(&["get", "nodes"])?["items"]
                .as_array()
                .is_some_and(|nodes| nodes.len() == 2),
            "the fixture needs two registered k3s nodes"
        );
        crate::run(&mut self.kubectl(&[
            "rollout",
            "status",
            "deployment/coredns",
            "-n",
            "kube-system",
            "--timeout=180s",
        ]))?;
        crate::run(&mut self.kubectl(&[
            "rollout",
            "status",
            "deployment/metrics-server",
            "-n",
            "kube-system",
            "--timeout=180s",
        ]))?;
        // The default context is deliberately unusable. Every deployment operation must bind its target.
        crate::run(&mut self.kubectl(&["config", "set", "current-context", "hirune-do-not-use"]))?;
        self.prepare(image)?;
        let dev_before = self.prepare_other_environment()?;
        self.check_unfinished_migration()?;
        self.check_automatic_migration_recovery()?;
        self.wait_probe("trusted-probe")?;
        self.wait_probe("denied-probe")?;
        let pods = self.api_pods()?;
        ensure!(
            pods.len() == 2 && pods[0]["spec"]["nodeName"] != pods[1]["spec"]["nodeName"],
            "the two API replicas must run on different nodes"
        );
        println!("API replicas are ready on two distinct k3s nodes.");
        self.initialize_file_integrations()?;
        self.verify_database_roles()?;
        println!("{}", self.probe("prepare")?.trim());
        println!("{}", self.probe("files-prepare")?.trim());
        let client_id = self.probe("client-id")?;
        let client_id = client_id.trim();
        Uuid::parse_str(client_id)?;
        crate::run(&mut self.kubectl(&[
            "exec",
            "deployment/hirune-api",
            "--",
            "manage",
            "set-api-client-quota",
            client_id,
            "100000",
            "3",
        ]))?;
        self.probe("consume-two")?;
        self.probe("start-traffic")?;
        crate::run(&mut self.kubectl(&["rollout", "restart", "deployment/hirune-api"]))?;
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            let deployment = self.json(&["get", "deployment", "hirune-api"])?;
            ensure!(
                deployment["status"]["availableReplicas"]
                    .as_u64()
                    .unwrap_or(0)
                    >= 2,
                "rolling update reduced API availability below two replicas"
            );
            let changed = self
                .api_pods()?
                .iter()
                .filter(|pod| {
                    pod["metadata"]["deletionTimestamp"].is_null()
                        && !pods
                            .iter()
                            .any(|old| old["metadata"]["uid"] == pod["metadata"]["uid"])
                })
                .count()
                == 2;
            if changed && deployment["status"]["updatedReplicas"] == 2 {
                break;
            }
            ensure!(Instant::now() < deadline, "API rolling update timed out");
            thread::sleep(Duration::from_millis(200));
        }
        crate::run(&mut self.kubectl(&[
            "rollout",
            "status",
            "deployment/hirune-api",
            "--timeout=180s",
        ]))?;
        let replaced = self.api_pods()?;
        let ready: Vec<_> = replaced
            .iter()
            .filter(|pod| {
                pod["metadata"]["deletionTimestamp"].is_null()
                    && pod["status"]["conditions"]
                        .as_array()
                        .is_some_and(|conditions| {
                            conditions.iter().any(|condition| {
                                condition["type"] == "Ready" && condition["status"] == "True"
                            })
                        })
            })
            .collect();
        ensure!(
            ready.len() == 2 && ready[0]["spec"]["nodeName"] != ready[1]["spec"]["nodeName"],
            "replaced API replicas must be ready on two distinct nodes"
        );
        println!("{}", self.probe("stop-traffic")?.trim());
        println!("{}", self.probe("after-rollout")?.trim());
        println!("{}", self.probe("files-retained")?.trim());
        self.probe("start-traffic")?;
        let gateway = self.json(&["get", "deployment", "hirune-apisix"])?;
        let api_before_gateway = self.api_pods()?;
        let route = self
            .directory
            .path()
            .join("deploy/k3s/base/apisix/apisix.yaml");
        let contents =
            fs::read_to_string(&route)?.replace("#END", "# configuration rollout acceptance\n#END");
        fs::write(route, contents)?;
        self.deploy("auto")?;
        let updated = self.json(&["get", "deployment", "hirune-apisix"])?;
        ensure!(
            gateway["spec"]["template"] != updated["spec"]["template"],
            "gateway ConfigMap did not trigger a rollout"
        );
        ensure!(
            self.api_pods()?.iter().all(|pod| api_before_gateway
                .iter()
                .any(|old| old["metadata"]["uid"] == pod["metadata"]["uid"])),
            "a gateway-only config change replaced API Pods"
        );
        println!("{}", self.probe("stop-traffic")?.trim());
        let autoscaler = self.enable_autoscaling()?;
        let counter_before_drain = self.json(&[
            "get",
            "pods",
            "-l",
            "app.kubernetes.io/component=edge-counter",
        ])?["items"][0]["metadata"]["uid"]
            .clone();
        self.check_autoscaled_migration_recovery(&autoscaler)?;
        ensure!(
            self.json(&[
                "get",
                "pods",
                "-l",
                "app.kubernetes.io/component=edge-counter"
            ])?["items"][0]["metadata"]["uid"]
                == counter_before_drain,
            "application drain replaced Valkey"
        );
        println!("{}", self.probe("retained-authority")?.trim());
        println!("{}", self.probe("files-retained")?.trim());
        self.check_failed_preflight()?;
        ensure!(
            self.other_environment_state()? == dev_before,
            "prod deployment changed dev workloads"
        );
        println!(
            "Prod rolling and drained deployments left dev Pods and replicas unchanged with an unusable default context."
        );
        self.verify_workflows()?;
        println!("{}", self.probe("files-delete")?.trim());
        self.check_edge_limits()?;
        self.check_cold_start(&autoscaler)?;
        ensure!(
            self.other_environment_state()? == dev_before,
            "prod counter operations changed dev state"
        );
        println!("{}", self.probe("private-ingress")?.trim());
        println!(
            "{}",
            self.read(&[
                "exec",
                "denied-probe",
                "--",
                "python",
                "/checks/probe.py",
                "denied-ingress"
            ])?
            .trim()
        );
        crate::run(&mut self.other_target().kubectl(&[
            "exec",
            "deployment/hirune-api",
            "--",
            "python",
            "-c",
            r#"import socket
for host, port in [('hirune-apisix.hirune-prod.svc.cluster.local', 9080),
                   ('hirune-api.hirune-prod.svc.cluster.local', 3000),
                   ('hirune-edge-valkey.hirune-prod.svc.cluster.local', 6379)]:
    address = socket.gethostbyname(host)
    try:
        connection = socket.create_connection((address, port), timeout=3)
    except OSError:
        continue
    connection.close()
    raise AssertionError('cross-namespace ingress was allowed')
print('Dev tunnel-labelled Pod cannot reach prod gateway, API, or Valkey.')"#,
        ]))?;
        Ok(())
    }

    fn edge_probe(&self, operation: &str) -> Result<String> {
        let pods = self.json(&["get", "pods", "-l", "app.kubernetes.io/component=gateway"])?;
        let gateways: Vec<_> = pods["items"]
            .as_array()
            .context("gateway Pods missing")?
            .iter()
            .filter(|pod| pod["metadata"]["deletionTimestamp"].is_null())
            .map(|pod| {
                pod["status"]["podIP"]
                    .as_str()
                    .map(|ip| format!("http://{ip}:9080"))
            })
            .collect::<Option<Vec<_>>>()
            .context("gateway addresses missing")?;
        ensure!(gateways.len() == 2, "edge acceptance requires two gateways");
        let mut command = self.kubectl(&[
            "exec",
            "trusted-probe",
            "--",
            "python",
            "/checks/edge.py",
            operation,
        ]);
        let output = command.args(gateways).output()?;
        ensure!(
            output.status.success(),
            "edge acceptance failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(String::from_utf8(output.stdout)?)
    }

    fn check_edge_limits(&self) -> Result<()> {
        let path = self.directory.path().join(".envs/.prod/k3s/gateway.env");
        fs::write(path, "EDGE_RATE_LIMIT_REQUESTS=8\n")?;
        // Keep the continuity bucket alive across slow fixture rollouts.
        let route = self
            .directory
            .path()
            .join("deploy/k3s/base/apisix/apisix.yaml");
        fs::write(
            &route,
            fs::read_to_string(&route)?.replace("time_window: 60", "time_window: 600"),
        )?;
        let counter_before = self.json(&[
            "get",
            "pods",
            "-l",
            "app.kubernetes.io/component=edge-counter",
        ])?["items"][0]["metadata"]["uid"]
            .clone();
        self.deploy("rolling")?;
        ensure!(
            self.json(&[
                "get",
                "pods",
                "-l",
                "app.kubernetes.io/component=edge-counter"
            ])?["items"][0]["metadata"]["uid"]
                == counter_before,
            "gateway settings replaced Valkey"
        );
        println!("{}", self.edge_probe("check")?.trim());
        ensure!(
            self.read(&[
                "exec",
                "deployment/hirune-edge-valkey",
                "--",
                "valkey-cli",
                "GET",
                "namespace-marker"
            ])?
            .trim()
            .is_empty(),
            "dev counter state leaked into prod"
        );
        crate::run(&mut self.kubectl(&["rollout", "restart", "deployment/hirune-apisix"]))?;
        crate::run(&mut self.kubectl(&[
            "rollout",
            "status",
            "deployment/hirune-apisix",
            "--timeout=180s",
        ]))?;
        println!("{}", self.edge_probe("after-gateway-restart")?.trim());
        crate::run(&mut self.kubectl(&[
            "exec",
            "deployment/hirune-edge-valkey",
            "--",
            "valkey-cli",
            "CLIENT",
            "PAUSE",
            "5000",
            "ALL",
        ]))?;
        println!("{}", self.edge_probe("outage")?.trim());
        crate::run(&mut self.kubectl(&["rollout", "restart", "deployment/hirune-edge-valkey"]))?;
        crate::run(&mut self.kubectl(&[
            "rollout",
            "status",
            "deployment/hirune-edge-valkey",
            "--timeout=180s",
        ]))?;
        ensure!(
            self.read(&[
                "exec",
                "deployment/hirune-edge-valkey",
                "--",
                "valkey-cli",
                "DBSIZE"
            ])?
            .trim()
                == "0",
            "replacement Valkey retained disposable counters"
        );
        println!("{}", self.edge_probe("after-valkey-restart")?.trim());
        Ok(())
    }

    fn other_target(&self) -> Target {
        Target {
            context: format!("k3d-{}", self.cluster),
            namespace: "hirune-dev".into(),
            kubeconfig: Some(self.directory.path().join("kubeconfig")),
        }
    }

    fn other_environment_state(&self) -> Result<Value> {
        let target = self.other_target();
        let deployment = target.json(&["get", "deployment", "hirune-api"])?;
        let pods = target.json(&["get", "pods"])?;
        let counter = target.json(&["get", "deployment", "hirune-edge-valkey"])?;
        let marker = target
            .kubectl(&[
                "exec",
                "deployment/hirune-edge-valkey",
                "--",
                "valkey-cli",
                "GET",
                "namespace-marker",
            ])
            .output()?;
        ensure!(marker.status.success(), "cannot inspect dev counter marker");
        Ok(
            json!({"uid":deployment["metadata"]["uid"],"generation":deployment["metadata"]["generation"],
            "spec":deployment["spec"], "counter":counter["spec"], "marker":String::from_utf8(marker.stdout)?,
            "pods":pods["items"].as_array().context("dev Pods missing")?.iter()
                .map(|pod| pod["metadata"]["uid"].clone()).collect::<Vec<_>>()}),
        )
    }

    fn prepare_other_environment(&self) -> Result<Value> {
        let target = self.other_target();
        crate::kubernetes::apply(
            &target,
            vec![json!({"apiVersion":"v1","kind":"Namespace","metadata":{"name":"hirune-dev"}})],
        )?;
        copy_directory(
            Path::new(".envs/.example/dev/k3s"),
            &self.directory.path().join(".envs/.dev/k3s"),
        )?;
        let dev = Deployment::load(self.directory.path(), Environment::Dev)?;
        crate::kubernetes::apply(
            &target,
            dev.resources
                .into_iter()
                .filter(|r| {
                    let name = r["metadata"]["name"].as_str().unwrap_or_default();
                    (r["kind"] == "ConfigMap" && name.starts_with("hirune-edge-valkey-"))
                        || (matches!(r["kind"].as_str(), Some("Deployment" | "Service"))
                            && name == "hirune-edge-valkey")
                        || (r["kind"] == "NetworkPolicy"
                            && matches!(
                                name,
                                "hirune-private-ingress" | "hirune-gateway-to-edge-counter"
                            ))
                })
                .collect(),
        )?;
        crate::run(&mut target.kubectl(&[
            "rollout",
            "status",
            "deployment/hirune-edge-valkey",
            "--timeout=180s",
        ]))?;
        crate::run(&mut target.kubectl(&[
            "exec",
            "deployment/hirune-edge-valkey",
            "--",
            "valkey-cli",
            "SET",
            "namespace-marker",
            "dev",
        ]))?;
        crate::kubernetes::apply(
            &target,
            vec![json!({"apiVersion":"apps/v1","kind":"Deployment",
            "metadata":{"name":"hirune-api","namespace":"hirune-dev"},"spec":{"replicas":1,
                "selector":{"matchLabels":{"app.kubernetes.io/part-of":"hirune-kit","app.kubernetes.io/component":"tunnel"}},
                "template":{"metadata":{"labels":{"app.kubernetes.io/part-of":"hirune-kit","app.kubernetes.io/component":"tunnel"}},
                    "spec":{"automountServiceAccountToken":false,"containers":[{"name":"sentinel","image":"python:3.14-alpine",
                        "command":["python","-c","import time; time.sleep(3600)"]}]}}}})],
        )?;
        crate::run(&mut target.kubectl(&[
            "rollout",
            "status",
            "deployment/hirune-api",
            "--timeout=180s",
        ]))?;
        self.other_environment_state()
    }

    fn api_pods(&self) -> Result<Vec<Value>> {
        let value = self.json(&["get", "pods", "-l", "app.kubernetes.io/component=api"])?;
        Ok(value["items"]
            .as_array()
            .context("Pod list missing")?
            .clone())
    }

    fn import_image(&self, image: &str) -> Result<()> {
        // Export the daemon's Linux platform only; cached indexes can reference absent variants.
        // k3d's importer assumes overlayfs. Match the native snapshotter explicitly.
        for role in ["server", "agent"] {
            let node = format!("k3d-{}-{role}-0", self.cluster);
            let mut save = Command::new("docker")
                .args(["image", "save", "--platform=linux", image])
                .stdout(Stdio::piped())
                .spawn()?;
            let result = Command::new("docker")
                .args([
                    "exec",
                    "-i",
                    &node,
                    "ctr",
                    "--address=/run/k3s/containerd/containerd.sock",
                    "--namespace=k8s.io",
                    "images",
                    "import",
                    "--local",
                    "--snapshotter=native",
                    "-",
                ])
                .stdin(save.stdout.take().context("docker save stdout missing")?)
                .status();
            let saved = save.wait()?;
            ensure!(
                result?.success() && saved.success(),
                "native image import failed for {image}"
            );
        }
        Ok(())
    }

    fn check_unfinished_migration(&self) -> Result<()> {
        self.apply(json!({"apiVersion":"batch/v1","kind":"Job","metadata":{"name":"hirune-migration"},
            "spec":{"suspend":true,"template":{"spec":{"restartPolicy":"Never",
                "containers":[{"name":"migration","image":"python:3.14-alpine","command":["true"]}]}}}}))?;
        let error = self
            .deploy("rolling")
            .err()
            .context("unfinished migration did not block deployment")?;
        ensure!(
            error.to_string().contains("unfinished"),
            "wrong migration guard error"
        );
        crate::run(&mut self.kubectl(&["delete", "job", "hirune-migration", "--wait=true"]))?;
        println!("An unfinished migration Job correctly blocked deployment.");
        Ok(())
    }

    fn check_automatic_migration_recovery(&self) -> Result<()> {
        let mut failing = self.deployment()?;
        let job = failing
            .resources
            .iter_mut()
            .find(|r| r["kind"] == "Job" && r["metadata"]["name"] == "hirune-migration")
            .context("migration Job missing")?;
        job["spec"]["template"]["spec"]["containers"][0]["command"] =
            json!(["sh", "-c", "migration && exit 1"]);
        let error = crate::kubernetes::deploy(&failing, crate::kubernetes::Mode::Auto)
            .err()
            .context("failed migration did not block deployment")?;
        ensure!(
            error.to_string().contains("hirune-migration failed"),
            "unexpected migration failure: {error}"
        );
        let check = self.json(&["get", "job", "hirune-migration-check"])?;
        ensure!(
            crate::kubernetes_jobs::pending_count(&failing.target, &check)? > 0,
            "fresh database must select maintenance automatically"
        );
        ensure!(
            self.api_pods()?.is_empty(),
            "applications started after a failed migration"
        );
        let gateways = self.json(&["get", "deployment", "hirune-apisix"])?;
        ensure!(
            gateways["status"]["availableReplicas"] == 2,
            "gateways must stay ready while the API is absent"
        );
        self.wait_probe("trusted-probe")?;
        println!("{}", self.probe("maintenance")?.trim());
        ensure!(
            self.deploy("rolling").is_err(),
            "explicit rolling bypassed active maintenance"
        );
        self.deploy("auto")?;
        let check = self.json(&["get", "job", "hirune-migration-check"])?;
        ensure!(
            crate::kubernetes_jobs::pending_count(&failing.target, &check)? == 0,
            "retry must recover after the migration was already applied"
        );
        println!(
            "Pending migrations selected maintenance automatically; failed deployment retained it; automatic retry recovered with zero pending migrations."
        );
        Ok(())
    }

    fn check_failed_preflight(&self) -> Result<()> {
        let before = self.api_pods()?;
        let mut failing = self.deployment()?;
        let job = failing
            .resources
            .iter_mut()
            .find(|r| r["kind"] == "Job" && r["metadata"]["name"] == "hirune-migration-check")
            .context("migration check missing")?;
        job["spec"]["template"]["spec"]["containers"][0]["command"] = json!(["sh", "-c", "exit 1"]);
        let error = crate::kubernetes::deploy(&failing, crate::kubernetes::Mode::Auto)
            .err()
            .context("failed preflight did not block deployment")?;
        ensure!(
            error.to_string().contains("hirune-migration-check failed"),
            "unexpected preflight failure: {error}"
        );
        ensure!(
            self.api_pods()?.iter().all(|pod| before
                .iter()
                .any(|old| old["metadata"]["uid"] == pod["metadata"]["uid"])),
            "failed preflight replaced running API Pods"
        );
        ensure!(
            !crate::kubernetes_maintenance::active(
                crate::kubernetes_maintenance::state(&failing.target)?.as_ref()
            ),
            "failed preflight changed maintenance state"
        );
        self.probe("retained-authority")?;
        println!("Failed read-only preflight preserved running APIs and open admission.");
        Ok(())
    }

    fn wait_probe(&self, name: &str) -> Result<()> {
        crate::run(&mut self.kubectl(&[
            "wait",
            "--for=condition=Ready",
            &format!("pod/{name}"),
            "--timeout=180s",
        ]))
    }

    fn prepare(&self, image: &str) -> Result<()> {
        copy_directory(
            Path::new("deploy/k3s/base"),
            &self.directory.path().join("deploy/k3s/base"),
        )?;
        let profile = self.directory.path().join(".envs/.prod/k3s");
        copy_directory(Path::new(".envs/.example/prod/k3s"), &profile)?;
        fs::write(
            profile.join("target.env"),
            format!(
                "KUBE_CONTEXT=k3d-{}\nKUBE_NAMESPACE={NAMESPACE}\n",
                self.cluster
            ),
        )?;
        let (name, tag) = image
            .rsplit_once(':')
            .context("K3S_TEST_IMAGE needs an explicit tag")?;
        let path = profile.join("kustomization.yaml");
        fs::write(
            &path,
            fs::read_to_string(&path)?
                .replace("registry.example.com/your-project/hirune-kit", name)
                .replace("newTag: 0.1.0", &format!("newTag: {tag}")),
        )?;
        // Each revision must span both fixture nodes while old Pods terminate.
        configure_api_spread(&profile)?;
        let path = self.directory.path().join("deploy/k3s/base/tunnel.yaml");
        fs::write(
            &path,
            fs::read_to_string(&path)?.replace("replicas: 2", "replicas: 0"),
        )?;
        let path = profile.join("app.env");
        fs::write(
            &path,
            fs::read_to_string(&path)?
                .replace("APP_ENV=production", "APP_ENV=test")
                .replace("https://app.prod.example.com/", "http://localhost/")
                .replace("https://app.prod.example.com", "http://localhost")
                .replace("smtp.example.com", "mailpit")
                .replace("SMTP_PORT=587", "SMTP_PORT=1025")
                .replace("SMTP_STARTTLS=true", "SMTP_STARTTLS=false"),
        )?;
        let mut app_environment = fs::OpenOptions::new().append(true).open(&path)?;
        writeln!(
            app_environment,
            "\nS3_ENDPOINT=http://seaweedfs:8333/\nS3_PUBLIC_ENDPOINT=http://storage-public:8081/\nS3_BUCKET=kit-files\nS3_REGION=us-east-1\nS3_ALLOW_HTTP=true\nMEILISEARCH_URL=http://meilisearch:7700/\nMEILISEARCH_ALLOW_HTTP=true\nMEILISEARCH_INDEX=files\nMEILISEARCH_POSTS_INDEX=posts\nMEILISEARCH_PROFILES_INDEX=profiles"
        )?;
        let password = auth_core::token::generate_secure_token();
        let schema_password = auth_core::token::generate_secure_token();
        let runtime_password = auth_core::token::generate_secure_token();
        let maintenance_password = auth_core::token::generate_secure_token();
        let storage_access = auth_core::token::generate_secure_token();
        let storage_secret = auth_core::token::generate_secure_token();
        let runtime_url = format!("postgres://runtime:{runtime_password}@postgres:5432/kit");
        let maintenance_url =
            format!("postgres://maintenance:{maintenance_password}@postgres:5432/kit");
        let migration_url = format!("postgres://schema_owner:{schema_password}@postgres:5432/kit");
        let key = base64::engine::general_purpose::STANDARD.encode([17_u8; 32]);
        fs::write(
            profile.join("secrets.env"),
            format!(
                "ACTIVE_SECRET_KEY=test-v1\nSECRET_KEYS={{\"test-v1\":\"{key}\"}}\nRATE_LIMIT_KEY={}\nINTERNAL_PROXY_SECRET={}\nS3_ACCESS_KEY={storage_access}\nS3_SECRET_KEY={storage_secret}\nMEILISEARCH_API_KEY={}\n",
                auth_core::token::generate_secure_token(),
                auth_core::token::generate_secure_token(),
                auth_core::token::generate_secure_token()
            ),
        )?;
        fs::write(
            profile.join("runtime.env"),
            format!("DATABASE_URL={runtime_url}\n"),
        )?;
        fs::write(
            profile.join("maintenance.env"),
            format!("DATABASE_URL={maintenance_url}\n"),
        )?;
        fs::write(
            profile.join("migration.env"),
            format!(
                "MIGRATION_DATABASE_URL={migration_url}\nMIGRATION_RUNTIME_ROLE=runtime\nMIGRATION_MAINTENANCE_ROLE=maintenance\n"
            ),
        )?;
        fs::write(
            profile.join("tunnel.env"),
            "TUNNEL_TOKEN=not-connected-in-local-test\n",
        )?;
        let deployment = self.deployment()?;
        crate::kubernetes::apply(
            &deployment.target,
            deployment
                .resources
                .iter()
                .filter(|r| matches!(r["kind"].as_str(), Some("Namespace" | "Secret")))
                .cloned()
                .collect(),
        )?;
        let app_secret = deployment
            .resources
            .iter()
            .find(|r| {
                r["kind"] == "Secret"
                    && r["metadata"]["name"]
                        .as_str()
                        .is_some_and(|name| name.starts_with("hirune-app-secrets-"))
            })
            .context("generated application Secret missing")?["metadata"]["name"]
            .clone();
        let mut resources = vec![
            json!({"apiVersion":"v1","kind":"ConfigMap","metadata":{"name":"probe-checks"},
                "data":{"probe.py":include_str!("../../deploy/k3s/verification/probe.py"),
                    "edge.py":include_str!("../../deploy/k3s/verification/edge.py"),
                    "files.py":include_str!("../../deploy/k3s/verification/files.py"),
                    "workflows.py":include_str!("../../deploy/k3s/verification/workflows.py")}}),
        ];
        resources.extend(self.file_service_resources(
            &app_secret,
            &storage_access,
            &storage_secret,
        ));
        for (name, image, port, env) in [
            (
                "postgres",
                "postgres:18-alpine",
                5432,
                json!([
                {"name":"POSTGRES_USER","value":"bootstrap"},{"name":"POSTGRES_DB","value":"postgres"},
                {"name":"POSTGRES_PASSWORD","value":password} ]),
            ),
            ("mailpit", "axllent/mailpit:v1.31.5", 1025, json!([])),
        ] {
            resources.push(json!({"apiVersion":"apps/v1","kind":"Deployment","metadata":{"name":name},
                "spec":{"replicas":1,"selector":{"matchLabels":{"fixture":name}},"template":{
                    "metadata":{"labels":{"fixture":name}},"spec":{"automountServiceAccountToken":false,
                    "containers":[{"name":name,"image":image,"env":env,"ports":[{"containerPort":port}],
                    "readinessProbe":{"tcpSocket":{"port":port},"periodSeconds":2}}]}}}}));
            resources.push(json!({"apiVersion":"v1","kind":"Service","metadata":{"name":name},
                "spec":{"selector":{"fixture":name},"ports":if name == "mailpit" {json!([
                    {"name":"smtp","port":1025},{"name":"http","port":8025}])} else {json!([{"port":port}])}}}));
        }
        resources.push(json!({"apiVersion":"networking.k8s.io/v1","kind":"NetworkPolicy",
            "metadata":{"name":"fixture-database-mail"},"spec":{"podSelector":{"matchExpressions":[
                {"key":"fixture","operator":"In","values":["postgres","mailpit"]}]},"policyTypes":["Ingress"],
                "ingress":[{"from":[{"podSelector":{}}]}]}}));
        for (name, component) in [("trusted-probe", "tunnel"), ("denied-probe", "probe")] {
            resources.push(json!({"apiVersion":"v1","kind":"Pod","metadata":{"name":name,
                "labels":{"app.kubernetes.io/part-of":"hirune-kit","app.kubernetes.io/component":component}},
                "spec":{"automountServiceAccountToken":false,"containers":[{"name":"probe","image":"python:3.14-alpine",
                    "command":["python","-c","import time; time.sleep(3600)"],
                    "env":[{"name":"K6_PROFILE","value":workflows::profile()?},
                        {"name":"INTERNAL_PROXY_SECRET","valueFrom":{"secretKeyRef":{"name":app_secret,"key":"INTERNAL_PROXY_SECRET"}}}],
                    "volumeMounts":[{"name":"checks","mountPath":"/checks","readOnly":true}]}],
                    "volumes":[{"name":"checks","configMap":{"name":"probe-checks"}}]}}));
        }
        self.apply(json!({"apiVersion":"v1","kind":"List","items":resources}))?;
        for name in [
            "postgres",
            "mailpit",
            "seaweedfs",
            "meilisearch",
            "storage-public",
        ] {
            crate::run(&mut self.kubectl(&[
                "rollout",
                "status",
                &format!("deployment/{name}"),
                "--timeout=180s",
            ]))?;
        }
        self.create_database_roles(&schema_password, &runtime_password, &maintenance_password)?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = Command::new("k3d")
            .args(["cluster", "delete", &self.cluster])
            .env("KUBECONFIG", self.directory.path().join("kubeconfig"))
            .status();
    }
}
