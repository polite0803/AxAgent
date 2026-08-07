//! 反思工作流（stock-reflection）事后验证
//!
//! 对应 `serenity_hit_rate_backtest.rs`（瓶颈掘金），本模块针对反思工作流的
//! `reflection_lessons` 表做规则有效性验证。
//!
//! # 核心问题
//! reflection-agent 提取的 lesson 直接写入 `reflection_lessons` 表，
//! `confidence` 仅基于 verdict 标签硬编码（wrong=0.7, partial=0.5, else=0.3），
//! 无后续命中率统计。本模块通过追踪规则被引用后的决策表现，验证规则是否真的
//! "避免重蹈覆辙"。
//!
//! # 验证维度
//! - 规则被引用次数（times_applied）
//! - 引用后决策成功率（success_count / times_applied）
//! - 规则置信度衰减/提升（基于实际表现调整 confidence）
//! - 按 verdict 分层的规则有效性（wrong 类规则 vs partial 类规则）
//!
//! # 与 serenity_hit_rate_backtest 的差异
//! - serenity 验证候选股在 T+N 内是否跑赢行业均值
//! - reflection 验证规则被引用后决策是否成功（基于 stock_analyses 表的 posterior）

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────
// 数据结构
// ─────────────────────────────────────────────────

/// 反思规则验证记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonValidation {
    /// 规则 ID（reflection_lessons.id）
    pub lesson_id: String,
    /// 规则摘要
    pub lesson_summary: String,
    /// 来源反思的 verdict（correct/partial/wrong）
    pub source_verdict: String,
    /// 适用股票代码（None=通用规则）
    pub stock_code: Option<String>,
    /// 验证时间点
    pub validated_at: String,
    /// 已应用次数（来自 reflection_lessons.times_applied）
    pub times_applied: i32,
    /// 应用后成功次数（来自 reflection_lessons.success_count）
    pub success_count: i32,
    /// 原始置信度（来自 reflection_lessons.confidence）
    pub original_confidence: f64,
    /// 调整后置信度（基于实际表现）
    pub adjusted_confidence: f64,
    /// 实际成功率 = success_count / times_applied
    pub actual_success_rate: f64,
    /// 验证状态：validated / insufficient_data / deprecated
    pub validation_status: String,
    /// 置信度调整说明
    pub adjustment_note: String,
    pub created_at: String,
}

/// 反思规则验证报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonValidationReport {
    /// 总规则数
    pub total_lessons: usize,
    /// 已验证规则数（times_applied > 0）
    pub validated_lessons: usize,
    /// 待验证规则数（times_applied == 0）
    pub pending_lessons: usize,
    /// 已废弃规则数（confidence < 0.2）
    pub deprecated_lessons: usize,
    /// 平均实际成功率
    pub avg_success_rate: f64,
    /// 按 verdict 分层的规则表现
    pub verdict_breakdown: HashMap<String, VerdictStats>,
    /// 置信度调整统计
    pub confidence_adjustment_stats: ConfidenceAdjustmentStats,
    pub generated_at: String,
}

/// 按 verdict 分层的规则统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerdictStats {
    pub total: usize,
    pub avg_success_rate: f64,
    pub avg_confidence: f64,
    pub avg_adjusted_confidence: f64,
}

/// 置信度调整统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfidenceAdjustmentStats {
    pub increased: usize,
    pub decreased: usize,
    pub unchanged: usize,
    pub avg_delta: f64,
}

// ─────────────────────────────────────────────────
// 核心函数
// ─────────────────────────────────────────────────

/// 计算反思规则的实际成功率
///
/// # 参数
/// - `success_count`: 应用后成功次数
/// - `times_applied`: 已应用次数
///
/// # 返回
/// - `times_applied == 0` → 0.0（无数据）
/// - `times_applied > 0` → success_count / times_applied
pub fn compute_actual_success_rate(success_count: i32, times_applied: i32) -> f64 {
    if times_applied <= 0 {
        return 0.0;
    }
    let rate = success_count as f64 / times_applied as f64;
    rate.clamp(0.0, 1.0)
}

/// 基于实际表现调整规则置信度
///
/// # 调整策略
/// - `times_applied == 0` → 保持原 confidence（无数据）
/// - `times_applied < 3` → 保持原 confidence（样本不足）
/// - `times_applied >= 3` 且实际成功率 ≥ 0.6 → confidence × 1.1（上限 0.95）
/// - `times_applied >= 3` 且实际成功率 < 0.6 → confidence × 0.8
/// - `times_applied >= 5` 且实际成功率 < 0.3 → confidence = 0.1（标记废弃）
///
/// # 参数
/// - `original_confidence`: 原始置信度
/// - `actual_success_rate`: 实际成功率
/// - `times_applied`: 已应用次数
pub fn adjust_lesson_confidence(
    original_confidence: f64,
    actual_success_rate: f64,
    times_applied: i32,
) -> (f64, String) {
    if times_applied == 0 {
        return (original_confidence, "无应用数据，保持原置信度".to_string());
    }

    if times_applied < 3 {
        return (original_confidence, format!("样本不足（{times_applied}<3），保持原置信度"));
    }

    // 强废弃：5 次以上应用且成功率 < 30%
    if times_applied >= 5 && actual_success_rate < 0.3 {
        return (
            0.1,
            format!(
                "废弃：{times_applied}次应用成功率仅{:.0}%，标记为 deprecated",
                actual_success_rate * 100.0
            ),
        );
    }

    // 提升：3 次以上应用且成功率 ≥ 60%
    if actual_success_rate >= 0.6 {
        let adjusted = (original_confidence * 1.1).min(0.95);
        return (
            adjusted,
            format!(
                "提升：{times_applied}次应用成功率{:.0}%≥60%，confidence {:.2}→{:.2}",
                actual_success_rate * 100.0,
                original_confidence,
                adjusted
            ),
        );
    }

    // 衰减：3 次以上应用且成功率 < 60%
    let adjusted = original_confidence * 0.8;
    (
        adjusted,
        format!(
            "衰减：{times_applied}次应用成功率{:.0}%<60%，confidence {:.2}→{:.2}",
            actual_success_rate * 100.0,
            original_confidence,
            adjusted
        ),
    )
}

/// 判定规则的验证状态
///
/// # 返回
/// - `validated`: times_applied >= 3
/// - `insufficient_data`: 0 < times_applied < 3
/// - `pending`: times_applied == 0
/// - `deprecated`: adjusted_confidence < 0.2
pub fn determine_validation_status(times_applied: i32, adjusted_confidence: f64) -> String {
    if adjusted_confidence < 0.2 {
        return "deprecated".to_string();
    }
    if times_applied == 0 {
        return "pending".to_string();
    }
    if times_applied < 3 {
        return "insufficient_data".to_string();
    }
    "validated".to_string()
}

/// 构建单条规则验证记录
///
/// # 参数
/// - `lesson_id`: 规则 ID
/// - `lesson_summary`: 规则摘要
/// - `source_verdict`: 来源反思的 verdict
/// - `stock_code`: 适用股票代码
/// - `times_applied`: 已应用次数
/// - `success_count`: 成功次数
/// - `original_confidence`: 原始置信度
pub fn build_lesson_validation(
    lesson_id: String,
    lesson_summary: String,
    source_verdict: String,
    stock_code: Option<String>,
    times_applied: i32,
    success_count: i32,
    original_confidence: f64,
) -> LessonValidation {
    let actual_success_rate = compute_actual_success_rate(success_count, times_applied);
    let (adjusted_confidence, adjustment_note) =
        adjust_lesson_confidence(original_confidence, actual_success_rate, times_applied);
    let validation_status = determine_validation_status(times_applied, adjusted_confidence);

    let now = Utc::now().to_rfc3339();
    LessonValidation {
        lesson_id,
        lesson_summary,
        source_verdict,
        stock_code,
        validated_at: now.clone(),
        times_applied,
        success_count,
        original_confidence,
        adjusted_confidence,
        actual_success_rate,
        validation_status,
        adjustment_note,
        created_at: now,
    }
}

/// 构建反思规则验证报告
///
/// # 参数
/// - `validations`: 规则验证记录列表
pub fn build_lesson_validation_report(validations: &[LessonValidation]) -> LessonValidationReport {
    let total_lessons = validations.len();
    let validated_lessons =
        validations.iter().filter(|v| v.validation_status == "validated").count();
    let pending_lessons = validations.iter().filter(|v| v.validation_status == "pending").count();
    let deprecated_lessons =
        validations.iter().filter(|v| v.validation_status == "deprecated").count();

    // 平均成功率（仅计算 times_applied > 0 的）
    let success_rates: Vec<f64> =
        validations.iter().filter(|v| v.times_applied > 0).map(|v| v.actual_success_rate).collect();
    let avg_success_rate = if success_rates.is_empty() {
        0.0
    } else {
        success_rates.iter().sum::<f64>() / success_rates.len() as f64
    };

    // 按 verdict 分层
    let mut verdict_breakdown: HashMap<String, VerdictStats> = HashMap::new();
    for v in validations {
        let entry = verdict_breakdown.entry(v.source_verdict.clone()).or_default();
        entry.total += 1;
        if v.times_applied > 0 {
            entry.avg_success_rate += v.actual_success_rate;
        }
        entry.avg_confidence += v.original_confidence;
        entry.avg_adjusted_confidence += v.adjusted_confidence;
    }
    // 计算平均值
    for stats in verdict_breakdown.values_mut() {
        if stats.total > 0 {
            stats.avg_success_rate /= stats.total as f64;
            stats.avg_confidence /= stats.total as f64;
            stats.avg_adjusted_confidence /= stats.total as f64;
        }
    }

    // 置信度调整统计
    let mut increased = 0usize;
    let mut decreased = 0usize;
    let mut unchanged = 0usize;
    let mut total_delta = 0.0f64;
    let mut delta_count = 0usize;
    for v in validations {
        let delta = v.adjusted_confidence - v.original_confidence;
        if delta.abs() < 1e-6 {
            unchanged += 1;
        } else if delta > 0.0 {
            increased += 1;
        } else {
            decreased += 1;
        }
        total_delta += delta;
        delta_count += 1;
    }
    let avg_delta = if delta_count > 0 {
        total_delta / delta_count as f64
    } else {
        0.0
    };

    LessonValidationReport {
        total_lessons,
        validated_lessons,
        pending_lessons,
        deprecated_lessons,
        avg_success_rate,
        verdict_breakdown,
        confidence_adjustment_stats: ConfidenceAdjustmentStats {
            increased,
            decreased,
            unchanged,
            avg_delta,
        },
        generated_at: Utc::now().to_rfc3339(),
    }
}

// ─────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_actual_success_rate_zero_applied() {
        assert_eq!(compute_actual_success_rate(0, 0), 0.0);
    }

    #[test]
    fn test_compute_actual_success_rate_normal() {
        assert_eq!(compute_actual_success_rate(7, 10), 0.7);
    }

    #[test]
    fn test_compute_actual_success_rate_clamped() {
        // success_count > times_applied 的异常情况
        let rate = compute_actual_success_rate(15, 10);
        assert_eq!(rate, 1.0);
    }

    #[test]
    fn test_adjust_confidence_no_data() {
        let (adjusted, note) = adjust_lesson_confidence(0.7, 0.0, 0);
        assert!((adjusted - 0.7).abs() < 1e-6);
        assert!(note.contains("无应用数据"));
    }

    #[test]
    fn test_adjust_confidence_insufficient_sample() {
        let (adjusted, note) = adjust_lesson_confidence(0.7, 0.5, 2);
        assert!((adjusted - 0.7).abs() < 1e-6);
        assert!(note.contains("样本不足"));
    }

    #[test]
    fn test_adjust_confidence_increase() {
        // 3 次应用，成功率 80% → confidence × 1.1
        let original = 0.6;
        let (adjusted, note) = adjust_lesson_confidence(original, 0.8, 3);
        let expected = (original * 1.1).min(0.95);
        assert!((adjusted - expected).abs() < 1e-6);
        assert!(note.contains("提升"));
    }

    #[test]
    fn test_adjust_confidence_decrease() {
        // 3 次应用，成功率 40% → confidence × 0.8
        let original = 0.7;
        let (adjusted, note) = adjust_lesson_confidence(original, 0.4, 3);
        let expected = original * 0.8;
        assert!((adjusted - expected).abs() < 1e-6);
        assert!(note.contains("衰减"));
    }

    #[test]
    fn test_adjust_confidence_deprecated() {
        // 5 次应用，成功率 20% → confidence = 0.1
        let (adjusted, note) = adjust_lesson_confidence(0.7, 0.2, 5);
        assert!((adjusted - 0.1).abs() < 1e-6);
        assert!(note.contains("废弃"));
    }

    #[test]
    fn test_determine_validation_status_deprecated() {
        assert_eq!(determine_validation_status(10, 0.1), "deprecated");
    }

    #[test]
    fn test_determine_validation_status_pending() {
        assert_eq!(determine_validation_status(0, 0.7), "pending");
    }

    #[test]
    fn test_determine_validation_status_insufficient() {
        assert_eq!(determine_validation_status(2, 0.7), "insufficient_data");
    }

    #[test]
    fn test_determine_validation_status_validated() {
        assert_eq!(determine_validation_status(5, 0.7), "validated");
    }

    #[test]
    fn test_build_lesson_validation_full() {
        let v = build_lesson_validation(
            "lesson-001".to_string(),
            "分批建仓节奏 ≤3 天".to_string(),
            "wrong".to_string(),
            Some("600000".to_string()),
            5,
            4,
            0.7,
        );
        assert_eq!(v.lesson_id, "lesson-001");
        assert_eq!(v.source_verdict, "wrong");
        assert_eq!(v.times_applied, 5);
        assert!((v.actual_success_rate - 0.8).abs() < 1e-6);
        // 5 次应用成功率 80% → confidence × 1.1
        let expected = (0.7_f64 * 1.1).min(0.95);
        assert!((v.adjusted_confidence - expected).abs() < 1e-6);
        assert_eq!(v.validation_status, "validated");
    }

    #[test]
    fn test_build_validation_report() {
        let validations = vec![
            build_lesson_validation(
                "l1".to_string(),
                "规则1".to_string(),
                "wrong".to_string(),
                None,
                5,
                4,
                0.7,
            ),
            build_lesson_validation(
                "l2".to_string(),
                "规则2".to_string(),
                "partial".to_string(),
                None,
                0,
                0,
                0.5,
            ),
            build_lesson_validation(
                "l3".to_string(),
                "规则3".to_string(),
                "wrong".to_string(),
                None,
                5,
                1,
                0.7,
            ),
        ];
        let report = build_lesson_validation_report(&validations);
        assert_eq!(report.total_lessons, 3);
        // l1: 5次应用成功率80% → validated
        // l2: 0次应用 → pending
        // l3: 5次应用成功率20% → deprecated
        assert_eq!(report.validated_lessons, 1);
        assert_eq!(report.pending_lessons, 1);
        assert_eq!(report.deprecated_lessons, 1);
    }

    #[test]
    fn test_build_validation_report_empty() {
        let report = build_lesson_validation_report(&[]);
        assert_eq!(report.total_lessons, 0);
        assert_eq!(report.avg_success_rate, 0.0);
    }

    #[test]
    fn test_verdict_breakdown_aggregation() {
        let validations = vec![
            build_lesson_validation(
                "l1".to_string(),
                "规则1".to_string(),
                "wrong".to_string(),
                None,
                4,
                3,
                0.7,
            ),
            build_lesson_validation(
                "l2".to_string(),
                "规则2".to_string(),
                "wrong".to_string(),
                None,
                4,
                2,
                0.6,
            ),
        ];
        let report = build_lesson_validation_report(&validations);
        let wrong_stats = report.verdict_breakdown.get("wrong").expect("wrong verdict 必须存在");
        assert_eq!(wrong_stats.total, 2);
        // 平均成功率 = (0.75 + 0.5) / 2 = 0.625
        assert!((wrong_stats.avg_success_rate - 0.625).abs() < 1e-6);
    }

    #[test]
    fn test_confidence_adjustment_stats() {
        let validations = vec![
            // 提升：3 次应用成功率 80% → confidence 0.6 → 0.66
            build_lesson_validation(
                "l1".to_string(),
                "规则1".to_string(),
                "wrong".to_string(),
                None,
                3,
                3,
                0.6,
            ),
            // 衰减：3 次应用成功率 40% → confidence 0.7 → 0.56
            build_lesson_validation(
                "l2".to_string(),
                "规则2".to_string(),
                "partial".to_string(),
                None,
                3,
                1,
                0.7,
            ),
            // 不变：0 次应用
            build_lesson_validation(
                "l3".to_string(),
                "规则3".to_string(),
                "correct".to_string(),
                None,
                0,
                0,
                0.5,
            ),
        ];
        let report = build_lesson_validation_report(&validations);
        assert_eq!(report.confidence_adjustment_stats.increased, 1);
        assert_eq!(report.confidence_adjustment_stats.decreased, 1);
        assert_eq!(report.confidence_adjustment_stats.unchanged, 1);
    }
}
