// SPDX-License-Identifier: AGPL-3.0-only
//! 市场主线（Market Mainline）实体
//!
//! 对应迁移：v205_market_mainline
//! 用途：G4 市场主线自动提炼 —— 每日由 daily-market-events 工作流自动产出，
//! 每条主线包含主题、叙述、代表性标的、强度评分、持续性判断。
//!
//! 数据流：
//! ```text
//! daily-market-events 工作流（cron 18:00）
//!   → collect_market_data → classify_themes → filter_signals → synthesize_mainlines
//!   → persist_to_db（写入 market_mainlines 表）
//!   → push_to_dashboard（前端订阅 SSE 推送）
//! ```

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 市场主线记录
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "market_mainlines")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 主线日期 YYYY-MM-DD
    pub mainline_date: String,
    /// 主题名（如 "AI 算力" / "光模块" / "新能源车"）
    pub theme: String,
    /// 主题大类（科技 / 消费 / 周期 / 金融 / 医药 / 政策 / 其他）
    #[sea_orm(default_value = "其他")]
    pub theme_category: String,
    /// 主线叙述（LLM 综合的 1-2 句话故事线）
    pub narrative: String,
    /// 代表性标的 JSON 数组（如 ["600519","000858"]）
    #[sea_orm(default_value = "[]")]
    pub representative_symbols: String,
    /// 强度评分 0-100
    #[sea_orm(default_value = 0.0)]
    pub strength_score: f64,
    /// 持续性判断（"1d" / "1w" / "1m" / "fading" / "emerging"）
    #[sea_orm(default_value = "1d")]
    pub persistence: String,
    /// 证据 JSON（涨停股 / 北向净流入 / 龙虎榜数据等原始数据快照）
    #[sea_orm(default_value = "{}")]
    pub evidence_json: String,
    /// 来源工作流执行 ID（可空，手动创建则为 null）
    pub source_workflow_execution_id: Option<String>,
    /// active / fading / archived
    #[sea_orm(default_value = "active")]
    pub status: String,
    /// 创建时间戳（ms）
    pub created_at: i64,
    /// 更新时间戳（ms）
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
