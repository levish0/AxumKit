use super::entities::file;
use crate::{ApplicationResult, files::FileState, search::SearchIndex};
use uuid::Uuid;

pub use crate::search::SearchSettings;
#[derive(Clone)]
pub(super) struct FileSearch(SearchIndex);
impl FileSearch {
    pub fn new(settings: SearchSettings) -> ApplicationResult<Self> {
        Ok(Self(SearchIndex::new(settings)?))
    }
    pub async fn configure_index(&self) -> ApplicationResult<()> {
        self.0
            .initialize(&["filename"], &["organization_id"], &["id"])
            .await
    }
    pub async fn synchronize(&self, file: &file::Model) -> ApplicationResult<()> {
        if file.state == FileState::Ready {
            self.0.upsert(serde_json::json!({"id":file.id,"organization_id":file.organization_id,"filename":file.filename})).await
        } else {
            self.0.delete(file.id).await
        }
    }
    pub async fn candidates(
        &self,
        organization_id: Uuid,
        query: &str,
    ) -> ApplicationResult<Vec<Uuid>> {
        Ok(self
            .0
            .candidates(
                query,
                format!("organization_id = \"{organization_id}\""),
                100,
            )
            .await?
            .into_iter()
            .map(|hit| hit.id)
            .collect())
    }
}
