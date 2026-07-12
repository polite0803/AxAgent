//! 动量 Agent — 追涨杀跌，趋势跟踪。
//!
//! ## 行为
//!
//! 维护一个近期价格窗口（`lookback`）。每次唤醒时：
//! - 计算 `return = (latest_price - oldest_price) / oldest_price`
//! - 如果 `return > momentum_threshold` → 买入（看涨趋势）
//! - 如果 `return < -momentum_threshold` → 卖出（看跌趋势）
//! - 持仓达到 `position_limit` 时停止加仓
//!
//! ## 参数
//!
//! | 参数 | 典型值 | 说明 |
//! |------|--------|------|
//! | lookback | 5-20 | 价格窗口大小 |
//! | momentum_threshold | 0.001-0.005 | 动量触发阈值 |
//! | order_size | 100-500 | 单次交易数量 |
//! | position_limit | 2000-5000 | 持仓上限 |
//! | wakeup_interval_ns | 1_000_000 (1ms) | 检查频率 |

use std::collections::VecDeque;

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::{OrderSide, *};

pub struct MomentumAgent {
    id: String,
    lookback: usize,
    momentum_threshold: f64,
    order_size: Quantity,
    position_limit: i64,
    wakeup_interval_ns: SimTimestamp,
    /// 价格窗口（最近 lookback 个成交价）
    // 修复 L-8: 原 Vec::remove(0) 是 O(n) 操作，改用 VecDeque 实现高效 pop_front。
    price_window: VecDeque<f64>,
    /// 当前持仓（正 = 多）
    position: i64,
}

impl MomentumAgent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        lookback: usize,
        momentum_threshold: f64,
        order_size: Quantity,
        position_limit: i64,
        wakeup_interval_ns: SimTimestamp,
        reference_price: f64,
    ) -> Self {
        let mut pw = VecDeque::with_capacity(lookback);
        pw.push_back(reference_price);
        Self {
            id: id.into(),
            lookback,
            momentum_threshold,
            order_size,
            position_limit,
            wakeup_interval_ns,
            price_window: pw,
            position: 0,
        }
    }

    fn push_price(&mut self, price: f64) {
        self.price_window.push_back(price);
        if self.price_window.len() > self.lookback {
            self.price_window.pop_front();
        }
    }

    fn compute_momentum(&self) -> Option<f64> {
        let n = self.price_window.len();
        if n < 2 {
            return None;
        }
        let oldest = *self.price_window.front()?;
        let latest = *self.price_window.back()?;
        if oldest > 0.0 {
            Some((latest - oldest) / oldest)
        } else {
            None
        }
    }
}

impl SimAgent for MomentumAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Momentum"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Momentum
    }

    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        vec![
            AgentAction::SendMessage { target: "exchange".into(), body: MessageBody::RequestQuote },
            AgentAction::WakeupAfter(self.wakeup_interval_ns),
        ]
    }

    fn on_message(&mut self, msg: &MessageBody, ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            MessageBody::QuoteReply(snapshot) => {
                if let Some(last) = snapshot.last_trade_price {
                    let price = last as f64;
                    self.push_price(price);

                    if let Some(momentum) = self.compute_momentum() {
                        let mut actions = Vec::new();

                        if momentum > self.momentum_threshold && self.position < self.position_limit
                        {
                            // 看涨 → 以市价买入
                            // 修复 C4.6: 不在此处更新 position；改为在 OrderFilled 中根据实际成交量更新
                            let order = MarketOrder {
                                id: 0,
                                side: OrderSide::Buy,
                                quantity: self.order_size,
                                agent_id: self.id.clone(),
                                timestamp: ctx.current_time,
                            };
                            actions.push(AgentAction::SendMessage {
                                target: "exchange".into(),
                                body: MessageBody::SubmitMarket(order),
                            });
                        } else if momentum < -self.momentum_threshold
                            && self.position > -self.position_limit
                        {
                            // 看跌 → 以市价卖出
                            // 修复 C4.6: 不在此处更新 position；改为在 OrderFilled 中根据实际成交量更新
                            let order = MarketOrder {
                                id: 0,
                                side: OrderSide::Sell,
                                quantity: self.order_size,
                                agent_id: self.id.clone(),
                                timestamp: ctx.current_time,
                            };
                            actions.push(AgentAction::SendMessage {
                                target: "exchange".into(),
                                body: MessageBody::SubmitMarket(order),
                            });
                        }

                        return actions;
                    }
                }
                Vec::new()
            },
            MessageBody::OrderFilled { fill, .. } => {
                for trade in &fill.trades {
                    self.push_price(trade.price as f64);
                    // 修复 C4.6: 在 OrderFilled 中根据实际成交更新 position
                    if trade.buyer_agent_id == self.id {
                        self.position += trade.quantity as i64;
                    } else if trade.seller_agent_id == self.id {
                        self.position -= trade.quantity as i64;
                    }
                }
                Vec::new()
            },
            _ => Vec::new(),
        }
    }

    fn on_wakeup(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        vec![
            AgentAction::SendMessage { target: "exchange".into(), body: MessageBody::RequestQuote },
            AgentAction::WakeupAfter(self.wakeup_interval_ns),
        ]
    }
}
