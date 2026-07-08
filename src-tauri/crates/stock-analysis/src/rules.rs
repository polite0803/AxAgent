use axagent_astock_data::indicators::TechnicalIndicators;

use crate::decision::RuleConfig;
use crate::scoring::ObjectiveScore;

/// 规则检查结果
#[derive(Debug, Clone)]
pub struct RuleCheckResult {
    pub passed: bool,
    pub violations: Vec<String>,
    pub corrections: Vec<String>,
    pub force_signal: Option<String>, // 强制覆盖的信号
}

/// 严进规则引擎 -- 巴菲特+段永平投资纪律（阈值可配置）
pub struct RuleEngine;

impl RuleEngine {
    /// 检查交易方案是否违反硬性规则
    ///
    /// - `catalyst_level`: a-catalyst 输出的催化剂等级（None=中性）
    /// - `institutional_trace`: a-catalyst 输出的机构建仓痕迹（None=无）
    ///
    /// 当 L2+ 催化剂 + 机构建仓 + "放量突破" 同时出现时，乖离率/RSI 改发 correction 而非 violation
    #[allow(clippy::too_many_arguments)]
    pub fn check(
        indicators: &TechnicalIndicators,
        score: &ObjectiveScore,
        proposed_action: &str,
        proposed_stop_loss: Option<f64>,
        proposed_entry_price: Option<f64>,
        config: &RuleConfig,
        catalyst_level: Option<&str>,
        institutional_trace: Option<&str>,
    ) -> RuleCheckResult {
        let mut violations = Vec::new();
        let mut corrections = Vec::new();
        let mut force_signals: Vec<String> = Vec::new();

        let is_buy = matches!(proposed_action, "买入" | "增持");

        // catalyst_override 路径：L2+ 催化剂 + 机构建仓 + 放量突破 → 容忍追高
        let catalyst_override =
            matches!(catalyst_level, Some("L2业绩拐点级") | Some("L3估值体系级"))
                && matches!(institutional_trace, Some("有建仓痕迹") | Some("疑似建仓"))
                && indicators.volume_signal == "放量突破";
        let effective_rsi_limit = if catalyst_override {
            95.0
        } else {
            config.rsi_overbought
        };
        let effective_bias_limit = if catalyst_override {
            12.0
        } else {
            config.bias_limit
        };

        if is_buy && indicators.rsi6 > effective_rsi_limit {
            violations.push(format!(
                "RSI6={:.1}>{:.0}处于严重超买，禁止买入。",
                indicators.rsi6, effective_rsi_limit
            ));
            force_signals.push("block_buy".to_string());
        }

        if is_buy && indicators.bias_ma5 > effective_bias_limit {
            if catalyst_override {
                // 突破 + 催化剂共振：只发 correction，不发 violation
                corrections.push(format!(
                    "乖离MA5={:.1}%>{:.0}%，但出现放量突破+L{}+机构建仓三重共振，\
                     容忍追高；建议减仓至 50%，止损设于 MA10={:.2}。",
                    indicators.bias_ma5,
                    effective_bias_limit,
                    if catalyst_level == Some("L3估值体系级") {
                        "3"
                    } else {
                        "2"
                    },
                    indicators.ma10
                ));
            } else {
                violations.push(format!(
                    "乖离MA5={:.1}%>{:.0}%，禁止追高。建议等待回调至MA5（{:.2}）附近。",
                    indicators.bias_ma5, effective_bias_limit, indicators.ma5
                ));
                force_signals.push("block_buy".to_string());
            }
        }

        if proposed_stop_loss.is_none() || proposed_stop_loss == Some(0.0) {
            violations.push("缺少止损价位，不符合严进策略要求。".to_string());
            let auto_stop = if let Some(entry) = proposed_entry_price {
                let ma20_stop = indicators.ma20;
                let pct_stop = entry * (1.0 - config.auto_stop_loss_pct / 100.0);
                if ma20_stop > pct_stop {
                    ma20_stop
                } else {
                    pct_stop
                }
            } else {
                indicators.ma20
            };
            corrections.push(format!(
                "自动设定止损价: {:.2}（MA20={:.2}和-{:.0}%止损={:.2}取较大值）",
                auto_stop,
                indicators.ma20,
                config.auto_stop_loss_pct,
                proposed_entry_price.unwrap_or(0.0) * (1.0 - config.auto_stop_loss_pct / 100.0)
            ));
        }

        if config.volume_signal_block && is_buy && indicators.volume_signal == "放量下跌" {
            violations.push("放量下跌中禁止买入，等待缩量企稳信号。".to_string());
            force_signals.push("block_buy".to_string());
        }

        if is_buy && indicators.ma_alignment == "空头排列" && score.total < config.bear_low_score
        {
            violations.push(format!(
                "空头排列+评分{}<{}，绝不适合买入。",
                score.total, config.bear_low_score
            ));
            force_signals.push("block_all".to_string());
        }

        if indicators.rsi6 < config.rsi_oversold {
            corrections.push(format!(
                "提示: RSI6={:.1}<{:.0}处于超卖区域，关注超跌反弹机会。",
                indicators.rsi6, config.rsi_oversold
            ));
        }

        let force_signal = if force_signals.iter().any(|s| s == "block_all") {
            Some("block_all".to_string())
        } else if force_signals.iter().any(|s| s == "block_buy") {
            Some("block_buy".to_string())
        } else {
            None
        };

        RuleCheckResult {
            passed: violations.is_empty(),
            violations,
            corrections,
            force_signal,
        }
    }
}
