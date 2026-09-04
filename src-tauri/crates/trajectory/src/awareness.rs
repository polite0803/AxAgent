// SPDX-License-Identifier: AGPL-3.0-only

//! 自主觉知生成 — 系统对自身内部状态的感知、聚合与结构化输出。
//!
//! 设计约束（docs/PLAN-awareness-saliency.md 三条红线）：
//! - R1 零关键词词表：所有输入为结构化运行时数据；
//! - R2 参数可溯源：每个状态量的数据链见各计算函数文档，单测以手算值精确匹配；
//! - R3 不做模板拼接叙事：输出为结构化 [`AwarenessFrame`]，无生成文本。

use crate::saliency::{BroadcastPacket, SignalSource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 觉知快照触发：每 N 帧定期快照
pub const SNAPSHOT_EVERY_FRAMES: usize = 50;
/// 觉知快照触发：相邻帧 arousal 阶跃阈值
pub const SNAPSHOT_AROUSAL_STEP: f64 = 0.3;
/// arousal 滑窗事件率归一化分母（10 分钟窗口的期望事件数）
pub const AROUSAL_WINDOW_EVENTS: f64 = 30.0;
/// cognitive_load 中会话数的归一化分母
pub const LOAD_SESSION_DENOMINATOR: f64 = 5.0;
/// self_efficacy 指数滑动平均学习率
pub const EFFICACY_EMA_ALPHA: f64 = 0.1;
/// self_efficacy 初始值（无数据时的中性先验）
pub const EFFICACY_INITIAL: f64 = 0.5;

/// 觉知观测输入。全部字段来自运行时真实数据，调用方负责取数。
/// 取不到的数据传 `None`，绝不喂伪造值（R2）。
#[derive(Debug, Clone, Copy, Default)]
pub struct AwarenessInput<'a> {
    /// 最近 10 分钟的轨迹事件数（TrajectoryStorage 近期记录数）
    pub recent_event_count: usize,
    /// 当前活跃会话数（会话管理器）
    pub active_sessions: usize,
    /// 各会话上下文占用比例均值 ∈ [0,1]（compression 数据）。
    /// `None` 表示当前无低成本的聚合数据源，负荷只由会话数决定。
    pub avg_context_ratio: Option<f64>,
    /// 最近工具调用结果序列（true=成功），来自 trajectory steps 的 is_error 取反。
    /// 空切片表示窗口内无工具调用，self_efficacy 保持不变。
    pub recent_tool_results: &'a [bool],
}

/// 单帧觉知快照（"似现在"）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwarenessFrame {
    /// 激活度 ∈ [0,1]：滑窗事件率归一化
    pub arousal: f64,
    /// 认知负荷 ∈ [0,1]：会话数与上下文占用的合成
    pub cognitive_load: f64,
    /// 效能感 ∈ [0,1]：工具成功率的 EMA
    pub self_efficacy: f64,
    /// 当前主导关注点：仲裁器上一次广播的 top-1 信号源
    pub dominant_source: Option<SignalSource>,
    /// 主导信号的溯源 id（可回查原始数据）
    pub dominant_origin_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 觉知监控器：维护内部状态与帧环形缓冲。
pub struct AwarenessMonitor {
    frames: VecDeque<AwarenessFrame>,
    frame_buffer_size: usize,
    self_efficacy: f64,
    frames_since_snapshot: usize,
}

impl Default for AwarenessMonitor {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl AwarenessMonitor {
    pub fn new(frame_buffer_size: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(frame_buffer_size.min(1024)),
            frame_buffer_size,
            self_efficacy: EFFICACY_INITIAL,
            frames_since_snapshot: 0,
        }
    }

    /// 观测一个周期，生成一帧。
    ///
    /// 数据链（R2）：
    /// - `arousal` ← `recent_event_count`（TrajectoryStorage 近 10 分钟记录数）
    /// - `cognitive_load` ← `active_sessions`（会话管理器）+ `avg_context_ratio`（compression）
    /// - `self_efficacy` ← `recent_tool_results`（trajectory steps 的 is_error 取反）
    /// - `dominant_source` ← 仲裁器 last_broadcast 的 top-1
    pub fn observe(
        &mut self,
        input: AwarenessInput,
        last_broadcast: Option<&BroadcastPacket>,
    ) -> AwarenessFrame {
        let arousal = compute_arousal(input.recent_event_count);
        let load = compute_cognitive_load(input.active_sessions, input.avg_context_ratio);

        // 效能感：窗口成功率 → EMA。无工具调用时状态保持（不虚构）。
        if !input.recent_tool_results.is_empty() {
            let successes = input.recent_tool_results.iter().filter(|ok| **ok).count();
            let rate = successes as f64 / input.recent_tool_results.len() as f64;
            self.self_efficacy =
                (1.0 - EFFICACY_EMA_ALPHA) * self.self_efficacy + EFFICACY_EMA_ALPHA * rate;
        }

        let dominant = last_broadcast
            .and_then(|p| p.winners.first())
            .map(|w| (w.signal.source, w.signal.origin_id.clone()));

        let frame = AwarenessFrame {
            arousal,
            cognitive_load: load,
            self_efficacy: self.self_efficacy,
            dominant_source: dominant.as_ref().map(|(s, _)| *s),
            dominant_origin_id: dominant.map(|(_, id)| id),
            created_at: Utc::now(),
        };

        if self.frames.len() >= self.frame_buffer_size {
            self.frames.pop_front();
        }
        self.frames.push_back(frame.clone());
        self.frames_since_snapshot += 1;
        frame
    }

    /// 是否应生成持久化快照：每 [`SNAPSHOT_EVERY_FRAMES`] 帧一次，
    /// 或与上一帧相比 arousal 阶跃超过 [`SNAPSHOT_AROUSAL_STEP`]。
    pub fn should_snapshot(&self, frame: &AwarenessFrame) -> bool {
        if self.frames_since_snapshot >= SNAPSHOT_EVERY_FRAMES {
            return true;
        }
        self.frames
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|prev| (frame.arousal - prev.arousal).abs() > SNAPSHOT_AROUSAL_STEP)
    }

    /// 生成快照 JSON 内容（供 memory_items 持久化，namespace `__sys_awareness__`）。
    /// 仅结构化数据，无生成文本（R3）。
    pub fn snapshot_content(&self, frame: &AwarenessFrame) -> String {
        serde_json::to_string(frame).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn frames(&self) -> &VecDeque<AwarenessFrame> {
        &self.frames
    }

    pub fn latest_frame(&self) -> Option<&AwarenessFrame> {
        self.frames.back()
    }

    /// 快照计数复位（调用方完成持久化后调用）。
    pub fn mark_snapshotted(&mut self) {
        self.frames_since_snapshot = 0;
    }
}

/// 激活度：滑窗事件率归一化。
/// 数据链：TrajectoryStorage 近 10 分钟轨迹记录数。手算：`min(1, count/30)`。
pub fn compute_arousal(recent_event_count: usize) -> f64 {
    (recent_event_count as f64 / AROUSAL_WINDOW_EVENTS).min(1.0)
}

/// 认知负荷：会话数与上下文占用的等权合成。
/// 数据链：会话管理器活跃会话数 + 各会话上下文压缩比例均值。
/// ratio 缺失（`None`）时只由会话数决定：`min(1, sessions/5)`；
/// ratio 可得时：`0.5 × min(1, sessions/5) + 0.5 × clamp(ratio, 0, 1)`。
pub fn compute_cognitive_load(active_sessions: usize, avg_context_ratio: Option<f64>) -> f64 {
    let session_term = (active_sessions as f64 / LOAD_SESSION_DENOMINATOR).min(1.0);
    match avg_context_ratio {
        Some(ratio) => 0.5 * session_term + 0.5 * ratio.clamp(0.0, 1.0),
        None => session_term,
    }
}

/// 觉知快照的持久化 namespace（复用 v101 sentinel namespace 模式，零新表）。
pub const AWARENESS_NAMESPACE: &str = "__sys_awareness__";

// ── 置信度校准器（A3）────────────────────────────────────────────────────
//
// 校准数据对的真实来源：因果边的 `confidence`（系统对该边的把握度，样本量驱动）
// 与 `strength`（该边的实际命中率 positive/observations）。
// 二者皆已持久化于 knowledge_relations.properties，数据链闭合、不可编造。

/// 校准分桶粒度（0.1）
pub const CALIBRATION_BUCKET_GRAIN: f64 = 0.1;
/// 分桶进入曲线的最低样本数
pub const CALIBRATION_MIN_BUCKET_SAMPLES: usize = 3;
/// 偏差摘要的最低样本数
pub const CALIBRATION_MIN_SUMMARY_SAMPLES: usize = 5;
/// 偏差摘要的滑动窗口大小
pub const CALIBRATION_SUMMARY_WINDOW: usize = 100;
/// "校准良好"的判定容差：|predicted - actual| ≤ 0.2
pub const CALIBRATION_TOLERANCE: f64 = 0.2;

/// 单条校准记录：预测把握度 vs 实际结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationRecord {
    pub predicted: f64,
    pub actual: f64,
}

/// 校准曲线上的一个桶
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationBucket {
    /// 预测把握度桶中心（0.1 粒度，如 0.7 代表 [0.7, 0.8)）
    pub predicted_bucket: f64,
    pub sample_count: usize,
    /// 该桶内实际结果均值
    pub mean_actual: f64,
}

/// 偏差摘要（滑动窗口统计）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiasSummary {
    /// mean(predicted - actual)。> 0 表示系统性过度自信
    pub avg_bias: f64,
    /// bias > 0.2 的记录占比
    pub overconfident_rate: f64,
    /// |bias| ≤ 0.2 的记录占比
    pub calibrated_rate: f64,
    /// bias < -0.2 的记录占比
    pub underconfident_rate: f64,
}

/// 置信度校准器。
///
/// 数据链（R2）：`predicted` ← 因果边 `CausalEdgeStats.confidence`，
/// `actual` ← 同一边的 `strength()`（= positive/observations）。
pub struct ConfidenceCalibrator {
    records: Vec<CalibrationRecord>,
    max_records: usize,
}

impl Default for ConfidenceCalibrator {
    fn default() -> Self {
        Self::new(CALIBRATION_SUMMARY_WINDOW * 10)
    }
}

impl ConfidenceCalibrator {
    pub fn new(max_records: usize) -> Self {
        Self { records: Vec::new(), max_records: max_records.max(1) }
    }

    /// 记录一对校准数据（越界值钳制到 [0,1]）。
    pub fn record(&mut self, predicted: f64, actual: f64) {
        self.records.push(CalibrationRecord {
            predicted: predicted.clamp(0.0, 1.0),
            actual: actual.clamp(0.0, 1.0),
        });
        if self.records.len() > self.max_records {
            let overflow = self.records.len() - self.max_records;
            self.records.drain(0..overflow);
        }
    }

    /// 校准曲线：0.1 粒度分桶，样本数达 [`CALIBRATION_MIN_BUCKET_SAMPLES`] 的桶才输出。
    ///
    /// 浮点修正：`floor(p/grain)` 前加 `1e-9` epsilon，规避
    /// `floor(0.3 * 10) = 2` 类二进制浮点陷阱（laap-AGI 同位置存在此 bug）。
    pub fn calibration_curve(&self) -> Vec<CalibrationBucket> {
        let mut buckets: std::collections::BTreeMap<u32, Vec<f64>> = Default::default();
        for r in &self.records {
            let idx = ((r.predicted / CALIBRATION_BUCKET_GRAIN) + 1e-9).floor() as u32;
            buckets.entry(idx).or_default().push(r.actual);
        }
        buckets
            .into_iter()
            .filter(|(_, samples)| samples.len() >= CALIBRATION_MIN_BUCKET_SAMPLES)
            .map(|(idx, samples)| CalibrationBucket {
                predicted_bucket: idx as f64 * CALIBRATION_BUCKET_GRAIN,
                sample_count: samples.len(),
                mean_actual: samples.iter().sum::<f64>() / samples.len() as f64,
            })
            .collect()
    }

    /// 偏差摘要：最近 [`CALIBRATION_SUMMARY_WINDOW`] 条的滑动统计。
    /// 样本不足 [`CALIBRATION_MIN_SUMMARY_SAMPLES`] 时返回 `None`（不虚构）。
    pub fn bias_summary(&self) -> Option<BiasSummary> {
        let window_start = self.records.len().saturating_sub(CALIBRATION_SUMMARY_WINDOW);
        let window = &self.records[window_start..];
        if window.len() < CALIBRATION_MIN_SUMMARY_SAMPLES {
            return None;
        }
        let n = window.len() as f64;
        let biases: Vec<f64> = window.iter().map(|r| r.predicted - r.actual).collect();
        let avg_bias = biases.iter().sum::<f64>() / n;
        let overconfident =
            biases.iter().filter(|b| **b > CALIBRATION_TOLERANCE).count() as f64 / n;
        let underconfident =
            biases.iter().filter(|b| **b < -CALIBRATION_TOLERANCE).count() as f64 / n;
        let calibrated = 1.0 - overconfident - underconfident;
        Some(BiasSummary {
            avg_bias,
            overconfident_rate: overconfident,
            calibrated_rate: calibrated,
            underconfident_rate: underconfident,
        })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saliency::SaliencySignal;

    /// R2 手算精确匹配：arousal = min(1, count/30)
    #[test]
    fn test_arousal_exact() {
        assert!((compute_arousal(0) - 0.0).abs() < 1e-9);
        assert!((compute_arousal(15) - 0.5).abs() < 1e-9);
        assert!((compute_arousal(30) - 1.0).abs() < 1e-9);
        assert!((compute_arousal(45) - 1.0).abs() < 1e-9); // 钳制
    }

    /// R2 手算精确匹配：load = 0.5×min(1,s/5) + 0.5×ratio；ratio 缺失时 = min(1,s/5)
    #[test]
    fn test_cognitive_load_exact() {
        // 2 会话 + 0.6 占比 → 0.5×0.4 + 0.5×0.6 = 0.5
        assert!((compute_cognitive_load(2, Some(0.6)) - 0.5).abs() < 1e-9);
        // 钳制：7 会话 → 会话项 1.0；ratio 1.5 → 1.0
        assert!((compute_cognitive_load(7, Some(1.5)) - 1.0).abs() < 1e-9);
        assert!((compute_cognitive_load(0, Some(0.0)) - 0.0).abs() < 1e-9);
        // ratio 不可得：只由会话数决定
        assert!((compute_cognitive_load(2, None) - 0.4).abs() < 1e-9);
        assert!((compute_cognitive_load(7, None) - 1.0).abs() < 1e-9);
    }

    /// R2 手算精确匹配：efficacy EMA，初值 0.5，α=0.1
    #[test]
    fn test_efficacy_ema_exact() {
        let mut monitor = AwarenessMonitor::new(10);
        // 窗口 [成功, 失败, 成功, 成功] → rate = 0.75 → 0.9×0.5 + 0.1×0.75 = 0.525
        let results = [true, false, true, true];
        let frame = monitor.observe(
            AwarenessInput {
                recent_event_count: 0,
                active_sessions: 0,
                avg_context_ratio: Some(0.0),
                recent_tool_results: &results,
            },
            None,
        );
        assert!((frame.self_efficacy - 0.525).abs() < 1e-9);
        // 第二个窗口全成功 → 0.9×0.525 + 0.1×1.0 = 0.5725
        let results2 = [true, true];
        let frame = monitor.observe(
            AwarenessInput {
                recent_event_count: 0,
                active_sessions: 0,
                avg_context_ratio: Some(0.0),
                recent_tool_results: &results2,
            },
            None,
        );
        assert!((frame.self_efficacy - 0.5725).abs() < 1e-9);
    }

    /// 无工具调用时效能感保持不变（不虚构状态）。
    #[test]
    fn test_no_tool_results_efficacy_unchanged() {
        let mut monitor = AwarenessMonitor::new(10);
        let frame = monitor.observe(
            AwarenessInput {
                recent_event_count: 5,
                active_sessions: 1,
                avg_context_ratio: Some(0.2),
                recent_tool_results: &[],
            },
            None,
        );
        assert!((frame.self_efficacy - EFFICACY_INITIAL).abs() < 1e-9);
    }

    /// 主导关注点来自仲裁器广播 top-1，origin_id 可溯源。
    #[test]
    fn test_dominant_from_broadcast() {
        let mut arbiter = crate::saliency::SaliencyArbiter::default();
        let packet = arbiter.arbitrate(vec![
            SaliencySignal::new(SignalSource::CausalInsight, 0.9, "sug-1"),
            SaliencySignal::new(SignalSource::Nudge, 0.4, "n-1"),
        ]);
        assert_eq!(packet.winners[0].signal.origin_id, "sug-1");

        let mut monitor = AwarenessMonitor::new(10);
        let frame = monitor.observe(
            AwarenessInput {
                recent_event_count: 0,
                active_sessions: 0,
                avg_context_ratio: Some(0.0),
                recent_tool_results: &[],
            },
            Some(&packet),
        );
        assert_eq!(frame.dominant_source, Some(SignalSource::CausalInsight));
        assert_eq!(frame.dominant_origin_id.as_deref(), Some("sug-1"));
    }

    /// arousal 阶跃触发快照。
    #[test]
    fn test_snapshot_on_arousal_step() {
        let mut monitor = AwarenessMonitor::new(100);
        let f1 = monitor.observe(
            AwarenessInput {
                recent_event_count: 3,
                active_sessions: 0,
                avg_context_ratio: Some(0.0),
                recent_tool_results: &[],
            },
            None,
        );
        assert!(!monitor.should_snapshot(&f1));
        // 3/30=0.1 → 24/30=0.8，阶跃 0.7 > 0.3
        let f2 = monitor.observe(
            AwarenessInput {
                recent_event_count: 24,
                active_sessions: 0,
                avg_context_ratio: Some(0.0),
                recent_tool_results: &[],
            },
            None,
        );
        assert!(monitor.should_snapshot(&f2));
        monitor.mark_snapshotted();
        // 平稳帧不再触发
        let f3 = monitor.observe(
            AwarenessInput {
                recent_event_count: 25,
                active_sessions: 0,
                avg_context_ratio: Some(0.0),
                recent_tool_results: &[],
            },
            None,
        );
        assert!(!monitor.should_snapshot(&f3));
    }

    /// 快照内容为合法 JSON 结构化数据（R3：无生成文本）。
    #[test]
    fn test_snapshot_content_is_structured_json() {
        let mut monitor = AwarenessMonitor::new(10);
        let frame = monitor.observe(
            AwarenessInput {
                recent_event_count: 10,
                active_sessions: 2,
                avg_context_ratio: Some(0.5),
                recent_tool_results: &[],
            },
            None,
        );
        let content = monitor.snapshot_content(&frame);
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("must be valid JSON");
        assert!(parsed.get("arousal").is_some());
        assert!(parsed.get("cognitiveLoad").is_some());
        assert!(parsed.get("selfEfficacy").is_some());
    }

    /// 帧环形缓冲按容量淘汰。
    #[test]
    fn test_frame_buffer_eviction() {
        let mut monitor = AwarenessMonitor::new(3);
        for i in 0..5 {
            monitor.observe(
                AwarenessInput {
                    recent_event_count: i,
                    active_sessions: 0,
                    avg_context_ratio: Some(0.0),
                    recent_tool_results: &[],
                },
                None,
            );
        }
        assert_eq!(monitor.frames().len(), 3);
        // 最旧的帧（count=0,1）已被淘汰，剩余 count=2,3,4
        let arousals: Vec<f64> = monitor.frames().iter().map(|f| f.arousal).collect();
        assert!((arousals[0] - 2.0 / 30.0).abs() < 1e-9);
        assert!((arousals[2] - 4.0 / 30.0).abs() < 1e-9);
    }

    /// R2 手算精确匹配：校准曲线分桶（含 0.3 浮点陷阱回归）
    #[test]
    fn test_calibration_curve_exact() {
        let mut cal = ConfidenceCalibrator::new(100);
        // 桶 0.7：三条记录（0.72 也应落入 0.7 桶）
        cal.record(0.7, 0.5);
        cal.record(0.72, 0.6);
        cal.record(0.75, 0.4);
        // 桶 0.3：0.3, 0.3, 0.35（0.35 ∈ [0.3,0.4)，floor(3.5)=3 正确归桶）→ 3 条
        cal.record(0.3, 0.9);
        cal.record(0.3, 0.2);
        cal.record(0.35, 0.7);

        let curve = cal.calibration_curve();
        assert_eq!(curve.len(), 2);
        let b03 = curve.iter().find(|b| (b.predicted_bucket - 0.3).abs() < 1e-9).unwrap();
        let b07 = curve.iter().find(|b| (b.predicted_bucket - 0.7).abs() < 1e-9).unwrap();
        assert_eq!(b03.sample_count, 3);
        assert!((b03.mean_actual - (0.9 + 0.2 + 0.7) / 3.0).abs() < 1e-9);
        assert_eq!(b07.sample_count, 3);
        assert!((b07.mean_actual - (0.5 + 0.6 + 0.4) / 3.0).abs() < 1e-9);
    }

    /// R2 手算精确匹配：0.3 精确落桶（无 epsilon 时 floor(0.3*10)=2）
    #[test]
    fn test_calibration_bucket_floating_point() {
        let mut cal = ConfidenceCalibrator::new(100);
        for actual in [0.5, 0.6, 0.4] {
            cal.record(0.3, actual);
        }
        let curve = cal.calibration_curve();
        assert_eq!(curve.len(), 1);
        assert!((curve[0].predicted_bucket - 0.3).abs() < 1e-9);
    }

    /// R2 手算精确匹配：偏差摘要
    #[test]
    fn test_bias_summary_exact() {
        let mut cal = ConfidenceCalibrator::new(100);
        // biases: +0.3, -0.1, +0.25, -0.05, +0.1 → avg=+0.1
        let pairs = [(0.8, 0.5), (0.5, 0.6), (0.75, 0.5), (0.55, 0.6), (0.6, 0.5)];
        for (p, a) in pairs {
            cal.record(p, a);
        }
        let summary = cal.bias_summary().expect("5 samples >= min 5");
        assert!((summary.avg_bias - 0.1).abs() < 1e-9);
        // overconfident(>0.2): 0.3, 0.25 → 2/5
        assert!((summary.overconfident_rate - 0.4).abs() < 1e-9);
        // underconfident(<-0.2): 0 → 0
        assert!(summary.underconfident_rate.abs() < 1e-9);
        // calibrated: 1 - 0.4 - 0 = 0.6
        assert!((summary.calibrated_rate - 0.6).abs() < 1e-9);
    }

    /// 样本不足时摘要返回 None（不虚构）。
    #[test]
    fn test_bias_summary_insufficient_samples() {
        let mut cal = ConfidenceCalibrator::new(100);
        cal.record(0.5, 0.4);
        cal.record(0.6, 0.5);
        assert!(cal.bias_summary().is_none());
    }

    /// 记录环形淘汰：超出 max_records 时淘汰最旧。
    #[test]
    fn test_calibrator_ring_eviction() {
        let mut cal = ConfidenceCalibrator::new(3);
        for i in 0..5 {
            cal.record(0.5, i as f64 / 10.0);
        }
        assert_eq!(cal.len(), 3);
        // 最旧两条 (0.0, 0.1) 已淘汰；验证剩余均值 = (0.2+0.3+0.4)/3
        let curve = cal.calibration_curve();
        assert_eq!(curve.len(), 1);
        assert!((curve[0].mean_actual - 0.3).abs() < 1e-9);
    }
}
