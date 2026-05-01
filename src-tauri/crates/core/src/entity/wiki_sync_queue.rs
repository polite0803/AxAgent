use sea_orm::entity::prelude::*;
use sea_orm::{NotSet, Set};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wiki_sync_queue")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub wiki_id: String,
    pub event_type: String,
    pub target_type: String,
    pub target_id: String,
    pub payload: Option<Json>,
    pub status: String,
    pub retry_count: i32,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub processed_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn new_pending(
        wiki_id: String,
        event_type: String,
        target_type: String,
        target_id: String,
        payload: Option<Json>,
    ) -> ActiveModel {
        ActiveModel {
            id: NotSet,
            wiki_id: Set(wiki_id),
            event_type: Set(event_type),
            target_type: Set(target_type),
            target_id: Set(target_id),
            payload: Set(payload),
            status: Set("pending".to_string()),
            retry_count: Set(0),
            error_message: Set(None),
            created_at: Set(chrono::Utc::now().timestamp()),
            processed_at: Set(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub wiki_id: String,
    pub event_type: SyncEventType,
    pub target_type: String,
    pub target_id: String,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncEventType {
    NoteCreated,
    NoteUpdated,
    NoteDeleted,
    SourceIngested,
    SchemaUpdated,
    WikiCreated,
    WikiDeleted,
}

impl SyncEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncEventType::NoteCreated => "note_created",
            SyncEventType::NoteUpdated => "note_updated",
            SyncEventType::NoteDeleted => "note_deleted",
            SyncEventType::SourceIngested => "source_ingested",
            SyncEventType::SchemaUpdated => "schema_updated",
            SyncEventType::WikiCreated => "wiki_created",
            SyncEventType::WikiDeleted => "wiki_deleted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub pending_count: i64,
    pub processing_count: i64,
    pub failed_count: i64,
    pub last_sync_at: Option<i64>,
}
