use crate::files::FileState;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "files")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub uploader_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(150))")]
    pub object_key: String,
    #[sea_orm(column_type = "String(StringLen::N(255))")]
    pub filename: String,
    #[sea_orm(column_type = "String(StringLen::N(120))")]
    pub media_type: Option<String>,
    pub byte_size: i64,
    pub state: FileState,
    pub version: i64,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub content_hash: Option<String>,
    pub upload_expires_at: DateTimeUtc,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub search_reconciled_at: Option<DateTimeUtc>,
    pub search_document_may_exist: bool,
    pub cleanup_scheduled_at: Option<DateTimeUtc>,
    pub cleanup_completed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
