use crate::kubernetes_environment::{Deployment, Environment, copy_directory};
use anyhow::{Context, Result, ensure};
use base64::Engine;
use serde_json::Value;
use std::{fs, path::Path, process::Command};

pub fn verify() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    copy_directory(Path::new("deploy/k3s/base"), &root.join("deploy/k3s/base"))?;
    copy_directory(
        Path::new("compose/data-services"),
        &root.join("compose/data-services"),
    )?;
    copy_directory(
        Path::new("compose/production/postgres"),
        &root.join("compose/production/postgres"),
    )?;
    for environment in [Environment::Dev, Environment::Prod] {
        copy_directory(
            &Path::new(".envs/.example")
                .join(environment.name())
                .join("k3s"),
            &root.join(format!(".envs/.{}/k3s", environment.name())),
        )?;
        copy_directory(
            &Path::new(".envs/.example")
                .join(environment.name())
                .join("data-services"),
            &root.join(format!(".envs/.{}/data-services", environment.name())),
        )?;
        let data = crate::data_services::DataServicesDeployment::load(root, environment)?;
        let rendered = data
            .compose(&["--profile", "recovery", "config", "--format", "json"])
            .env_remove("DATA_BUILD_CA_FILE")
            .current_dir(root)
            .output()?;
        ensure!(rendered.status.success(), "data Compose render failed");
        let rendered: Value = serde_json::from_slice(&rendered.stdout)?;
        let services = &rendered["services"];
        for name in ["postgres", "backup-scheduler", "restore", "recovery-check"] {
            ensure!(
                services[name].get("build").is_none()
                    && services[name]["image"] == data.values["POSTGRES_IMAGE"],
                "data service {name} must use the selected published image without building"
            );
        }
        ensure!(
            rendered.get("secrets").is_none(),
            "production data profile includes build secrets"
        );
        let local_build = data
            .compose(&[
                "--profile",
                "recovery",
                "--file",
                "compose/data-services/compose.build.yaml",
                "config",
                "--format",
                "json",
            ])
            .env_remove("DATA_BUILD_CA_FILE")
            .current_dir(root)
            .output()?;
        ensure!(
            local_build.status.success(),
            "local data build Compose render failed"
        );
        let local_build: Value = serde_json::from_slice(&local_build.stdout)?;
        let build_context = Path::new(
            local_build["services"]["postgres"]["build"]["context"]
                .as_str()
                .context("database build context missing")?,
        )
        .canonicalize()?;
        let default_ca = Path::new(
            local_build["secrets"]["proxy_ca"]["file"]
                .as_str()
                .context("database build CA missing")?,
        )
        .canonicalize()?;
        ensure!(
            default_ca.starts_with(build_context) && fs::metadata(default_ca)?.len() == 0,
            "default build CA must be empty and inside the build context"
        );
        for name in ["postgres", "meilisearch"] {
            ensure!(
                services[name]["ports"][0]["host_ip"] == "10.0.0.10",
                "data ports lost their private binding"
            );
        }
        let scheduler = &services["backup-scheduler"];
        ensure!(
            scheduler["user"] == "postgres" && scheduler["read_only"] == true,
            "backup scheduler must run without root or writable image state"
        );
        ensure!(
            !scheduler["environment"].to_string().contains("PASSWORD"),
            "backup scheduler received database passwords"
        );
        ensure!(
            scheduler["volumes"]
                .as_array()
                .is_some_and(|volumes| volumes
                    .iter()
                    .any(|v| v["target"] == "/var/lib/postgresql" && v["read_only"] == true)),
            "scheduler data access must be read-only"
        );
    }
    let dev = Deployment::load(root, Environment::Dev)?;
    let dev_data_path = root.join(".envs/.dev/data-services/data.env");
    let original_data = fs::read_to_string(&dev_data_path)?;
    let dev_data = crate::data_services::DataServicesDeployment::load(root, Environment::Dev)?;
    let original_image = dev_data.values["POSTGRES_IMAGE"].clone();
    for image in [
        "",
        "postgres",
        "registry.example:5000/postgres",
        "postgres:latest",
        "postgres:",
        "postgres:-invalid",
        "postgres@sha256:1234",
    ] {
        fs::write(
            &dev_data_path,
            original_data.replace(&original_image, image),
        )?;
        ensure!(
            crate::data_services::DataServicesDeployment::load(root, Environment::Dev).is_err(),
            "unversioned or invalid PostgreSQL image was accepted"
        );
    }
    for image in [
        "registry.example:5000/service-postgres:18.6-1".to_owned(),
        format!("ghcr.io/example/service-postgres@sha256:{}", "a".repeat(64)),
    ] {
        fs::write(
            &dev_data_path,
            original_data.replace(&original_image, &image),
        )?;
        ensure!(
            crate::data_services::DataServicesDeployment::load(root, Environment::Dev).is_ok(),
            "explicit PostgreSQL version or digest was rejected"
        );
    }
    fs::write(&dev_data_path, &original_data)?;
    fs::write(
        &dev_data_path,
        format!("{original_data}\nDEPLOY_ENV=prod\n"),
    )?;
    ensure!(
        crate::data_services::DataServicesDeployment::load(root, Environment::Dev).is_err(),
        "data environment selector was overridden"
    );
    fs::write(&dev_data_path, original_data)?;
    let prod_data_path = root.join(".envs/.prod/data-services/data.env");
    let original_prod_data = fs::read_to_string(&prod_data_path)?;
    fs::write(
        &prod_data_path,
        original_prod_data.replace("hirune-prod-postgres", "hirune-dev-postgres"),
    )?;
    ensure!(
        crate::data_services::DataServicesDeployment::load(root, Environment::Prod).is_err(),
        "dev and prod shared a data volume"
    );
    fs::write(&prod_data_path, original_prod_data)?;
    let prod = Deployment::load(root, Environment::Prod)?;
    for (deployment, name) in [(&dev, "dev"), (&prod, "prod")] {
        ensure!(
            deployment.target.context == format!("k3s-{name}"),
            "wrong environment context"
        );
        ensure!(
            deployment.target.namespace == format!("hirune-{name}"),
            "wrong environment namespace"
        );
        verify_roles(deployment)?;
        let gateway =
            &resource(deployment, "hirune-apisix")["spec"]["template"]["spec"]["containers"][0];
        ensure!(
            gateway["envFrom"].is_null(),
            "gateway received the full application Secret"
        );
        ensure!(
            gateway["readinessProbe"]["httpGet"]["path"] == "/health/gateway",
            "gateway readiness must survive application maintenance"
        );
        ensure!(
            resource(deployment, "hirune-gateway-maintenance")["data"]["state"] == "on\n",
            "new maintenance state must start closed"
        );
        let preflight = &resource(deployment, "hirune-migration-check")["spec"]["template"]["spec"]
            ["containers"][0];
        ensure!(
            preflight["command"] == serde_json::json!(["migration", "--check-pending"]),
            "preflight must check the selected release image without migrating"
        );
        ensure!(
            preflight["image"]
                == resource(deployment, "hirune-migration")["spec"]["template"]["spec"]["containers"]
                    [0]["image"],
            "preflight and migration must use the same image"
        );
        let secret_keys: Vec<_> = gateway["env"]
            .as_array()
            .context("gateway environment missing")?
            .iter()
            .filter_map(|entry| entry["valueFrom"]["secretKeyRef"]["key"].as_str())
            .collect();
        ensure!(
            secret_keys == ["INTERNAL_PROXY_SECRET"],
            "gateway received unrelated application credentials"
        );
        ensure!(
            resource(deployment, "hirune-api")["spec"]["replicas"] == 2,
            "two API replicas required"
        );
        for service in deployment
            .resources
            .iter()
            .filter(|r| r["kind"] == "Service")
        {
            ensure!(
                service["spec"]["type"] == "ClusterIP",
                "ingress must stay private"
            );
        }
        for policy in deployment
            .resources
            .iter()
            .filter(|r| r["kind"] == "NetworkPolicy")
        {
            ensure!(
                !policy.to_string().contains("namespaceSelector"),
                "ingress peers must stay within the selected namespace"
            );
        }
        let secrets = generated_secret(deployment, "hirune-app-secrets-");
        let keys = base64::engine::general_purpose::STANDARD.decode(
            secrets["data"]["SECRET_KEYS"]
                .as_str()
                .context("keys missing")?,
        )?;
        let _: Value = serde_json::from_slice(&keys)
            .context("Kubernetes JSON must use raw env-file values")?;
    }
    let mut invalid = Deployment::load(root, Environment::Prod)?;
    let counter = invalid
        .resources
        .iter_mut()
        .find(|r| r["kind"] == "Deployment" && r["metadata"]["name"] == "hirune-edge-valkey")
        .context("counter missing")?;
    counter["spec"]["replicas"] = 2.into();
    ensure!(
        invalid.validate().is_err(),
        "independent counter replicas were accepted"
    );
    let settings = root.join(".envs/.prod/k3s/gateway.env");
    fs::write(&settings, "EDGE_RATE_LIMIT_REQUESTS=0\n")?;
    ensure!(
        Deployment::load(root, Environment::Prod).is_err(),
        "invalid edge limit was accepted"
    );
    fs::write(&settings, "EDGE_RATE_LIMIT_REQUESTS=120000\n")?;
    let changed = Deployment::load(root, Environment::Prod)?;
    ensure!(
        resource(&prod, "hirune-apisix")["spec"]["template"]
            != resource(&changed, "hirune-apisix")["spec"]["template"],
        "edge limit did not replace gateways"
    );
    for name in crate::kubernetes::APPLICATIONS
        .iter()
        .copied()
        .chain(["hirune-edge-valkey", "hirune-cloudflared"])
    {
        ensure!(
            resource(&prod, name) == resource(&changed, name),
            "edge limit changed unrelated runtime state"
        );
    }
    let secrets = root.join(".envs/.prod/k3s/secrets.env");
    fs::write(
        &secrets,
        fs::read_to_string(&secrets)?
            .replace("SMTP_PASSWORD=", "SMTP_PASSWORD=changed-for-rollout"),
    )?;
    let changed = Deployment::load(root, Environment::Prod)?;
    for name in crate::kubernetes::APPLICATIONS {
        ensure!(
            resource(&prod, name)["spec"]["template"]
                != resource(&changed, name)["spec"]["template"],
            "Secret changes must replace affected application Pods"
        );
    }
    ensure!(
        resource(&prod, "hirune-cloudflared")["spec"]["template"]
            == resource(&changed, "hirune-cloudflared")["spec"]["template"],
        "application Secret changes must not replace tunnel Pods"
    );
    ensure!(
        Deployment::load(root, Environment::Dev)?.resources == dev.resources,
        "changing prod settings changed dev resources"
    );
    let before_owner_change = changed;
    let migration = root.join(".envs/.prod/k3s/migration.env");
    fs::write(
        &migration,
        fs::read_to_string(&migration)?.replace("REPLACE_ME", "new-owner-password"),
    )?;
    let changed = Deployment::load(root, Environment::Prod)?;
    for name in crate::kubernetes::APPLICATIONS {
        ensure!(
            resource(&before_owner_change, name) == resource(&changed, name),
            "schema-owner Secret leaked into runtime"
        );
    }
    ensure!(
        resource(&before_owner_change, "hirune-migration")
            != resource(&changed, "hirune-migration"),
        "migration Job must use the new owner Secret"
    );
    let production_spread =
        &resource(&changed, "hirune-api")["spec"]["template"]["spec"]["topologySpreadConstraints"];
    ensure!(
        production_spread[0]["whenUnsatisfiable"] == "ScheduleAnyway",
        "production API spreading must remain preferred"
    );
    crate::kubernetes_test::configure_api_spread(&root.join(".envs/.prod/k3s"))?;
    let fixture = Deployment::load(root, Environment::Prod)?;
    let fixture_spread =
        &resource(&fixture, "hirune-api")["spec"]["template"]["spec"]["topologySpreadConstraints"];
    ensure!(
        fixture_spread
            .as_array()
            .is_some_and(|spread| spread.len() == 1)
            && fixture_spread[0]["whenUnsatisfiable"] == "DoNotSchedule"
            && fixture_spread[0]["matchLabelKeys"] == serde_json::json!(["pod-template-hash"])
            && fixture_spread[0]["labelSelector"]["matchLabels"]["app.kubernetes.io/component"]
                == "api",
        "the fixture must spread each API revision independently"
    );
    fs::write(
        root.join(".envs/.prod/k3s/target.env"),
        "KUBE_CONTEXT=k3s-prod\nKUBE_NAMESPACE=hirune-dev\n",
    )?;
    ensure!(
        Deployment::load(root, Environment::Prod).is_err(),
        "namespace mismatch must fail before mutation"
    );
    ensure!(
        Environment::parse("production").is_err(),
        "unknown deployment environment accepted"
    );
    let local = Command::new("docker")
        .args([
            "compose",
            "-f",
            "compose/compose.local.yml",
            "config",
            "--quiet",
        ])
        .status()?;
    ensure!(local.success(), "local Compose validation failed");
    println!(
        "Kubernetes dev/prod isolation, role separation, Secret-triggered rollouts, singleton Valkey, gateway-only settings, namespace validation, and local Compose contracts passed."
    );
    Ok(())
}

fn resource<'a>(deployment: &'a Deployment, name: &str) -> &'a Value {
    deployment
        .resources
        .iter()
        .find(|r| {
            matches!(r["kind"].as_str(), Some("Deployment" | "Job" | "ConfigMap"))
                && r["metadata"]["name"] == name
        })
        .expect("required resource missing")
}

fn generated_secret<'a>(deployment: &'a Deployment, prefix: &str) -> &'a Value {
    deployment
        .resources
        .iter()
        .find(|r| {
            r["kind"] == "Secret"
                && r["metadata"]["name"]
                    .as_str()
                    .is_some_and(|name| name.starts_with(prefix))
        })
        .expect("generated Secret missing")
}

fn verify_roles(deployment: &Deployment) -> Result<()> {
    let runtime = generated_secret(deployment, "hirune-runtime-database-");
    let maintenance = generated_secret(deployment, "hirune-maintenance-database-");
    let owner = generated_secret(deployment, "hirune-migration-database-");
    for (secret, key, role) in [
        (runtime, "DATABASE_URL", "runtime:"),
        (maintenance, "DATABASE_URL", "maintenance:"),
        (owner, "MIGRATION_DATABASE_URL", "schema_owner:"),
    ] {
        let values = secret["data"].as_object().context("Secret data missing")?;
        ensure!(
            values.len()
                == if key == "MIGRATION_DATABASE_URL" {
                    3
                } else {
                    1
                },
            "database roles must use separate Secrets"
        );
        let value = base64::engine::general_purpose::STANDARD.decode(
            values[key]
                .as_str()
                .context("database URL missing")?
                .split_whitespace()
                .collect::<String>(),
        )?;
        ensure!(
            String::from_utf8(value)?.contains(role),
            "incorrect example database role"
        );
    }
    for (name, database) in [
        ("hirune-api", runtime),
        ("hirune-worker", runtime),
        ("hirune-maintenance", maintenance),
        ("hirune-migration", owner),
        ("hirune-migration-check", owner),
    ] {
        let workload = resource(deployment, name);
        let container = &workload["spec"]["template"]["spec"]["containers"][0];
        ensure!(
            container["securityContext"]["readOnlyRootFilesystem"] == true,
            "application filesystem must be read-only"
        );
        let reference = &container["env"][0]["valueFrom"]["secretKeyRef"]["name"];
        ensure!(
            reference == &database["metadata"]["name"],
            "incorrect database Secret reference"
        );
        if !matches!(name, "hirune-migration" | "hirune-migration-check") {
            ensure!(
                !container.to_string().contains("hirune-migration-database"),
                "runtime received owner credentials"
            );
        }
    }
    Ok(())
}
