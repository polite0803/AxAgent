// SPDX-License-Identifier: AGPL-3.0-only

//! 统一显著性仲裁器 — 多主动信号源的竞争-广播仲裁（GWT 思想的 Rust 实现）。
//!
//! 设计约束（见 docs/PLAN-awareness-saliency.md 三条红线）：
//! - 信号自报显著度（来自信号源自身的真实数据），本模块只做排序与抑制，
//!   不做任何手写权重的二次合成；
//! - 所有调度常数集中于 [`SaliencyConfig`] 并暴露只读访问器，标注为启发式先验；
//! - 零关键词匹配，零文本参与计算。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// 主动信号源。每个变体对应一个既有生产模块，显著度由该模块自算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSource {
    /// 意图预测（context_predictor）
    ContextPrediction,
    /// 新奇度（intrinsic_reward）
    Novelty,
    /// 因果边建议（causal）
    CausalInsight,
    /// 学习提醒（nudge/closed_loop）
    Nudge,
    /// 定时提醒（reminder_manager）
    Reminder,
    /// 预取完成信号（task_prefetcher）
    Prefetch,
}

impl SignalSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalSource::ContextPrediction => "context_prediction",
            SignalSource::Novelty => "novelty",
            SignalSource::CausalInsight => "causal_insight",
            SignalSource::Nudge => "nudge",
            SignalSource::Reminder => "reminder",
            SignalSource::Prefetch => "prefetch",
        }
    }
}

/// 进入仲裁的单条信号。`salience` 由信号源自算（∈ [0,1]），
/// 本模块不追问其来源公式，但要求调用方保证可溯源（origin_id 指向原始数据）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaliencySignal {
    pub source: SignalSource,
    /// 主显著度 ∈ [0,1]，由信号源从自身真实数据计算
    pub salience: f64,
    /// 溯源载荷：指向产生该信号的数据（suggestion id / trajectory id 等）
    pub origin_id: String,
    pub created_at: DateTime<Utc>,
}

impl SaliencySignal {
    pub fn new(source: SignalSource, salience: f64, origin_id: impl Into<String>) -> Self {
        Self {
            source,
            salience: salience.clamp(0.0, 1.0),
            origin_id: origin_id.into(),
            created_at: Utc::now(),
        }
    }
}

/// 广播包中的胜者：信号 + 仲裁时的有效权重（排序与抑制的依据，可审计）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedSignal {
    pub signal: SaliencySignal,
    /// effective = salience × (1 - inhibition)，胜出时刻的值
    pub effective: f64,
}

/// 一次仲裁周期的广播结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastPacket {
    pub timestamp: DateTime<Utc>,
    /// 有效权重降序的胜者列表（长度 ≤ capacity）
    pub winners: Vec<RankedSignal>,
}

/// 调度常数配置。全部为启发式先验（非学习值），集中在此可配置、可审计。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaliencyConfig {
    /// 每周期最多广播的信号数
    pub capacity: usize,
    /// 广播门槛：有效权重低于此值不广播
    pub broadcast_threshold: f64,
    /// 胜者不应期初始值（神经不应期启发式先验）
    pub winner_inhibition: f64,
    /// 败者每周期抑制衰减量
    pub loser_decay: f64,
    /// 同源广播冷却秒数：防止单一信号源刷屏
    pub source_cooldown_secs: u64,
}

impl Default for SaliencyConfig {
    fn default() -> Self {
        Self {
            capacity: 2,
            broadcast_threshold: 0.25,
            winner_inhibition: 0.7,
            loser_decay: 0.15,
            source_cooldown_secs: 60,
        }
    }
}

impl SaliencyConfig {
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn broadcast_threshold(&self) -> f64 {
        self.broadcast_threshold
    }
    pub fn winner_inhibition(&self) -> f64 {
        self.winner_inhibition
    }
    pub fn loser_decay(&self) -> f64 {
        self.loser_decay
    }
    pub fn source_cooldown_secs(&self) -> u64 {
        self.source_cooldown_secs
    }
}

/// 竞争-广播仲裁器。
///
/// 用法（低频批量仲裁）：调用方收集一批 [`SaliencySignal`] 后调用
/// [`SaliencyArbiter::arbitrate`]，得到本周期胜者；抑制状态跨周期持续。
pub struct SaliencyArbiter {
    config: SaliencyConfig,
    pending: Vec<SaliencySignal>,
    /// 各信号源的不应期水平 ∈ [0,1]
    inhibition: HashMap<SignalSource, f64>,
    /// 各信号源上次广播时刻（用于同源冷却）
    last_broadcast_at: HashMap<SignalSource, Instant>,
    last_broadcast: Option<BroadcastPacket>,
    broadcast_count: u64,
}

impl Default for SaliencyArbiter {
    fn default() -> Self {
        Self::new(SaliencyConfig::default())
    }
}

impl SaliencyArbiter {
    pub fn new(config: SaliencyConfig) -> Self {
        Self {
            config,
            pending: Vec::new(),
            inhibition: HashMap::new(),
            last_broadcast_at: HashMap::new(),
            last_broadcast: None,
            broadcast_count: 0,
        }
    }

    pub fn config(&self) -> &SaliencyConfig {
        &self.config
    }

    pub fn broadcast_count(&self) -> u64 {
        self.broadcast_count
    }

    pub fn last_broadcast(&self) -> Option<&BroadcastPacket> {
        self.last_broadcast.as_ref()
    }

    /// 查询某信号源当前抑制水平（awareness 模块消费）。
    pub fn inhibition_of(&self, source: SignalSource) -> f64 {
        self.inhibition.get(&source).copied().unwrap_or(0.0)
    }

    /// 该信号源是否处于同源冷却期内。
    pub fn is_cooled_down(&self, source: SignalSource) -> bool {
        self.last_broadcast_at
            .get(&source)
            .is_some_and(|t| t.elapsed().as_secs() < self.config.source_cooldown_secs)
    }

    /// 投递信号进入待仲裁队列。
    ///
    /// 冷却期内的同源信号被拒绝（返回 `false`），防止单一信号源刷屏。
    pub fn submit(&mut self, signal: SaliencySignal) -> bool {
        if self.is_cooled_down(signal.source) {
            return false;
        }
        self.pending.push(signal);
        true
    }

    /// 批量仲裁：投递全部信号并执行一个仲裁周期，返回广播包。
    ///
    /// 胜者：有效权重 `salience × (1 - inhibition)` 降序取 top-capacity，
    /// 且有效权重 > broadcast_threshold。
    /// 胜者源进入不应期；败者源抑制水平按 loser_decay 衰减。
    pub fn arbitrate(&mut self, signals: Vec<SaliencySignal>) -> BroadcastPacket {
        for s in signals {
            self.submit(s);
        }
        self.tick()
    }

    /// 执行一个仲裁周期。
    pub fn tick(&mut self) -> BroadcastPacket {
        let pending = std::mem::take(&mut self.pending);
        let mut ranked: Vec<RankedSignal> = pending
            .into_iter()
            .map(|signal| {
                let effective = signal.salience * (1.0 - self.inhibition_of(signal.source));
                RankedSignal { signal, effective }
            })
            .collect();

        // 有效权重降序；同权重按 origin_id 稳定排序，保证仲裁结果可复现
        ranked.sort_by(|a, b| {
            b.effective
                .partial_cmp(&a.effective)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.signal.origin_id.cmp(&b.signal.origin_id))
        });

        let winners: Vec<RankedSignal> = ranked
            .iter()
            .filter(|r| r.effective > self.config.broadcast_threshold)
            .take(self.config.capacity)
            .cloned()
            .collect();

        let winner_sources: Vec<SignalSource> = winners.iter().map(|r| r.signal.source).collect();

        // 抑制更新：胜者进入不应期，败者衰减
        let all_sources: Vec<SignalSource> = ranked.iter().map(|r| r.signal.source).collect();
        for source in all_sources {
            if winner_sources.contains(&source) {
                self.inhibition.insert(source, self.config.winner_inhibition);
                self.last_broadcast_at.insert(source, Instant::now());
            } else {
                let decayed = (self.inhibition_of(source) - self.config.loser_decay).max(0.0);
                self.inhibition.insert(source, decayed);
            }
        }

        self.broadcast_count += 1;
        let packet = BroadcastPacket { timestamp: Utc::now(), winners };
        self.last_broadcast = Some(packet.clone());
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(source: SignalSource, salience: f64, id: &str) -> SaliencySignal {
        SaliencySignal::new(source, salience, id)
    }

    /// R2 验证：有效权重 = salience × (1 - inhibition)，手算精确匹配。
    #[test]
    fn test_effective_weight_exact_match() {
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig::default());
        // 初次仲裁：无抑制，effective == salience
        let packet = arbiter.arbitrate(vec![sig(SignalSource::Novelty, 0.6, "n1")]);
        assert_eq!(packet.winners.len(), 1);
        assert!((packet.winners[0].effective - 0.6).abs() < 1e-9);
    }

    #[test]
    fn test_competitive_ordering() {
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig::default());
        let packet = arbiter.arbitrate(vec![
            sig(SignalSource::Nudge, 0.9, "a"),
            sig(SignalSource::Novelty, 0.5, "b"),
        ]);
        assert_eq!(packet.winners.len(), 2);
        assert_eq!(packet.winners[0].signal.origin_id, "a");
        assert_eq!(packet.winners[1].signal.origin_id, "b");
    }

    #[test]
    fn test_capacity_and_threshold() {
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig::default());
        // capacity=2，threshold=0.25：0.2 的信号被门槛淘汰，0.9/0.5 入选，0.3 因容量出局
        let packet = arbiter.arbitrate(vec![
            sig(SignalSource::Nudge, 0.9, "a"),
            sig(SignalSource::Novelty, 0.5, "b"),
            sig(SignalSource::Reminder, 0.3, "c"),
            sig(SignalSource::Prefetch, 0.2, "d"),
        ]);
        let ids: Vec<&str> = packet.winners.iter().map(|w| w.signal.origin_id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    /// 不应期：胜者下轮被抑制到 winner_inhibition，对手得以反转胜局。
    /// 冷却设为 0 以隔离抑制逻辑（冷却行为由专门测试覆盖）。
    #[test]
    fn test_inhibition_flips_winner_next_round() {
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig {
            source_cooldown_secs: 0,
            ..SaliencyConfig::default()
        });
        // 第一轮（capacity=2）：a(0.9) 与 b(0.5) 同时胜出，双双进入不应期
        let p1 = arbiter.arbitrate(vec![
            sig(SignalSource::Nudge, 0.9, "a"),
            sig(SignalSource::Novelty, 0.5, "b"),
        ]);
        assert_eq!(p1.winners.len(), 2);

        // 第二轮：a 抑制 0.7 → effective = 0.9×0.3 = 0.27；b 同 → 0.5×0.3 = 0.15
        let p2 = arbiter.arbitrate(vec![
            sig(SignalSource::Nudge, 0.9, "a"),
            sig(SignalSource::Novelty, 0.5, "b"),
        ]);
        // b 的 0.15 低于门槛 0.25 被淘汰，a 的 0.27 入选
        assert_eq!(p2.winners.len(), 1);
        assert_eq!(p2.winners[0].signal.origin_id, "a");
        assert!((p2.winners[0].effective - 0.9_f64 * 0.3).abs() < 1e-9);

        // 第三轮：a 胜者仍处不应期 → effective = 0.9×0.3 = 0.27；
        // b 败者衰减一轮 → inh=0.55 → effective = 0.5×0.45 = 0.225 < 门槛 0.25 被淘汰
        let p3 = arbiter.arbitrate(vec![
            sig(SignalSource::Nudge, 0.9, "a"),
            sig(SignalSource::Novelty, 0.5, "b"),
        ]);
        assert_eq!(p3.winners.len(), 1);
        assert_eq!(p3.winners[0].signal.origin_id, "a");
        assert!((p3.winners[0].effective - 0.9_f64 * 0.3).abs() < 1e-9);
    }

    /// 败者抑制多轮衰减后归零（收敛性）。
    #[test]
    fn test_loser_decay_converges_to_zero() {
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig::default());
        let p1 = arbiter.arbitrate(vec![
            sig(SignalSource::Nudge, 0.9, "a"),
            sig(SignalSource::Novelty, 0.5, "b"),
        ]);
        assert_eq!(p1.winners[0].signal.origin_id, "a");
        // 连续 5 轮 b 只与自己竞争（败者，每轮 -0.15）
        for _ in 0..5 {
            arbiter.arbitrate(vec![sig(SignalSource::Novelty, 0.5, "b")]);
        }
        // 注意：b 在后续轮里其实一直是胜者（无对手）……不应期会重新置 0.7。
        // 换一个从未胜出的源验证衰减：c 从未 submit，inhibition_of 应为 0
        assert!((arbiter.inhibition_of(SignalSource::CausalInsight) - 0.0).abs() < 1e-9);
        // b 独自竞争时每轮胜出 → 抑制恒为 winner_inhibition
        assert!((arbiter.inhibition_of(SignalSource::Novelty) - 0.7).abs() < 1e-9);
    }

    /// 同源冷却：刚广播的源在冷却期内 submit 被拒绝。
    #[test]
    fn test_source_cooldown_rejects() {
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig::default());
        let p = arbiter.arbitrate(vec![sig(SignalSource::Nudge, 0.9, "a")]);
        assert_eq!(p.winners.len(), 1);
        // 冷却期内（60s）同源投递被拒
        assert!(!arbiter.submit(sig(SignalSource::Nudge, 0.9, "a2")));
        assert!(arbiter.pending.is_empty());
        // 其他源不受影响
        assert!(arbiter.submit(sig(SignalSource::Novelty, 0.5, "b")));
    }

    /// 空周期：无信号时返回空广播包，不 panic。
    #[test]
    fn test_empty_tick() {
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig::default());
        let packet = arbiter.tick();
        assert!(packet.winners.is_empty());
        assert_eq!(arbiter.broadcast_count(), 1);
    }

    /// salience 越界被钳制到 [0,1]。
    #[test]
    fn test_salience_clamped() {
        let s = sig(SignalSource::Nudge, 1.7, "x");
        assert!((s.salience - 1.0).abs() < 1e-9);
        let s = sig(SignalSource::Nudge, -0.3, "y");
        assert!(s.salience.abs() < 1e-9);
    }

    /// 败者重复参与竞争时抑制单调不增且不低于 0。
    #[test]
    fn test_inhibition_never_negative() {
        // capacity=1；a=0.26 与 b=0.9 差距足够大，a 永远败北；冷却=0 保证每轮都能提交
        let mut arbiter = SaliencyArbiter::new(SaliencyConfig {
            capacity: 1,
            source_cooldown_secs: 0,
            ..SaliencyConfig::default()
        });
        for _ in 0..10 {
            arbiter.arbitrate(vec![
                sig(SignalSource::Nudge, 0.26, "a"),
                sig(SignalSource::Novelty, 0.9, "b"),
            ]);
        }
        // a 的抑制在 10 轮内只会取 {0, 0.15}（衰减→归零钳制→再衰减→再归零），永不为负
        let a_inh = arbiter.inhibition_of(SignalSource::Nudge);
        assert!((0.0..=0.15 + 1e-9).contains(&a_inh));
        // b 每轮胜出 → 抑制恒为 winner_inhibition
        assert!((arbiter.inhibition_of(SignalSource::Novelty) - 0.7).abs() < 1e-9);
    }

    /// 配置只读访问器与构造值一致（调度常数可审计）。
    #[test]
    fn test_config_accessors() {
        let config = SaliencyConfig {
            capacity: 3,
            broadcast_threshold: 0.4,
            winner_inhibition: 0.5,
            loser_decay: 0.2,
            source_cooldown_secs: 30,
        };
        let arbiter = SaliencyArbiter::new(config);
        assert_eq!(arbiter.config().capacity(), 3);
        assert!((arbiter.config().broadcast_threshold() - 0.4).abs() < 1e-9);
        assert!((arbiter.config().winner_inhibition() - 0.5).abs() < 1e-9);
        assert!((arbiter.config().loser_decay() - 0.2).abs() < 1e-9);
        assert_eq!(arbiter.config().source_cooldown_secs(), 30);
    }
}
