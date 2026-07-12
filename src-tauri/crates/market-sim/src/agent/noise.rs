//! 噪声 Agent — 以随机频率提交随机小订单，模拟散户行为。
//!
//! ## 行为
//!
//! 每次唤醒时，以概率 `p(act)` 决定是否交易。如果决定交易：
//! - 随机选择方向（买/卖，各 50%）
//! - 随机选择订单量（1 ~ max_qty）
//! - 随机选择订单类型（限价 70% / 市价 30%）
//! - 限价单价格：在参考价 ± random_bps 范围内
//!
//! 唤醒间隔服从指数分布，平均 `avg_interval_ns`。

use rand::{Rng, SeedableRng};

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::{OrderSide, *};

pub struct NoiseAgent {
    id: String,
    /// 平均唤醒间隔（ns）
    avg_interval_ns: SimTimestamp,
    /// 每次唤醒时下单的概率
    act_probability: f64,
    /// 最大单次订单量
    max_order_qty: Quantity,
    /// 限价单价格偏移范围（基点）
    price_noise_bps: i64,
    /// 参考价
    reference_price: Price,
    /// 自增 ID
    next_id: u64,
    /// 修复 H3.6: 可复现 RNG（替代 thread_rng）
    rng: rand::rngs::StdRng,
}

impl NoiseAgent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        avg_interval_ns: SimTimestamp,
        act_probability: f64,
        max_order_qty: Quantity,
        price_noise_bps: i64,
        reference_price: Price,
        seed: u64,
    ) -> Self {
        Self {
            id: id.into(),
            avg_interval_ns,
            act_probability,
            max_order_qty,
            price_noise_bps,
            reference_price,
            next_id: 1,
            // 修复 H3.6: 用种子初始化 RNG，保证仿真可复现
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }
}

impl SimAgent for NoiseAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Noise"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Noise
    }

    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        vec![AgentAction::WakeupAfter(self.avg_interval_ns)]
    }

    fn on_message(&mut self, msg: &MessageBody, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        if let MessageBody::OrderFilled { .. } = msg {
            // 记录成交（无实际反应）
        }
        Vec::new()
    }

    fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        // 修复 H3.6: 使用结构体内的可复现 RNG 替代 thread_rng
        let rng = &mut self.rng;
        let mut actions: Vec<AgentAction> = vec![
            // 下次唤醒（指数分布）
            AgentAction::WakeupAfter(
                self.avg_interval_ns + rng.r#gen_range(0..self.avg_interval_ns) / 2,
            ),
        ];

        // 决定是否下单
        if rng.r#gen::<f64>() >= self.act_probability {
            return actions;
        }

        let side = if rng.r#gen_bool(0.5) {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };

        let qty = rng.r#gen_range(1..=self.max_order_qty);

        // 70% 限价单 + 30% 市价单
        if rng.r#gen_bool(0.7) {
            let max_noise = (self.reference_price as f64 * self.price_noise_bps as f64 / 10000.0)
                .round() as Price;
            // 修复 H3.7: price_noise_bps < 0 时 max_noise < 0，
            // gen_range(-max_noise..=max_noise) 会因 range 起止颠倒而 panic
            let offset = if max_noise > 0 {
                rng.r#gen_range(-max_noise..=max_noise)
            } else {
                0
            };
            let price = (self.reference_price as f64 + offset as f64).round().max(1.0) as Price;

            let id = self.next_id;
            self.next_id += 1;

            let order = LimitOrder {
                id,
                side,
                price,
                quantity: qty,
                filled_quantity: 0,
                timestamp: ctx.current_time,
                agent_id: self.id.clone(),
            };
            actions.push(AgentAction::SendMessage {
                target: "exchange".into(),
                body: MessageBody::SubmitLimit(order),
            });
        } else {
            let order = MarketOrder {
                id: 0,
                side,
                quantity: qty,
                agent_id: self.id.clone(),
                timestamp: ctx.current_time,
            };
            actions.push(AgentAction::SendMessage {
                target: "exchange".into(),
                body: MessageBody::SubmitMarket(order),
            });
        }

        actions
    }
}
