// SPDX-License-Identifier: AGPL-3.0-only

//! 跨股票信号聚合器（P3-1）
//!
//! 在 `signals.rs`（单股信号检测）和 `portfolio_monitor.rs`（持仓指标聚合）
//! 之间填补"信号聚合层"架构缺口。
//!
//! ## 核心抽象
//!
//! - [`SignalType`] — 统一信号类型枚举（替代 signals.rs 中分散的 String 字面量）
//! - [`StockSignal`] — 单股信号载体（stock_code / signal_type / direction / strength / source / timestamp）
//! - [`PortfolioSignal`] — 聚合后的组合级信号（多股同方向 → 组合级卖出/买入提示）
//! - [`CrossStockSignalAggregator`] — 运行时聚合器，接收单股信号流，按时间窗口聚合
//!
//! ## 聚合规则
//!
//! 1. **同向聚集规则**：在 `window_secs` 时间窗内，同方向（多头/空头）信号数 ≥ `min_signal_count`
//!    时触发组合级信号（避免单股噪声误判）
//! 2. **强度加权**：组合信号强度 = Σ(单股强度) / max_possible_strength，归一化到 [0, 1]
//! 3. **去重冷却**：同类型组合信号在 `cooldown_secs` 内不重复触发
//!
//! ## 使用场景
//!
//! - `RealtimeMonitor` 在每次轮询后将单股告警喂给聚合器
//! - 聚合器输出 `PortfolioSignal` 通过 `broadcast::Sender` 推给订阅者
//! - 前端 / 风控系统订阅组合级信号做调仓决策

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 组合快照回调类型别名（避免 clippy::type_complexity 警告）
pub type SnapshotProvider = Arc<dyn Fn() -> Option<PortfolioSnapshot> + Send + Sync>;

/// 统一信号类型枚举。
///
/// 替代 `signals.rs` 中 `MACrossResult.signal: String` 和 `BreakoutResult.breakout_type: String`
/// 的字符串字面量，提供类型安全。新增信号类型时在此扩展。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// 金叉（短期均线上穿长期均线）
    GoldenCross,
    /// 死叉（短期均线下穿长期均线）
    DeathCross,
    /// 突破压力位
    ResistanceBreak,
    /// 跌破支撑位
    SupportBreak,
    /// 涨跌幅异常
    ChangeSpike,
    /// 换手率异常（量能异动）
    VolumeSpike,
    /// 触及止损
    StopLossHit,
    /// 触及止盈
    TakeProfitHit,
}

impl SignalType {
    /// 信号方向：+1 = 多头/看涨，-1 = 空头/看跌，0 = 中性
    pub fn direction(&self) -> i8 {
        match self {
            SignalType::GoldenCross | SignalType::ResistanceBreak | SignalType::TakeProfitHit => 1,
            SignalType::DeathCross
            | SignalType::SupportBreak
            | SignalType::StopLossHit
            | SignalType::ChangeSpike
            | SignalType::VolumeSpike => -1,
        }
    }

    /// 人类可读名称（用于告警消息）
    pub fn label(&self) -> &'static str {
        match self {
            SignalType::GoldenCross => "金叉",
            SignalType::DeathCross => "死叉",
            SignalType::ResistanceBreak => "突破压力位",
            SignalType::SupportBreak => "跌破支撑位",
            SignalType::ChangeSpike => "涨跌幅异常",
            SignalType::VolumeSpike => "换手率异常",
            SignalType::StopLossHit => "触及止损",
            SignalType::TakeProfitHit => "触及止盈",
        }
    }
}

/// 信号方向枚举（用于组合级聚合时的同向判定）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalDirection {
    Bullish,
    Bearish,
    Neutral,
}

impl SignalDirection {
    pub fn from_i8(d: i8) -> Self {
        if d > 0 {
            SignalDirection::Bullish
        } else if d < 0 {
            SignalDirection::Bearish
        } else {
            SignalDirection::Neutral
        }
    }
}

/// 单股信号载体。
///
/// 由 `RealtimeMonitor` 在每次轮询时构造，喂给 `CrossStockSignalAggregator`。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockSignal {
    pub stock_code: String,
    pub stock_name: String,
    pub signal_type: SignalType,
    /// 信号强度 [0.0, 1.0]，由调用方根据价格偏离度 / 量能放大倍数等估算
    pub strength: f64,
    /// 信号来源（"monitor" / "signals.detect_ma_cross" / "signals.detect_breakout"）
    pub source: String,
    /// Unix 时间戳（秒）
    pub timestamp: i64,
    /// 触发信号时的当前价（可选，便于前端展示）
    pub current_price: Option<f64>,
    /// 触发信号时的涨跌幅（可选）
    pub change_pct: Option<f64>,
}

/// 组合级信号（聚合后输出）。
///
/// 当多只股票在时间窗内同方向触发信号时生成。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioSignal {
    /// 组合信号方向（多头聚集 / 空头聚集 / 中性）
    pub direction: SignalDirection,
    /// 触发该组合信号的股票集合
    pub stocks: Vec<String>,
    /// 主导信号类型（多数派）
    pub dominant_signal: SignalType,
    /// 组合信号强度 [0.0, 1.0]
    pub strength: f64,
    /// 触发时的市场快照（可选，便于前端展示）
    pub snapshot: Option<PortfolioSnapshot>,
    /// 建议操作（如 "组合集中度风险升高，建议减仓 20%"）
    pub suggested_action: String,
    /// Unix 时间戳（秒）
    pub timestamp: i64,
}

/// 组合快照（聚合时记录的持仓上下文）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioSnapshot {
    /// 持仓股票数
    pub position_count: usize,
    /// 头部集中度（top1 市值占比）
    pub top_concentration_pct: Option<f64>,
    /// 行业最大集中度
    pub max_sector_pct: Option<f64>,
    /// 风险等级
    pub risk_level: Option<String>,
}

/// 聚合器配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatorConfig {
    /// 时间窗口（秒），默认 300（5 分钟）
    pub window_secs: i64,
    /// 触发组合信号的最小同向信号数，默认 3
    pub min_signal_count: usize,
    /// 同类组合信号冷却时间（秒），默认 600（10 分钟）
    pub cooldown_secs: i64,
    /// 最小强度阈值（单股强度低于此值不参与聚合），默认 0.3
    pub min_strength: f64,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self { window_secs: 300, min_signal_count: 3, cooldown_secs: 600, min_strength: 0.3 }
    }
}

/// 跨股票信号聚合器。
///
/// 内部维护时间窗口内的信号缓冲区，每次喂入新信号时尝试触发组合级信号。
/// 通过 `broadcast::Sender<PortfolioSignal>` 推给订阅者。
///
/// # 线程安全
///
/// 内部状态用 `RwLock` 保护，可被 `Arc<RealtimeMonitor>` 等多 owner 共享。
pub struct CrossStockSignalAggregator {
    config: RwLock<AggregatorConfig>,
    /// 时间窗内的信号缓冲区（key = stock_code, value = 该股票最新信号）
    /// 注：同一只股票在窗内只保留最新信号，避免单股高频刷屏
    signal_buffer: RwLock<HashMap<String, StockSignal>>,
    /// 各方向最后触发组合信号的时间戳（用于冷却去重）
    last_triggered: RwLock<HashMap<SignalDirection, i64>>,
    /// 组合信号广播器
    signal_tx: tokio::sync::broadcast::Sender<PortfolioSignal>,
    /// 可选的组合快照回调（每次触发聚合时调用以获取当前持仓上下文）
    snapshot_provider: RwLock<Option<SnapshotProvider>>,
}

impl CrossStockSignalAggregator {
    pub fn new(config: AggregatorConfig) -> Self {
        let (signal_tx, _) = tokio::sync::broadcast::channel(64);
        Self {
            config: RwLock::new(config),
            signal_buffer: RwLock::new(HashMap::new()),
            last_triggered: RwLock::new(HashMap::new()),
            signal_tx,
            snapshot_provider: RwLock::new(None),
        }
    }

    /// 订阅组合级信号流
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<PortfolioSignal> {
        self.signal_tx.subscribe()
    }

    /// 更新聚合器配置（运行时热更新）
    pub async fn set_config(&self, config: AggregatorConfig) {
        *self.config.write().await = config;
    }

    /// 查询当前配置
    pub async fn config(&self) -> AggregatorConfig {
        self.config.read().await.clone()
    }

    /// 设置组合快照回调（由 wiring 层注入，从 AppState 读取当前持仓指标）
    pub async fn set_snapshot_provider(&self, provider: SnapshotProvider) {
        *self.snapshot_provider.write().await = Some(provider);
    }

    /// 喂入单股信号并尝试触发组合级信号。
    ///
    /// 调用时机：`RealtimeMonitor::check_alerts` 检测到告警后调用此方法。
    /// 返回 `Some(PortfolioSignal)` 表示触发了组合级信号，`None` 表示未触发。
    pub async fn feed(&self, signal: StockSignal) -> Option<PortfolioSignal> {
        let cfg = self.config.read().await.clone();
        if signal.strength < cfg.min_strength {
            return None;
        }

        // 1) 写入缓冲区（覆盖同股票旧信号）
        {
            let mut buf = self.signal_buffer.write().await;
            buf.insert(signal.stock_code.clone(), signal.clone());
        }

        // 2) 清理过期信号（窗口外）
        let now = signal.timestamp;
        let window_start = now - cfg.window_secs;
        {
            let mut buf = self.signal_buffer.write().await;
            buf.retain(|_, s| s.timestamp >= window_start);
        }

        // 3) 按方向统计
        let buf = self.signal_buffer.read().await;
        let mut bull_stocks: Vec<&StockSignal> = Vec::new();
        let mut bear_stocks: Vec<&StockSignal> = Vec::new();
        for s in buf.values() {
            match SignalDirection::from_i8(s.signal_type.direction()) {
                SignalDirection::Bullish => bull_stocks.push(s),
                SignalDirection::Bearish => bear_stocks.push(s),
                SignalDirection::Neutral => {},
            }
        }

        // 4) 选择信号数较多的一方作为候选方向
        let (candidate_dir, candidate_stocks): (SignalDirection, Vec<&StockSignal>) =
            if bull_stocks.len() >= bear_stocks.len() {
                (SignalDirection::Bullish, bull_stocks)
            } else {
                (SignalDirection::Bearish, bear_stocks)
            };

        if candidate_stocks.len() < cfg.min_signal_count {
            return None;
        }

        // 5) 冷却检查
        {
            let last = self.last_triggered.read().await;
            if let Some(&last_ts) = last.get(&candidate_dir) {
                if now - last_ts < cfg.cooldown_secs {
                    return None;
                }
            }
        }

        // 6) 计算组合信号强度（归一化 [0, 1]）
        // 强度公式：Σ(单股强度) / min_signal_count，上限 1.0
        let total_strength: f64 = candidate_stocks.iter().map(|s| s.strength).sum();
        let portfolio_strength = (total_strength / cfg.min_signal_count as f64).min(1.0);

        // 7) 找多数派信号类型（同向信号中的众数）
        let mut type_counts: HashMap<SignalType, usize> = HashMap::new();
        for s in &candidate_stocks {
            *type_counts.entry(s.signal_type).or_insert(0) += 1;
        }
        let dominant_signal = type_counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(t, _)| t)
            .unwrap_or(candidate_stocks[0].signal_type);

        // 8) 调用 snapshot provider 获取持仓上下文
        let snapshot = {
            let provider = self.snapshot_provider.read().await;
            provider.as_ref().and_then(|p| p())
        };

        // 9) 构造建议操作文案
        let suggested_action = build_suggested_action(
            candidate_dir,
            candidate_stocks.len(),
            portfolio_strength,
            &snapshot,
        );

        let portfolio_signal = PortfolioSignal {
            direction: candidate_dir,
            stocks: candidate_stocks.iter().map(|s| s.stock_code.clone()).collect(),
            dominant_signal,
            strength: portfolio_strength,
            snapshot: snapshot.clone(),
            suggested_action,
            timestamp: now,
        };

        // 10) 更新冷却时间
        {
            let mut last = self.last_triggered.write().await;
            last.insert(candidate_dir, now);
        }

        // 11) 广播
        let _ = self.signal_tx.send(portfolio_signal.clone());

        Some(portfolio_signal)
    }

    /// 清空信号缓冲区（用于交易日重置或测试）
    pub async fn clear(&self) {
        self.signal_buffer.write().await.clear();
        self.last_triggered.write().await.clear();
    }

    /// 查询当前缓冲区快照（调试用）
    pub async fn buffer_snapshot(&self) -> Vec<StockSignal> {
        self.signal_buffer.read().await.values().cloned().collect()
    }
}

/// 构造组合信号的操作建议文案
fn build_suggested_action(
    direction: SignalDirection,
    count: usize,
    strength: f64,
    snapshot: &Option<PortfolioSnapshot>,
) -> String {
    let dir_label = match direction {
        SignalDirection::Bullish => "多头聚集",
        SignalDirection::Bearish => "空头聚集",
        SignalDirection::Neutral => "中性",
    };

    let strength_label = if strength >= 0.7 {
        "强"
    } else if strength >= 0.4 {
        "中"
    } else {
        "弱"
    };

    let risk_note = snapshot
        .as_ref()
        .and_then(|s| {
            s.top_concentration_pct.map(|pct| {
                if pct > 0.4 {
                    format!("（头部集中度 {:.0}% 已超 40% 警戒线）", pct * 100.0)
                } else {
                    String::new()
                }
            })
        })
        .unwrap_or_default();

    match direction {
        SignalDirection::Bearish => format!(
            "组合出现{}{}信号（{} 只股票同向，强度 {}），建议评估减仓或对冲{}",
            strength_label, dir_label, count, strength_label, risk_note
        ),
        SignalDirection::Bullish => format!(
            "组合出现{}{}信号（{} 只股票同向，强度 {}），可考虑分批加仓或布局相关 ETF{}",
            strength_label, dir_label, count, strength_label, risk_note
        ),
        SignalDirection::Neutral => format!("组合信号中性（{} 只），保持观察", count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(code: &str, signal_type: SignalType, strength: f64, ts: i64) -> StockSignal {
        StockSignal {
            stock_code: code.to_string(),
            stock_name: format!("股票{}", code),
            signal_type,
            strength,
            source: "test".to_string(),
            timestamp: ts,
            current_price: Some(10.0),
            change_pct: Some(-2.0),
        }
    }

    #[tokio::test]
    async fn test_aggregator_triggers_after_min_count() {
        let agg = CrossStockSignalAggregator::new(AggregatorConfig {
            window_secs: 300,
            min_signal_count: 3,
            cooldown_secs: 600,
            min_strength: 0.3,
        });
        let now = 1700000000;

        // 喂入 2 个空头信号，不应触发
        assert!(agg.feed(make_signal("000001", SignalType::StopLossHit, 0.6, now)).await.is_none());
        assert!(agg
            .feed(make_signal("000002", SignalType::SupportBreak, 0.5, now + 10))
            .await
            .is_none());

        // 第 3 个空头信号应触发组合信号
        let result = agg.feed(make_signal("000003", SignalType::DeathCross, 0.7, now + 20)).await;
        assert!(result.is_some(), "第三个同向信号应触发组合信号");
        let ps = result.unwrap();
        assert_eq!(ps.direction, SignalDirection::Bearish);
        assert_eq!(ps.stocks.len(), 3);
        assert!(ps.strength > 0.0);
    }

    #[tokio::test]
    async fn test_aggregator_respects_cooldown() {
        let agg = CrossStockSignalAggregator::new(AggregatorConfig {
            window_secs: 300,
            min_signal_count: 2,
            cooldown_secs: 600,
            min_strength: 0.3,
        });
        let now = 1700000000;

        // 触发一次
        let r1 = agg.feed(make_signal("000001", SignalType::StopLossHit, 0.6, now)).await;
        assert!(r1.is_none()); // 只 1 个信号
        let r2 = agg.feed(make_signal("000002", SignalType::StopLossHit, 0.6, now + 10)).await;
        assert!(r2.is_some(), "2 个信号应触发");

        // 立即再喂入新信号，应被冷却拦截
        let r3 = agg.feed(make_signal("000003", SignalType::StopLossHit, 0.6, now + 20)).await;
        assert!(r3.is_none(), "冷却期内不应再次触发");
    }

    #[tokio::test]
    async fn test_low_strength_filtered() {
        let agg = CrossStockSignalAggregator::new(AggregatorConfig {
            window_secs: 300,
            min_signal_count: 2,
            cooldown_secs: 600,
            min_strength: 0.5,
        });
        let now = 1700000000;

        // 强度低于阈值，不应进入缓冲区
        let r1 = agg.feed(make_signal("000001", SignalType::StopLossHit, 0.3, now)).await;
        assert!(r1.is_none());
        let r2 = agg.feed(make_signal("000002", SignalType::StopLossHit, 0.3, now + 10)).await;
        assert!(r2.is_none(), "低强度信号不应参与聚合");
    }
}
