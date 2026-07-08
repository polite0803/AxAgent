//! 手动交易日志引擎 — 用户录入买卖，自动聚合持仓、计算成本、跟踪盈亏。
//!
//! 核心功能:
//! 1. 交易校验（涨跌停、手数、持仓充足性、交易日历）
//! 2. 执行交易（写入 trades 表 + 自动更新 portfolio_holdings）
//! 3. 持仓汇总（加权平均成本 + 实时盈亏）
//! 4. 交易历史查询

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::sync::Arc;

use axagent_astock_data::AStockClient;
use axagent_astock_data::{detect_market_type, get_st_price_limit_pct};
use axagent_core::entity::{portfolio_holdings, stock_analyses, trades};

/// 实际交易 vs 分析预测对比
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePredictionComparison {
    pub analysis_action: String,
    pub analysis_target: Option<f64>,
    pub analysis_stop: Option<f64>,
    pub actual_price: f64,
    pub target_deviation_pct: f64,
}

/// 手动交易引擎
pub struct TradingEngine {
    db: Arc<DatabaseConnection>,
    astock_client: Arc<AStockClient>,
    /// 交易功能开关（默认 false，需用户主动开启）
    pub enabled: bool,
}

/// 交易校验结果
#[derive(Debug, Clone)]
pub struct TradeValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// 持仓汇总（含实时盈亏）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSummary {
    pub stock_code: String,
    pub stock_name: String,
    pub total_shares: i32,
    pub avg_cost: f64,
    pub current_price: Option<f64>,
    pub market_value: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub unrealized_pnl_pct: Option<f64>,
    pub total_realized_pnl: f64,
    pub sector_name: Option<String>,
}

impl TradingEngine {
    pub fn new(db: Arc<DatabaseConnection>, astock_client: Arc<AStockClient>) -> Self {
        Self {
            db,
            astock_client,
            enabled: false,
        }
    }

    // ── 交易校验 ──

    /// 校验一笔交易是否合法（涨跌停、手数、持仓充足性、交易日历）
    /// 使用默认的 5% 目标价偏离阈值。
    pub async fn validate_trade(
        &self,
        stock_code: &str,
        direction: &str,
        quantity: i32,
        price: f64,
    ) -> TradeValidation {
        self.validate_trade_impl(stock_code, direction, quantity, price, 5.0)
            .await
    }

    /// 带自定义目标价偏离阈值的交易校验。
    /// `price_deviation_limit`: 入场价偏离分析目标价的允许百分比（默认 5.0）
    pub async fn validate_trade_with_config(
        &self,
        stock_code: &str,
        direction: &str,
        quantity: i32,
        price: f64,
        price_deviation_limit: f64,
    ) -> TradeValidation {
        self.validate_trade_impl(stock_code, direction, quantity, price, price_deviation_limit)
            .await
    }

    async fn validate_trade_impl(
        &self,
        stock_code: &str,
        direction: &str,
        quantity: i32,
        price: f64,
        price_deviation_limit: f64,
    ) -> TradeValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 基本合法性
        if quantity <= 0 {
            errors.push("交易数量必须 > 0".to_string());
        }
        if price <= 0.0 {
            errors.push("交易价格必须 > 0".to_string());
        }

        // 数量必须是整手数
        let market = detect_market_type(stock_code);
        let min_lot: i32 = 100;
        if quantity % min_lot != 0 {
            errors.push(format!("数量必须是 {} 股的整数倍（{}手）", min_lot, min_lot));
        }

        // 涨跌停校验（优先使用 vendor 预计算的 limit_up/limit_down，回退到 pre_close 计算）
        if let Ok(quote) = self.astock_client.get_quote(stock_code).await {
            let limit_up = quote.limit_up.unwrap_or_else(|| {
                let limit_pct = get_st_price_limit_pct(quote.is_st, market);
                let ref_price = if quote.pre_close > 0.0 {
                    quote.pre_close
                } else {
                    quote.open
                };
                ref_price * (1.0 + limit_pct / 100.0)
            });
            let limit_down = quote.limit_down.unwrap_or_else(|| {
                let limit_pct = get_st_price_limit_pct(quote.is_st, market);
                let ref_price = if quote.pre_close > 0.0 {
                    quote.pre_close
                } else {
                    quote.open
                };
                ref_price * (1.0 - limit_pct / 100.0)
            });

            if direction == "buy" && price > limit_up {
                errors.push(format!("买入价 {:.2} 超过涨停价 {:.2}", price, limit_up));
            }
            if direction == "buy" && price < limit_down {
                warnings.push(format!(
                    "买入价 {:.2} 低于跌停价 {:.2}，可能难以成交",
                    price, limit_down
                ));
            }
            if direction == "sell" && price < limit_down {
                errors.push(format!("卖出价 {:.2} 低于跌停价 {:.2}", price, limit_down));
            }
            if direction == "sell" && price > limit_up {
                warnings
                    .push(format!("卖出价 {:.2} 超过涨停价 {:.2}，可能难以成交", price, limit_up));
            }
        }

        // 卖出时检查持仓
        if direction == "sell" {
            let holdings = portfolio_holdings::Entity::find()
                .filter(portfolio_holdings::Column::StockCode.eq(stock_code))
                .one(self.db.as_ref())
                .await
                .ok()
                .flatten();

            match holdings {
                Some(h) => {
                    if h.shares < quantity as f64 {
                        errors.push(format!(
                            "持仓不足: 持有 {} 股, 试图卖出 {} 股",
                            h.shares as i32, quantity
                        ));
                    }
                },
                None => {
                    errors.push("没有该股票持仓，无法卖出".to_string());
                },
            }
        }

        // 分析一致性检查（仅买入时）
        if direction == "buy" {
            let last_analysis = stock_analyses::Entity::find()
                .filter(stock_analyses::Column::StockCode.eq(stock_code))
                .filter(stock_analyses::Column::Status.eq("completed"))
                .order_by_desc(stock_analyses::Column::CreatedAt)
                .one(self.db.as_ref())
                .await
                .ok()
                .flatten();

            if let Some(analysis) = last_analysis {
                if let Some(ref decision_json) = analysis.decision_json {
                    if let Ok(decision) = serde_json::from_str::<serde_json::Value>(decision_json) {
                        let suggested_action = decision["action"].as_str().unwrap_or("");
                        let suggested_target = decision["targetPrice"].as_f64();

                        if suggested_action == "卖出" || suggested_action == "减持" {
                            warnings.push(format!(
                                "分析建议「{}」而非买入，请二次确认",
                                suggested_action
                            ));
                        }

                        if let Some(target) = suggested_target {
                            let deviation = ((price - target) / target).abs() * 100.0;
                            if deviation > price_deviation_limit {
                                warnings.push(format!(
                                    "入场价 {:.2} 偏离分析目标价 {:.2} ({:.1}%)",
                                    price, target, deviation
                                ));
                            }
                        }
                    }
                }
            }
        }

        // 仓位上限检查（买入时）
        if direction == "buy" {
            let limits = crate::position_limits::PositionLimits::default();
            let positions = self.get_positions().await.unwrap_or_default();
            let current_count = positions.len();
            let total_mv: f64 = positions
                .iter()
                .map(|p| p.market_value.unwrap_or(0.0))
                .sum();
            let new_position_value = price * quantity as f64;
            if let Err(e) =
                limits.check_new_position(new_position_value, total_mv, current_count, None, &[])
            {
                warnings.push(format!("仓位限制: {}", e));
            }
        }

        // 交易日历检查
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if !is_trading_day(&today) {
            warnings.push("当前非交易日，已记录但仅供参考".to_string());
        }

        TradeValidation {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    // ── 执行交易 ──

    /// 执行一笔交易（写入 trades 表 + 自动更新 portfolio_holdings）
    ///
    /// - 买入：更新持仓（加权平均成本），若已持有则合并且权重平均成本
    /// - 卖出：计算已实现盈亏，减仓或清仓
    /// - 需要 `enabled = true` 且通过校验才执行
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_trade(
        &self,
        stock_code: &str,
        stock_name: &str,
        direction: &str,
        price: f64,
        quantity: i32,
        trade_date: &str,
        trade_time: &str,
        notes: Option<&str>,
        analysis_id: Option<&str>,
    ) -> Result<trades::Model, String> {
        // 从 settings 表读取交易开关状态
        let enabled: bool =
            axagent_core::repo::settings::get_setting(self.db.as_ref(), "trading_enabled")
                .await
                .unwrap_or_default()
                .map(|s| s == "true")
                .unwrap_or(false);
        if !self.enabled && !enabled {
            return Err("交易功能未启用，请先在设置中开启".into());
        }
        // 校验
        let validation = self
            .validate_trade(stock_code, direction, quantity, price)
            .await;
        if !validation.valid {
            return Err(validation.errors.join("; "));
        }

        let now = chrono::Utc::now().timestamp_millis();
        let trade_id = uuid::Uuid::new_v4().to_string();
        let mut realized_pnl = None;

        // 更新持仓
        if direction == "buy" {
            upsert_position(&self.db, stock_code, stock_name, quantity as f64, price).await?;
        } else {
            // sell: 计算实现盈亏
            let holdings = portfolio_holdings::Entity::find()
                .filter(portfolio_holdings::Column::StockCode.eq(stock_code))
                .one(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;

            if let Some(h) = holdings {
                let sell_qty = quantity as f64;
                let cost = h.avg_cost * sell_qty;
                let revenue = price * sell_qty;
                realized_pnl = Some(revenue - cost);

                let remaining = h.shares - sell_qty;
                if remaining <= 0.0 {
                    // 清仓：删除持仓记录
                    portfolio_holdings::Entity::delete_by_id(&h.id)
                        .exec(self.db.as_ref())
                        .await
                        .map_err(|e| e.to_string())?;
                } else {
                    // 减仓：更新剩余数量（avg_cost 不变）
                    portfolio_holdings::Entity::update_many()
                        .col_expr(
                            portfolio_holdings::Column::Shares,
                            sea_orm::sea_query::Expr::value(remaining),
                        )
                        .col_expr(
                            portfolio_holdings::Column::UpdatedAt,
                            sea_orm::sea_query::Expr::value(now),
                        )
                        .filter(portfolio_holdings::Column::Id.eq(&h.id))
                        .exec(self.db.as_ref())
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        // 写入交易记录
        let trade = trades::ActiveModel {
            id: Set(trade_id),
            stock_code: Set(stock_code.to_string()),
            stock_name: Set(stock_name.to_string()),
            direction: Set(direction.to_string()),
            price: Set(price),
            quantity: Set(quantity),
            trade_date: Set(trade_date.to_string()),
            trade_time: Set(trade_time.to_string()),
            fee: Set(None),
            realized_pnl: Set(realized_pnl),
            // R2-遗留修复: trades entity 有 strategy 字段,之前漏写导致 cargo check E0063。
            // 用户手动 record_trade 走的是 stock_analysis command,不带 strategy 上下文,
            // 与 trade_import.rs:412 的 Set(None) 保持一致(手动录入不属于任何自动策略)。
            strategy: Set(None),
            notes: Set(notes.map(|s| s.to_string())),
            // analysis_id 当前未持久化（trades entity 暂未挂该字段，留作未来扩展）
            created_at: Set(now),
        };
        let _ = analysis_id;

        trade
            .insert(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())
    }

    // ── 持仓汇总 ──

    /// 获取持仓汇总（含实时盈亏）
    pub async fn get_positions(&self) -> Result<Vec<PositionSummary>, String> {
        let holdings = portfolio_holdings::Entity::find()
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let mut positions = Vec::new();
        for h in holdings {
            let quote = self.astock_client.get_quote(&h.stock_code).await.ok();
            let current_price = quote.as_ref().map(|q| q.price);
            let market_value = current_price.map(|p| p * h.shares);
            let unrealized_pnl = market_value.map(|mv| mv - h.avg_cost * h.shares);
            let unrealized_pnl_pct = market_value.map(|mv| {
                let cost = h.avg_cost * h.shares;
                if cost > 0.0 {
                    ((mv - cost) / cost) * 100.0
                } else {
                    0.0
                }
            });

            // 累计已实现盈亏
            let realized: f64 = trades::Entity::find()
                .filter(trades::Column::StockCode.eq(&h.stock_code))
                .filter(trades::Column::Direction.eq("sell"))
                .all(self.db.as_ref())
                .await
                .map(|rows| rows.iter().filter_map(|t| t.realized_pnl).sum())
                .unwrap_or(0.0);

            // 查询行业分类
            let sector_name = self
                .astock_client
                .get_sector_info(&h.stock_code)
                .await
                .ok()
                .flatten()
                .map(|s| s.sector_name)
                .filter(|n| !n.is_empty());

            positions.push(PositionSummary {
                stock_code: h.stock_code.clone(),
                stock_name: h.stock_name.clone(),
                total_shares: h.shares as i32,
                avg_cost: h.avg_cost,
                current_price,
                market_value,
                unrealized_pnl,
                unrealized_pnl_pct,
                total_realized_pnl: realized,
                sector_name,
            });
        }
        Ok(positions)
    }

    // ── 交易历史 ──

    /// 获取交易历史
    pub async fn get_trades(
        &self,
        stock_code: Option<&str>,
        limit: u32,
    ) -> Result<Vec<trades::Model>, String> {
        let mut query = trades::Entity::find()
            .order_by_desc(trades::Column::CreatedAt)
            .limit(Some(limit as u64));

        if let Some(code) = stock_code {
            query = query.filter(trades::Column::StockCode.eq(code));
        }

        query.all(self.db.as_ref()).await.map_err(|e| e.to_string())
    }

    // ── 出场价 vs 分析预测对比 ──

    /// 对比实际交易出场价与最近分析预测价位
    pub async fn compare_trade_vs_prediction(
        &self,
        trade: &trades::Model,
    ) -> Result<TradePredictionComparison, String> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

        let mut comparison = TradePredictionComparison {
            analysis_action: String::new(),
            analysis_target: None,
            analysis_stop: None,
            actual_price: trade.price,
            target_deviation_pct: 0.0,
        };

        let last_analysis = stock_analyses::Entity::find()
            .filter(stock_analyses::Column::StockCode.eq(&trade.stock_code))
            .filter(stock_analyses::Column::Status.eq("completed"))
            .filter(stock_analyses::Column::CreatedAt.lt(trade.created_at))
            .order_by_desc(stock_analyses::Column::CreatedAt)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or("没有找到在交易前的分析记录".to_string())?;

        if let Some(ref decision_json) = last_analysis.decision_json {
            if let Ok(decision) = serde_json::from_str::<serde_json::Value>(decision_json) {
                comparison.analysis_action = decision["action"].as_str().unwrap_or("").to_string();
                comparison.analysis_target = decision["targetPrice"].as_f64();
                comparison.analysis_stop = decision["stopLoss"].as_f64();

                if let Some(target) = comparison.analysis_target {
                    if target != 0.0 {
                        comparison.target_deviation_pct = ((trade.price - target) / target) * 100.0;
                    }
                }
            }
        }

        Ok(comparison)
    }
}

// ── 内部 helper ──

/// 更新或创建持仓（买入时调用）
///
/// - 已有持仓：加权平均成本合并
/// - 新持仓：创建记录
/// - 注意：SQLite 单写入序列化已提供基础并发保护；桌面单用户场景下竞态概率极低
async fn upsert_position(
    db: &DatabaseConnection,
    stock_code: &str,
    stock_name: &str,
    shares: f64,
    price: f64,
) -> Result<(), String> {
    let existing = portfolio_holdings::Entity::find()
        .filter(portfolio_holdings::Column::StockCode.eq(stock_code))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();

    if let Some(h) = existing {
        let old_total = h.shares * h.avg_cost;
        let new_total = shares * price;
        let new_shares = h.shares + shares;
        let new_avg_cost = if new_shares > 0.0 {
            (old_total + new_total) / new_shares
        } else {
            price
        };
        portfolio_holdings::Entity::update_many()
            .col_expr(
                portfolio_holdings::Column::Shares,
                sea_orm::sea_query::Expr::value(new_shares),
            )
            .col_expr(
                portfolio_holdings::Column::AvgCost,
                sea_orm::sea_query::Expr::value(new_avg_cost),
            )
            .col_expr(portfolio_holdings::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
            .filter(portfolio_holdings::Column::Id.eq(&h.id))
            .exec(db)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        let model = portfolio_holdings::ActiveModel {
            id: Set(uuid::Uuid::new_v4().to_string()),
            stock_code: Set(stock_code.to_string()),
            stock_name: Set(stock_name.to_string()),
            shares: Set(shares),
            avg_cost: Set(price),
            notes: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model.insert(db).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 判断是否为 A 股交易日（委托给 astock-data 的交易日历，含节假日和调休）
fn is_trading_day(date_str: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| axagent_astock_data::calendar::is_trading_day(&d))
        .unwrap_or(true)
}
