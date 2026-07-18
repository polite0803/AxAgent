// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流执行统计 —— 记录每次工作流执行的效果数据（成功率/延迟/token 成本），
//! 用于驱动效果导向的工作流优化（区别于失败驱动的 replan）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_execution_stats")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 任务哈希（mission 文本的归一化哈希，用于聚合相同任务的不同执行）
    #[sea_orm(column_name = "mission_hash")]
    pub mission_hash: Option<String>,
    /// 关联工作流模板
    #[sea_orm(column_name = "template_id")]
    pub template_id: Option<String>,
    /// 关联工作流执行记录
    #[sea_orm(column_name = "execution_id")]
    pub execution_id: Option<String>,
    /// 执行状态：success / failed / partial / cancelled
    pub status: String,
    /// 总耗时（毫秒）
    #[sea_orm(column_name = "total_time_ms")]
    pub total_time_ms: i64,
    /// 输入 token 数
    #[sea_orm(column_name = "input_tokens")]
    pub input_tokens: i64,
    /// 输出 token 数
    #[sea_orm(column_name = "output_tokens")]
    pub output_tokens: i64,
    /// 失败原因（失败时填）
    #[sea_orm(column_name = "error_message")]
    pub error_message: Option<String>,
    /// 用户满意度反馈（可选，0.0 ~ 1.0）
    #[sea_orm(column_name = "user_rating")]
    pub user_rating: Option<f64>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
