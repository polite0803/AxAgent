//! Rhai 脚本定义的 Agent —— 将 Rhai 脚本作为 Agent 行为逻辑接入 DES。
//!
//! 允许用户在运行时编写/修改 Rhai 脚本控制 Agent 的交易行为。
//! 脚本应定义 `on_event(event_type, data)` 函数，返回决策数组。
//!
//! ## 脚本接口
//!
//! ```rhai
//! fn on_event(event_type, data) {
//!     if event_type == "wakeup" {
//!         [#{ "action": "request_quote" }]
//!     } else {
//!         []
//!     }
//! }
//! ```

use crate::agent::traits::{
    AgentAction, AgentContext, AgentType, MessageBody, SimAgent,
};
use crate::types::{LimitOrder, MarketOrder, OrderSide};

/// Rhai 脚本 Agent
pub struct RhaiAgent {
    id: String,
    script: String,
    next_id: u64,
}

impl RhaiAgent {
    pub fn new(id: impl Into<String>, script: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            script: script.into(),
            next_id: 1,
        }
    }

    fn gen_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn call_script(&mut self, event_type: &str, ctx: &AgentContext) -> Vec<AgentAction> {
        let engine = rhai::Engine::new();
        let full_script = format!(
            "{}\n\non_event(\"{}\")",
            self.script, event_type
        );

        let result: Result<Vec<rhai::Dynamic>, _> = engine.eval(&full_script);
        let decisions = match result {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("RhaiAgent[{}]: 脚本执行失败: {}", self.id, e);
                return Vec::new();
            },
        };

        let mut actions = Vec::new();
        for decision in decisions {
            if let Some(map) = decision.try_cast::<rhai::Map>() {
                let action = match map.get("action") {
                    Some(v) => match v.clone().try_cast::<String>() {
                        Some(s) => s,
                        None => continue,
                    },
                    None => continue,
                };

                match action.as_str() {
                    "submit_market" => {
                        let side = match map.get("side").and_then(|v| v.clone().try_cast::<String>()) {
                            Some(s) if s == "buy" => OrderSide::Buy,
                            Some(s) if s == "sell" => OrderSide::Sell,
                            _ => continue,
                        };
                        let qty = match map.get("quantity").and_then(|v| v.clone().try_cast::<i64>()) {
                            Some(q) => q as u64,
                            None => continue,
                        };
                        actions.push(AgentAction::SendMessage {
                            target: "exchange".into(),
                            body: MessageBody::SubmitMarket(MarketOrder {
                                id: self.gen_id(),
                                side,
                                quantity: qty,
                                agent_id: self.id.clone(),
                                timestamp: ctx.current_time,
                            }),
                        });
                    },
                    "submit_limit" => {
                        let side = match map.get("side").and_then(|v| v.clone().try_cast::<String>()) {
                            Some(s) if s == "buy" => OrderSide::Buy,
                            Some(s) if s == "sell" => OrderSide::Sell,
                            _ => continue,
                        };
                        let price = match map.get("price").and_then(|v| v.clone().try_cast::<i64>()) {
                            Some(p) => p,
                            None => continue,
                        };
                        let qty = match map.get("quantity").and_then(|v| v.clone().try_cast::<i64>()) {
                            Some(q) => q as u64,
                            None => continue,
                        };
                        actions.push(AgentAction::SendMessage {
                            target: "exchange".into(),
                            body: MessageBody::SubmitLimit(LimitOrder {
                                id: self.gen_id(),
                                side,
                                price,
                                quantity: qty,
                                filled_quantity: 0,
                                agent_id: self.id.clone(),
                                timestamp: ctx.current_time,
                            }),
                        });
                    },
                    "request_quote" => {
                        actions.push(AgentAction::SendMessage {
                            target: "exchange".into(),
                            body: MessageBody::RequestQuote,
                        });
                    },
                    _ => {},
                }
            }
        }
        actions
    }
}

impl SimAgent for RhaiAgent {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { "Rhai" }
    fn agent_type(&self) -> AgentType { AgentType::Rhai }

    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        self.call_script("init", _ctx)
    }

    fn on_message(&mut self, _msg: &MessageBody, ctx: &mut AgentContext) -> Vec<AgentAction> {
        self.call_script("message", ctx)
    }

    fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let mut actions = self.call_script("wakeup", ctx);
        actions.push(AgentAction::WakeupAfter(1_000_000)); // 1ms 后自动唤醒
        actions
    }

    fn on_sim_end(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> { Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::*;
    use crate::config::SimConfig;
    use crate::kernel::SimKernel;

    #[test]
    fn test_rhai_agent_basic() {
        let price = 1000;
        let mut kernel = SimKernel::new(SimConfig {
            max_time_ns: 200_000_000,
            default_latency_ns: 1_000,
            reference_price: price,
            ..Default::default()
        });
        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
        kernel.register(Box::new(RhaiAgent::new("rhai", r#"
            fn on_event(event_type) {
                if event_type == "init" || event_type == "wakeup" {
                    [#{ "action": "request_quote" }]
                } else {
                    []
                }
            }
        "#.to_string())));

        let result = kernel.run().unwrap();
        assert!(result.total_events > 5);
        eprintln!("RhaiAgent: events={}", result.total_events);
    }
}
