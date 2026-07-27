use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "memory_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub namespace_id: String,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub source: String,
    pub index_status: String,
    pub index_error: Option<String>,
    pub updated_at: String,
    // v101: trajectory memory fields
    pub tier: String,
    pub importance: f64,
    pub access_count: i32,
    pub last_accessed: Option<i64>,
    pub decay_rate: f64,
    pub expires_at: Option<i64>,
    pub source_conversation_id: Option<String>,
    pub source_message_id: Option<String>,
    pub memory_nature: String,
    pub tags: String,
    // v108: 自进化闭环 — 记忆适用范围边界 + 人工确认门
    /// JSON 数组字符串，如 `["rust","frontend"]`，标记记忆的适用范围边界。
    /// RAG 检索时可按当前任务上下文标签过滤，降低无关记忆干扰。
    /// 默认 '[]' 表示不限制。
    pub applicability_tags: String,
    /// 0=未确认（Reflector 自动沉淀的默认状态），1=已人工确认。
    /// 晋升到 core 层（promote_memory_entry）需要 confirmed=1 门槛。
    pub confirmed: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::memory_namespaces::Entity",
        from = "Column::NamespaceId",
        to = "super::memory_namespaces::Column::Id",
        on_delete = "Cascade"
    )]
    MemoryNamespace,
}

impl Related<super::memory_namespaces::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MemoryNamespace.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
