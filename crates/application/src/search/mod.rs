use crate::pagination::SEARCH_WINDOW_LIMIT;
use crate::{ApplicationError, ApplicationResult, config::SecretString};
use futures_util::StreamExt;
use reqwest::{Client, Method};
use sea_orm::ExprTrait;
use serde::{Deserialize, de::DeserializeOwned};
use std::collections::HashSet;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SearchSettings {
    pub endpoint: Url,
    pub api_key: SecretString,
    pub index: String,
    pub allow_http: bool,
}

impl SearchSettings {
    pub fn validate(&self) -> ApplicationResult<()> {
        if !(self.endpoint.scheme() == "https"
            || (self.allow_http && self.endpoint.scheme() == "http"))
            || self.endpoint.host_str().is_none()
            || !self.endpoint.username().is_empty()
            || self.endpoint.password().is_some()
            || self.endpoint.query().is_some()
            || self.endpoint.fragment().is_some()
            || self.api_key.expose().is_empty()
            || self.index.is_empty()
            || self.index.len() > 64
            || !self
                .index
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ApplicationError::InvalidInput(
                "invalid Meilisearch settings",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct SearchIndex {
    client: Client,
    settings: SearchSettings,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmittedTask {
    task_uid: u64,
}
#[derive(Deserialize)]
struct TaskStatus {
    status: TaskState,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaskState {
    Enqueued,
    Processing,
    Succeeded,
    Failed,
    Canceled,
}
#[derive(Deserialize)]
pub(crate) struct SearchCandidate {
    pub id: Uuid,
    #[serde(default)]
    pub version: Option<i64>,
}
#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<SearchCandidate>,
}

impl SearchIndex {
    pub(crate) fn new(settings: SearchSettings) -> ApplicationResult<Self> {
        settings.validate()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ApplicationError::internal)?;
        Ok(Self { client, settings })
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> ApplicationResult<T> {
        let url = self
            .settings
            .endpoint
            .join(path)
            .map_err(ApplicationError::internal)?;
        let mut request = self
            .client
            .request(method, url)
            .bearer_auth(self.settings.api_key.expose());
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ApplicationError::IntegrationUnavailable)?;
        if !response.status().is_success() {
            return Err(ApplicationError::IntegrationUnavailable);
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| ApplicationError::IntegrationUnavailable)?;
            if bytes.len() + chunk.len() > 1024 * 1024 {
                return Err(ApplicationError::IntegrationUnavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| ApplicationError::IntegrationUnavailable)
    }

    async fn wait_for_task(&self, submitted: SubmittedTask) -> ApplicationResult<()> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let task: TaskStatus = self
                .request(Method::GET, &format!("tasks/{}", submitted.task_uid), None)
                .await?;
            match task.status {
                TaskState::Succeeded => return Ok(()),
                TaskState::Failed | TaskState::Canceled => {
                    return Err(ApplicationError::IntegrationUnavailable);
                }
                TaskState::Enqueued | TaskState::Processing => {}
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(ApplicationError::IntegrationUnavailable);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub(crate) async fn initialize(
        &self,
        searchable: &[&str],
        filterable: &[&str],
        displayed: &[&str],
    ) -> ApplicationResult<()> {
        let url = self
            .settings
            .endpoint
            .join(&format!("indexes/{}", self.settings.index))
            .map_err(ApplicationError::internal)?;
        let response = self
            .client
            .get(url)
            .bearer_auth(self.settings.api_key.expose())
            .send()
            .await
            .map_err(|_| ApplicationError::IntegrationUnavailable)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            let task = self
                .request(
                    Method::POST,
                    "indexes",
                    Some(serde_json::json!({"uid": self.settings.index, "primaryKey": "id"})),
                )
                .await?;
            self.wait_for_task(task).await?;
        } else if response.status().is_success() {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct IndexDescription {
                primary_key: Option<String>,
            }
            let index: IndexDescription = self
                .request(
                    Method::GET,
                    &format!("indexes/{}", self.settings.index),
                    None,
                )
                .await?;
            if index.primary_key.as_deref() != Some("id") {
                return Err(ApplicationError::InvalidInput(
                    "search index must use id as its primary key",
                ));
            }
        } else {
            return Err(ApplicationError::IntegrationUnavailable);
        }
        let task = self.request(Method::PATCH, &format!("indexes/{}/settings", self.settings.index),
            Some(serde_json::json!({"searchableAttributes": searchable, "filterableAttributes": filterable, "displayedAttributes": displayed, "pagination":{"maxTotalHits": SEARCH_WINDOW_LIMIT}}))).await?;
        self.wait_for_task(task).await
    }

    pub(crate) async fn upsert(&self, document: serde_json::Value) -> ApplicationResult<()> {
        let task = self
            .request(
                Method::POST,
                &format!("indexes/{}/documents?primaryKey=id", self.settings.index),
                Some(serde_json::json!([document])),
            )
            .await?;
        self.wait_for_task(task).await
    }

    pub(crate) async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        let task = self
            .request(
                Method::DELETE,
                &format!("indexes/{}/documents/{id}", self.settings.index),
                None,
            )
            .await?;
        self.wait_for_task(task).await
    }

    pub(crate) async fn candidates(
        &self,
        query: &str,
        filter: String,
        limit: usize,
    ) -> ApplicationResult<Vec<SearchCandidate>> {
        validate_query(query)?;
        let limit = limit.min(SEARCH_WINDOW_LIMIT);
        let response: SearchResponse = self.request(Method::POST, &format!("indexes/{}/search", self.settings.index),
            Some(serde_json::json!({"q":query,"filter":filter,"limit":limit,"attributesToRetrieve":["id","version"]}))).await?;
        let mut seen = HashSet::new();
        Ok(response
            .hits
            .into_iter()
            .take(limit)
            .filter(|hit| seen.insert(hit.id))
            .collect())
    }
}

pub fn validate_query(query: &str) -> ApplicationResult<()> {
    if query.trim().is_empty() || query.chars().count() > 200 || query.chars().any(char::is_control)
    {
        return Err(ApplicationError::InvalidInput(
            "search query must contain 1 to 200 characters",
        ));
    }
    Ok(())
}

pub(crate) fn versioned_candidates(
    hits: &[SearchCandidate],
    id: impl sea_orm::sea_query::IntoColumnRef + Clone,
    version: impl sea_orm::sea_query::IntoColumnRef + Clone,
) -> sea_orm::Condition {
    use sea_orm::sea_query::Expr;
    let mut condition = sea_orm::Condition::any().add(Expr::val(false));
    for hit in hits {
        if let Some(revision) = hit.version {
            condition = condition.add(
                sea_orm::Condition::all()
                    .add(Expr::col(id.clone()).eq(hit.id))
                    .add(Expr::col(version.clone()).eq(revision)),
            );
        }
    }
    condition
}
