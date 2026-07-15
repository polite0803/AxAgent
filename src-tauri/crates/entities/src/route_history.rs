// SPDX-License-Identifier: AGPL-3.0-only

//! Smart Router 路由历史记录实体 —— 持久化 CostAwareRouter 的决策与反馈数据。
//!
//! 表 `route_history` 由 v101 migration 创建，存储每次路由决策及其后续反馈：
//! - 决策时：插入一行（outcome_* 字段为 NULL）
//! - 反馈到达：UPDATE 对应 prompt_hash 的 outcome_* 字段
//!
//! 程序启动时 CostAwareRouter::load_from_db 会读取全部历史并重建内存统计。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "route_history")]
pub struct Model {
    /// 主键 UUID。
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 提示词哈希，用于反馈回写时定位记录（建索引用）。
    pub prompt_hash: String,
    /// 提示词前 200 字预览，便于调试。
    #[sea_orm(column_type = "Text")]
    pub prompt_preview: String,
    /// 启发式分层建议（"budget" / "balanced" / "premium"）。
    pub heuristic_tier: String,
    /// 实际选中的分层（可能被 ML 覆盖）。
    pub selected_tier: String,
    /// 反馈：调用是否成功。
    pub outcome_success: Option<bool>,
    /// 反馈：用户质量评分 (0.0-1.0)。
    pub outcome_quality_score: Option<f64>,
    /// 反馈：用户是否手动切换分层。
    pub outcome_user_override: Option<bool>,
    /// 反馈：用户切换到的分层。
    pub outcome_user_tier: Option<String>,
    /// 反馈：实际延迟（毫秒）。
    pub outcome_latency_ms: Option<i64>,
    /// 反馈：实际消耗 token 数。
    pub outcome_tokens_used: Option<i64>,
    /// 反馈：实际成本（美元）。
    pub outcome_cost_usd: Option<f64>,
    /// 决策时间戳（Unix 秒）。
    pub timestamp: i64,
    /// TaskFeatureVector 的 JSON 序列化，用于相似度匹配重建。
    pub features_json: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
