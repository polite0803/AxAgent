//! Oracle 预言机系统 —— 为模拟提供外生信号源。
//!
//! 在 ABIDES 中，Oracle 是"上帝视角"的价格信号提供者。
//! 这里的 Oracle 生成合成价格轨迹、注入市场事件、提供基本面估值，
//! 让 Agent 的市场行为围绕这些外生信号展开。

use crate::types::{Price, SimTimestamp};
use rand::SeedableRng;
use rand::rngs::StdRng;

// ── 市场事件 ──

/// 可注入的市场事件
#[derive(Debug, Clone)]
pub enum MarketEvent {
    /// 价格冲击（如财报超预期）
    PriceShock {
        /// 方向：1.0 = 上涨, -1.0 = 下跌
        direction: f64,
        /// 幅度（基点）
        magnitude_bps: i64,
    },
    /// 成交量异常
    VolumeAnomaly {
        /// 成交量倍数（1.0 = 正常）
        multiplier: f64,
        /// 持续时间（ns）
        duration_ns: SimTimestamp,
    },
}

/// 预定事件（带触发时间）
#[derive(Debug, Clone)]
pub struct ScriptedEvent {
    pub at_time: SimTimestamp,
    pub event: MarketEvent,
}

// ── Oracle 输出 ──

/// Oracle 在某一时间点的信号输出
#[derive(Debug, Clone)]
pub struct OracleSignal {
    /// 当前模拟时间
    pub time: SimTimestamp,
    /// 公允价值（Agent 用于参考）
    pub fundamental_value: Price,
    /// 市场情绪因子（1.0=中性，>1.0=看涨，<1.0=看跌）
    pub sentiment: f64,
    /// 当前激活的市场事件（无事件时为 None）
    pub active_event: Option<MarketEvent>,
}

// ── Oracle Trait ──

/// Oracle 预言机接口
///
/// 负责生成合成价格信号和市场事件。每个模拟场景对应一个 Oracle。
pub trait Oracle: Send + Sync {
    fn name(&self) -> &str;

    /// 获取指定时间的信号
    fn signal_at(&mut self, time: SimTimestamp) -> OracleSignal;
}

// ── 内置 Oracle 实现 ──

/// 基准 Oracle —— 纯随机游走（无趋势、无事件）
///
/// 修复 P0-M5: 原实现用 `(time * 1.618).sin()` 确定性函数冒充随机游走，
/// 导致所有 Monte Carlo path 完全相同。改为用 StdRng 生成真随机步进。
pub struct BaselineOracle {
    #[allow(dead_code)]
    reference_price: Price,
    /// 当前累积信号
    current_fv: f64,
    /// 前一次查询时间（用于计算随机游走步进）
    last_time: SimTimestamp,
    /// 随机游走日波动率（基点）
    daily_vol_bps: i64,
    /// 修复 P0-M5: 可种子化的 PRNG，替代确定性 sin
    rng: StdRng,
}

impl BaselineOracle {
    pub fn new(reference_price: Price, daily_vol_bps: i64) -> Self {
        Self {
            reference_price,
            current_fv: reference_price as f64,
            last_time: 0,
            daily_vol_bps,
            rng: StdRng::from_entropy(),
        }
    }

    /// 修复 P0-M4: 带 seed 的构造函数，让 Monte Carlo 可复现
    pub fn with_seed(reference_price: Price, daily_vol_bps: i64, seed: u64) -> Self {
        Self {
            reference_price,
            current_fv: reference_price as f64,
            last_time: 0,
            daily_vol_bps,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl Oracle for BaselineOracle {
    fn name(&self) -> &str {
        "baseline"
    }

    fn signal_at(&mut self, time: SimTimestamp) -> OracleSignal {
        use rand::Rng;
        // 计算时间差（ns → 模拟天数）
        let dt_days = (time.saturating_sub(self.last_time)) as f64 / 1_000_000_000.0 / 86_400.0;
        if dt_days > 0.0 {
            // 修复 P0-M5: 用 StdRng 生成正态分布近似（Box-Muller 简化版）
            let u1: f64 = self.rng.r#gen_range(0.0001..1.0);
            let u2: f64 = self.rng.r#gen_range(0.0001..1.0);
            let normal = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            let step = normal * self.daily_vol_bps as f64 / 10000.0;
            self.current_fv *= 1.0 + step * dt_days.sqrt();
            self.last_time = time;
        }
        // 修复 M-DS-8: 累积漂移或异常波动可能让 current_fv 变为 NaN/Inf，
        // 直接 `as Price` 会得到 UB 行为（饱和转换或截断）。
        // 检测到非有限值或负数时，记录 warn 并返回兜底值 1，避免污染下游 Agent。
        if !self.current_fv.is_finite() || self.current_fv < 0.0 {
            tracing::warn!(
                "[market-sim] BaselineOracle current_fv 异常: {}, 重置为 reference_price={}",
                self.current_fv,
                self.reference_price
            );
            self.current_fv = self.reference_price as f64;
            return OracleSignal { time, fundamental_value: 1, sentiment: 1.0, active_event: None };
        }
        let fv = self.current_fv.round() as Price;

        OracleSignal { time, fundamental_value: fv.max(1), sentiment: 1.0, active_event: None }
    }
}

/// 趋势 Oracle —— 带定向漂移（用于模拟牛市/熊市）
///
/// 修复 P0-M5: 原 noise 用 `(time * PI).sin()` 确定性函数，改为 StdRng。
pub struct DriftOracle {
    #[allow(dead_code)]
    reference_price: Price,
    /// 日漂移率（基点，正=看涨，负=看跌）
    drift_per_day_bps: i64,
    /// 日波动率（基点）
    vol_bps: i64,
    current_fv: f64,
    last_time: SimTimestamp,
    /// 修复 P0-M5: 可种子化 PRNG
    rng: StdRng,
}

impl DriftOracle {
    pub fn new(reference_price: Price, drift_per_day_bps: i64, vol_bps: i64) -> Self {
        Self {
            reference_price,
            drift_per_day_bps,
            vol_bps,
            current_fv: reference_price as f64,
            last_time: 0,
            rng: StdRng::from_entropy(),
        }
    }

    /// 修复 P0-M4: 带 seed 的构造函数
    pub fn with_seed(
        reference_price: Price,
        drift_per_day_bps: i64,
        vol_bps: i64,
        seed: u64,
    ) -> Self {
        Self {
            reference_price,
            drift_per_day_bps,
            vol_bps,
            current_fv: reference_price as f64,
            last_time: 0,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// 创建看涨趋势 Oracle（牛市）
    pub fn bull(reference_price: Price) -> Self {
        Self::new(reference_price, 30, 20) // 每日 +30bps, 20bps 波动
    }

    /// 创建看跌趋势 Oracle（熊市）
    pub fn bear(reference_price: Price) -> Self {
        Self::new(reference_price, -50, 40) // 每日 -50bps, 40bps 波动
    }
}

impl Oracle for DriftOracle {
    fn name(&self) -> &str {
        "drift"
    }

    fn signal_at(&mut self, time: SimTimestamp) -> OracleSignal {
        use rand::Rng;
        let dt_days = (time.saturating_sub(self.last_time)) as f64 / 1_000_000_000.0 / 86_400.0;
        if dt_days > 0.0 {
            let drift = self.drift_per_day_bps as f64 / 10000.0;
            // 修复 P0-M5: 用 StdRng Box-Muller 替代确定性 sin
            let u1: f64 = self.rng.r#gen_range(0.0001..1.0);
            let u2: f64 = self.rng.r#gen_range(0.0001..1.0);
            let normal = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            let noise = normal * self.vol_bps as f64 / 10000.0;
            self.current_fv *= 1.0 + (drift + noise) * dt_days.sqrt();
            self.last_time = time;
        }
        // 修复 P1-2: 累积漂移或异常波动可能让 current_fv 变为 NaN/Inf，
        // 与 BaselineOracle 保持一致的兜底检查：检测到非有限值或负数时，
        // 记录 warn 并重置为 reference_price，避免污染下游 Agent。
        if !self.current_fv.is_finite() || self.current_fv < 0.0 {
            tracing::warn!(
                "[market-sim] DriftOracle current_fv 异常: {}, 重置为 reference_price={}",
                self.current_fv,
                self.reference_price
            );
            self.current_fv = self.reference_price as f64;
            return OracleSignal { time, fundamental_value: 1, sentiment: 1.0, active_event: None };
        }
        let fv = self.current_fv.round() as Price;

        let sentiment_factor = if self.drift_per_day_bps > 0 {
            1.0 + 0.1
        } else {
            1.0 - 0.1
        };

        OracleSignal {
            time,
            fundamental_value: fv.max(1),
            sentiment: sentiment_factor,
            active_event: None,
        }
    }
}

/// 事件 Oracle —— 基线趋势 + 可注入的预定事件
pub struct EventOracle {
    base_oracle: DriftOracle,
    events: Vec<ScriptedEvent>,
    active_event: Option<MarketEvent>,
    event_end_time: SimTimestamp,
}

impl EventOracle {
    /// 创建闪崩情景 Oracle
    pub fn flash_crash(
        reference_price: Price,
        crash_time: SimTimestamp,
        recovery_time_ns: SimTimestamp,
    ) -> Self {
        let drop_bps = 500; // 5% 闪崩
        Self {
            base_oracle: DriftOracle::new(reference_price, 0, 15),
            events: vec![
                ScriptedEvent {
                    at_time: crash_time,
                    event: MarketEvent::PriceShock { direction: -1.0, magnitude_bps: drop_bps },
                },
                ScriptedEvent {
                    at_time: crash_time + recovery_time_ns,
                    event: MarketEvent::PriceShock { direction: 1.0, magnitude_bps: drop_bps },
                },
            ],
            active_event: None,
            event_end_time: 0,
        }
    }

    /// 创建高波动 Oracle
    pub fn high_volatility(reference_price: Price) -> Self {
        Self {
            base_oracle: DriftOracle::new(reference_price, 0, 80),
            events: vec![],
            active_event: None,
            event_end_time: 0,
        }
    }

    /// 修复 P1-1: 带 seed 的闪崩情景 Oracle，让蒙特卡洛路径可复现。
    /// seed 由蒙特卡洛引擎按 (base_seed + path_index) 派生，
    /// 保证每条路径独立且可复现。
    pub fn flash_crash_with_seed(
        reference_price: Price,
        crash_time: SimTimestamp,
        recovery_time_ns: SimTimestamp,
        seed: u64,
    ) -> Self {
        let drop_bps = 500; // 5% 闪崩
        Self {
            base_oracle: DriftOracle::with_seed(reference_price, 0, 15, seed),
            events: vec![
                ScriptedEvent {
                    at_time: crash_time,
                    event: MarketEvent::PriceShock { direction: -1.0, magnitude_bps: drop_bps },
                },
                ScriptedEvent {
                    at_time: crash_time + recovery_time_ns,
                    event: MarketEvent::PriceShock { direction: 1.0, magnitude_bps: drop_bps },
                },
            ],
            active_event: None,
            event_end_time: 0,
        }
    }

    /// 修复 P1-1: 带 seed 的高波动 Oracle，让蒙特卡洛路径可复现。
    pub fn high_volatility_with_seed(reference_price: Price, seed: u64) -> Self {
        Self {
            base_oracle: DriftOracle::with_seed(reference_price, 0, 80, seed),
            events: vec![],
            active_event: None,
            event_end_time: 0,
        }
    }
}

impl Oracle for EventOracle {
    fn name(&self) -> &str {
        "event"
    }

    fn signal_at(&mut self, time: SimTimestamp) -> OracleSignal {
        // 检查是否有事件触发
        // 修复 M-RES-13: 原实现 `event.at_time == time` 严格相等，
        // 若 event_at_time 落在两个 tick 之间（time 步进大于 1ns），
        // 事件永远不触发。改为 `event.at_time <= time` 且当前不在活跃事件期内。
        // event_end_time == 0 表示无活跃事件（sentinel）。
        for event in &self.events {
            let not_in_active = self.event_end_time == 0 || time < self.event_end_time;
            if event.at_time <= time && not_in_active {
                self.active_event = Some(event.event.clone());
                self.event_end_time = time + 1_000_000; // 1ms 后恢复
            }
        }

        // 检查事件是否过期
        if self.active_event.is_some() && time >= self.event_end_time {
            self.active_event = None;
        }

        let mut signal = self.base_oracle.signal_at(time);
        signal.active_event = self.active_event.clone();
        signal
    }
}
