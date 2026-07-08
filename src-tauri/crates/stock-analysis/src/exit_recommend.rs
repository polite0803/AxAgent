//! 持仓退出建议引擎 — 根据持仓情况、分析结论、技术指标、组合风险，
//! 推荐在未来什么时间以什么价格挂出哪些持仓股票。

use axagent_astock_data::indicators::compute_indicators;
use axagent_astock_data::AStockClient;
use axagent_entities::{portfolio_holdings, stock_analyses};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;

// ── 公开类型 ──

/// 退出建议
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitRecommendation {
    pub stock_code: String,
    pub stock_name: String,
    /// 当前持仓信息
    pub shares: i32,
    pub avg_cost: f64,
    pub current_price: f64,
    /// 盈亏
    pub pnl_pct: f64,
    pub pnl_amount: f64,
    /// 仓位占组合百分比
    pub position_pct: f64,
    /// 退出紧迫度 0-100（越高越紧迫）
    pub exit_score: f64,
    /// 建议操作
    pub action: ExitAction,
    /// 建议出场价（含具体价位建议）
    pub suggested_price: Option<f64>,
    /// 建议时间段
    pub timeframe: String,
    /// 买入持有天数
    pub holding_days: i64,
    /// 触发信号列表
    pub signals: Vec<ExitSignal>,
    /// 综合理由
    pub reasoning: String,
}

/// 建议操作
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExitAction {
    /// 立即卖出（高紧迫）
    SellNow,
    /// 挂限价单卖出
    SellAtLimit,
    /// 设置止损价告警
    SetStopLoss,
    /// 继续持有
    Hold,
    /// 可以考虑加仓
    ConsiderAdd,
}

/// 触发信号
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitSignal {
    pub signal_type: String,
    pub severity: String, // "critical" | "high" | "medium" | "low"
    pub detail: String,
}

/// 退出建议汇总
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitSummary {
    pub total_positions: usize,
    pub urgent_exits: usize,
    pub limit_exits: usize,
    pub stop_loss_needed: usize,
    pub holds: usize,
    pub recommendations: Vec<ExitRecommendation>,
}

// ── 评分常数 ──

const SCORE_ANALYSIS_SELL: f64 = 30.0;
const SCORE_PRICE_ABOVE_TARGET: f64 = 20.0;
const SCORE_PRICE_BELOW_STOP: f64 = 35.0;
const SCORE_RSI_OVERBOUGHT: f64 = 15.0;
const SCORE_MACD_DEATH_CROSS: f64 = 15.0;
const SCORE_MA_BEARISH: f64 = 10.0;
const SCORE_VOLUME_SELLOFF: f64 = 8.0;
const SCORE_CONCENTRATION_HIGH: f64 = 15.0;
const SCORE_CONCENTRATION_MED: f64 = 8.0;
const SCORE_SECTOR_OVEREXPOSED: f64 = 8.0;
const SCORE_TAKE_PROFIT: f64 = 12.0;
const SCORE_STOP_LOSS_NEAR: f64 = 18.0;
const SCORE_ROTATE: f64 = 5.0;
const SCORE_RISK_PARITY: f64 = 8.0;

// ── 主函数 ──

/// 获取所有持仓的退出建议
pub async fn get_exit_recommendations(
    db: &DatabaseConnection,
    astock_client: &Arc<AStockClient>,
) -> Result<ExitSummary, String> {
    // 1. 获取所有持仓
    let holdings = portfolio_holdings::Entity::find()
        .all(db)
        .await
        .map_err(|e| format!("读取持仓失败: {e}"))?;

    if holdings.is_empty() {
        return Ok(ExitSummary {
            total_positions: 0,
            urgent_exits: 0,
            limit_exits: 0,
            stop_loss_needed: 0,
            holds: 0,
            recommendations: vec![],
        });
    }

    // 2. 获取组合总市值（用于计算仓位占比）
    let total_mv = get_total_portfolio_value(db, astock_client).await;

    // 3. 获取行业暴露（用于行业集中度检查）
    let sector_exposure = get_sector_exposure(db, astock_client).await;

    let mut recommendations: Vec<ExitRecommendation> = Vec::new();

    for holding in &holdings {
        let rec = evaluate_position(db, astock_client, holding, total_mv, &sector_exposure).await;
        recommendations.push(rec);
    }

    // 按紧迫度降序排列
    recommendations.sort_by(|a, b| {
        b.exit_score
            .partial_cmp(&a.exit_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let urgent = recommendations
        .iter()
        .filter(|r| matches!(r.action, ExitAction::SellNow))
        .count();
    let limit = recommendations
        .iter()
        .filter(|r| matches!(r.action, ExitAction::SellAtLimit))
        .count();
    let stop = recommendations
        .iter()
        .filter(|r| matches!(r.action, ExitAction::SetStopLoss))
        .count();
    let hold = recommendations
        .iter()
        .filter(|r| matches!(r.action, ExitAction::Hold))
        .count();

    Ok(ExitSummary {
        total_positions: holdings.len(),
        urgent_exits: urgent,
        limit_exits: limit,
        stop_loss_needed: stop,
        holds: hold,
        recommendations,
    })
}

// ── 单只持仓评估 ──

async fn evaluate_position(
    db: &DatabaseConnection,
    astock_client: &Arc<AStockClient>,
    holding: &portfolio_holdings::Model,
    total_mv: f64,
    sector_exposure: &[(String, f64)],
) -> ExitRecommendation {
    let mut score = 0.0_f64;
    let mut signals: Vec<ExitSignal> = Vec::new();
    let mut suggested_price: Option<f64> = None;
    let mut pnl_pct = 0.0;
    let mut pnl_amount = 0.0;

    // --- 获取实时行情 ---
    let quote = astock_client.get_quote(&holding.stock_code).await.ok();
    let current_price_f64 = quote.as_ref().map(|q| q.price).unwrap_or(0.0);
    let _pre_close = quote.as_ref().map(|q| q.pre_close).unwrap_or(0.0);

    // 计算盈亏
    if holding.avg_cost > 0.0 && current_price_f64 > 0.0 {
        pnl_amount = (current_price_f64 - holding.avg_cost) * holding.shares;
        pnl_pct = (current_price_f64 - holding.avg_cost) / holding.avg_cost * 100.0;
    }

    // 计算仓位占比
    let position_value = current_price_f64 * holding.shares;
    let position_pct = if total_mv > 0.0 {
        position_value / total_mv * 100.0
    } else {
        0.0
    };

    // 计算持有天数
    let holding_days = compute_holding_days(holding);

    // --- Signal 1: 分析结论检查 ---
    let last_analysis = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::StockCode.eq(&holding.stock_code))
        .filter(stock_analyses::Column::Status.eq("completed"))
        .order_by_desc(stock_analyses::Column::CreatedAt)
        .one(db)
        .await
        .ok()
        .flatten();

    let mut target_price: Option<f64> = None;
    let mut stop_loss: Option<f64> = None;

    if let Some(ref analysis) = last_analysis {
        if let Some(ref decision_json) = analysis.decision_json {
            if let Ok(decision) = serde_json::from_str::<serde_json::Value>(decision_json) {
                let action_str = decision["action"].as_str().unwrap_or("");
                target_price = decision["targetPrice"].as_f64();
                stop_loss = decision["stopLoss"].as_f64();

                // 分析建议卖出/减持
                if action_str == "卖出" || action_str == "减持" {
                    score += SCORE_ANALYSIS_SELL;
                    signals.push(ExitSignal {
                        signal_type: "analysis_sell".into(),
                        severity: "high".into(),
                        detail: format!("最近分析建议「{action_str}」"),
                    });
                }

                // 已达目标价（止盈）
                if let Some(tp) = target_price {
                    if current_price_f64 > 0.0 && current_price_f64 >= tp {
                        score += SCORE_PRICE_ABOVE_TARGET;
                        suggested_price = Some(tp);
                        signals.push(ExitSignal {
                            signal_type: "take_profit_reached".into(),
                            severity: "high".into(),
                            detail: format!("现价 {:.2} 已达到目标价 {:.2}", current_price_f64, tp),
                        });
                    }
                }

                // 跌破止损
                if let Some(sl) = stop_loss {
                    if current_price_f64 > 0.0 && current_price_f64 <= sl {
                        score += SCORE_PRICE_BELOW_STOP;
                        suggested_price = Some(sl);
                        signals.push(ExitSignal {
                            signal_type: "stop_loss_hit".into(),
                            severity: "critical".into(),
                            detail: format!("现价 {:.2} 已跌破止损价 {:.2}", current_price_f64, sl),
                        });
                    } else if let Some(sl) = stop_loss {
                        // 接近止损（距止损 < 3%）
                        let distance = (current_price_f64 - sl) / sl * 100.0;
                        if (0.0..3.0).contains(&distance) {
                            score += SCORE_STOP_LOSS_NEAR;
                            signals.push(ExitSignal {
                                signal_type: "stop_loss_near".into(),
                                severity: "high".into(),
                                detail: format!(
                                    "现价 {:.2} 距止损 {:.2} 仅 {:.1}%",
                                    current_price_f64, sl, distance
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    // --- Signal 2: 技术指标检查 ---
    if current_price_f64 > 0.0 {
        // 获取 K 线数据计算技术指标
        let klines = astock_client
            .get_klines(&holding.stock_code, "daily", 120)
            .await
            .ok();

        if let Some(ref klines) = klines {
            let indicators = compute_indicators(&holding.stock_code, klines);

            // RSI 超买
            if indicators.rsi_signal == "超买" || indicators.rsi12 > 70.0 {
                score += SCORE_RSI_OVERBOUGHT;
                signals.push(ExitSignal {
                    signal_type: "rsi_overbought".into(),
                    severity: "medium".into(),
                    detail: format!("RSI {:.1}，超买区域，短期回调风险", indicators.rsi12),
                });
            }

            // MACD 死叉
            if indicators.macd_signal == "死叉" {
                score += SCORE_MACD_DEATH_CROSS;
                signals.push(ExitSignal {
                    signal_type: "macd_death_cross".into(),
                    severity: "high".into(),
                    detail: "MACD 死叉，趋势转弱".into(),
                });
            }

            // 均线空头排列
            if indicators.ma_alignment == "空头排列" {
                score += SCORE_MA_BEARISH;
                signals.push(ExitSignal {
                    signal_type: "ma_bearish".into(),
                    severity: "medium".into(),
                    detail: "均线空头排列，趋势偏弱".into(),
                });
            }

            // 放量下跌
            if indicators.volume_signal == "放量下跌" {
                score += SCORE_VOLUME_SELLOFF;
                signals.push(ExitSignal {
                    signal_type: "volume_selloff".into(),
                    severity: "medium".into(),
                    detail: "放量下跌，抛压加重".into(),
                });
            }
        }
    }

    // --- Signal 3: 组合风险 ---
    // 集中度风险
    if position_pct > 30.0 {
        score += SCORE_CONCENTRATION_HIGH;
        signals.push(ExitSignal {
            signal_type: "concentration_high".into(),
            severity: "high".into(),
            detail: format!("仓位占比 {:.0}%，集中度过高，建议减仓", position_pct),
        });
    } else if position_pct > 20.0 {
        score += SCORE_CONCENTRATION_MED;
        signals.push(ExitSignal {
            signal_type: "concentration_medium".into(),
            severity: "medium".into(),
            detail: format!("仓位占比 {:.0}%，适当控制集中度", position_pct),
        });
    }

    // 行业集中度
    if let Some(sector) = holding.stock_name.split(' ').next() {
        let sector_total: f64 = sector_exposure
            .iter()
            .filter(|(name, _)| name.contains(sector) || sector.contains(name))
            .map(|(_, pct)| *pct)
            .sum();
        if sector_total > 45.0 && position_pct / total_mv.max(1.0) * 100.0 > 5.0 {
            score += SCORE_SECTOR_OVEREXPOSED;
            signals.push(ExitSignal {
                signal_type: "sector_overexposed".into(),
                severity: "medium".into(),
                detail: format!("行业总暴露 {:.0}%，单一持仓占比过高", sector_total),
            });
        }
    }

    // --- Signal 4: 盈亏情况 ---
    if pnl_pct > 30.0 {
        // 大幅盈利，可以考虑止盈
        score += SCORE_TAKE_PROFIT;
        signals.push(ExitSignal {
            signal_type: "large_profit".into(),
            severity: "medium".into(),
            detail: format!("盈利 {:.1}%，建议分批止盈保护利润", pnl_pct),
        });
    } else if pnl_pct < -20.0 {
        // 大幅亏损
        score += SCORE_STOP_LOSS_NEAR;
        signals.push(ExitSignal {
            signal_type: "large_loss".into(),
            severity: "high".into(),
            detail: format!("亏损 {:.1}%，需要关注风险控制", pnl_pct),
        });
    }

    // 持有时间过长但收益一般 → 资金效率低
    if holding_days > 120 && pnl_pct < 10.0 && pnl_pct > -5.0 {
        score += SCORE_ROTATE;
        signals.push(ExitSignal {
            signal_type: "low_efficiency".into(),
            severity: "low".into(),
            detail: format!("持有 {holding_days} 天收益仅 {:.1}%，资金效率偏低", pnl_pct),
        });
    }

    // 风险平价：如果浮盈很高且仓位很重，建议减仓
    if pnl_pct > 40.0 && position_pct > 15.0 {
        score += SCORE_RISK_PARITY;
        signals.push(ExitSignal {
            signal_type: "risk_parity".into(),
            severity: "medium".into(),
            detail: format!(
                "盈利 {:.1}% 且仓位 {:.0}%，建议部分止盈恢复风险平衡",
                pnl_pct, position_pct
            ),
        });
    }

    // --- 确定最终操作 ---
    let exit_score = score.min(100.0);
    #[allow(unused_mut)]
    let mut action;
    #[allow(unused_mut)]
    let mut timeframe;

    if exit_score >= 40.0 {
        // 高紧迫：止损、严重超买、分析明确卖出
        if signals.iter().any(|s| s.severity == "critical") {
            action = ExitAction::SellNow;
            timeframe = "今日".into();
            suggested_price = suggested_price.or(Some(current_price_f64 * 0.995));
        } else {
            action = ExitAction::SellAtLimit;
            // 建议价：取 targetPrice / stopLoss / 当前价 * 1.005 三者的最优
            let limit = target_price
                .or(stop_loss)
                .unwrap_or(current_price_f64 * 1.005);
            suggested_price = Some(limit);
            timeframe = "本周内".into();
        }
    } else if exit_score >= 20.0 {
        // 中等紧迫：需要关注
        if stop_loss.is_some() {
            action = ExitAction::SetStopLoss;
            timeframe = "设置止损".into();
        } else {
            action = ExitAction::SellAtLimit;
            suggested_price = target_price.or(Some(current_price_f64 * 1.03));
            timeframe = "1-2 周内".into();
        }
    } else if pnl_pct > 15.0 {
        // 盈利但信号不强
        action = ExitAction::SetStopLoss;
        let sl = current_price_f64 * 0.95;
        suggested_price = Some(sl);
        timeframe = "设置移动止损".into();
    } else if exit_score < 5.0 && pnl_pct < -10.0 {
        // 轻微亏损但无明确信号 → 继续观察
        action = ExitAction::Hold;
        timeframe = "继续持有观察".into();
    } else {
        action = ExitAction::Hold;
        timeframe = "继续持有".into();
    }

    // --- 生成理由 ---
    let reasoning = if signals.is_empty() {
        format!(
            "持仓 {:.0} 股，成本 {:.2}，现价 {:.2}，盈亏 {:.1}%。暂无明确退出信号",
            holding.shares, holding.avg_cost, current_price_f64, pnl_pct
        )
    } else {
        let main_signal = &signals[0];
        format!(
            "{}。持有 {} 天，盈亏 {:.1}%，共 {} 项信号触发",
            main_signal.detail,
            holding_days,
            pnl_pct,
            signals.len()
        )
    };

    ExitRecommendation {
        stock_code: holding.stock_code.clone(),
        stock_name: holding.stock_name.clone(),
        shares: holding.shares as i32,
        avg_cost: holding.avg_cost,
        current_price: current_price_f64,
        pnl_pct,
        pnl_amount,
        position_pct,
        exit_score,
        action,
        suggested_price,
        timeframe,
        holding_days,
        signals,
        reasoning,
    }
}

// ── 辅助函数 ──

/// 计算持仓总市值
async fn get_total_portfolio_value(
    db: &DatabaseConnection,
    astock_client: &Arc<AStockClient>,
) -> f64 {
    let holdings = portfolio_holdings::Entity::find()
        .all(db)
        .await
        .unwrap_or_default();
    let mut total = 0.0;
    for h in &holdings {
        if let Ok(quote) = astock_client.get_quote(&h.stock_code).await {
            total += quote.price * h.shares;
        } else {
            total += h.avg_cost * h.shares;
        }
    }
    total
}

/// 计算行业暴露分布
async fn get_sector_exposure(
    db: &DatabaseConnection,
    astock_client: &Arc<AStockClient>,
) -> Vec<(String, f64)> {
    let holdings = portfolio_holdings::Entity::find()
        .all(db)
        .await
        .unwrap_or_default();
    let mut sectors: Vec<(String, f64)> = Vec::new();
    for h in &holdings {
        let mv = if let Ok(quote) = astock_client.get_quote(&h.stock_code).await {
            quote.price * h.shares
        } else {
            h.avg_cost * h.shares
        };
        let sector = h.stock_name.split(' ').next().unwrap_or("其他").to_string();
        if let Some(existing) = sectors.iter_mut().find(|(n, _)| *n == sector) {
            existing.1 += mv;
        } else {
            sectors.push((sector, mv));
        }
    }
    sectors
}

/// 计算持仓天数
fn compute_holding_days(holding: &portfolio_holdings::Model) -> i64 {
    let now = chrono::Utc::now().timestamp_millis();
    let diff_ms = now - holding.created_at;
    (diff_ms / (1000 * 60 * 60 * 24)).max(0)
}
