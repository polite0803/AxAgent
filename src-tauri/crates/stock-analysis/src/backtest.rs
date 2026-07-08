use serde::{Deserialize, Serialize};

use crate::decision::ScoringWeights;
use axagent_astock_data::AStockClient;
use sea_orm::DatabaseConnection;

/// 回测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestResult {
    pub stock_code: String,
    pub analysis_date: String,
    pub decision_action: String,
    pub decision_confidence: f64,
    /// 原始决策时间维度
    pub time_horizon: Option<String>,
    /// 原始期望持有天数
    pub expected_holding_days: Option<u32>,
    /// 分析日的收盘价（入场价）
    pub entry_price: Option<f64>,
    /// 持有N日后的收盘价（出场价）
    pub exit_price: f64,
    pub holding_days: u32,
    /// 收益率（%）
    pub return_pct: f64,
    /// 决策是否正确（买入/增持应涨，减持/卖出应跌，持有应平）
    pub was_correct: bool,
    /// 持有期间最大回撤（%）
    pub max_drawdown: f64,
}

/// 回测统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestStats {
    pub total_analyses: u32,
    /// 准确率（%）
    pub accuracy_pct: f64,
    /// 平均收益率（%）
    pub avg_return_pct: f64,
    /// 平均最大回撤（%）
    pub avg_max_drawdown_pct: f64,
    /// 平均置信度
    pub avg_confidence: f64,
    /// 超额收益 alpha（%），相对沪深300
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_pct: Option<f64>,
    /// 按时间维度分组交叉统计
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_breakdown: Option<Vec<PeriodStats>>,
}

/// 按时间维度分组的回测统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodStats {
    pub time_horizon: String,
    pub count: u32,
    pub accuracy_pct: f64,
    pub avg_return_pct: f64,
    pub avg_max_drawdown_pct: f64,
}

/// 基准对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub stock_return_pct: f64,
    pub csi300_return_pct: f64,
    pub alpha_pct: f64,
    pub outperformed: bool,
    pub benchmark_name: String,
}

/// 历史分析记录（用于回测输入）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAnalysis {
    pub stock_code: String,
    pub analysis_date: String,
    pub decision_action: String,
    pub decision_confidence: f64,
    /// 原始决策时间维度
    #[serde(default)]
    pub time_horizon: Option<String>,
    /// 个性化持有天数（优先于 backtest_history 的统一参数）
    #[serde(default)]
    pub expected_holding_days: Option<u32>,
}

/// 回测引擎
pub struct BacktestEngine;

impl BacktestEngine {
    /// 回测单个分析决策。
    ///
    /// 策略：假设在 `analysis_date` 以收盘价入场，持有 `holding_days` 后以收盘价出场。
    /// 从 API 获取 `holding_days + 10` 根日 K 线以覆盖分析日之后的数据。
    #[allow(clippy::too_many_arguments)]
    pub async fn backtest_decision(
        client: &axagent_astock_data::AStockClient,
        stock_code: &str,
        analysis_date: &str,
        decision_action: &str,
        decision_confidence: f64,
        holding_days: u32,
        time_horizon: Option<String>,
        expected_holding_days: Option<u32>,
    ) -> Result<BacktestResult, String> {
        // 取最近 500 日K线（约两个交易年），最大化覆盖 analysis_date 的概率
        // 注：get_klines 返回最近 N 根K线（按时间升序），若 analysis_date 超出范围则回测失败
        let klines = client
            .get_klines(stock_code, "daily", 500)
            .await
            .map_err(|e| format!("获取K线失败: {e}"))?;
        if klines.iter().all(|k| k.date.as_str() < analysis_date) {
            return Err(format!("{stock_code} 无 {analysis_date} 之后的K线数据"));
        }

        if klines.is_empty() {
            return Err(format!("{stock_code} 无K线数据"));
        }

        let entry_idx = klines.iter().position(|k| k.date.as_str() >= analysis_date);
        let entry_price = match entry_idx {
            Some(i) => Some(klines[i].close),
            None => return Err(format!("{stock_code} 在 {analysis_date} 无K线数据，无法回测")),
        };

        let exit_idx = entry_idx
            .map(|i| (i + holding_days as usize).min(klines.len() - 1))
            .unwrap();
        let exit_price = klines[exit_idx].close;

        // 计算持有期间最大回撤
        let relevant: Vec<_> = klines
            .iter()
            .skip_while(|k| k.date.as_str() < analysis_date)
            .take(holding_days as usize + 1)
            .collect();

        let mut peak = 0.0_f64;
        let mut max_dd = 0.0;
        for k in &relevant {
            if k.close > peak && k.close > 0.0 {
                peak = k.close;
            }
            let dd = if peak > 0.0 {
                (peak - k.close) / peak
            } else {
                0.0
            };
            if dd > max_dd {
                max_dd = dd;
            }
        }

        // 计算收益率
        let return_pct = match entry_price {
            Some(entry) if entry > 0.0 => ((exit_price - entry) / entry) * 100.0,
            _ => 0.0,
        };

        // 判断决策是否正确：
        //   买入/增持 → 上涨为正确（return_pct > 0）
        //   减持/卖出 → 下跌为正确（return_pct < 0）
        //   持有/观望 → |return_pct| < 0.5% 为正确（窄幅震荡），超出则为判断错误
        let was_correct = match decision_action {
            "买入" | "增持" => return_pct > 0.0,
            "减持" | "卖出" => return_pct < 0.0,
            _ => return_pct.abs() < 0.5, // 持有：窄幅震荡算正确，超出算错
        };

        Ok(BacktestResult {
            stock_code: stock_code.to_string(),
            analysis_date: analysis_date.to_string(),
            decision_action: decision_action.to_string(),
            decision_confidence,
            time_horizon,
            expected_holding_days,
            entry_price,
            exit_price,
            holding_days,
            return_pct,
            was_correct,
            max_drawdown: max_dd * 100.0,
        })
    }

    /// 批量回测历史分析记录
    ///
    /// 优先使用每条记录的 `expected_holding_days`（个性化持有期），
    /// 仅当该字段为 None 时回退到统一的 `default_holding_days`。
    pub async fn backtest_history(
        client: &axagent_astock_data::AStockClient,
        analyses: Vec<HistoricalAnalysis>,
        default_holding_days: u32,
    ) -> Result<Vec<BacktestResult>, String> {
        let mut results = Vec::new();
        for analysis in analyses {
            let holding_days = analysis
                .expected_holding_days
                .unwrap_or(default_holding_days);
            match Self::backtest_decision(
                client,
                &analysis.stock_code,
                &analysis.analysis_date,
                &analysis.decision_action,
                analysis.decision_confidence,
                holding_days,
                analysis.time_horizon.clone(),
                analysis.expected_holding_days,
            )
            .await
            {
                Ok(r) => results.push(r),
                Err(e) => {
                    tracing::warn!(
                        "回测失败 {} {}: {}",
                        analysis.stock_code,
                        analysis.analysis_date,
                        e
                    );
                },
            }
        }
        Ok(results)
    }

    /// 计算回测统计指标（含按时间维度分组交叉统计）
    pub fn compute_stats(results: &[BacktestResult]) -> BacktestStats {
        let total = results.len() as f64;
        let correct = results.iter().filter(|r| r.was_correct).count() as f64;

        let accuracy = if total > 0.0 {
            (correct / total) * 100.0
        } else {
            0.0
        };
        let avg_return: f64 = if total > 0.0 {
            results.iter().map(|r| r.return_pct).sum::<f64>() / total
        } else {
            0.0
        };
        let avg_max_dd: f64 = if total > 0.0 {
            results.iter().map(|r| r.max_drawdown).sum::<f64>() / total
        } else {
            0.0
        };
        let avg_confidence = if total > 0.0 {
            results.iter().map(|r| r.decision_confidence).sum::<f64>() / total
        } else {
            0.0
        };

        // 按时间维度分组统计
        let mut period_groups: std::collections::HashMap<String, Vec<&BacktestResult>> =
            std::collections::HashMap::new();
        for r in results {
            let key = r.time_horizon.as_deref().unwrap_or("unknown").to_string();
            period_groups.entry(key).or_default().push(r);
        }
        let mut period_breakdown: Vec<PeriodStats> = period_groups
            .into_iter()
            .map(|(key, group)| {
                let n = group.len() as f64;
                let correct_n = group.iter().filter(|r| r.was_correct).count() as f64;
                let acc = if n > 0.0 {
                    (correct_n / n) * 100.0
                } else {
                    0.0
                };
                let avg_r = if n > 0.0 {
                    group.iter().map(|r| r.return_pct).sum::<f64>() / n
                } else {
                    0.0
                };
                let avg_dd = if n > 0.0 {
                    group.iter().map(|r| r.max_drawdown).sum::<f64>() / n
                } else {
                    0.0
                };
                PeriodStats {
                    time_horizon: key,
                    count: group.len() as u32,
                    accuracy_pct: acc,
                    avg_return_pct: avg_r,
                    avg_max_drawdown_pct: avg_dd,
                }
            })
            .collect();
        period_breakdown.sort_by_key(|p| match p.time_horizon.as_str() {
            "ultra_short" => 0,
            "short" => 1,
            "mid" => 2,
            "long" => 3,
            _ => 9,
        });

        BacktestStats {
            total_analyses: results.len() as u32,
            accuracy_pct: accuracy,
            avg_return_pct: avg_return,
            avg_max_drawdown_pct: avg_max_dd,
            avg_confidence,
            alpha_pct: None,
            period_breakdown: Some(period_breakdown),
        }
    }

    /// 对比沪深300基准计算超额收益
    pub async fn benchmark_against_csi300(
        client: &axagent_astock_data::AStockClient,
        start_date: &str,
        end_date: &str,
        stock_return_pct: f64,
    ) -> Result<BenchmarkResult, String> {
        // 获取沪深300 (000300) 同期表现
        let klines = client
            .get_klines("000300", "daily", 500)
            .await
            .map_err(|e| format!("获取沪深300K线失败: {e}"))?;

        let start_kline = klines.iter().find(|k| k.date.as_str() >= start_date);
        let end_kline = klines.iter().rev().find(|k| k.date.as_str() <= end_date);

        match (start_kline, end_kline) {
            (Some(start), Some(end)) => {
                let csi300_return = if start.close > 0.0 {
                    ((end.close - start.close) / start.close) * 100.0
                } else {
                    0.0
                };
                let alpha = stock_return_pct - csi300_return;
                let outperformed = alpha > 0.0;

                Ok(BenchmarkResult {
                    stock_return_pct,
                    csi300_return_pct: csi300_return,
                    alpha_pct: alpha,
                    outperformed,
                    benchmark_name: "沪深300".to_string(),
                })
            },
            _ => Err("无法计算CSI 300基准收益".into()),
        }
    }
}

/// 同时运行回测和CSI 300基准对比的便捷函数
pub async fn backtest_with_benchmark(
    client: &axagent_astock_data::AStockClient,
    analyses: Vec<HistoricalAnalysis>,
    holding_days: u32,
) -> Result<(BacktestStats, Option<BenchmarkResult>), String> {
    let results = BacktestEngine::backtest_history(client, analyses, holding_days).await?;
    let mut stats = BacktestEngine::compute_stats(&results);

    // 取第一条分析的日期范围计算CSI300基准
    if let (Some(first), Some(last)) = (results.first(), results.last()) {
        match BacktestEngine::benchmark_against_csi300(
            client,
            &first.analysis_date,
            &last.analysis_date,
            stats.avg_return_pct,
        )
        .await
        {
            Ok(bench) => {
                stats.alpha_pct = Some(bench.alpha_pct);
                return Ok((stats, Some(bench)));
            },
            Err(e) => {
                tracing::warn!("CSI300基准计算失败: {e}");
            },
        }
    }

    Ok((stats, None))
}

/// 基于回测结果优化评分权重
pub async fn optimize_weights(
    _client: &AStockClient,
    db: &DatabaseConnection,
) -> Result<ScoringWeights, String> {
    // 简化版：从所有已完成分析中获取平均得分分布
    use axagent_core::entity::stock_analyses;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let completed = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::Status.eq("completed"))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;

    if completed < 10 {
        return Ok(ScoringWeights::default()); // 样本不足
    }

    // 基于历史决策分布自适应调整评分权重
    let analyses = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::Status.eq("completed"))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if analyses.len() < 10 {
        return Ok(ScoringWeights::default());
    }
    let (mut buy, mut sell, mut hold) = (0u32, 0u32, 0u32);
    for a in &analyses {
        match a.decision_action.as_deref() {
            Some("买入") | Some("增持") => buy += 1,
            Some("卖出") | Some("减持") => sell += 1,
            _ => hold += 1,
        }
    }
    let total = (buy + sell + hold).max(1) as f64;
    let buy_r = buy as f64 / total;
    let d = ScoringWeights::default();
    let correct_avg = analyses
        .iter()
        .filter(|a| {
            a.decision_action.as_deref() == Some("买入")
                || a.decision_action.as_deref() == Some("增持")
        })
        .count();
    let adj = if buy_r > 0.5 && correct_avg as f64 / total > 0.5 {
        ScoringWeights {
            trend: (d.trend * 0.85).max(10.0),
            deviation: (d.deviation * 0.90).max(10.0),
            macd: d.macd,
            volume: d.volume,
            rsi: (d.rsi * 1.15).min(20.0),
            support: (d.support * 1.20).min(20.0),
            boll: d.boll,
        }
    } else if buy_r < 0.3 {
        ScoringWeights {
            trend: (d.trend * 1.10).min(40.0),
            deviation: (d.deviation * 1.05).min(30.0),
            macd: d.macd,
            volume: d.volume,
            rsi: (d.rsi * 0.90).max(5.0),
            support: (d.support * 0.85).max(5.0),
            boll: d.boll,
        }
    } else {
        d
    };
    tracing::info!("optimize_weights: {buy}B/{sell}S/{hold}H trend={:.1}", adj.trend);
    Ok(adj)
}
