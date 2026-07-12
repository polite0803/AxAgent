//! 做市商 Agent — 双边报价，赚取价差，控制库存。
//!
//! ## 行为
//!
//! 每隔 `wakeup_interval_ns` 唤醒一次，调整双边报价：
//! - **基准价** = 上次成交价（或 reference_price）
//! - **买价** = 基准价 - spread/2 × (1 + inventory_skew × 净库存/库存上限)
//! - **卖价** = 基准价 + spread/2 × (1 + inventory_skew × 净库存/库存上限)
//! - 取消旧订单，提交新限价单
//!
//! ## 参数
//!
//! | 参数 | 典型值 | 说明 |
//! |------|--------|------|
//! | spread_bps | 20-50 | 报价价差（基点） |
//! | quote_size | 100-1000 | 每档挂单量 |
//! | position_limit | 5000 | 净库存上限（正=净买，负=净卖） |
//! | inventory_skew | 0.05-0.2 | 库存偏移灵敏度 |
//! | wakeup_interval_ns | 500_000 (0.5ms) | 报价刷新间隔 |

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::{OrderSide, *};

pub struct MarketMakerAgent {
    id: String,
    /// 目标价差（基点）
    spread_bps: i64,
    /// 每档挂单量
    quote_size: Quantity,
    /// 净库存上限
    position_limit: i64,
    /// 库存偏移灵敏度
    inventory_skew: f64,
    /// 唤醒间隔（ns）
    wakeup_interval_ns: SimTimestamp,
    /// 当前基准价
    reference_price: Price,
    /// 净库存（正 = 净多）
    net_position: i64,
    /// 上一次的订单 ID（用于撤单）
    bid_order_id: Option<OrderId>,
    ask_order_id: Option<OrderId>,
    /// 自增 order ID 来源
    next_id: u64,
    /// 当前最佳中间价
    last_mid: Option<f64>,
    /// 已处理的消息数
    trade_count: u64,
}

impl MarketMakerAgent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        spread_bps: i64,
        quote_size: Quantity,
        position_limit: i64,
        inventory_skew: f64,
        wakeup_interval_ns: SimTimestamp,
        reference_price: Price,
    ) -> Self {
        Self {
            id: id.into(),
            spread_bps,
            quote_size,
            position_limit,
            inventory_skew,
            wakeup_interval_ns,
            reference_price,
            net_position: 0,
            bid_order_id: None,
            ask_order_id: None,
            next_id: 1,
            last_mid: None,
            trade_count: 0,
        }
    }

    fn gen_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// 计算报价并提交订单
    fn submit_quotes(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let base = self.reference_price;
        let spread_raw = (base as f64 * self.spread_bps as f64 / 10000.0).round() as Price;

        // 库存偏移：有净多头时压买价提卖价
        let position_ratio = self.net_position as f64 / self.position_limit.max(1) as f64;
        let skew = position_ratio * self.inventory_skew;

        let bid_price = (base as f64 - spread_raw as f64 / 2.0 * (1.0 + skew)).round() as Price;
        let ask_price = (base as f64 + spread_raw as f64 / 2.0 * (1.0 - skew)).round() as Price;

        let mut actions = Vec::new();

        // 取消旧订单
        if let Some(bid_id) = self.bid_order_id.take() {
            actions.push(AgentAction::SendMessage {
                target: "exchange".into(),
                body: MessageBody::CancelOrder(bid_id),
            });
        }
        if let Some(ask_id) = self.ask_order_id.take() {
            actions.push(AgentAction::SendMessage {
                target: "exchange".into(),
                body: MessageBody::CancelOrder(ask_id),
            });
        }

        let bid_order = LimitOrder {
            id: self.gen_id(),
            side: OrderSide::Buy,
            price: bid_price,
            quantity: self.quote_size,
            filled_quantity: 0,
            timestamp: ctx.current_time,
            agent_id: self.id.clone(),
        };
        self.bid_order_id = Some(bid_order.id);
        actions.push(AgentAction::SendMessage {
            target: "exchange".into(),
            body: MessageBody::SubmitLimit(bid_order),
        });

        let ask_order = LimitOrder {
            id: self.gen_id(),
            side: OrderSide::Sell,
            price: ask_price,
            quantity: self.quote_size,
            filled_quantity: 0,
            timestamp: ctx.current_time,
            agent_id: self.id.clone(),
        };
        self.ask_order_id = Some(ask_order.id);
        actions.push(AgentAction::SendMessage {
            target: "exchange".into(),
            body: MessageBody::SubmitLimit(ask_order),
        });

        actions
    }
}

impl SimAgent for MarketMakerAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "MarketMaker"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::MarketMaker
    }

    fn on_init(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        // 首次唤醒报价
        let mut actions = self.submit_quotes(ctx);
        actions.push(AgentAction::WakeupAfter(self.wakeup_interval_ns));
        actions
    }

    fn on_message(&mut self, msg: &MessageBody, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            MessageBody::OrderFilled { fill, .. } => {
                self.trade_count += 1;
                for trade in &fill.trades {
                    if trade.buyer_agent_id == self.id {
                        self.net_position += trade.quantity as i64;
                        self.last_mid = Some(trade.price as f64);
                        self.reference_price = trade.price;
                    } else if trade.seller_agent_id == self.id {
                        self.net_position -= trade.quantity as i64;
                        self.last_mid = Some(trade.price as f64);
                        self.reference_price = trade.price;
                    }
                }
            },
            // 修复 P0-6: 处理 OrderPlaced 消息，用 ExchangeAgent 分配的真实 order_id
            // 覆盖 submit_quotes 中 gen_id() 生成的临时 ID。
            //
            // 原因：OrderBook::place_order_internal 调用 next_order_id() 分配新 ID
            // 并覆盖传入的 order.id，导致 MM 记录的 bid_order_id / ask_order_id
            // 是无效 ID，撤单时 ExchangeAgent 返回 OrderNotFound，旧挂单永不被撤销。
            // 现在 P0-5 修复后 MM 能收到 OrderPlaced 通知，此处更新记录的 ID。
            MessageBody::OrderPlaced { order_id, side } => match side {
                OrderSide::Buy => {
                    self.bid_order_id = Some(*order_id);
                },
                OrderSide::Sell => {
                    self.ask_order_id = Some(*order_id);
                },
            },
            _ => {},
        }
        Vec::new()
    }

    fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let mut actions = self.submit_quotes(ctx);
        actions.push(AgentAction::WakeupAfter(self.wakeup_interval_ns));
        actions
    }
}
