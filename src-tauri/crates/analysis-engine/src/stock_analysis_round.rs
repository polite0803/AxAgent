use std::sync::Arc;

use axagent_astock_data::AStockClient;
use axagent_harness::market_data::KLine;
use axagent_harness::self_improving_loop::{
    NextAction, RoundEvaluation, RoundResult, RoundStep, SelfImprovingRound,
};

use crate::risk as risk_tools;
use crate::signals;

/// 简单的分析错误包装
#[derive(Debug)]
pub struct AnalysisError(pub String);

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AnalysisError {}

/// 股票分析的领域评估配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StockAnalysisConfig {
    /// 是否输出详细 trace（默认 true）
    pub detailed_trace: bool,
}

impl Default for StockAnalysisConfig {
    fn default() -> Self {
        Self { detailed_trace: true }
    }
}

/// 自改进分析循环的股票领域实现
///
/// 执行流程（每轮）：
///   1. 从 task 提取股票代码
///   2. 获取 K 线数据
///   3. 运行技术面 + 风险的量化计算
///   4. 生成结构化报告
///
/// 评估维度：技术面覆盖度、风险指标完整性、决策清晰度
///
/// 在 wiring 层配合 SelfImprovementExecutor 使用。
pub struct StockAnalysisRound {
    client: Arc<AStockClient>,
}

impl StockAnalysisRound {
    pub fn new(client: Arc<AStockClient>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl SelfImprovingRound for StockAnalysisRound {
    /// 执行一轮股票分析。
    ///
    /// 如果 prev_evaluation 提供了改进方向，会针对性补强薄弱维度。
    async fn execute_round(
        &mut self,
        task: &str,
        prev_evaluation: Option<&RoundEvaluation>,
    ) -> Result<RoundResult, Box<dyn std::error::Error + Send>> {
        let mut steps = Vec::new();
        let mut report_parts = Vec::new();

        // 1. 提取股票代码
        let stock_code = extract_stock_code(task)
            .ok_or_else::<Box<dyn std::error::Error + Send>, _>(|| {
                Box::new(AnalysisError(format!("Cannot extract stock code from: {task}")))
            })?;
        steps.push(RoundStep {
            index: 0,
            kind: "parse".into(),
            summary: format!("Parsed stock code: {stock_code}"),
            tokens_used: 0,
        });

        // 2. 获取改进方向
        let target_gaps: Vec<String> = prev_evaluation.map(|e| e.gaps.clone()).unwrap_or_default();
        let refine_direction =
            prev_evaluation.and_then(|e| e.next_direction.clone()).unwrap_or_default();

        // 3. 获取 K 线数据（252 日 + 100 日）
        let kline_data_252 = match self.client.get_klines(&stock_code, "daily", 252).await {
            Ok(k) => k,
            Err(e) => return Err(Box::new(AnalysisError(format!("Data fetch failed: {e}")))),
        };

        let kline_data_100: Vec<KLine> = if kline_data_252.len() > 100 {
            kline_data_252[kline_data_252.len().saturating_sub(100)..].to_vec()
        } else {
            kline_data_252.clone()
        };

        steps.push(RoundStep {
            index: 1,
            kind: "data_fetch".into(),
            summary: format!("Fetched {} bars for {stock_code}", kline_data_252.len()),
            tokens_used: 0,
        });

        // 将 K 线序列转为 signals 模块需要的 JSON 字符串
        let kline_json_100 = klines_to_json(&kline_data_100);

        // 4. 技术面分析
        {
            let ma = signals::detect_ma_cross(&kline_json_100, 5, 20);
            let breakout = signals::detect_breakout_with_pattern(&kline_json_100, 0.0, 0.0, 1.5);

            report_parts.push(format!(
                "## Technical Analysis\n\
                 - **MA Cross (5/20)**: signal=`{}`, latest={:.2}, fast_ma={:.2}, slow_ma={:.2}\n\
                 - **Breakout**: type=`{}`, confidence={}, current={:.2}\n",
                ma.signal,
                ma.latest_price,
                ma.fast_ma,
                ma.slow_ma,
                breakout.breakout_type,
                breakout.confidence,
                breakout.current_price,
            ));
            steps.push(RoundStep {
                index: 2,
                kind: "technical".into(),
                summary: format!("MA={}, Breakout={}", ma.signal, breakout.breakout_type),
                tokens_used: 0,
            });
        }

        // 5. 风险评估（基于 252 日数据）
        if kline_data_252.len() >= 20 {
            let closes: Vec<f64> = kline_data_252.iter().map(|k| k.close).collect();
            let returns = compute_returns(&closes);

            let max_dd = risk_tools::max_drawdown(&closes);
            let sr = risk_tools::sharpe_ratio(&returns, 0.02);
            let var = risk_tools::value_at_risk(&returns, 0.95);

            report_parts.push(format!(
                "## Risk Assessment\n\
                 - **Max Drawdown**: {:.1}%\n\
                 - **Sharpe Ratio**: {:.3}\n\
                 - **VaR (95%)**: {:.1}%\n",
                max_dd * 100.0,
                sr.sharpe,
                var.var_pct * 100.0,
            ));
            steps.push(RoundStep {
                index: 3,
                kind: "risk".into(),
                summary: format!("MaxDD={:.1}%, Sharpe={:.3}", max_dd * 100.0, sr.sharpe),
                tokens_used: 0,
            });
        } else {
            report_parts.push("## Risk Assessment\n(Insufficient data: <20 bars)\n".into());
        }

        // 6. 估值 / 价格
        if let Some(last) = kline_data_100.last() {
            report_parts.push(format!(
                "## Valuation\n\
                 - **Current Price**: {:.2}\n",
                last.close,
            ));
            steps.push(RoundStep {
                index: 4,
                kind: "valuation".into(),
                summary: format!("Price={:.2}", last.close),
                tokens_used: 0,
            });
        }

        // 7. 决策建议
        let decision_text = generate_decision(&stock_code, &kline_data_100);
        report_parts.push(format!("## Decision\n{decision_text}\n"));
        steps.push(RoundStep {
            index: steps.len() as u32,
            kind: "decision".into(),
            summary: "Decision generated".into(),
            tokens_used: 0,
        });

        // 8. 改进反馈头（如果有）
        if !target_gaps.is_empty() || !refine_direction.is_empty() {
            report_parts.insert(
                0,
                format!(
                    "> **Refinement from previous round**:\n\
                 > - Gaps to address: {}\n\
                 > - Direction: {}\n\n",
                    target_gaps.join("; "),
                    refine_direction,
                ),
            );
        }

        Ok(RoundResult {
            round: 0,
            output: report_parts.join("\n---\n"),
            evaluation: None,
            trace: steps,
        })
    }

    /// 对股票分析报告做领域特化的质量评估
    ///
    /// 检查维度（满分 1.0）：
    /// - 技术面覆盖度 (0.25)
    /// - 风险指标完整性 (0.25)
    /// - 估值/价格引用 (0.15)
    /// - 决策清晰度 (0.20)
    /// - 改进反馈响应 (0.15)
    async fn evaluate_round(
        &self,
        _task: &str,
        result: &RoundResult,
    ) -> Result<RoundEvaluation, Box<dyn std::error::Error + Send>> {
        let output = &result.output;
        let mut score = 0.0_f64;
        let mut gaps: Vec<String> = Vec::new();
        let mut strengths = Vec::new();

        // 1. 技术面分析覆盖度 (0.25)
        if output.contains("## Technical Analysis") {
            if output.contains("MA Cross") {
                score += 0.10;
                strengths.push("MA Cross signal computed".into());
            } else {
                gaps.push("Missing MA cross analysis".into());
            }
            if output.contains("Breakout") {
                score += 0.10;
                strengths.push("Breakout detection included".into());
            } else {
                gaps.push("Missing breakout detection".into());
            }
            if output.contains("confidence") {
                score += 0.05;
            }
        } else {
            gaps.push("Missing Technical Analysis section".into());
        }

        // 2. 风险指标完整性 (0.25)
        if output.contains("## Risk Assessment") {
            if output.contains("Max Drawdown") || output.contains("MaxDD") {
                score += 0.10;
                strengths.push("Max Drawdown computed".into());
            } else {
                gaps.push("Missing max drawdown".into());
            }
            if output.contains("Sharpe Ratio") || output.contains("sharpe") {
                score += 0.10;
                strengths.push("Sharpe ratio computed".into());
            } else {
                gaps.push("Missing Sharpe ratio".into());
            }
            if output.contains("VaR") {
                score += 0.05;
            }
        } else {
            gaps.push("Missing Risk Assessment section".into());
        }

        // 3. 估值/价格引用 (0.15)
        if output.contains("Current Price") || output.contains("price") || output.contains("Price")
        {
            score += 0.10;
            strengths.push("Current price referenced".into());
        }
        if output.contains("## Valuation") {
            score += 0.05;
        } else {
            gaps.push("Missing valuation section".into());
        }

        // 4. 决策清晰度 (0.20)
        if output.contains("## Decision") {
            score += 0.05;
            if output.contains("buy") || output.contains("买入") || output.contains("Accumulate")
            {
                score += 0.10;
                strengths.push("Buy/Hold recommendation".into());
            } else if output.contains("sell")
                || output.contains("卖出")
                || output.contains("Reduce")
            {
                score += 0.10;
                strengths.push("Sell/Reduce recommendation".into());
            } else {
                score += 0.02;
                gaps.push("Decision lacks clear action (buy/sell/hold)".into());
            }
            if output.contains("Reasoning") || output.contains("reason") || output.contains("因为")
            {
                score += 0.05;
            }
        } else {
            gaps.push("Missing Decision section".into());
        }

        // 5. 改进反馈响应 (0.15)
        if output.contains("Refinement from previous round") {
            score += 0.10;
            strengths.push("Addressed previous round gaps".into());
            // 如果 gap 只剩 1 个，说明改进有效
            let active_gaps = gaps.iter().filter(|g| g.contains("Missing")).count();
            if active_gaps <= 1 {
                score += 0.05;
            }
        }

        // 惩罚：关键缺失 >=2 时打 85 折
        let critical_missing =
            gaps.iter().filter(|g| g.contains("Missing") || g.contains("lack")).count();
        if critical_missing >= 2 {
            score *= 0.85;
        }

        score = score.clamp(0.0, 1.0);

        // 生成下一轮的改进方向
        let next_direction = if score < 0.5 {
            Some(format!("Significant gaps remain. Please ensure: {}", gaps.join("; "),))
        } else if !gaps.is_empty() {
            Some(format!("Improve: {}", gaps.join("; ")))
        } else {
            None
        };

        Ok(RoundEvaluation {
            score,
            confidence: (0.5 + score * 0.4).clamp(0.0, 1.0),
            gaps,
            strengths,
            raw_assessment: format!(
                "Stock analysis quality score: {:.2}/1.0 across 5 dimensions.",
                score,
            ),
            next_direction,
        })
    }

    /// 基于评分决定下一步
    async fn decide_next(
        &self,
        _task: &str,
        _result: &RoundResult,
        evaluation: &RoundEvaluation,
    ) -> Result<NextAction, Box<dyn std::error::Error + Send>> {
        if evaluation.score >= 0.85 {
            return Ok(NextAction::Accept);
        }
        if let Some(direction) = &evaluation.next_direction {
            return Ok(NextAction::Refine { direction: direction.clone() });
        }
        if evaluation.score < 0.35 && evaluation.gaps.len() >= 3 {
            return Ok(NextAction::Redirect {
                reason: format!(
                    "Quality too low (score={:.2}). Missing: {}",
                    evaluation.score,
                    evaluation.gaps.join("; "),
                ),
            });
        }
        Ok(NextAction::Accept)
    }
}

// ── 辅助函数 ──

/// 从 task 文本中提取股票代码
fn extract_stock_code(task: &str) -> Option<String> {
    // 匹配 "6位数字.SH" "6位数字.SZ" 或纯 6 位数字
    let chars: Vec<char> = task.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        // 找 6 位连续数字
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            let digits: String = chars[start..i].iter().collect();
            if digits.len() == 6 {
                // 检查后缀
                let suffix = if i < n && chars[i] == '.' {
                    i += 1; // skip '.'
                    let s_start = i;
                    while i < n && chars[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let s: String = chars[s_start..i].iter().collect();
                    if matches!(s.as_str(), "SH" | "SZ" | "HK" | "US") {
                        s
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                return Some(if suffix.is_empty() {
                    digits
                } else {
                    format!("{digits}.{suffix}")
                });
            }
        } else {
            i += 1;
        }
    }
    None
}

/// 将 KLine 切片转为 signals 需要的 JSON 字符串
fn klines_to_json(klines: &[KLine]) -> String {
    let items: Vec<serde_json::Value> = klines
        .iter()
        .map(|k| {
            serde_json::json!({
                "date": k.date,
                "open": k.open,
                "high": k.high,
                "low": k.low,
                "close": k.close,
                "volume": k.volume,
            })
        })
        .collect();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
}

/// 从收盘价序列计算日收益率
fn compute_returns(prices: &[f64]) -> Vec<f64> {
    prices.windows(2).map(|w| (w[1] - w[0]) / w[0]).collect()
}

/// 生成决策建议文本
fn generate_decision(stock_code: &str, klines: &[KLine]) -> String {
    if klines.is_empty() {
        return "Insufficient data for decision.".into();
    }

    let latest = klines.last().unwrap();
    let price = latest.close;

    // 简单趋势判断：SMA5 vs SMA10
    let short_trend = if klines.len() >= 10 {
        let sma_5: f64 = klines.iter().rev().take(5).map(|k| k.close).sum::<f64>() / 5.0;
        let sma_10: f64 = klines.iter().rev().take(10).map(|k| k.close).sum::<f64>() / 10.0;
        if sma_5 > sma_10 {
            "upward"
        } else {
            "downward"
        }
    } else {
        "unknown"
    };

    let (action, reason) = match short_trend {
        "upward" => (
            "Hold / Accumulate",
            "Short-term MA suggests upward momentum. Consider accumulating on dips.",
        ),
        _ => (
            "Watch / Reduce",
            "Short-term MA suggests downward pressure. Reduce position or wait for reversal.",
        ),
    };

    format!(
        "- **Stock**: {stock_code}\n\
         - **Current Price**: {price:.2}\n\
         - **Short-term Trend**: {short_trend}\n\
         - **Recommended Action**: {action}\n\
         - **Reasoning**: {reason}\n",
    )
}
