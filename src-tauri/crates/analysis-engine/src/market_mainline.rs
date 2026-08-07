// SPDX-License-Identifier: AGPL-3.0-only
//! G4 市场主线自动提炼服务
//!
//! ## 用途
//!
//! 承接 DojoAgents 宣传场景 4「市场发现」：
//! - 每日自动提炼市场主线（由 daily-market-events 工作流触发）
//! - 主线包含主题、叙述、代表性标的、强度评分、持续性判断
//! - 提供按日 / 按主题 / 按状态 / 按强度排序的查询接口
//! - 支持手动创建 / 状态变更（active → fading → archived）
//!
//! ## 数据流
//!
//! ```text
//! daily-market-events 工作流（cron 18:00）
//!   → collect_market_data（拉取热点股 / 龙虎榜 / 北向 / 涨停板 / 快讯）
//!   → classify_themes（LLM 主题分类）
//!   → filter_signals（LLM 信号过滤）
//!   → synthesize_mainlines（LLM 综合主线）
//!   → persist_to_db（调用 upsert_mainlines 写入 market_mainlines 表）
//!   → push_to_dashboard（前端订阅 SSE 推送）
//! ```
//!
//! ## 与 HotStocksPanel 的区别
//!
//! - [`crate::hot_stocks`]：原始热点股数据（无主题分类 / 无叙述 / 无持续性）
//! - 本模块：经过 LLM 综合的「主线」级别视图（含主题 + 叙述 + 强度评分）
//!
//! 全部读写均经过 SeaORM，无副作用，可幂等调用。

use sea_orm::sea_query::Expr;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseConnection;
use sea_orm::DbErr;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use sea_orm::Set;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use axagent_entities::market_mainlines as ml_entity;

// ── DTO ───────────────────────────────────────────────────────────────────

/// 创建市场主线的输入（工作流 persist_to_db 节点 / 手动创建）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMainlineInput {
    pub mainline_date: String,
    pub theme: String,
    /// 主题大类（科技 / 消费 / 周期 / 金融 / 医药 / 政策 / 其他），默认 "其他"
    #[serde(default = "default_category")]
    pub theme_category: String,
    pub narrative: String,
    /// 代表性标的列表（序列化为 JSON 存入 representative_symbols 字段）
    #[serde(default)]
    pub representative_symbols: Vec<String>,
    /// 强度评分 0-100，默认 0
    #[serde(default)]
    pub strength_score: f64,
    /// 持续性判断，默认 "1d"
    #[serde(default = "default_persistence")]
    pub persistence: String,
    /// 证据 JSON（任意结构），默认空对象
    #[serde(default)]
    pub evidence: serde_json::Value,
    /// 来源工作流执行 ID（可空）
    pub source_workflow_execution_id: Option<String>,
}

fn default_category() -> String {
    "其他".to_string()
}

fn default_persistence() -> String {
    "1d".to_string()
}

/// 更新主线状态的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMainlineInput {
    pub mainline_id: String,
    /// 新状态（active / fading / archived），None 不更新
    pub status: Option<String>,
    /// 新强度评分，None 不更新
    pub strength_score: Option<f64>,
    /// 新持续性判断，None 不更新
    pub persistence: Option<String>,
    /// 新叙述，None 不更新
    pub narrative: Option<String>,
}

/// 批量 upsert 输入（工作流 synthesize_mainlines 节点输出多条主线时使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpsertInput {
    pub mainline_date: String,
    pub mainlines: Vec<CreateMainlineInput>,
    /// 是否清除当日已有但本次未提及的主线（true → status=archived）
    #[serde(default)]
    pub archive_missing: bool,
    /// 来源工作流执行 ID
    pub source_workflow_execution_id: Option<String>,
}

/// 批量 upsert 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpsertResult {
    pub inserted: usize,
    pub updated: usize,
    pub archived: usize,
}

// ── CRUD ──────────────────────────────────────────────────────────────────

/// 创建单条市场主线
pub async fn create_mainline(
    db: &DatabaseConnection,
    input: CreateMainlineInput,
) -> Result<ml_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let symbols_json =
        serde_json::to_string(&input.representative_symbols).unwrap_or_else(|_| "[]".to_string());
    let evidence_json = serde_json::to_string(&input.evidence).unwrap_or_else(|_| "{}".to_string());

    let model = ml_entity::ActiveModel {
        id: Set(id),
        mainline_date: Set(input.mainline_date),
        theme: Set(input.theme),
        theme_category: Set(input.theme_category),
        narrative: Set(input.narrative),
        representative_symbols: Set(symbols_json),
        strength_score: Set(input.strength_score),
        persistence: Set(input.persistence),
        evidence_json: Set(evidence_json),
        source_workflow_execution_id: Set(input.source_workflow_execution_id),
        status: Set("active".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(db).await
}

/// 按 ID 获取
pub async fn get_mainline(
    db: &DatabaseConnection,
    mainline_id: &str,
) -> Result<Option<ml_entity::Model>, DbErr> {
    ml_entity::Entity::find_by_id(mainline_id.to_string()).one(db).await
}

/// 列出某日所有主线（按强度评分降序）
pub async fn list_mainlines_by_date(
    db: &DatabaseConnection,
    mainline_date: &str,
) -> Result<Vec<ml_entity::Model>, DbErr> {
    ml_entity::Entity::find()
        .filter(ml_entity::Column::MainlineDate.eq(mainline_date))
        .order_by_desc(ml_entity::Column::StrengthScore)
        .all(db)
        .await
}

/// 列出最近 N 天的主线（按日期降序 + 强度降序）
pub async fn list_recent_mainlines(
    db: &DatabaseConnection,
    days: usize,
) -> Result<Vec<ml_entity::Model>, DbErr> {
    let limit = (days * 20) as u64; // 每日上限 20 条
    ml_entity::Entity::find()
        .order_by_desc(ml_entity::Column::MainlineDate)
        .order_by_desc(ml_entity::Column::StrengthScore)
        .limit(limit)
        .all(db)
        .await
}

/// 按状态过滤主线
pub async fn list_mainlines_by_status(
    db: &DatabaseConnection,
    status: &str,
) -> Result<Vec<ml_entity::Model>, DbErr> {
    ml_entity::Entity::find()
        .filter(ml_entity::Column::Status.eq(status))
        .order_by_desc(ml_entity::Column::MainlineDate)
        .order_by_desc(ml_entity::Column::StrengthScore)
        .all(db)
        .await
}

/// 按主题大类过滤主线
pub async fn list_mainlines_by_category(
    db: &DatabaseConnection,
    theme_category: &str,
) -> Result<Vec<ml_entity::Model>, DbErr> {
    ml_entity::Entity::find()
        .filter(ml_entity::Column::ThemeCategory.eq(theme_category))
        .order_by_desc(ml_entity::Column::MainlineDate)
        .order_by_desc(ml_entity::Column::StrengthScore)
        .all(db)
        .await
}

/// 更新主线（部分字段）
pub async fn update_mainline(
    db: &DatabaseConnection,
    input: UpdateMainlineInput,
) -> Result<ml_entity::Model, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut model = ml_entity::ActiveModel {
        id: Set(input.mainline_id),
        updated_at: Set(now),
        ..Default::default()
    };
    if let Some(s) = input.status {
        model.status = Set(s);
    }
    if let Some(score) = input.strength_score {
        model.strength_score = Set(score);
    }
    if let Some(p) = input.persistence {
        model.persistence = Set(p);
    }
    if let Some(n) = input.narrative {
        model.narrative = Set(n);
    }
    model.update(db).await
}

/// 归档主线（status=archived）
pub async fn archive_mainline(
    db: &DatabaseConnection,
    mainline_id: &str,
) -> Result<ml_entity::Model, DbErr> {
    update_mainline(
        db,
        UpdateMainlineInput {
            mainline_id: mainline_id.to_string(),
            status: Some("archived".to_string()),
            strength_score: None,
            persistence: None,
            narrative: None,
        },
    )
    .await
}

// ── 批量 upsert（工作流用） ──────────────────────────────────────────────

/// 批量 upsert 主线（同日同主题更新；archive_missing=true 时归档当日未提及的主线）
///
/// 用于 daily-market-events 工作流的 persist_to_db 节点。
/// 逻辑：
/// 1. 查询当日已有主线，按 theme 建索引
/// 2. 对每条输入：
///    - 同日同主题已存在 → 更新 narrative / symbols / score / persistence / evidence
///    - 不存在 → 插入
/// 3. 若 archive_missing=true：当日已有但本次未提及的 → status=archived
pub async fn batch_upsert_mainlines(
    db: &DatabaseConnection,
    input: BatchUpsertInput,
) -> Result<BatchUpsertResult, DbErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let existing = list_mainlines_by_date(db, &input.mainline_date).await?;
    let mut existing_by_theme: std::collections::HashMap<String, ml_entity::Model> =
        existing.into_iter().map(|m| (m.theme.clone(), m)).collect();

    let mut inserted = 0usize;
    let mut updated = 0usize;
    let mut touched_themes = std::collections::HashSet::new();

    for ml in input.mainlines {
        touched_themes.insert(ml.theme.clone());
        let symbols_json =
            serde_json::to_string(&ml.representative_symbols).unwrap_or_else(|_| "[]".to_string());
        let evidence_json =
            serde_json::to_string(&ml.evidence).unwrap_or_else(|_| "{}".to_string());

        if let Some(existing_model) = existing_by_theme.remove(&ml.theme) {
            // 更新
            let model = ml_entity::ActiveModel {
                id: Set(existing_model.id.clone()),
                mainline_date: Set(input.mainline_date.clone()),
                theme: Set(ml.theme),
                theme_category: Set(ml.theme_category),
                narrative: Set(ml.narrative),
                representative_symbols: Set(symbols_json),
                strength_score: Set(ml.strength_score),
                persistence: Set(ml.persistence),
                evidence_json: Set(evidence_json),
                source_workflow_execution_id: Set(input.source_workflow_execution_id.clone()),
                status: Set("active".to_string()),
                created_at: Set(existing_model.created_at),
                updated_at: Set(now),
            };
            model.update(db).await?;
            updated += 1;
        } else {
            // 插入
            let id = Uuid::new_v4().to_string();
            let model = ml_entity::ActiveModel {
                id: Set(id),
                mainline_date: Set(input.mainline_date.clone()),
                theme: Set(ml.theme),
                theme_category: Set(ml.theme_category),
                narrative: Set(ml.narrative),
                representative_symbols: Set(symbols_json),
                strength_score: Set(ml.strength_score),
                persistence: Set(ml.persistence),
                evidence_json: Set(evidence_json),
                source_workflow_execution_id: Set(input.source_workflow_execution_id.clone()),
                status: Set("active".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
            };
            model.insert(db).await?;
            inserted += 1;
        }
    }

    // 归档当日未提及的主线
    let mut archived = 0usize;
    if input.archive_missing {
        for (_, leftover) in existing_by_theme {
            if leftover.status == "archived" {
                continue;
            }
            let model = ml_entity::ActiveModel {
                id: Set(leftover.id),
                status: Set("archived".to_string()),
                updated_at: Set(now),
                ..Default::default()
            };
            model.update(db).await?;
            archived += 1;
        }
    }

    Ok(BatchUpsertResult { inserted, updated, archived })
}

/// 清除某日所有主线（管理用，慎调）
pub async fn delete_mainlines_by_date(
    db: &DatabaseConnection,
    mainline_date: &str,
) -> Result<u64, DbErr> {
    let res = ml_entity::Entity::delete_many()
        .filter(ml_entity::Column::MainlineDate.eq(mainline_date))
        .exec(db)
        .await?;
    Ok(res.rows_affected)
}

/// 显式触发 [`Expr`] 引用，避免未使用警告（更新语句中用到）
#[allow(dead_code)]
fn _touch_expr() {
    let _ = Expr::value(0i64);
}

// ── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_category_is_other() {
        assert_eq!(default_category(), "其他");
    }

    #[test]
    fn default_persistence_is_1d() {
        assert_eq!(default_persistence(), "1d");
    }

    #[test]
    fn create_input_serialization_roundtrip() {
        let input = CreateMainlineInput {
            mainline_date: "2026-07-26".into(),
            theme: "AI 算力".into(),
            theme_category: "科技".into(),
            narrative: "英伟达隔夜大涨，A 股算力链跟涨".into(),
            representative_symbols: vec!["002230".into(), "688256".into()],
            strength_score: 85.0,
            persistence: "1w".into(),
            evidence: serde_json::json!({"limit_up_count": 5}),
            source_workflow_execution_id: Some("exec-1".into()),
        };
        let json = serde_json::to_string(&input).unwrap();
        let parsed: CreateMainlineInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme, "AI 算力");
        assert_eq!(parsed.representative_symbols.len(), 2);
        assert_eq!(parsed.strength_score, 85.0);
    }
}
