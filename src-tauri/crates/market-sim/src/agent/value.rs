//! 价值 Agent — 基于公平价值估算的逆势交易。
//!
//! ## 行为
//!
//! 每次唤醒时：
//! - 对比 `reference_price`（公平价值）与当前最优买/卖价
//! - 如果价格远低于公平价值 → 买入
//! - 如果价格远高于公平价值 → 卖出
//! - 持仓达到上限后停止
//!
//! 区别于动量 Agent：价值 Agent 在下跌时买（逆势），动量在上涨时买（顺势）。

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::{OrderSide, *};

pub struct ValueAgent {
    id: String,
    /// 公平价值（分），可视为基本面目标价
    fair_value: Price,
    /// 价值偏差阈值（基点），超过此值才交易
    threshold_bps: i64,
    /// 订单大小
    order_size: Quantity,
    /// 持仓上限
    position_limit: i64,
    /// 唤醒间隔
    wakeup_interval_ns: SimTimestamp,
    /// 当前持仓
    position: i64,
}

impl ValueAgent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        fair_value: Price,
        threshold_bps: i64,
        order_size: Quantity,
        position_limit: i64,
        wakeup_interval_ns: SimTimestamp,
    ) -> Self {
        Self {
            id: id.into(),
            fair_value,
            threshold_bps,
            order_size,
            position_limit,
            wakeup_interval_ns,
            position: 0,
        }
    }
}

impl SimAgent for ValueAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Value"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Value
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
                // 修复 L-9: 删除 has_request 字段（死代码，从未被读取）。
                let mid = if let (Some(bid), Some(ask)) =
                    (snapshot.bids.first(), snapshot.asks.first())
                {
                    (bid.price + ask.price) as f64 / 2.0
                } else {
                    let last = snapshot.last_trade_price.unwrap_or(self.fair_value);
                    last as f64
                };

                let deviation = (mid - self.fair_value as f64).abs() / self.fair_value as f64;
                let threshold = self.threshold_bps as f64 / 10000.0;

                if deviation < threshold {
                    return Vec::new();
                }

                let mut actions = Vec::new();
                if mid < self.fair_value as f64 && self.position < self.position_limit {
                    // 低于公平价值 → 买入
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
                } else if mid > self.fair_value as f64 && self.position > -self.position_limit {
                    // 高于公平价值 → 卖出
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

                actions
            },
            MessageBody::OrderFilled { fill, .. } => {
                // 修复 C4.6: 在 OrderFilled 中根据实际成交更新 position
                for trade in &fill.trades {
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
