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

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::{LimitOrder, MarketOrder, OrderSide};

/// Rhai 脚本 Agent
///
/// 修复 P0-M6: 原实现每次 on_wakeup/on_message 都新建 Engine + 重新编译脚本，
/// 且无沙箱限制（`while true {}` 可吃满 CPU）。改为：
/// 1. 缓存编译后的 AST（避免重复编译）
/// 2. 设置 max_operations 限制死循环
/// 3. 仅在脚本显式返回 `request_quote` 时才自动续约 WakeupAfter
pub struct RhaiAgent {
    id: String,
    script: String,
    next_id: u64,
    /// 修复 P0-M6: 缓存编译后的 AST，避免每次事件都重新编译
    cached_ast: Option<rhai::AST>,
    /// 修复 P0-M6: 共享 Engine（只读，线程安全）
    engine: rhai::Engine,
}

impl RhaiAgent {
    pub fn new(id: impl Into<String>, script: impl Into<String>) -> Self {
        let mut engine = rhai::Engine::new();
        // 修复 P0-M6: 设置操作数上限防止死循环（10 万次足够正常策略逻辑）
        engine.set_max_operations(100_000);
        // 限制字符串/数组大小防止内存炸弹
        engine.set_max_string_size(1_000_000); // 1MB
        engine.set_max_array_size(10_000);
        engine.set_max_map_size(10_000);
        // 修复 M-DEF-3: 设置最大调用栈深度，防止递归策略脚本栈溢出
        engine.set_max_call_levels(64);
        // P1-3: 复用 harness 通用 Rhai 函数（clamp/join/json_parse）
        axagent_harness::register_common_functions(&mut engine);

        Self { id: id.into(), script: script.into(), next_id: 1, cached_ast: None, engine }
    }

    fn gen_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn call_script(&mut self, event_type: &str, ctx: &AgentContext) -> Vec<AgentAction> {
        // 修复 C4 编译错误: 原 get_ast(&mut self) 返回 &AST 持有可变借用，
        // 随后 self.engine.call_fn 又需不可变借用 → 借用冲突。
        // 改为内联编译逻辑，让 &mut self.engine.compile 的借用在本块内结束。
        if self.cached_ast.is_none() {
            match self.engine.compile(&self.script) {
                Ok(ast) => self.cached_ast = Some(ast),
                Err(e) => {
                    tracing::warn!("RhaiAgent[{}]: 脚本编译失败: {}", self.id, e);
                    return Vec::new();
                },
            }
        }

        // 此时 self.cached_ast 已填充，与 self.engine 同时为不可变借用，可共存
        let ast = match self.cached_ast.as_ref() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // 用预编译 AST + 动态参数调用 on_event
        let result: Result<rhai::Dynamic, _> = self.engine.call_fn(
            &mut rhai::Scope::new(),
            ast,
            "on_event",
            (event_type.to_string(),),
        );

        let decisions: Vec<rhai::Dynamic> = match result {
            Ok(d) => d.try_cast().unwrap_or_default(),
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
                        let side =
                            match map.get("side").and_then(|v| v.clone().try_cast::<String>()) {
                                Some(s) if s == "buy" => OrderSide::Buy,
                                Some(s) if s == "sell" => OrderSide::Sell,
                                _ => continue,
                            };
                        let qty =
                            match map.get("quantity").and_then(|v| v.clone().try_cast::<i64>()) {
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
                        let side =
                            match map.get("side").and_then(|v| v.clone().try_cast::<String>()) {
                                Some(s) if s == "buy" => OrderSide::Buy,
                                Some(s) if s == "sell" => OrderSide::Sell,
                                _ => continue,
                            };
                        let price = match map.get("price").and_then(|v| v.clone().try_cast::<i64>())
                        {
                            Some(p) => p,
                            None => continue,
                        };
                        let qty =
                            match map.get("quantity").and_then(|v| v.clone().try_cast::<i64>()) {
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
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Rhai"
    }
    fn agent_type(&self) -> AgentType {
        AgentType::Rhai
    }

    fn on_init(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        // 修复 C4.1 引入的测试失败: P0-M6 将 on_wakeup 改为有条件续约后，
        // on_init 也需要追加 WakeupAfter 启动事件循环，否则 RhaiAgent 永远不会被唤醒。
        // 与 on_wakeup 保持一致：仅在脚本返回至少一个 action 时续约。
        let actions = self.call_script("init", _ctx);
        if !actions.is_empty() {
            let mut all = actions;
            all.push(AgentAction::WakeupAfter(100_000_000)); // 100ms 后唤醒
            all
        } else {
            actions
        }
    }

    fn on_message(&mut self, _msg: &MessageBody, ctx: &mut AgentContext) -> Vec<AgentAction> {
        self.call_script("message", ctx)
    }

    fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let actions = self.call_script("wakeup", ctx);
        // 修复 P0-M6: 原实现无条件追加 WakeupAfter(1_000_000)（1ms），
        // 即使脚本什么都不做也会每 1ms 唤醒一次，在长仿真中产生 O(10^6) 事件。
        // 改为：仅在脚本返回了至少一个 action 时续约，且用 100ms 而非 1ms。
        if !actions.is_empty() {
            let mut all = actions;
            all.push(AgentAction::WakeupAfter(100_000_000)); // 100ms 后唤醒
            all
        } else {
            actions
        }
    }

    fn on_sim_end(&mut self, _ctx: &mut AgentContext) -> Vec<AgentAction> {
        Vec::new()
    }
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
        kernel.register(Box::new(RhaiAgent::new(
            "rhai",
            r#"
            fn on_event(event_type) {
                if event_type == "init" || event_type == "wakeup" {
                    [#{ "action": "request_quote" }]
                } else {
                    []
                }
            }
        "#
            .to_string(),
        )));

        let result = kernel.run().unwrap();
        assert!(result.total_events > 5);
        eprintln!("RhaiAgent: events={}", result.total_events);
    }
}
