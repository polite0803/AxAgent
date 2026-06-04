use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "background_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    /// "bash" | "agent"
    pub task_type: String,
    /// Shell 命令（bash 类型）
    #[sea_orm(column_type = "Text", nullable)]
    pub command: Option<String>,
    /// Agent prompt（agent 类型）
    #[sea_orm(column_type = "Text", nullable)]
    pub prompt: Option<String>,
    /// pending | running | completed | failed | stopped
    pub status: String,
    /// 累积输出（追加写入）
    #[sea_orm(column_type = "Text")]
    pub output: String,
    /// 退出码（bash 类型完成时）
    pub exit_code: Option<i32>,
    /// 关联的会话 ID（agent 类型）
    pub conversation_id: Option<String>,
    /// 创建者会话 ID
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
