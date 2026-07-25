// SPDX-License-Identifier: AGPL-3.0-only
//! 截图持仓诊断（Screenshot Diagnosis）实体
//!
//! 对应迁移：v206_screenshot_diagnosis
//! 用途：G6 截图持仓诊断完整闭环 —— 用户上传券商 App / 同花顺 /
//! 东方财富 / 雪球截图，LLM 视觉管线 OCR + 结构化解析 + 风险诊断，
//! 输出观察列表（可一键转为 paper_portfolio）。
//!
//! 数据流：
//! ```text
//! 用户上传截图
//!   → vision_pipeline OCR 提取文本
//!   → LLM 结构化解析为 positions_json
//!   → 风险诊断 schema 计算（7 项指标）
//!   → 持久化到 screenshot_diagnoses 表
//!   → 前端展示 + 一键转 paper_portfolio
//! ```

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 截图诊断记录
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "screenshot_diagnoses")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 截图 SHA256（用于去重，避免同一截图重复诊断）
    pub image_hash: Option<String>,
    /// 截图本地存储路径（可选，若存了原图）
    pub image_path: Option<String>,
    /// 缩略图 base64（可选，前端列表预览用）
    pub image_thumbnail_base64: Option<String>,
    /// 原图宽度
    pub image_width: Option<i32>,
    /// 原图高度
    pub image_height: Option<i32>,
    /// 截图来源 App（同花顺 / 东方财富 / 雪球 / 通达信 / 其他）
    pub source_app: Option<String>,
    /// LLM 视觉管线 OCR 提取的完整文本（debug 用）
    pub ocr_text: Option<String>,
    /// 结构化持仓 JSON 数组
    /// 每项含 code/name/qty/cost_price/market_value/weight
    #[sea_orm(default_value = "[]")]
    pub positions_json: String,
    /// 截图时刻总市值（用于权重计算）
    #[sea_orm(default_value = 0.0)]
    pub total_market_value: f64,
    /// 风险诊断 JSON（7 项指标）
    /// concentration_risk / overlap_positions / defense_ratio /
    /// us_exposure / weak_exposure / repeated_positions / core_concentration
    #[sea_orm(default_value = "{}")]
    pub diagnosis_json: String,
    /// LLM 自然语言诊断说明（1-3 段中文）
    #[sea_orm(default_value = "")]
    pub narrative: String,
    /// 建议动作 JSON 数组
    /// 如 ["减持 X" / "分散 Y" / "关注 Z 行业"]
    #[sea_orm(default_value = "[]")]
    pub recommended_actions: String,
    /// 来源工作流执行 ID（可空，手动上传则为 null）
    pub source_workflow_execution_id: Option<String>,
    /// 使用的 LLM provider ID（溯源 + 复算用）
    pub provider_id: Option<String>,
    /// 使用的 LLM model ID
    pub model_id: Option<String>,
    /// active / archived / failed
    #[sea_orm(default_value = "active")]
    pub status: String,
    /// 若失败，错误原因
    pub error_message: Option<String>,
    /// 创建时间戳（ms）
    pub created_at: i64,
    /// 更新时间戳（ms）
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
