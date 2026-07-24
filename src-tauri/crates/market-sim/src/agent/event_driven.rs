//! 事件驱动 Agent（P2-C9）— 基于外部事件（新闻/公告/财报）触发交易。
//!
//! ## 行为
//!
//! 收到 `MessageBody::ExternalEvent` 时：
//! 1. 计算 `signal_strength = |sentiment| * impact`
//! 2. 若 `signal_strength > signal_threshold` 且冷却期已过：
//!    - `sentiment > 0` → 以市价买入（利好）
//!    - `sentiment < 0` → 以市价卖出（利空）
//! 3. 持仓达 `position_limit` 时停止加仓
//!
//! ## 与其他 Agent 的区别
//!
//! - `MomentumAgent`：基于价格动量交易（技术面）
//! - `ValueAgent`：基于估值交易（基本面）
//! - `EventDrivenAgent`：基于外部事件交易（消息面）
//!
//! ## 参数
//!
//! | 参数 | 典型值 | 说明 |
//! |------|--------|------|
//! | signal_threshold | 0.1-0.3 | 信号触发阈值 |
//! | order_size | 100-500 | 单次交易数量 |
//! | position_limit | 2000-5000 | 持仓上限 |
//! | cooldown_ns | 1_000_000 (1ms) | 冷却期（防止过度反应） |

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::events::{ExternalEvent, ExternalEventKind};
use crate::types::{MarketOrder, OrderSide, Quantity, SimTimestamp};

/// 事件驱动 Agent
pub struct EventDrivenAgent {
    id: String,
    /// 信号触发阈值（signal_strength > threshold 才交易）
    signal_threshold: f64,
    /// 单次交易数量
    order_size: Quantity,
    /// 持仓上限（正数，双向限制）
    position_limit: i64,
    /// 冷却期（ns，同一方向两次交易的最小间隔）
    cooldown_ns: SimTimestamp,
    /// 当前持仓（正 = 多，负 = 空）
    position: i64,
    /// 上次买入时间（用于冷却控制）
    last_buy_time: SimTimestamp,
    /// 上次卖出时间（用于冷却控制）
    last_sell_time: SimTimestamp,
    /// 订阅的事件类型（空 = 订阅所有类型）
    subscribed_kinds: Vec<ExternalEventKind>,
    /// 关联股票代码（只处理匹配此代码的事件，None = 处理所有事件）
    stock_code: Option<String>,
    /// 自增订单 ID
    next_order_id: u64,
    /// 收到的事件计数（统计用）
    events_received: u64,
    /// 触发交易的事件计数（统计用）
    events_triggered: u64,
}

impl EventDrivenAgent {
    /// 创建事件驱动 Agent
    ///
    /// - `signal_threshold`: 信号触发阈值，推荐 0.1-0.3
    /// - `order_size`: 单次交易数量（A 股最小 100 股）
    /// - `position_limit`: 持仓上限（正数，双向限制）
    /// - `cooldown_ns`: 冷却期（ns）
    /// - `subscribed_kinds`: 订阅的事件类型，空 Vec = 订阅所有
    /// - `stock_code`: 关联股票代码，None = 处理所有股票的事件
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        signal_threshold: f64,
        order_size: Quantity,
        position_limit: i64,
        cooldown_ns: SimTimestamp,
        subscribed_kinds: Vec<ExternalEventKind>,
        stock_code: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            signal_threshold,
            order_size,
            position_limit,
            cooldown_ns,
            position: 0,
            last_buy_time: 0,
            last_sell_time: 0,
            subscribed_kinds,
            stock_code,
            next_order_id: 1,
            events_received: 0,
            events_triggered: 0,
        }
    }

    /// 创建订阅所有事件类型的 Agent（便捷构造）
    pub fn new_all_events(
        id: impl Into<String>,
        signal_threshold: f64,
        order_size: Quantity,
        position_limit: i64,
        cooldown_ns: SimTimestamp,
        stock_code: Option<String>,
    ) -> Self {
        Self::new(
            id,
            signal_threshold,
            order_size,
            position_limit,
            cooldown_ns,
            vec![], // 空 = 订阅所有
            stock_code,
        )
    }

    fn gen_order_id(&mut self) -> u64 {
        let id = self.next_order_id;
        self.next_order_id += 1;
        id
    }

    /// 判断是否订阅了该事件类型
    fn is_subscribed(&self, kind: &ExternalEventKind) -> bool {
        self.subscribed_kinds.is_empty() || self.subscribed_kinds.contains(kind)
    }

    /// 判断事件是否匹配关联股票
    fn matches_stock(&self, event: &ExternalEvent) -> bool {
        match (&self.stock_code, &event.stock_code) {
            // Agent 未指定股票 → 处理所有事件
            (None, _) => true,
            // Agent 指定了股票，事件是市场级（None）→ 处理
            (Some(_), None) => true,
            // Agent 指定了股票，事件也指定了股票 → 必须匹配
            (Some(agent_code), Some(event_code)) => agent_code == event_code,
        }
    }

    /// 判断冷却期是否已过
    fn cooldown_passed(&self, current_time: SimTimestamp, is_buy: bool) -> bool {
        let last_time = if is_buy {
            self.last_buy_time
        } else {
            self.last_sell_time
        };
        current_time >= last_time + self.cooldown_ns
    }

    /// 处理外部事件，返回交易动作
    fn handle_event(&mut self, event: &ExternalEvent, ctx: &mut AgentContext) -> Vec<AgentAction> {
        self.events_received += 1;

        // 1. 检查订阅
        if !self.is_subscribed(&event.kind) {
            return Vec::new();
        }

        // 2. 检查股票匹配
        if !self.matches_stock(event) {
            return Vec::new();
        }

        // 3. 计算信号强度
        let signal = event.signal_strength();
        if signal <= self.signal_threshold {
            return Vec::new();
        }

        // 4. 根据情感方向决定交易
        let mut actions = Vec::new();

        if event.is_positive() && self.position < self.position_limit {
            // 利好 → 买入（需冷却期已过）
            if self.cooldown_passed(ctx.current_time, true) {
                let order = MarketOrder {
                    id: self.gen_order_id(),
                    side: OrderSide::Buy,
                    quantity: self.order_size,
                    agent_id: self.id.clone(),
                    timestamp: ctx.current_time,
                };
                actions.push(AgentAction::SendMessage {
                    target: "exchange".into(),
                    body: MessageBody::SubmitMarket(order),
                });
                self.last_buy_time = ctx.current_time;
                self.events_triggered += 1;
            }
        } else if event.is_negative() && self.position > -self.position_limit {
            // 利空 → 卖出（需冷却期已过）
            if self.cooldown_passed(ctx.current_time, false) {
                let order = MarketOrder {
                    id: self.gen_order_id(),
                    side: OrderSide::Sell,
                    quantity: self.order_size,
                    agent_id: self.id.clone(),
                    timestamp: ctx.current_time,
                };
                actions.push(AgentAction::SendMessage {
                    target: "exchange".into(),
                    body: MessageBody::SubmitMarket(order),
                });
                self.last_sell_time = ctx.current_time;
                self.events_triggered += 1;
            }
        }

        actions
    }
}

impl SimAgent for EventDrivenAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "EventDriven"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::EventDriven
    }

    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        // 事件驱动 Agent 无需主动唤醒——等待 Kernel 注入的外部事件
        // 但仍需请求初始报价以感知市场状态
        vec![AgentAction::SendMessage {
            target: "exchange".into(),
            body: MessageBody::RequestQuote,
        }]
    }

    fn on_message(&mut self, msg: &MessageBody, ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            MessageBody::ExternalEvent(event) => self.handle_event(event, ctx),
            MessageBody::OrderFilled { fill, .. } => {
                // 根据实际成交更新持仓
                for trade in &fill.trades {
                    if trade.buyer_agent_id == self.id {
                        self.position += trade.quantity as i64;
                    } else if trade.seller_agent_id == self.id {
                        self.position -= trade.quantity as i64;
                    }
                }
                Vec::new()
            },
            MessageBody::QuoteReply(_) => {
                // 仅更新市场感知，不触发交易
                Vec::new()
            },
            _ => Vec::new(),
        }
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::traits::AgentContext;
    use crate::types::FillResult;

    fn make_ctx(agent_id: &str, time: SimTimestamp) -> AgentContext {
        AgentContext::new(time, "000001".to_string(), 1000, agent_id.to_string(), None)
    }

    fn make_event(sentiment: f64, impact: f64, stock_code: Option<String>) -> ExternalEvent {
        ExternalEvent::news("测试事件", "测试摘要", sentiment, impact, 0, stock_code)
    }

    #[test]
    fn test_positive_event_triggers_buy() {
        let mut agent = EventDrivenAgent::new_all_events(
            "ed1",
            0.1,
            100,
            1000,
            0, // 无冷却
            Some("000001".to_string()),
        );
        let mut ctx = make_ctx("ed1", 1_000_000);

        // sentiment=0.8, impact=0.7 → signal=0.56 > 0.1 → 买入
        let event = make_event(0.8, 0.7, Some("000001".to_string()));
        let actions = agent.on_message(&MessageBody::ExternalEvent(event), &mut ctx);

        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            AgentAction::SendMessage { body: MessageBody::SubmitMarket(_), .. }
        ));
        assert_eq!(agent.events_received, 1);
        assert_eq!(agent.events_triggered, 1);
    }

    #[test]
    fn test_negative_event_triggers_sell() {
        let mut agent =
            EventDrivenAgent::new_all_events("ed2", 0.1, 100, 1000, 0, Some("000001".to_string()));
        let mut ctx = make_ctx("ed2", 1_000_000);

        // 先建立多头持仓（模拟已买入）
        agent.position = 200;

        // sentiment=-0.6, impact=0.5 → signal=0.30 > 0.1 → 卖出
        let event = make_event(-0.6, 0.5, Some("000001".to_string()));
        let actions = agent.on_message(&MessageBody::ExternalEvent(event), &mut ctx);

        assert_eq!(actions.len(), 1);
        if let AgentAction::SendMessage { body: MessageBody::SubmitMarket(order), .. } = &actions[0]
        {
            assert_eq!(order.side, OrderSide::Sell);
        } else {
            panic!("期望 SubmitMarket(Sell)");
        }
    }

    #[test]
    fn test_low_signal_no_trade() {
        let mut agent = EventDrivenAgent::new_all_events(
            "ed3", 0.3, // 较高阈值
            100, 1000, 0, None,
        );
        let mut ctx = make_ctx("ed3", 1_000_000);

        // signal = 0.5 * 0.4 = 0.20 < 0.3 → 不交易
        let event = make_event(0.5, 0.4, None);
        let actions = agent.on_message(&MessageBody::ExternalEvent(event), &mut ctx);

        assert_eq!(actions.len(), 0);
        assert_eq!(agent.events_triggered, 0);
    }

    #[test]
    fn test_cooldown_blocks_rapid_trades() {
        let mut agent = EventDrivenAgent::new_all_events(
            "ed4", 0.1, 100, 1000, 1_000_000, // 1ms 冷却
            None,
        );

        // 第一次事件 → 买入
        let mut ctx1 = make_ctx("ed4", 1_000_000);
        let event1 = make_event(0.8, 0.7, None);
        let actions1 = agent.on_message(&MessageBody::ExternalEvent(event1), &mut ctx1);
        assert_eq!(actions1.len(), 1);

        // 冷却期内第二次事件 → 不交易
        let mut ctx2 = make_ctx("ed4", 1_500_000); // 0.5ms 后，仍在冷却期
        let event2 = make_event(0.8, 0.7, None);
        let actions2 = agent.on_message(&MessageBody::ExternalEvent(event2), &mut ctx2);
        assert_eq!(actions2.len(), 0);

        // 冷却期后第三次事件 → 买入
        let mut ctx3 = make_ctx("ed4", 2_100_000); // 1.1ms 后，冷却期已过
        let event3 = make_event(0.8, 0.7, None);
        let actions3 = agent.on_message(&MessageBody::ExternalEvent(event3), &mut ctx3);
        assert_eq!(actions3.len(), 1);
    }

    #[test]
    fn test_position_limit_blocks_buy() {
        let mut agent = EventDrivenAgent::new_all_events(
            "ed5", 0.1, 100, 500, // 持仓上限 500
            0, None,
        );
        // 已达持仓上限
        agent.position = 500;

        let mut ctx = make_ctx("ed5", 1_000_000);
        let event = make_event(0.8, 0.7, None);
        let actions = agent.on_message(&MessageBody::ExternalEvent(event), &mut ctx);

        assert_eq!(actions.len(), 0);
    }

    #[test]
    fn test_stock_code_filter() {
        let mut agent = EventDrivenAgent::new_all_events(
            "ed6",
            0.1,
            100,
            1000,
            0,
            Some("000001".to_string()), // 只处理 000001 的事件
        );

        // 不匹配的股票代码 → 不交易
        let mut ctx1 = make_ctx("ed6", 1_000_000);
        let event1 = make_event(0.8, 0.7, Some("000002".to_string()));
        let actions1 = agent.on_message(&MessageBody::ExternalEvent(event1), &mut ctx1);
        assert_eq!(actions1.len(), 0);

        // 匹配的股票代码 → 交易
        let mut ctx2 = make_ctx("ed6", 1_000_000);
        let event2 = make_event(0.8, 0.7, Some("000001".to_string()));
        let actions2 = agent.on_message(&MessageBody::ExternalEvent(event2), &mut ctx2);
        assert_eq!(actions2.len(), 1);

        // 市场级事件（stock_code=None）→ 交易（Agent 指定股票也应处理市场级事件）
        let mut ctx3 = make_ctx("ed6", 1_000_000);
        let event3 = make_event(0.8, 0.7, None);
        let actions3 = agent.on_message(&MessageBody::ExternalEvent(event3), &mut ctx3);
        assert_eq!(actions3.len(), 1);
    }

    #[test]
    fn test_subscribed_kinds_filter() {
        let mut agent = EventDrivenAgent::new(
            "ed7",
            0.1,
            100,
            1000,
            0,
            vec![ExternalEventKind::Earnings], // 只订阅财报事件
            None,
        );

        // 新闻事件 → 不处理
        let mut ctx1 = make_ctx("ed7", 1_000_000);
        let event1 = ExternalEvent::news("新闻", "", 0.8, 0.7, 0, None);
        let actions1 = agent.on_message(&MessageBody::ExternalEvent(event1), &mut ctx1);
        assert_eq!(actions1.len(), 0);

        // 财报事件 → 处理
        let mut ctx2 = make_ctx("ed7", 1_000_000);
        let event2 = ExternalEvent::earnings("财报", "", 0.8, 0.7, 0, None);
        let actions2 = agent.on_message(&MessageBody::ExternalEvent(event2), &mut ctx2);
        assert_eq!(actions2.len(), 1);
    }

    #[test]
    fn test_order_filled_updates_position() {
        let mut agent = EventDrivenAgent::new_all_events("ed8", 0.1, 100, 1000, 0, None);
        assert_eq!(agent.position, 0);

        // 模拟成交回调（FillResult 真实字段见 types.rs）
        let fill = FillResult {
            trades: vec![crate::types::TradeRecord {
                price: 1000,
                quantity: 100,
                buyer_agent_id: "ed8".to_string(),
                seller_agent_id: "exchange".to_string(),
                buyer_order_id: 1,
                seller_order_id: 0,
                timestamp: 1_000_000,
            }],
            vwap: 1000.0,
            filled_quantity: 100,
            unfilled_quantity: 0,
            market_impact_bps: 0.0,
            levels_consumed: 1,
        };

        let mut ctx = make_ctx("ed8", 1_000_000);
        let _ = agent.on_message(&MessageBody::OrderFilled { order_id: 1, fill }, &mut ctx);

        assert_eq!(agent.position, 100);
    }

    #[test]
    fn test_neutral_event_no_trade() {
        let mut agent = EventDrivenAgent::new_all_events("ed9", 0.0, 100, 1000, 0, None);
        let mut ctx = make_ctx("ed9", 1_000_000);

        // sentiment=0.0 → is_positive=false, is_negative=false → 不交易
        let event = make_event(0.0, 0.5, None);
        let actions = agent.on_message(&MessageBody::ExternalEvent(event), &mut ctx);

        assert_eq!(actions.len(), 0);
    }
}
