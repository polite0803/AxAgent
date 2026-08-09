//! 交易意图服务 — 安全自动化的交易记录层
//!
//! ## 设计理念
//!
//! 在"不执行真实交易"的前提下，自动捕获所有分析决策、条件单触发、
//! 量化信号，生成结构化的交易意图记录供人工审核。
//!
//! ## 状态流转
//!
//! ```text
//! pending (待审核) → reviewed (已审核) → executed (已执行)
//!                                   → rejected (已驳回)
//!                                   → expired (已过期)
//! ```
//!
//! ## 数据源
//!
//! - stock_analyses 决策字段 (decision_action / decision_position_pct)
//! - ConditionalOrderEngine 触发事件
//! - quant_signals 信号
//!
//! ## 安全保证
//!
//! - 本服务仅记录意图，不执行任何真实交易
//! - 所有状态变更均需人工触发（reviewed/executed/rejected）
//! - 审核操作留下完整审计痕迹

use axagent_entities::stock_analyses;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 交易意图状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TradeIntentStatus {
    /// 待审核
    Pending,
    /// 已审核（人工确认）
    Reviewed,
    /// 已执行（关联到真实交易）
    Executed,
    /// 已过期（未在有效期内处理）
    Expired,
    /// 已驳回
    Rejected,
}

impl TradeIntentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Reviewed => "reviewed",
            Self::Executed => "executed",
            Self::Expired => "expired",
            Self::Rejected => "rejected",
        }
    }
}

impl std::str::FromStr for TradeIntentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "reviewed" => Ok(Self::Reviewed),
            "executed" => Ok(Self::Executed),
            "expired" => Ok(Self::Expired),
            "rejected" => Ok(Self::Rejected),
            "pending" => Ok(Self::Pending),
            _ => Err(format!("unknown TradeIntentStatus: {s}")),
        }
    }
}

/// 交易意图来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TradeIntentSource {
    /// 分析决策
    Analysis,
    /// 条件单触发
    ConditionalOrder,
    /// 量化信号
    QuantSignal,
    /// 组合监控
    PortfolioMonitor,
}

impl TradeIntentSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Analysis => "analysis",
            Self::ConditionalOrder => "conditional_order",
            Self::QuantSignal => "quant_signal",
            Self::PortfolioMonitor => "portfolio_monitor",
        }
    }
}

/// 待审核交易意图列表项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeIntentItem {
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub source: String,
    pub source_ref_id: Option<String>,
    pub decision_action: Option<String>,
    pub decision_position_pct: Option<f64>,
    pub decision_reasoning: Option<String>,
    pub status: String,
    pub trade_intent_status: String,
    pub trade_intent_source: Option<String>,
    pub reviewed_at: Option<i64>,
    pub reviewed_by: Option<String>,
    pub review_notes: Option<String>,
    pub actual_trade_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 交易意图审核请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewTradeIntentRequest {
    pub analysis_id: String,
    pub reviewed_by: String,
    pub notes: Option<String>,
}

/// 交易意图审核结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewTradeIntentResult {
    pub success: bool,
    pub analysis_id: String,
    pub new_status: String,
}

/// 交易意图服务
pub struct TradeIntentService;

impl TradeIntentService {
    /// 分析完成后自动记录交易意图
    ///
    /// 在分析引擎完成决策后调用，将决策字段自动标记为"待审核"状态。
    /// 如果分析结果为中性（持有/观望），则不生成交易意图。
    pub async fn record_analysis_intent(
        db: &DatabaseConnection,
        analysis_id: &str,
        source: TradeIntentSource,
        source_ref_id: Option<String>,
    ) -> Result<(), String> {
        let now = now_ms();

        let model = stock_analyses::Entity::find_by_id(analysis_id)
            .one(db)
            .await
            .map_err(|e| format!("查询分析记录失败: {e}"))?
            .ok_or_else(|| format!("分析记录不存在: {analysis_id}"))?;

        // 只有当分析有明确的决策时才生成交易意图
        let decision_action = model.decision_action.clone();
        let has_decision = decision_action
            .as_deref()
            .map(|a| !matches!(a, "持有" | "观望" | "hold" | "watch"))
            .unwrap_or(false);

        if !has_decision {
            tracing::debug!("[trade_intent] 分析 {} 无明确交易决策，跳过意图记录", analysis_id);
            return Ok(());
        }

        let mut active: stock_analyses::ActiveModel = model.into();
        active.trade_intent_status = Set(TradeIntentStatus::Pending.as_str().into());
        active.trade_intent_source = Set(Some(source.as_str().into()));
        active.trade_intent_source_ref_id = Set(source_ref_id);
        active.updated_at = Set(now);

        active.save(db).await.map_err(|e| format!("更新交易意图状态失败: {e}"))?;

        tracing::info!(
            "[trade_intent] 已记录交易意图: analysis_id={} source={} action={:?}",
            analysis_id,
            source.as_str(),
            decision_action
        );

        Ok(())
    }

    /// 条件单触发时自动记录交易意图
    pub async fn record_conditional_order_intent(
        db: &DatabaseConnection,
        stock_code: &str,
        stock_name: &str,
        order_id: &str,
        action: &str,
        reasoning: &str,
        decision_json: Option<String>,
    ) -> Result<String, String> {
        let now = now_ms();
        let id = uuid::Uuid::new_v4().to_string();

        // 创建一条新的 stock_analyses 记录作为条件单触发的交易意图
        let model = stock_analyses::ActiveModel {
            id: Set(id.clone()),
            stock_code: Set(stock_code.to_string()),
            stock_name: Set(stock_name.to_string()),
            analysis_date: Set(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            provider_id: Set("system".to_string()),
            conversation_id: Set(format!("co-{order_id}")),
            status: Set("completed".to_string()),
            decision_action: Set(Some(action.to_string())),
            decision_position_pct: Set(None),
            decision_reasoning: Set(Some(reasoning.to_string())),
            decision_json: Set(decision_json),
            blackboard_snapshot: Set(None),
            config_id: Set(None),
            analysis_kind: Set("live".to_string()),
            as_of_date: Set(None),
            decision_time_horizon: Set(None),
            decision_expected_holding_days: Set(None),
            model_version: Set(None),
            data_snapshot_id: Set(None),
            outcome: Set(None),
            llm_decision_json: Set(None),
            parent_analysis_id: Set(None),
            trade_intent_status: Set(TradeIntentStatus::Pending.as_str().to_string()),
            trade_intent_source: Set(Some(
                TradeIntentSource::ConditionalOrder.as_str().to_string(),
            )),
            trade_intent_source_ref_id: Set(Some(order_id.to_string())),
            trade_intent_reviewed_at: Set(None),
            trade_intent_reviewed_by: Set(None),
            trade_intent_review_notes: Set(None),
            trade_intent_actual_trade_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        model.insert(db).await.map_err(|e| format!("创建条件单交易意图失败: {e}"))?;

        tracing::info!(
            "[trade_intent] 条件单触发交易意图已记录: order_id={} stock={}",
            order_id,
            stock_code
        );

        Ok(id)
    }

    /// 审核交易意图（通过）
    pub async fn approve_intent(
        db: &DatabaseConnection,
        req: ReviewTradeIntentRequest,
    ) -> Result<ReviewTradeIntentResult, String> {
        Self::transition_status(
            db,
            &req.analysis_id,
            TradeIntentStatus::Pending,
            TradeIntentStatus::Reviewed,
            req.reviewed_by,
            req.notes,
            None,
        )
        .await
    }

    /// 驳回交易意图
    pub async fn reject_intent(
        db: &DatabaseConnection,
        req: ReviewTradeIntentRequest,
    ) -> Result<ReviewTradeIntentResult, String> {
        Self::transition_status(
            db,
            &req.analysis_id,
            TradeIntentStatus::Pending,
            TradeIntentStatus::Rejected,
            req.reviewed_by,
            req.notes,
            None,
        )
        .await
    }

    /// 关联实际交易（执行后标记为 executed）
    pub async fn link_actual_trade(
        db: &DatabaseConnection,
        analysis_id: &str,
        trade_id: &str,
        reviewed_by: &str,
    ) -> Result<ReviewTradeIntentResult, String> {
        Self::transition_status(
            db,
            analysis_id,
            TradeIntentStatus::Reviewed,
            TradeIntentStatus::Executed,
            reviewed_by.to_string(),
            None,
            Some(trade_id.to_string()),
        )
        .await
    }

    /// 查询待审核的交易意图列表
    pub async fn list_pending(
        db: &DatabaseConnection,
        limit: u64,
    ) -> Result<Vec<TradeIntentItem>, String> {
        Self::list_by_status(db, TradeIntentStatus::Pending, limit).await
    }

    /// 查询指定状态的交易意图列表
    pub async fn list_by_status(
        db: &DatabaseConnection,
        status: TradeIntentStatus,
        limit: u64,
    ) -> Result<Vec<TradeIntentItem>, String> {
        let rows = stock_analyses::Entity::find()
            .filter(stock_analyses::Column::TradeIntentStatus.eq(status.as_str()))
            .order_by_desc(stock_analyses::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await
            .map_err(|e| format!("查询交易意图列表失败: {e}"))?;

        Ok(rows.into_iter().map(to_item).collect())
    }

    /// 查询某只股票的交易意图历史
    pub async fn list_by_stock(
        db: &DatabaseConnection,
        stock_code: &str,
        limit: u64,
    ) -> Result<Vec<TradeIntentItem>, String> {
        let rows = stock_analyses::Entity::find()
            .filter(stock_analyses::Column::StockCode.eq(stock_code))
            .filter(
                stock_analyses::Column::TradeIntentStatus
                    .is_in(["pending", "reviewed", "executed", "rejected", "expired"]),
            )
            .order_by_desc(stock_analyses::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await
            .map_err(|e| format!("查询股票交易意图失败: {e}"))?;

        Ok(rows.into_iter().map(to_item).collect())
    }

    /// 批量过期处理（将超时的 pending 标记为 expired）
    pub async fn expire_old_intents(
        db: &DatabaseConnection,
        max_age_hours: i64,
    ) -> Result<u64, String> {
        let cutoff = now_ms() - max_age_hours * 3600 * 1000;

        let result = stock_analyses::Entity::update_many()
            .col_expr(
                stock_analyses::Column::TradeIntentStatus,
                sea_orm::sea_query::Expr::value(TradeIntentStatus::Expired.as_str()),
            )
            .col_expr(stock_analyses::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now_ms()))
            .filter(
                stock_analyses::Column::TradeIntentStatus.eq(TradeIntentStatus::Pending.as_str()),
            )
            .filter(stock_analyses::Column::CreatedAt.lt(cutoff))
            .exec(db)
            .await
            .map_err(|e| format!("过期处理失败: {e}"))?;

        if result.rows_affected > 0 {
            tracing::info!(
                "[trade_intent] 已过期 {} 条交易意图（超过 {} 小时）",
                result.rows_affected,
                max_age_hours
            );
        }

        Ok(result.rows_affected)
    }

    // ── 内部方法 ──

    async fn transition_status(
        db: &DatabaseConnection,
        analysis_id: &str,
        _from: TradeIntentStatus,
        to: TradeIntentStatus,
        reviewed_by: String,
        notes: Option<String>,
        actual_trade_id: Option<String>,
    ) -> Result<ReviewTradeIntentResult, String> {
        let now = now_ms();

        let model = stock_analyses::Entity::find_by_id(analysis_id)
            .one(db)
            .await
            .map_err(|e| format!("查询分析记录失败: {e}"))?
            .ok_or_else(|| format!("分析记录不存在: {analysis_id}"))?;

        let mut active: stock_analyses::ActiveModel = model.into();
        active.trade_intent_status = Set(to.as_str().to_string());
        active.trade_intent_reviewed_at = Set(Some(now));
        active.trade_intent_reviewed_by = Set(Some(reviewed_by));
        if let Some(notes_val) = notes {
            active.trade_intent_review_notes = Set(Some(notes_val));
        }
        if let Some(trade_id) = actual_trade_id {
            active.trade_intent_actual_trade_id = Set(Some(trade_id));
        }
        active.updated_at = Set(now);

        active.save(db).await.map_err(|e| format!("更新交易意图状态失败: {e}"))?;

        tracing::info!("[trade_intent] 交易意图状态变更: id={} →{}", analysis_id, to.as_str());

        Ok(ReviewTradeIntentResult {
            success: true,
            analysis_id: analysis_id.to_string(),
            new_status: to.as_str().to_string(),
        })
    }
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn to_item(model: stock_analyses::Model) -> TradeIntentItem {
    TradeIntentItem {
        id: model.id,
        stock_code: model.stock_code,
        stock_name: model.stock_name,
        source: model.status.clone(),
        source_ref_id: model.parent_analysis_id,
        decision_action: model.decision_action,
        decision_position_pct: model.decision_position_pct,
        decision_reasoning: model.decision_reasoning,
        status: model.status,
        trade_intent_status: model.trade_intent_status,
        trade_intent_source: model.trade_intent_source,
        reviewed_at: model.trade_intent_reviewed_at,
        reviewed_by: model.trade_intent_reviewed_by,
        review_notes: model.trade_intent_review_notes,
        actual_trade_id: model.trade_intent_actual_trade_id,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ConnectionTrait;

    /// 在 SQLite 内存库中创建 stock_analyses 表
    async fn setup_db() -> DatabaseConnection {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE stock_analyses (
                id TEXT PRIMARY KEY NOT NULL,
                stock_code TEXT NOT NULL,
                stock_name TEXT NOT NULL,
                analysis_date TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                status TEXT NOT NULL,
                decision_action TEXT,
                decision_position_pct REAL,
                decision_reasoning TEXT,
                decision_json TEXT,
                blackboard_snapshot TEXT,
                config_id TEXT,
                analysis_kind TEXT NOT NULL DEFAULT 'live',
                as_of_date TEXT,
                decision_time_horizon TEXT,
                decision_expected_holding_days INTEGER,
                model_version TEXT,
                data_snapshot_id TEXT,
                outcome TEXT,
                llm_decision_json TEXT,
                parent_analysis_id TEXT,
                trade_intent_status TEXT NOT NULL DEFAULT 'pending',
                trade_intent_source TEXT,
                trade_intent_source_ref_id TEXT,
                trade_intent_reviewed_at INTEGER,
                trade_intent_reviewed_by TEXT,
                trade_intent_review_notes TEXT,
                trade_intent_actual_trade_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .await
        .unwrap();
        db
    }

    #[test]
    fn test_status_transitions() {
        assert_eq!(TradeIntentStatus::Pending.as_str(), "pending");
        assert_eq!(TradeIntentStatus::Reviewed.as_str(), "reviewed");
        assert_eq!(TradeIntentStatus::Executed.as_str(), "executed");
        assert_eq!(TradeIntentStatus::Expired.as_str(), "expired");
        assert_eq!(TradeIntentStatus::Rejected.as_str(), "rejected");

        use std::str::FromStr;
        assert_eq!(TradeIntentStatus::from_str("pending").unwrap(), TradeIntentStatus::Pending);
        assert_eq!(TradeIntentStatus::from_str("reviewed").unwrap(), TradeIntentStatus::Reviewed);
        assert_eq!(TradeIntentStatus::from_str("executed").unwrap(), TradeIntentStatus::Executed);
        assert_eq!(TradeIntentStatus::from_str("expired").unwrap(), TradeIntentStatus::Expired);
        assert_eq!(TradeIntentStatus::from_str("rejected").unwrap(), TradeIntentStatus::Rejected);
        assert!(TradeIntentStatus::from_str("unknown").is_err());
    }

    #[test]
    fn test_source_strings() {
        assert_eq!(TradeIntentSource::Analysis.as_str(), "analysis");
        assert_eq!(TradeIntentSource::ConditionalOrder.as_str(), "conditional_order");
        assert_eq!(TradeIntentSource::QuantSignal.as_str(), "quant_signal");
        assert_eq!(TradeIntentSource::PortfolioMonitor.as_str(), "portfolio_monitor");
    }

    #[tokio::test]
    async fn test_record_analysis_intent_no_decision() {
        let db = setup_db().await;

        // 创建一条无交易决策的分析记录
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let model = stock_analyses::ActiveModel {
            id: Set(id.clone()),
            stock_code: Set("600519".to_string()),
            stock_name: Set("贵州茅台".to_string()),
            analysis_date: Set("2026-08-09".to_string()),
            provider_id: Set("test".to_string()),
            conversation_id: Set("test-conv".to_string()),
            status: Set("completed".to_string()),
            decision_action: Set(Some("持有".to_string())),
            decision_position_pct: Set(Some(0.0)),
            decision_reasoning: Set(Some("观望中".to_string())),
            decision_json: Set(None),
            blackboard_snapshot: Set(None),
            config_id: Set(None),
            analysis_kind: Set("live".to_string()),
            as_of_date: Set(None),
            decision_time_horizon: Set(None),
            decision_expected_holding_days: Set(None),
            model_version: Set(None),
            data_snapshot_id: Set(None),
            outcome: Set(None),
            llm_decision_json: Set(None),
            parent_analysis_id: Set(None),
            trade_intent_status: Set("pending".to_string()),
            trade_intent_source: Set(None),
            trade_intent_source_ref_id: Set(None),
            trade_intent_reviewed_at: Set(None),
            trade_intent_reviewed_by: Set(None),
            trade_intent_review_notes: Set(None),
            trade_intent_actual_trade_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model.insert(&db).await.unwrap();

        // 尝试记录：应为中性决策，不生成意图
        let result =
            TradeIntentService::record_analysis_intent(&db, &id, TradeIntentSource::Analysis, None)
                .await;
        assert!(result.is_ok());

        // 验证 trade_intent_status 未被修改（仍为默认 pending，但 source 应为 None）
        let record = stock_analyses::Entity::find_by_id(&id).one(&db).await.unwrap().unwrap();
        // 持有决策不应被标记为待审核交易意图
        assert_eq!(record.trade_intent_source, None);
    }

    #[tokio::test]
    async fn test_record_analysis_intent_with_decision() {
        let db = setup_db().await;

        // 创建有买入决策的分析记录
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let model = stock_analyses::ActiveModel {
            id: Set(id.clone()),
            stock_code: Set("600519".to_string()),
            stock_name: Set("贵州茅台".to_string()),
            analysis_date: Set("2026-08-09".to_string()),
            provider_id: Set("test".to_string()),
            conversation_id: Set("test-conv".to_string()),
            status: Set("completed".to_string()),
            decision_action: Set(Some("买入".to_string())),
            decision_position_pct: Set(Some(0.3)),
            decision_reasoning: Set(Some("技术面突破".to_string())),
            decision_json: Set(Some(
                serde_json::json!({
                    "action": "买入",
                    "positionPct": 0.3,
                    "confidence": 0.85,
                })
                .to_string(),
            )),
            blackboard_snapshot: Set(None),
            config_id: Set(None),
            analysis_kind: Set("live".to_string()),
            as_of_date: Set(None),
            decision_time_horizon: Set(None),
            decision_expected_holding_days: Set(None),
            model_version: Set(None),
            data_snapshot_id: Set(None),
            outcome: Set(None),
            llm_decision_json: Set(None),
            parent_analysis_id: Set(None),
            trade_intent_status: Set("pending".to_string()),
            trade_intent_source: Set(None),
            trade_intent_source_ref_id: Set(None),
            trade_intent_reviewed_at: Set(None),
            trade_intent_reviewed_by: Set(None),
            trade_intent_review_notes: Set(None),
            trade_intent_actual_trade_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model.insert(&db).await.unwrap();

        // 记录交易意图
        let result = TradeIntentService::record_analysis_intent(
            &db,
            &id,
            TradeIntentSource::Analysis,
            Some("wf-001".to_string()),
        )
        .await;
        assert!(result.is_ok());

        // 验证
        let record = stock_analyses::Entity::find_by_id(&id).one(&db).await.unwrap().unwrap();
        assert_eq!(record.trade_intent_status, "pending");
        assert_eq!(record.trade_intent_source, Some("analysis".to_string()));
        assert_eq!(record.trade_intent_source_ref_id, Some("wf-001".to_string()));
    }

    #[tokio::test]
    async fn test_review_flow() {
        let db = setup_db().await;

        // 创建待审核记录
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let model = stock_analyses::ActiveModel {
            id: Set(id.clone()),
            stock_code: Set("000001".to_string()),
            stock_name: Set("平安银行".to_string()),
            analysis_date: Set("2026-08-09".to_string()),
            provider_id: Set("test".to_string()),
            conversation_id: Set("test-conv".to_string()),
            status: Set("completed".to_string()),
            decision_action: Set(Some("卖出".to_string())),
            decision_position_pct: Set(Some(1.0)),
            decision_reasoning: Set(Some("止损".to_string())),
            decision_json: Set(None),
            blackboard_snapshot: Set(None),
            config_id: Set(None),
            analysis_kind: Set("live".to_string()),
            as_of_date: Set(None),
            decision_time_horizon: Set(None),
            decision_expected_holding_days: Set(None),
            model_version: Set(None),
            data_snapshot_id: Set(None),
            outcome: Set(None),
            llm_decision_json: Set(None),
            parent_analysis_id: Set(None),
            trade_intent_status: Set("pending".to_string()),
            trade_intent_source: Set(Some("analysis".to_string())),
            trade_intent_source_ref_id: Set(None),
            trade_intent_reviewed_at: Set(None),
            trade_intent_reviewed_by: Set(None),
            trade_intent_review_notes: Set(None),
            trade_intent_actual_trade_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model.insert(&db).await.unwrap();

        // 审核通过
        let result = TradeIntentService::approve_intent(
            &db,
            ReviewTradeIntentRequest {
                analysis_id: id.clone(),
                reviewed_by: "admin".to_string(),
                notes: Some("同意".to_string()),
            },
        )
        .await
        .unwrap();
        assert!(result.success);
        assert_eq!(result.new_status, "reviewed");

        // 验证
        let record = stock_analyses::Entity::find_by_id(&id).one(&db).await.unwrap().unwrap();
        assert_eq!(record.trade_intent_status, "reviewed");
        assert!(record.trade_intent_reviewed_at.is_some());
        assert_eq!(record.trade_intent_reviewed_by, Some("admin".to_string()));
        assert_eq!(record.trade_intent_review_notes, Some("同意".to_string()));

        // 关联实际交易
        let result =
            TradeIntentService::link_actual_trade(&db, &id, "trade-001", "admin").await.unwrap();
        assert!(result.success);
        assert_eq!(result.new_status, "executed");

        let record = stock_analyses::Entity::find_by_id(&id).one(&db).await.unwrap().unwrap();
        assert_eq!(record.trade_intent_status, "executed");
        assert_eq!(record.trade_intent_actual_trade_id, Some("trade-001".to_string()));
    }

    #[tokio::test]
    async fn test_reject_intent() {
        let db = setup_db().await;

        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let model = stock_analyses::ActiveModel {
            id: Set(id.clone()),
            stock_code: Set("600000".to_string()),
            stock_name: Set("浦发银行".to_string()),
            analysis_date: Set("2026-08-09".to_string()),
            provider_id: Set("test".to_string()),
            conversation_id: Set("test-conv".to_string()),
            status: Set("completed".to_string()),
            decision_action: Set(Some("增持".to_string())),
            decision_position_pct: Set(Some(0.2)),
            decision_reasoning: Set(Some("趋势向好".to_string())),
            decision_json: Set(None),
            blackboard_snapshot: Set(None),
            config_id: Set(None),
            analysis_kind: Set("live".to_string()),
            as_of_date: Set(None),
            decision_time_horizon: Set(None),
            decision_expected_holding_days: Set(None),
            model_version: Set(None),
            data_snapshot_id: Set(None),
            outcome: Set(None),
            llm_decision_json: Set(None),
            parent_analysis_id: Set(None),
            trade_intent_status: Set("pending".to_string()),
            trade_intent_source: Set(Some("analysis".to_string())),
            trade_intent_source_ref_id: Set(None),
            trade_intent_reviewed_at: Set(None),
            trade_intent_reviewed_by: Set(None),
            trade_intent_review_notes: Set(None),
            trade_intent_actual_trade_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model.insert(&db).await.unwrap();

        // 驳回
        let result = TradeIntentService::reject_intent(
            &db,
            ReviewTradeIntentRequest {
                analysis_id: id.clone(),
                reviewed_by: "admin".to_string(),
                notes: Some("估值过高".to_string()),
            },
        )
        .await
        .unwrap();
        assert!(result.success);
        assert_eq!(result.new_status, "rejected");
    }
}
