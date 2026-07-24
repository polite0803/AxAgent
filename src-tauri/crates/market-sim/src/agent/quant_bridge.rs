//! 量化策略桥接 Agent —— 将 quant crate 的 Strategy trait 接入 DES 模拟。
//!
//! 这是 ABIDES 设计理念的核心落地：用户通过 `Strategy` trait 实现的真实策略
//! （双均线/MACD/RSI/布林/海龟等）作为 Agent 参与模拟，与做市商/动量/噪声
//! 交易者在同一个事件循环中博弈。
//!
//! ## 桥接机制
//!
//! ```text
//! DES 虚拟时钟
//!   │
//!   ├─ on_wakeup()
//!   │   ├─ 从 OrderBook 中间价合成 Bar
//!   │   ├─ block_on(strategy.on_bar()) → Vec<Signal>
//!   │   ├─ Signal → AgentAction(SubmitLimit/SubmitMarket)
//!   │   └─ 提交到 ExchangeAgent
//!   │
//!   ├─ on_message(OrderFilled)
//!   │   └─ 更新 ctx.cash / ctx.positions / ctx.trades
//!   │
//!   └─ on_message(QuoteReply)
//!       └─ 更新 last_price 用于 Bar 合成
//! ```

// 从 harness 引入策略契约类型（Strategy trait + Bar/Signal/SignalAction/StrategyCtx/Position/Side）
// 这消除了 market-sim 对 axagent-quant 的违规依赖（两者均为 consumer，应仅依赖 harness）
use axagent_harness::strategy_contract::{
    Bar, Position, Side, Signal, SignalAction, Strategy, StrategyCtx,
};

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::{FillResult, MarketOrder, OrderSide, Price, SimTimestamp, TradeRecord};

/// 在同步上下文中执行 async future
///
/// 修复 H3.5: 原实现用 futures::executor::block_on，在 tokio runtime 上下文中
/// 会创建嵌套 runtime（违反 AGENTS.md Rule 9）。改为优先复用已有 tokio runtime：
/// - 若当前线程在 tokio runtime 中，用 block_in_place + Handle::block_on
/// - 若无 runtime（如单元测试），创建临时 runtime 执行
///
/// 注意：block_in_place 必须在 multi-thread runtime 中调用。
/// Tauri 应用使用 tokio::main（multi-thread），生产路径安全。
fn block_on_async<F: std::future::Future>(fut: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // 在已有 tokio runtime 中执行（block_in_place 要求 multi-thread runtime）
            tokio::task::block_in_place(|| handle.block_on(fut))
        },
        Err(_) => {
            // 无 tokio runtime（如单元测试），创建临时 runtime 执行
            tokio::runtime::Runtime::new().expect("无法创建 tokio runtime").block_on(fut)
        },
    }
}

/// 量化策略桥接 Agent
pub struct QuantStrategyAgent {
    id: String,
    /// 被包装的真实策略
    strategy: Box<dyn Strategy>,
    /// 策略运行上下文（持仓/现金/K 线历史）
    ctx: StrategyCtx,
    /// 股票代码
    stock_code: String,
    /// 唤醒间隔
    wakeup_interval_ns: SimTimestamp,
    /// 当前最新价格
    last_price: f64,
    /// ── Bar 合成状态 ──
    bar_open: f64,
    bar_high: f64,
    bar_low: f64,
    bar_close: f64,
    bar_volume: u64,
    bar_start_time: SimTimestamp,
    /// 上一个完成的 bar 时间
    last_bar_time: Option<String>,
    /// 是否已执行 on_init
    initialized: bool,
    /// 自增 ID
    next_id: u64,
    /// 最大 Bar 缓存数
    max_bars: usize,
}

impl QuantStrategyAgent {
    /// 创建量化策略桥接 Agent
    ///
    /// - `strategy`: 实现了 quant::Strategy trait 的策略实例
    /// - `stock_code`: 股票代码
    /// - `reference_price`: 参考价
    /// - `initial_cash`: 起始资金
    /// - `wakeup_interval_ns`: 唤醒间隔（即 Bar 周期）
    pub fn new(
        id: impl Into<String>,
        strategy: Box<dyn Strategy>,
        stock_code: impl Into<String>,
        reference_price: Price,
        initial_cash: f64,
        wakeup_interval_ns: SimTimestamp,
    ) -> Self {
        let code = stock_code.into();
        Self {
            id: id.into(),
            strategy,
            ctx: StrategyCtx { cash: initial_cash, ..Default::default() },
            stock_code: code,
            wakeup_interval_ns,
            last_price: reference_price as f64,
            bar_open: reference_price as f64,
            bar_high: reference_price as f64,
            bar_low: reference_price as f64,
            bar_close: reference_price as f64,
            bar_volume: 0,
            bar_start_time: 0,
            last_bar_time: None,
            initialized: false,
            next_id: 1,
            max_bars: 500,
        }
    }

    fn gen_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// 将模拟时间戳转为日期字符串
    fn ts_to_date(time: SimTimestamp) -> String {
        let total_secs = time / 1_000_000_000;
        let h = (total_secs / 3600) % 24;
        let m = (total_secs / 60) % 60;
        let s = total_secs % 60;
        format!("1970-01-01 {:02}:{:02}:{:02}", h, m, s)
    }

    /// 从当前累积数据合成一个 Bar，推入 ctx.bar_history
    fn finalize_bar(&mut self, time: SimTimestamp) -> Bar {
        let date_str = self.last_bar_time.clone().unwrap_or_else(|| Self::ts_to_date(time));
        let bar = Bar {
            code: self.stock_code.clone(),
            open: self.bar_open,
            high: self.bar_high,
            low: self.bar_low,
            close: self.bar_close,
            volume: self.bar_volume as f64,
            amount: 0.0,
            date: date_str,
            limit_up: None,
            limit_down: None,
            turnover_rate: None,
            adj_factor: None,
            is_st: false,
        };

        // 推入 ctx.bar_history
        self.ctx.bar_history.entry(self.stock_code.clone()).or_default().push(bar.clone());

        // 限制缓存大小
        if let Some(bars) = self.ctx.bar_history.get_mut(&self.stock_code) {
            while bars.len() > self.max_bars {
                bars.remove(0);
            }
        }

        bar
    }

    /// 重置 Bar 累积器
    fn reset_bar(&mut self, price: f64, time: SimTimestamp) {
        self.bar_open = price;
        self.bar_high = price;
        self.bar_low = price;
        self.bar_close = price;
        self.bar_volume = 0;
        self.bar_start_time = time;
        self.last_bar_time = Some(Self::ts_to_date(time));
    }

    /// 更新 Bar 高开低收
    fn update_bar(&mut self, price: f64, volume: u64) {
        self.bar_close = price;
        self.bar_high = self.bar_high.max(price);
        self.bar_low = self.bar_low.min(price);
        self.bar_volume += volume;
    }

    /// 将 quant::Signal 转换为 AgentAction
    fn signal_to_action(&mut self, signal: &Signal, ctx: &AgentContext) -> Vec<AgentAction> {
        let mut actions = Vec::new();

        let price = signal.strength * self.last_price;
        // 修复 M-DS-9: 原代码 `100.max(...)` 永远取较大值，
        // 资金不够 100 股时仍下 100 股，导致超出资金上限。
        // 现在按资金上限计算整手股数（A 股最小交易单位 100 股），
        // 若资金不够 100 股则 qty = 0（不下单）。
        let max_affordable_lots = (self.ctx.cash / price.max(1.0)) as u64 / 100;
        let qty = max_affordable_lots * 100;

        match signal.action {
            SignalAction::Buy => {
                // 修复 M-DS-9: qty = 0（资金不够 100 股）时跳过下单，避免空单
                if qty > 0 {
                    let order = MarketOrder {
                        id: self.gen_id(),
                        side: OrderSide::Buy,
                        quantity: qty,
                        agent_id: self.id.clone(),
                        timestamp: ctx.current_time,
                    };
                    actions.push(AgentAction::SendMessage {
                        target: "exchange".into(),
                        body: MessageBody::SubmitMarket(order),
                    });
                }
            },
            SignalAction::Sell => {
                // 卖：减当前持仓
                let pos = self.ctx.positions.get(&self.stock_code).map(|p| p.quantity).unwrap_or(0);
                if pos > 0 {
                    let order = MarketOrder {
                        id: self.gen_id(),
                        side: OrderSide::Sell,
                        quantity: pos,
                        agent_id: self.id.clone(),
                        timestamp: ctx.current_time,
                    };
                    actions.push(AgentAction::SendMessage {
                        target: "exchange".into(),
                        body: MessageBody::SubmitMarket(order),
                    });
                }
            },
            SignalAction::Hold => {},
        }

        actions
    }

    /// 模拟 Engine 的 apply_fill，更新 ctx
    fn apply_fill(&mut self, fill_result: &FillResult) {
        let code = self.stock_code.clone();
        for trade in &fill_result.trades {
            let qty = trade.quantity as f64;
            let price_f = trade.price as f64;
            let cost = qty * price_f;

            if trade.buyer_agent_id == self.id {
                // 买入
                self.ctx.cash -= cost;
                let entry = self.ctx.positions.entry(code.clone()).or_insert_with(|| Position {
                    code: code.clone(),
                    name: None,
                    side: Side::Long,
                    quantity: 0,
                    cost_basis: 0.0,
                    last_price: price_f,
                    market_value: 0.0,
                    unrealized_pnl: 0.0,
                    realized_pnl: 0.0,
                    entry_date: "sim".into(),
                    entry_timestamp: "sim".into(),
                });
                let total_qty = entry.quantity as f64;
                let total_cost = entry.cost_basis * total_qty;
                let new_qty = total_qty + qty;
                entry.cost_basis = if new_qty > 0.0 {
                    (total_cost + cost) / new_qty
                } else {
                    0.0
                };
                entry.quantity = new_qty as u64;
                entry.last_price = price_f;
                entry.market_value = entry.quantity as f64 * price_f;
            } else if trade.seller_agent_id == self.id {
                // 卖出
                self.ctx.cash += cost;
                if let Some(pos) = self.ctx.positions.get_mut(&code) {
                    let sell_qty = qty.min(pos.quantity as f64);
                    let pnl = sell_qty * (price_f - pos.cost_basis);
                    pos.quantity = (pos.quantity as f64 - sell_qty).max(0.0) as u64;
                    pos.realized_pnl += pnl;
                    pos.last_price = price_f;
                    pos.market_value = pos.quantity as f64 * price_f;
                }
            }
        }
    }
}

impl SimAgent for QuantStrategyAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "QuantStrategy"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Strategy
    }

    fn trade_history(&self) -> &[TradeRecord] {
        &[]
    }

    fn on_init(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        // 调用策略的 on_init（同步执行）
        let _ = block_on_async(self.strategy.on_init(&mut self.ctx));

        self.initialized = true;
        self.bar_start_time = ctx.current_time;
        self.last_bar_time = Some(Self::ts_to_date(ctx.current_time));

        // 首次启动：请求报价 + 定时唤醒
        vec![
            AgentAction::SendMessage { target: "exchange".into(), body: MessageBody::RequestQuote },
            AgentAction::WakeupAfter(self.wakeup_interval_ns),
        ]
    }

    fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let mut actions = Vec::new();

        // 1. 合成 Bar
        let bar = self.finalize_bar(ctx.current_time);
        self.reset_bar(self.last_price, ctx.current_time);

        // 更新 ctx 时间信息
        self.ctx.current_date = bar.date[..10].to_string();
        self.ctx.current_time = bar.date.clone();

        // 2. 调用策略的 on_bar
        let signal_result = block_on_async(self.strategy.on_bar(&bar, &mut self.ctx));
        match signal_result {
            Ok(signals) => {
                for signal in signals {
                    actions.extend(self.signal_to_action(&signal, ctx));
                }
            },
            Err(e) => {
                tracing::warn!("QuantStrategyAgent[{}]: on_bar error: {:?}", self.id, e);
            },
        }

        // 3. 持续获取报价 + 下次唤醒
        actions.push(AgentAction::SendMessage {
            target: "exchange".into(),
            body: MessageBody::RequestQuote,
        });
        actions.push(AgentAction::WakeupAfter(self.wakeup_interval_ns));

        actions
    }

    fn on_message(&mut self, msg: &MessageBody, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            MessageBody::OrderFilled { fill, .. } => {
                // 更新持仓/现金
                self.apply_fill(fill);
                // 更新最新价格
                for trade in &fill.trades {
                    self.last_price = trade.price as f64;
                    self.update_bar(trade.price as f64, trade.quantity);
                }
                Vec::new()
            },
            MessageBody::QuoteReply(snapshot) => {
                if let Some(last) = snapshot.last_trade_price {
                    self.last_price = last as f64;
                    self.update_bar(last as f64, 0u64);
                }
                Vec::new()
            },
            MessageBody::OrderPlaced { .. } => Vec::new(),
            MessageBody::OrderCancelled { .. } => Vec::new(),
            _ => Vec::new(),
        }
    }

    fn on_sim_end(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        let _ = block_on_async(self.strategy.on_finish(&mut self.ctx));
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axagent_harness::indicators::sma;
    use axagent_harness::strategy_contract::CloseReason;
    use serde_json::{Value, json};

    /// 本地测试用 MockStrategy
    ///
    /// market-sim 是 consumer，按 AGENTS.md 铁律 5，测试不得依赖 quant crate
    /// （同为 consumer）。这里实现一个简单的均线策略：累积 N 根 bar 后发出 Buy 信号，
    /// 用于验证 QuantStrategyAgent 桥接事件循环的正确性，与具体业务策略解耦。
    struct MockMaCrossStrategy {
        short_period: usize,
        long_period: usize,
    }

    impl MockMaCrossStrategy {
        fn new(short_period: usize, long_period: usize) -> Self {
            Self { short_period, long_period }
        }
    }

    #[async_trait]
    impl Strategy for MockMaCrossStrategy {
        fn name(&self) -> &str {
            "mock_ma_cross"
        }

        fn params(&self) -> Value {
            json!({
                "short_period": self.short_period,
                "long_period": self.long_period,
            })
        }

        fn set_param(
            &mut self,
            key: &str,
            value: Value,
        ) -> axagent_harness::core_error::Result<()> {
            use axagent_harness::core_error::AxAgentError;
            match key {
                "short_period" => {
                    self.short_period =
                        value.as_u64().ok_or_else(|| AxAgentError::Validation(key.to_string()))?
                            as usize;
                },
                "long_period" => {
                    self.long_period =
                        value.as_u64().ok_or_else(|| AxAgentError::Validation(key.to_string()))?
                            as usize;
                },
                _ => {
                    return Err(AxAgentError::Validation(format!("未知参数: {}", key)));
                },
            }
            Ok(())
        }

        async fn on_bar(
            &mut self,
            bar: &Bar,
            ctx: &mut StrategyCtx,
        ) -> axagent_harness::core_error::Result<Vec<Signal>> {
            let history = match ctx.bar_history.get(&bar.code) {
                Some(h) if h.len() > self.long_period => h,
                _ => return Ok(vec![]),
            };
            let cs: Vec<f64> = history.iter().map(|b| b.close).collect();
            let cur_short = sma(&cs, self.short_period);
            let cur_long = sma(&cs, self.long_period);
            let prev_short = sma(&cs[..cs.len() - 1], self.short_period);
            let prev_long = sma(&cs[..cs.len() - 1], self.long_period);

            if let (Some(cs_), Some(cl_), Some(ps_), Some(pl_)) =
                (cur_short, cur_long, prev_short, prev_long)
            {
                if ps_ <= pl_ && cs_ > cl_ {
                    return Ok(vec![Signal {
                        code: bar.code.clone(),
                        action: SignalAction::Buy,
                        strength: 0.7,
                        reason: format!("Mock MA{} 上穿 MA{}", self.short_period, self.long_period),
                        target_weight: None,
                        close_reason: None,
                    }]);
                }
                if ps_ >= pl_ && cs_ < cl_ {
                    return Ok(vec![Signal {
                        code: bar.code.clone(),
                        action: SignalAction::Sell,
                        strength: 0.7,
                        reason: format!("Mock MA{} 下穿 MA{}", self.short_period, self.long_period),
                        target_weight: None,
                        close_reason: Some(CloseReason::SignalReverse),
                    }]);
                }
            }
            Ok(vec![])
        }
    }

    fn run_quant_sim(
        strategy: Box<dyn Strategy>,
        duration_ns: SimTimestamp,
    ) -> crate::kernel::SimResult {
        use crate::agent::*;
        use crate::config::SimConfig;
        use crate::kernel::SimKernel;

        let price = 1000;
        let mut kernel = SimKernel::new(SimConfig {
            max_time_ns: duration_ns,
            default_latency_ns: 1_000,
            reference_price: price,
            ..Default::default()
        });

        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
        kernel.register(Box::new(MarketMakerAgent::new("mm", 35, 500, 5000, 0.1, 500_000, price)));
        kernel.register(Box::new(QuantStrategyAgent::new(
            "quant",
            strategy,
            "000001",
            price,
            1_000_000.0,
            1_000_000,
        )));
        kernel.register(Box::new(NoiseAgent::new("noise", 500_000, 0.27, 50, 32, price, 42)));

        kernel.run().unwrap()
    }

    #[test]
    fn test_quant_ma_cross_produces_events() {
        // 原 test 依赖 axagent_quant::MaCrossStrategy，按铁律 5 改用本地 MockMaCrossStrategy
        let strategy = Box::new(MockMaCrossStrategy::new(5, 20));
        let result = run_quant_sim(strategy, 500_000_000);

        assert!(result.total_events > 50, "events={}", result.total_events);
        eprintln!(
            "MockMaCross: events={} trades={} mid={:?}",
            result.total_events,
            result.trades.len(),
            result.final_mid_price
        );
    }

    #[test]
    fn test_quant_macd_produces_events() {
        // 用同一 Mock 策略验证事件循环（原 test 依赖 axagent_quant::MacdStrategy）
        let strategy = Box::new(MockMaCrossStrategy::new(12, 26));
        let result = run_quant_sim(strategy, 500_000_000);

        assert!(result.total_events > 50);
        eprintln!("MockMacd: events={} trades={}", result.total_events, result.trades.len());
    }

    #[test]
    fn test_quant_rsi_produces_events() {
        // 用同一 Mock 策略验证事件循环（原 test 依赖 axagent_quant::RsiStrategy）
        let strategy = Box::new(MockMaCrossStrategy::new(6, 14));
        let result = run_quant_sim(strategy, 500_000_000);

        assert!(result.total_events > 50);
        eprintln!("MockRsi: events={} trades={}", result.total_events, result.trades.len());
    }
}
