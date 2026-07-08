//! 回测→Prompt 优化闭环 (P0-2)
//!
//! 借鉴 TradingAgents-AShare 的"回测驱动的 Prompt 优化"理念，
//! 将 BacktestEngine 的分析结果反馈到 Agent Prompt 的自动优化和参数调整。
//!
//! ## 核心设计
//!
//! 1. **分析师级回测分析**：统计每位分析师在回测中的表现（准确率、方向偏差、置信度校准）
//! 2. **Prompt 调整建议生成**：基于表现数据，生成可操作的 Prompt 修改建议
//! 3. **版本追踪**：记录每次 Prompt 调整的历史，关联表现变化
//!
//! ## 工作流
//!
//! ```text
//! 回测完成 → backtest_feedback::analyze()
//!   → 每位分析师的准确率/偏差分析
//!   → 生成 PromptAdjustmentReport
//!   → 可选的自动权重调整 (调用 weight_decay)
//!   → 记录到 strategy_weight_history 表
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 分析结果结构 ──

/// 单分析师回测表现
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalystBacktestPerformance {
    /// 分析师 ID
    pub analyst_id: String,
    /// 总参与次数
    pub total_participations: u32,
    /// 准确次数（判断方向正确的次数）
    pub correct_count: u32,
    /// 准确率
    pub accuracy: f64,
    /// 方向偏差: >0 表示偏多, <0 表示偏空
    pub direction_bias: f64,
    /// 置信度校准: 预测置信度 vs 实际准确率的差距
    pub confidence_calibration: f64,
    /// 建议：调整方向
    pub suggestion: PromptSuggestion,
    /// 胜率趋势: "improving" | "declining" | "stable" | "insufficient_data"
    pub trend: String,
    /// 关联的 time_horizon 维度
    pub time_horizon: String,
}

/// Prompt 调整建议
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSuggestion {
    /// 建议类型:
    /// - "none": 无需调整
    /// - "adjust_weight": 仅调整权重
    /// - "tweak_prompt": 微调提示词(降低/提升某方面 bias)
    /// - "review_logic": 重构分析逻辑
    /// - "disable": 暂时禁用(表现极差)
    pub suggestion_type: String,
    /// 权重调整建议 (0-3, 1=不变)
    pub suggested_weight: f64,
    /// 可读的建议描述
    pub description: String,
}

/// 完整反馈报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestFeedbackReport {
    /// 总回测样本数
    pub total_samples: u32,
    /// 整体准确率
    pub overall_accuracy: f64,
    /// 每位分析师的表现分析
    pub analyst_performances: Vec<AnalystBacktestPerformance>,
    /// 表现最好的分析师
    pub top_performers: Vec<String>,
    /// 表现最差的分析师（需要关注）
    pub bottom_performers: Vec<String>,
    /// 是否需要调整
    pub requires_adjustment: bool,
    /// 生成时间戳
    pub generated_at: i64,
}

// ── 输入数据结构 ──

/// 单次分析的参与记录（从回测结果反推）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisParticipation {
    /// 分析师 ID
    pub analyst_id: String,
    /// 时间维度
    pub time_horizon: String,
    /// 分析日期 (YYYY-MM-DD)
    pub date: String,
    /// 该分析师的立场方向: "bullish" | "bearish" | "neutral"
    pub stance: String,
    /// 该分析师的置信度 (0-1)
    pub confidence: f64,
    /// 实际走势是否正确
    pub was_correct: bool,
}

/// 用于生成反馈的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackInput {
    pub participations: Vec<AnalysisParticipation>,
}

// ── 核心分析逻辑 ──

/// 分析回测结果，生成反馈报告
pub fn analyze_backtest_feedback(input: FeedbackInput) -> BacktestFeedbackReport {
    let now_ms = chrono::Utc::now().timestamp_millis();

    // 1. 按 analyst_id + time_horizon 分组统计
    let mut grouped: HashMap<(String, String), Vec<&AnalysisParticipation>> = HashMap::new();
    for p in &input.participations {
        grouped
            .entry((p.analyst_id.clone(), p.time_horizon.clone()))
            .or_default()
            .push(p);
    }

    // 2. 计算每位分析师的表现
    let mut performances: Vec<AnalystBacktestPerformance> = grouped
        .into_iter()
        .map(|((analyst_id, time_horizon), participations)| {
            let total = participations.len() as u32;
            let correct = participations.iter().filter(|p| p.was_correct).count() as u32;
            let accuracy = if total > 0 {
                correct as f64 / total as f64
            } else {
                0.0
            };

            // 方向偏差: bullish占比 - bearish占比
            let bullish_count = participations
                .iter()
                .filter(|p| p.stance == "bullish")
                .count() as f64;
            let bearish_count = participations
                .iter()
                .filter(|p| p.stance == "bearish")
                .count() as f64;
            let direction_bias = if total > 0 {
                (bullish_count - bearish_count) / total as f64
            } else {
                0.0
            };

            // 置信度校准: 平均置信度 - 准确率
            let avg_confidence: f64 = if total > 0 {
                participations.iter().map(|p| p.confidence).sum::<f64>() / total as f64
            } else {
                0.0
            };
            let confidence_calibration = avg_confidence - accuracy;

            // 趋势判断（简单的一半比较）
            let trend = if total < 5 {
                "insufficient_data"
            } else {
                let mid = total as usize / 2;
                let recent_half: Vec<_> = participations.iter().skip(mid).collect();
                let recent_correct = recent_half.iter().filter(|p| p.was_correct).count();
                let recent_accuracy = recent_correct as f64 / recent_half.len() as f64;

                let early_half: Vec<_> = participations.iter().take(mid).collect();
                let early_correct = early_half.iter().filter(|p| p.was_correct).count();
                let early_accuracy = early_correct as f64 / early_half.len() as f64;

                if recent_accuracy > early_accuracy + 0.1 {
                    "improving"
                } else if recent_accuracy < early_accuracy - 0.1 {
                    "declining"
                } else {
                    "stable"
                }
            };

            // 生成 Prompt 调整建议
            let suggestion = generate_suggestion(accuracy, confidence_calibration, trend, total);

            AnalystBacktestPerformance {
                analyst_id: analyst_id.clone(),
                total_participations: total,
                correct_count: correct,
                accuracy,
                direction_bias,
                confidence_calibration,
                suggestion,
                trend: trend.to_string(),
                time_horizon,
            }
        })
        .collect();

    // 3. 排序：按准确率降序
    performances.sort_by(|a, b| {
        b.accuracy
            .partial_cmp(&a.accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 4. 找最好和最差（不考虑 insufficient_data）
    let meaningful: Vec<&AnalystBacktestPerformance> = performances
        .iter()
        .filter(|p| p.total_participations >= 5)
        .collect();
    let top_performers: Vec<String> = meaningful
        .iter()
        .take(3)
        .map(|p| p.analyst_id.clone())
        .collect();
    let bottom_performers: Vec<String> = meaningful
        .iter()
        .rev()
        .take(3)
        .map(|p| p.analyst_id.clone())
        .collect();

    // 5. 计算整体准确率
    let total_samples = input.participations.len() as u32;
    let total_correct = input
        .participations
        .iter()
        .filter(|p| p.was_correct)
        .count() as u32;
    let overall_accuracy = if total_samples > 0 {
        total_correct as f64 / total_samples as f64
    } else {
        0.0
    };

    // 6. 是否需要调整
    let requires_adjustment = performances
        .iter()
        .any(|p| p.suggestion.suggestion_type != "none");

    BacktestFeedbackReport {
        total_samples,
        overall_accuracy,
        analyst_performances: performances,
        top_performers,
        bottom_performers,
        requires_adjustment,
        generated_at: now_ms,
    }
}

/// 根据分析师表现生成 Prompt 调整建议
fn generate_suggestion(
    accuracy: f64,
    calibration: f64,
    trend: &str,
    samples: u32,
) -> PromptSuggestion {
    if samples < 3 {
        return PromptSuggestion {
            suggestion_type: "none".into(),
            suggested_weight: 1.0,
            description: format!("样本不足({}),暂不调整", samples),
        };
    }

    // 准确率极低 → 需审视逻辑
    if accuracy < 0.3 && samples >= 10 {
        return PromptSuggestion {
            suggestion_type: "review_logic".into(),
            suggested_weight: 0.5, // 降权到 50%
            description: format!(
                "准确率仅 {:.0}%,样本 {} 个,建议审查分析逻辑是否出系统性偏差",
                accuracy * 100.0,
                samples
            ),
        };
    }

    // 准确率偏低 → 微调 + 降权
    if accuracy < 0.4 {
        return PromptSuggestion {
            suggestion_type: "tweak_prompt".into(),
            suggested_weight: 0.7,
            description: format!("准确率 {:.0}%,建议降低 bias 强度,权重降至 0.7", accuracy * 100.0),
        };
    }

    // 置信度校准: 过度自信 (confidence >> accuracy)
    if calibration > 0.2 {
        return PromptSuggestion {
            suggestion_type: "tweak_prompt".into(),
            suggested_weight: 1.0,
            description: format!(
                "置信度校准偏移({:+.0}%),过于自信,建议提示词中降低确定性表述",
                calibration * 100.0
            ),
        };
    }

    // 趋势下降
    if trend == "declining" && accuracy <= 0.55 {
        return PromptSuggestion {
            suggestion_type: "tweak_prompt".into(),
            suggested_weight: 0.85,
            description: format!(
                "胜率呈下降趋势(当前 {:.0}%),建议降权至 0.85 并关注",
                accuracy * 100.0
            ),
        };
    }

    // 表现稳定或优秀
    if accuracy >= 0.6 {
        return PromptSuggestion {
            suggestion_type: "adjust_weight".into(),
            suggested_weight: (1.0 + (accuracy - 0.5) * 2.0).clamp(1.0, 1.5),
            description: format!(
                "表现优秀(准确率 {:.0}%),建议提权至 {:.2}",
                accuracy * 100.0,
                (1.0 + (accuracy - 0.5) * 2.0).clamp(1.0, 1.5)
            ),
        };
    }

    PromptSuggestion {
        suggestion_type: "none".into(),
        suggested_weight: 1.0,
        description: "表现正常,无需调整".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participation(
        analyst_id: &str,
        horizon: &str,
        stance: &str,
        confidence: f64,
        correct: bool,
        days_ago: i32,
    ) -> AnalysisParticipation {
        let date = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(days_ago as i64))
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        AnalysisParticipation {
            analyst_id: analyst_id.to_string(),
            time_horizon: horizon.to_string(),
            date,
            stance: stance.to_string(),
            confidence,
            was_correct: correct,
        }
    }

    #[test]
    fn empty_input_returns_empty_report() {
        let report = analyze_backtest_feedback(FeedbackInput {
            participations: vec![],
        });
        assert_eq!(report.total_samples, 0);
        assert!(report.analyst_performances.is_empty());
        assert!(!report.requires_adjustment);
    }

    #[test]
    fn high_accuracy_earns_weight_boost() {
        let participations: Vec<_> = (0..30)
            .map(|i| participation("a-technical", "short", "bullish", 0.8, i % 3 != 0, i))
            .collect();
        let report = analyze_backtest_feedback(FeedbackInput { participations });
        let tech = report
            .analyst_performances
            .iter()
            .find(|p| p.analyst_id == "a-technical")
            .unwrap();
        // 30 次中约 20 次正确(~66% 准确率)
        assert!(tech.accuracy > 0.5, "高准确率应 > 0.5, 实际={}", tech.accuracy);
        assert!(
            tech.suggestion.suggested_weight > 1.0,
            "高准确率应提权, 实际={}",
            tech.suggestion.suggested_weight
        );
    }

    #[test]
    fn low_accuracy_triggers_review() {
        let participations: Vec<_> = (0..20)
            .map(|i| participation("bad-analyst", "mid", "bullish", 0.9, false, i))
            .collect();
        let report = analyze_backtest_feedback(FeedbackInput { participations });
        let bad = report
            .analyst_performances
            .iter()
            .find(|p| p.analyst_id == "bad-analyst")
            .unwrap();
        assert!(bad.accuracy < 0.3, "故意设错应低于 0.3, 实际={}", bad.accuracy);
        assert_eq!(bad.suggestion.suggestion_type, "review_logic", "极低准确率应触发 review_logic");
    }

    #[test]
    fn overconfidence_detected() {
        // 高置信度但低准确率
        let participations: Vec<_> = (0..15)
            .map(|i| participation("overconfident", "short", "bullish", 0.95, i % 2 == 0, i))
            .collect();
        let report = analyze_backtest_feedback(FeedbackInput { participations });
        let oc = report
            .analyst_performances
            .iter()
            .find(|p| p.analyst_id == "overconfident")
            .unwrap();
        // 50% 准确率,但置信度 0.95 → 校准偏移 0.45
        assert!(
            oc.confidence_calibration > 0.3,
            "过度自信应被检测: calibration={}",
            oc.confidence_calibration
        );
        assert_eq!(oc.suggestion.suggestion_type, "tweak_prompt", "过度自信应触发 tweak_prompt");
    }

    #[test]
    fn declining_trend_identified() {
        // 前 10 次全对,后 10 次全错
        let mut participations = vec![];
        for i in 0..10 {
            participations.push(participation("declining", "long", "bullish", 0.7, true, 30 - i));
        }
        for i in 0..10 {
            participations.push(participation("declining", "long", "bullish", 0.7, false, 10 - i));
        }
        let report = analyze_backtest_feedback(FeedbackInput { participations });
        let decl = report
            .analyst_performances
            .iter()
            .find(|p| p.analyst_id == "declining")
            .unwrap();
        assert_eq!(decl.trend, "declining", "下降趋势应被检测, 实际={}", decl.trend);
        assert!(decl.suggestion.suggested_weight < 1.0, "下降趋势应降权");
    }

    #[test]
    fn insufficient_samples_no_adjustment() {
        let participations = vec![
            participation("newbie", "short", "bullish", 0.7, true, 1),
            participation("newbie", "short", "bullish", 0.7, false, 2),
        ];
        let report = analyze_backtest_feedback(FeedbackInput { participations });
        let n = report
            .analyst_performances
            .iter()
            .find(|p| p.analyst_id == "newbie")
            .unwrap();
        assert_eq!(n.suggestion.suggestion_type, "none");
    }

    #[test]
    fn strong_performers_listed_in_top() {
        let mut participations = vec![];
        for i in 0..20 {
            participations.push(participation("star", "short", "bullish", 0.8, true, i));
        }
        for i in 0..20 {
            participations.push(participation("laggard", "short", "bullish", 0.8, false, i));
        }
        let report = analyze_backtest_feedback(FeedbackInput { participations });
        assert!(report.top_performers.contains(&"star".to_string()), "星号分析师应在 top 列表");
        assert!(
            report.bottom_performers.contains(&"laggard".to_string()),
            "落后分析师应在 bottom 列表"
        );
    }
}
