use super::*;

impl Fixture {
    pub(super) fn file_service_resources(
        &self,
        app_secret: &Value,
        access_key: &str,
        secret_key: &str,
    ) -> Vec<Value> {
        let storage_credentials = json!({"identities":[{"name":"fixture-app","credentials":[{
            "accessKey":access_key,"secretKey":secret_key}],"actions":["Admin","Read","Write","List","Tagging"]}]});
        let mut resources = vec![
            json!({"apiVersion":"v1","kind":"Secret","metadata":{"name":"fixture-storage"},
                "stringData":{"s3.json":storage_credentials.to_string()}}),
            json!({"apiVersion":"v1","kind":"ConfigMap","metadata":{"name":"fixture-storage-proxy"},
                "data":{"Caddyfile":include_str!("../../../deploy/k3s/verification/storage.Caddyfile")}}),
        ];
        let services = [
            (
                "seaweedfs",
                "chrislusf/seaweedfs:4.48",
                8333,
                json!({
                    "args":["server","-s3","-dir=/data","-ip=$(POD_IP)","-ip.bind=0.0.0.0",
                        "-volume.max=20","-master.volumeSizeLimitMB=100","-s3.config=/etc/seaweedfs/s3.json"],
                    "env":[{"name":"POD_IP","valueFrom":{"fieldRef":{"fieldPath":"status.podIP"}}}],
                    "volumeMounts":[{"name":"data","mountPath":"/data"},{"name":"credentials","mountPath":"/etc/seaweedfs","readOnly":true}],
                    "readinessProbe":{"exec":{"command":["sh","-c","wget -q -O /dev/null http://127.0.0.1:9333/cluster/status && wget -q -O /dev/null http://127.0.0.1:8888/"]},"periodSeconds":2},
                }),
                json!([{"name":"data","emptyDir":{}},{"name":"credentials","secret":{"secretName":"fixture-storage"}}]),
            ),
            (
                "meilisearch",
                "getmeili/meilisearch:v1.55.0",
                7700,
                json!({
                    "env":[{"name":"MEILI_ENV","value":"development"},{"name":"MEILI_NO_ANALYTICS","value":"true"},
                        {"name":"MEILI_MASTER_KEY","valueFrom":{"secretKeyRef":{"name":app_secret,"key":"MEILISEARCH_API_KEY"}}}],
                    "volumeMounts":[{"name":"data","mountPath":"/meili_data"}],
                    "readinessProbe":{"httpGet":{"path":"/health","port":7700},"periodSeconds":2},
                }),
                json!([{"name":"data","emptyDir":{}}]),
            ),
            (
                "storage-public",
                "caddy:2.11.7-alpine",
                8081,
                json!({
                    "args":["caddy","run","--config","/etc/caddy/Caddyfile","--adapter","caddyfile"],
                    "volumeMounts":[{"name":"configuration","mountPath":"/etc/caddy","readOnly":true}],
                    "readinessProbe":{"tcpSocket":{"port":8081},"periodSeconds":2},
                }),
                json!([{"name":"configuration","configMap":{"name":"fixture-storage-proxy"}}]),
            ),
        ];
        for (name, image, port, extra, volumes) in services {
            let mut container = json!({"name":name,"image":image,"ports":[{"containerPort":port}],
                "resources":{"requests":{"cpu":"100m","memory":"128Mi"},"limits":{"cpu":"2","memory":"1Gi"}}});
            container
                .as_object_mut()
                .expect("container object")
                .extend(extra.as_object().expect("container fields").clone());
            resources.push(json!({"apiVersion":"apps/v1","kind":"Deployment","metadata":{"name":name},
                "spec":{"replicas":1,"selector":{"matchLabels":{"fixture":name}},"template":{
                    "metadata":{"labels":{"fixture":name}},"spec":{"automountServiceAccountToken":false,
                        "containers":[container],"volumes":volumes}}}}));
            resources.push(
                json!({"apiVersion":"v1","kind":"Service","metadata":{"name":name},
                "spec":{"selector":{"fixture":name},"ports":[{"port":port,"targetPort":port}]}}),
            );
        }
        let applications = json!({"podSelector":{"matchLabels":{"app.kubernetes.io/part-of":"hirune-kit"},
            "matchExpressions":[{"key":"app.kubernetes.io/component","operator":"In","values":["api","worker","maintenance"]}]}});
        for (name, port, from) in [
            (
                "seaweedfs",
                8333,
                json!([applications.clone(),{"podSelector":{"matchLabels":{"fixture":"storage-public"}}}]),
            ),
            ("meilisearch", 7700, json!([applications])),
            (
                "storage-public",
                8081,
                json!([{"podSelector":{"matchLabels":{
                "app.kubernetes.io/part-of":"hirune-kit","app.kubernetes.io/component":"tunnel"}}}]),
            ),
        ] {
            resources.push(json!({"apiVersion":"networking.k8s.io/v1","kind":"NetworkPolicy",
                "metadata":{"name":format!("fixture-{name}")},"spec":{"podSelector":{"matchLabels":{"fixture":name}},
                    "policyTypes":["Ingress"],"ingress":[{"from":from,"ports":[{"protocol":"TCP","port":port}]}]}}));
        }
        resources
    }

    pub(super) fn initialize_file_integrations(&self) -> Result<()> {
        let deployment = self.deployment()?;
        let worker = deployment
            .resources
            .iter()
            .find(|resource| {
                resource["kind"] == "Deployment" && resource["metadata"]["name"] == "hirune-worker"
            })
            .context("worker deployment missing")?;
        let mut template = worker["spec"]["template"].clone();
        template["spec"]["restartPolicy"] = json!("Never");
        template["spec"]["containers"][0]["command"] = json!(["manage", "initialize-integrations"]);
        crate::kubernetes_jobs::run(
            &deployment.target,
            json!({"apiVersion":"batch/v1","kind":"Job",
            "metadata":{"name":"fixture-initialize-integrations"},
            "spec":{"backoffLimit":0,"activeDeadlineSeconds":120,"template":template}}),
            Duration::from_secs(150),
        )?;
        // This Job uses worker ingress labels. Its terminal Pod must not block a later drain.
        crate::run(&mut self.kubectl(&[
            "delete",
            "job",
            "fixture-initialize-integrations",
            "--cascade=foreground",
            "--wait=true",
        ]))
    }
}
