// NOTE: import_jobs 表已在数据库迁移中定义但尚未被任何导入命令写入。
// 这是预留的统一导入追踪基础设施，计划后续接入各导入命令（workflow_template、
// agency_expert、agent_role 等）以提供导入进度、冲突和结果的可视化追踪。
// 待实现时，各导入命令应在操作开始时创建 job 记录（status="running"），
// 完成后更新 status="completed" 并填充 summary_json。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "import_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub source_type: String,
    pub status: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub summary_json: Option<String>,
    pub conflict_count: i32,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
