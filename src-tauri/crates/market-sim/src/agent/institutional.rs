//! 机构投资者 Agent —— TWAP/VWAP 拆单执行，模拟大单对市场的影响。
//!
//! Institutional 交易者将大额订单拆分为多个小单，在指定时间窗口内均匀执行。
//! 与 MarketMaker/Momentum/Noise 不同，Institutional 的核心特征是：
//!
//! - 大资金量（min_order_size >> Noise）
//! - 时间加权执行（TWAP），避免单笔冲击市场
//! - 成交价跟踪基准价（不追涨杀跌）

use rand::{Rng, SeedableRng};

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::*;

pub struct InstitutionalAgent {
    id: String,
    /// 参考价
    reference_price: Price,
    /// 总资金（分）
    total_capital: i64,
    /// 已使用资金
    used_capital: i64,
    /// 单次最大下单股数
    max_order_size: Quantity,
    /// 剩余待执行订单数（拆分计数）
    remaining_slices: u32,
    /// 每 slice 间隔（ns）
    slice_interval_ns: SimTimestamp,
    /// 自增 ID
    next_id: u64,
    /// 修复 H3.6: 可复现 RNG（替代 thread_rng）
    rng: rand::rngs::StdRng,
}

impl InstitutionalAgent {
    /// 创建机构投资者 Agent
    ///
    /// - `id`: Agent ID
    /// - `reference_price`: 参考价（分）
    /// - `total_capital`: 总资金（分）
    /// - `slices`: 拆单份数（默认 10）
    /// - `slice_interval_ns`: 每份间隔（默认 200ms）
    /// - `seed`: 随机种子（保证仿真可复现）
    pub fn new(
        id: impl Into<String>,
        reference_price: Price,
        total_capital: i64,
        slices: u32,
        slice_interval_ns: SimTimestamp,
        seed: u64,
    ) -> Self {
        Self {
            id: id.into(),
            reference_price,
            total_capital,
            used_capital: 0,
            max_order_size: (total_capital / slices.max(1) as i64 / reference_price.max(1)).max(100)
                as Quantity,
            remaining_slices: slices,
            slice_interval_ns,
            next_id: 1,
            // 修复 H3.6: 用种子初始化 RNG，保证仿真可复现
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    fn gen_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl SimAgent for InstitutionalAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Institutional"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Institutional
    }

    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        vec![
            AgentAction::SendMessage { target: "exchange".into(), body: MessageBody::RequestQuote },
            AgentAction::WakeupAfter(self.slice_interval_ns),
        ]
    }

    fn on_message(&mut self, msg: &MessageBody, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            MessageBody::OrderFilled { fill, .. } => {
                // 修复 C4.6: 在 OrderFilled 中根据实际成交更新 used_capital
                // （原实现在 SendMessage 后立即按挂单价 used_capital +=，与实际成交价不一致）
                for trade in &fill.trades {
                    if trade.buyer_agent_id == self.id {
                        self.used_capital += trade.quantity as i64 * trade.price;
                    }
                }
                Vec::new()
            },
            MessageBody::QuoteReply(_snapshot) => {
                // 获取市场深度，调整限价单价格
                Vec::new()
            },
            MessageBody::OrderPlaced { .. } => Vec::new(),
            MessageBody::OrderCancelled { .. } => Vec::new(),
            _ => Vec::new(),
        }
    }

    fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let mut actions = Vec::new();

        if self.remaining_slices > 0 && self.used_capital < self.total_capital {
            let slice_size = self.max_order_size;

            // TWAP 执行：按当前价格提交限价买入单
            // 使用参考价 ± 随机偏移模拟对市场价格的适应
            // 修复 H3.6: 使用结构体内的可复现 RNG 替代 thread_rng
            let rng = &mut self.rng;
            let offset = rng.r#gen_range(-5..=5);
            let limit_price = (self.reference_price + offset).max(1);

            let order = LimitOrder {
                id: self.gen_id(),
                side: OrderSide::Buy,
                price: limit_price,
                quantity: slice_size,
                filled_quantity: 0,
                agent_id: self.id.clone(),
                timestamp: ctx.current_time,
            };
            actions.push(AgentAction::SendMessage {
                target: "exchange".into(),
                body: MessageBody::SubmitLimit(order),
            });

            // 修复 C4.6: 不在此处更新 used_capital；改为在 OrderFilled 中根据实际成交价更新
            self.remaining_slices -= 1;
        }

        // 持续监控
        actions.push(AgentAction::SendMessage {
            target: "exchange".into(),
            body: MessageBody::RequestQuote,
        });
        if self.remaining_slices > 0 {
            actions.push(AgentAction::WakeupAfter(self.slice_interval_ns));
        }

        actions
    }

    fn on_sim_end(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::*;
    use crate::config::*;
    use crate::kernel::*;

    #[test]
    fn test_institutional_submits_orders() {
        let price = 1000;
        let mut kernel = SimKernel::new(SimConfig {
            max_time_ns: 2_000_000_000,
            default_latency_ns: 1_000,
            reference_price: price,
            ..Default::default()
        });
        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
        kernel.register(Box::new(InstitutionalAgent::new(
            "inst",
            price,
            1_000_000,
            5,
            200_000_000,
            42,
        )));

        let result = kernel.run().unwrap();
        assert!(result.total_events > 10, "events={}", result.total_events);
        eprintln!("Institutional: events={}", result.total_events);
    }
}
