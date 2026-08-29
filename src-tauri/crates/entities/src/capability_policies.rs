// SPDX-License-Identifier: AGPL-3.0-only

//! 能力发现策略 —— 可注册的后置过滤器规则（Phase 3 策略对象化）。
//!
//! 策略不参与语义检索，仅对候选列表执行裁剪（如"内网环境删掉所有云 API 调用能力"）。
//! `rules_json` 为排除型规则：
//! ```json
//! {
//!   "exclude_domains": ["ai_media"],
//!   "exclude_tags": ["cloud_api"],
//!   "exclude_capability_ids": ["tool:web_search"]
//! }
//! ```

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "capability_policies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// 排除型规则 JSON（见模块注释）
    #[sea_orm(column_name = "rules_json")]
    pub rules_json: String,
    pub enabled: bool,
    /// 执行优先级（越小越先执行）
    pub priority: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
