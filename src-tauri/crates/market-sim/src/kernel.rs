//! 离散事件模拟内核（DES Kernel）—— ABIDES Phase 2 核心组件。
//!
//! ## 执行模型
//!
//! 1. 初始化所有 Agent（调用 `on_init`）
//! 2. 主循环：从优先级队列弹事件 → 推进虚拟时钟 → 投递消息 → 收集动作 → 调度新事件
//! 3. 队列空或到达 `max_time_ns` 时停止
//! 4. 调用 `on_sim_end` 清理
//!
//! ## 事件优先级
//!
//! - 第一优先级：`scheduled_at`（模拟时间戳，越小越优先）
//! - 第二优先级：`priority`（同时间戳内的排序，0 = 最高优先）
//!
//! ## 延迟模型
//!
//! - Agent 间消息传递延迟通过 `LatencyMatrix` 配置
//! - 发送消息时：`deliver_at = current_time + latency(source, target)`
//! - 不建模计算延迟（Phase 3+ 可选）

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::agent::traits::{
    AgentAction, AgentContext, AgentMessage, AgentType, MessageBody, SimAgent,
};
use crate::config::{LatencyMatrix, SimConfig};
use crate::error::SimError;
use crate::types::*;

// ── 内部类型 ──

/// 模拟事件（优先级队列条目）
///
/// Reverse 包装实现最小堆——scheduled_at 最小的先出队。
#[derive(Debug, Clone)]
struct SimEvent {
    /// 投递时间（模拟时间戳）
    scheduled_at: SimTimestamp,
    /// 同时间戳内的优先级（越小越先）
    priority: u32,
    /// 消息体
    message: AgentMessage,
}

impl PartialEq for SimEvent {
    fn eq(&self, other: &Self) -> bool {
        self.scheduled_at == other.scheduled_at && self.priority == other.priority
    }
}

impl Eq for SimEvent {}

impl PartialOrd for SimEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 先按时间，再按优先级（都从小到大）
        self.scheduled_at.cmp(&other.scheduled_at).then(self.priority.cmp(&other.priority))
    }
}

/// 运行时 Agent 包装（持有 trait object + 元数据）
struct AgentEntry {
    agent: Box<dyn SimAgent>,
    agent_type: AgentType,
}

// ── 公共结构 ──

/// 模拟运行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimResult {
    /// 股票代码
    pub stock_code: String,
    /// 参考价格（分）
    pub reference_price: Price,
    /// 模拟持续的真实时间（ms）
    pub wall_clock_ms: u64,
    /// 模拟到达的虚拟时间（ns）
    pub sim_time_ns: SimTimestamp,
    /// 总处理事件数
    pub total_events: u64,
    /// 全部成交记录
    pub trades: Vec<TradeRecord>,
    /// 最终中间价（如有）
    pub final_mid_price: Option<f64>,
    /// 执行统计
    pub stats: SimStats,
}

/// 模拟执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimStats {
    pub total_orders: u64,
    pub total_trades: u64,
    pub total_volume: Quantity,
    pub total_messages: u64,
    pub max_queue_depth: usize,
    pub agent_count: usize,
}

/// 离散事件模拟内核
///
/// # 使用示例
///
/// ```rust,no_run
/// use axagent_market_sim::{SimKernel, SimConfig, ExchangeAgent};
///
/// let config = SimConfig::default();
/// let mut kernel = SimKernel::new(config);
///
/// // 注册 Agent
/// kernel.register(Box::new(ExchangeAgent::new("exchange")));
///
/// // 运行模拟
/// let result = kernel.run().unwrap();
/// println!("处理了 {} 个事件", result.total_events);
/// ```
pub struct SimKernel {
    config: SimConfig,
    /// 虚拟时钟（当前模拟时间，ns）
    clock: SimTimestamp,
    /// 事件优先级队列
    event_queue: BinaryHeap<Reverse<SimEvent>>,
    /// 已注册的 Agent
    agents: HashMap<String, AgentEntry>,
    /// Agent 间通信延迟
    latency: LatencyMatrix,
    /// 事件计数器
    event_count: u64,
    /// 队列深度峰值（用于统计）
    max_queue_depth: usize,
    /// 运行状态
    running: bool,
    /// 修复 P0-M3: 已 panic 毒化的 Agent 名单。后续消息直接丢弃，避免重入触发
    /// 二次 panic（std::sync Mutex 跨 panic 是 UB）。用 HashSet 而非 Vec 是因为
    /// O(1) 查找；规模 N<=几十个 Agent 内存可忽略。
    poisoned_agents: HashSet<String>,
}

impl SimKernel {
    /// 创建模拟内核
    pub fn new(config: SimConfig) -> Self {
        Self {
            config,
            clock: 0,
            event_queue: BinaryHeap::new(),
            agents: HashMap::new(),
            latency: LatencyMatrix::new(),
            event_count: 0,
            max_queue_depth: 0,
            running: false,
            poisoned_agents: HashSet::new(),
        }
    }

    /// 配置延迟矩阵
    pub fn with_latency(mut self, latency: LatencyMatrix) -> Self {
        self.latency = latency;
        self
    }

    /// 注册 Agent
    pub fn register(&mut self, agent: Box<dyn SimAgent>) {
        let id = agent.id().to_string();
        let agent_type = agent.agent_type();
        self.agents.insert(id, AgentEntry { agent, agent_type });
    }

    /// 获取 Agent 数量
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// 运行模拟（同步执行，返回结果）
    pub fn run(&mut self) -> Result<SimResult, SimError> {
        if self.agents.is_empty() {
            return Err(SimError::EmptyBook); // 复用现有错误：没有 Agent
        }

        let wall_start = std::time::Instant::now();

        // 1. 初始化所有 Agent
        // 修复 C4.3: 在 on_init 调用周围添加 catch_unwind，panic 时加入 poisoned_agents
        {
            let agent_ids: Vec<String> = self.agents.keys().cloned().collect();
            for agent_id in agent_ids {
                let mut ctx = self.make_ctx(&agent_id, None);
                let agent_id_for_panic = agent_id.clone();
                let actions = {
                    let entry = self.agents.get_mut(&agent_id).unwrap();
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        entry.agent.on_init(&mut ctx)
                    }))
                    .unwrap_or_else(|e| {
                        let panic_msg = if let Some(s) = e.downcast_ref::<&str>() {
                            (*s).to_string()
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        tracing::error!(
                            "[market-sim] Agent '{}' on_init panic, 已加入黑名单: {}",
                            agent_id_for_panic,
                            panic_msg
                        );
                        self.poisoned_agents.insert(agent_id_for_panic);
                        Vec::new()
                    })
                };
                // 已毒化的 Agent 跳过后续处理
                if self.poisoned_agents.contains(&agent_id) {
                    continue;
                }
                let mut all_actions = ctx.drain_actions();
                all_actions.extend(actions);
                let agent_id_for_process = agent_id.clone();
                // 修复 M-DEF-4: process_actions 返回 ()，不再需要 ?
                self.process_actions(&agent_id_for_process, &all_actions, 0);
            }
        }

        // 2. 主事件循环
        self.running = true;
        while self.running && !self.event_queue.is_empty() {
            self.max_queue_depth = self.max_queue_depth.max(self.event_queue.len());

            let Reverse(event) = self.event_queue.pop().unwrap();

            // 检查时间限制
            if self.config.max_time_ns > 0 && event.scheduled_at > self.config.max_time_ns {
                self.running = false;
                break;
            }

            // 推进虚拟时钟
            // 修复 M-DS-5: 添加倒流检查。事件队列按 scheduled_at 反向优先级（
            // Reverse 包装 BinaryHeap<MaxHeap>）弹出，正常情况下事件时间应单调
            // 不减。倒流意味着 schedule_message 写入了比当前更早的时间戳（如
            // on_sim_end 后再调度 on_init 的 bug），记录 warn 便于排查。
            if event.scheduled_at < self.clock {
                tracing::warn!(
                    "[market-sim] Kernel: 事件时间倒流 scheduled_at={} < clock={}, target={} from={}",
                    event.scheduled_at,
                    self.clock,
                    event.message.target,
                    event.message.source
                );
            }
            self.clock = event.scheduled_at;
            self.event_count += 1;

            // 投递消息到目标 Agent。Wakeup 事件路由到 on_wakeup，
            // 其他消息路由到 on_message。
            let target_id = event.message.target.clone();
            let body = event.message.body.clone();
            let has_agent = self.agents.contains_key(&target_id);
            if has_agent {
                let mut ctx = self.make_ctx(&target_id, Some(&event.message.source));
                let actions = {
                    let entry = self.agents.get_mut(&target_id).unwrap();
                    let agent_id_for_panic = target_id.clone();
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        // Wakeup 路由到 on_wakeup，其余消息路由到 on_message
                        if matches!(body, MessageBody::Wakeup) {
                            entry.agent.on_wakeup(&mut ctx)
                        } else {
                            entry.agent.on_message(&body, &mut ctx)
                        }
                    }))
                    .unwrap_or_else(|e| {
                        let panic_msg = if let Some(s) = e.downcast_ref::<&str>() {
                            (*s).to_string()
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        tracing::error!(
                            "[market-sim] Agent '{}' on_message panic, 已加入黑名单: {}",
                            agent_id_for_panic,
                            panic_msg
                        );
                        self.poisoned_agents.insert(agent_id_for_panic);
                        Vec::new()
                    })
                };

                // 已毒化的 Agent 不再处理消息
                if self.poisoned_agents.contains(&target_id) {
                    continue;
                }

                let mut all_actions = ctx.drain_actions();
                all_actions.extend(actions);

                self.process_actions(&target_id, &all_actions, self.clock);
            } else if self.config.trace {
                tracing::warn!(
                    "Kernel: message to unknown agent '{}' from '{}'",
                    target_id,
                    event.message.source
                );
            }
        }

        // 3. 通知所有 Agent 模拟结束
        // 修复 C4.3: 在 on_sim_end 调用周围添加 catch_unwind，panic 时加入 poisoned_agents
        {
            let agent_ids: Vec<String> = self.agents.keys().cloned().collect();
            for agent_id in agent_ids {
                // 已毒化的 Agent 不再调用 on_sim_end，避免二次 panic
                if self.poisoned_agents.contains(&agent_id) {
                    continue;
                }
                let mut ctx = self.make_ctx(&agent_id, None);
                let agent_id_for_panic = agent_id.clone();
                let actions = {
                    let entry = self.agents.get_mut(&agent_id).unwrap();
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        entry.agent.on_sim_end(&mut ctx)
                    }))
                    .unwrap_or_else(|e| {
                        let panic_msg = if let Some(s) = e.downcast_ref::<&str>() {
                            (*s).to_string()
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        tracing::error!(
                            "[market-sim] Agent '{}' on_sim_end panic, 已加入黑名单: {}",
                            agent_id_for_panic,
                            panic_msg
                        );
                        self.poisoned_agents.insert(agent_id_for_panic);
                        Vec::new()
                    })
                };
                let mut all_actions = ctx.drain_actions();
                all_actions.extend(actions);
                self.process_actions(&agent_id, &all_actions, self.clock);
            }
        }

        // 修复 M-DS-10: on_sim_end 中 Agent 调度的消息（如结算通知）被推入
        // event_queue，但主循环（步骤 2）已退出。需要在 collect_results 前再跑
        // 一轮事件处理循环，让这些"模拟收尾"消息被目标 Agent 接收。
        // 为避免 Agent 在收尾阶段无限循环发送消息，限制最大处理事件数为
        // 当前队列长度的快照（即只处理 on_sim_end 直接产生的那一批）。
        {
            let mut pending = self.event_queue.len();
            while pending > 0 && !self.event_queue.is_empty() {
                pending = pending.saturating_sub(1);
                self.max_queue_depth = self.max_queue_depth.max(self.event_queue.len());

                let Reverse(event) = self.event_queue.pop().unwrap();

                // 收尾阶段不再受 max_time_ns 限制（on_sim_end 调度的事件
                // 可能略晚于 max_time_ns，但仍是收尾流程的一部分）

                if event.scheduled_at < self.clock {
                    tracing::warn!(
                        "[market-sim] Kernel(收尾): 事件时间倒流 scheduled_at={} < clock={}, target={} from={}",
                        event.scheduled_at,
                        self.clock,
                        event.message.target,
                        event.message.source
                    );
                }
                self.clock = event.scheduled_at;
                self.event_count += 1;

                let target_id = event.message.target.clone();
                let body = event.message.body.clone();
                let has_agent = self.agents.contains_key(&target_id);
                if has_agent {
                    if self.poisoned_agents.contains(&target_id) {
                        continue;
                    }
                    let mut ctx = self.make_ctx(&target_id, Some(&event.message.source));
                    let actions = {
                        let entry = self.agents.get_mut(&target_id).unwrap();
                        let agent_id_for_panic = target_id.clone();
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            if matches!(body, MessageBody::Wakeup) {
                                entry.agent.on_wakeup(&mut ctx)
                            } else {
                                entry.agent.on_message(&body, &mut ctx)
                            }
                        }))
                        .unwrap_or_else(|e| {
                            let panic_msg = if let Some(s) = e.downcast_ref::<&str>() {
                                (*s).to_string()
                            } else if let Some(s) = e.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "unknown panic".to_string()
                            };
                            tracing::error!(
                                "[market-sim] Agent '{}' on_message(收尾) panic, 已加入黑名单: {}",
                                agent_id_for_panic,
                                panic_msg
                            );
                            self.poisoned_agents.insert(agent_id_for_panic);
                            Vec::new()
                        })
                    };
                    if self.poisoned_agents.contains(&target_id) {
                        continue;
                    }
                    let mut all_actions = ctx.drain_actions();
                    all_actions.extend(actions);
                    self.process_actions(&target_id, &all_actions, self.clock);
                } else if self.config.trace {
                    tracing::warn!(
                        "Kernel(收尾): message to unknown agent '{}' from '{}'",
                        target_id,
                        event.message.source
                    );
                }
                // 收尾阶段新产生的事件不纳入本轮处理（避免无限循环），
                // 直接由后续 collect_results 忽略。
            }
            if !self.event_queue.is_empty() {
                tracing::warn!(
                    "[market-sim] Kernel: on_sim_end 收尾阶段仍残留 {} 个未处理事件（已忽略）",
                    self.event_queue.len()
                );
            }
        }

        // 4. 收集结果
        let wall_clock_ms = wall_start.elapsed().as_millis() as u64;
        let (trades, mid_price, exchange_stats) = self.collect_results();

        Ok(SimResult {
            stock_code: self.config.stock_code.clone(),
            reference_price: self.config.reference_price,
            wall_clock_ms,
            sim_time_ns: self.clock,
            total_events: self.event_count,
            trades,
            final_mid_price: mid_price,
            stats: SimStats {
                total_orders: exchange_stats.0,
                total_trades: exchange_stats.1,
                total_volume: exchange_stats.2,
                total_messages: self.event_count,
                max_queue_depth: self.max_queue_depth,
                agent_count: self.agents.len(),
            },
        })
    }

    // ── 内部方法 ──

    /// 创建 Agent 上下文
    ///
    /// 修复 P0-5: 增加 `message_source` 参数，由调用方从 `event.message.source`
    /// 传入。ExchangeAgent 处理 CancelOrder / RequestQuote 等不含来源 ID 的消息时，
    /// 需通过此字段获取通知目标，避免把通知发给自己。
    fn make_ctx(&self, agent_id: &str, message_source: Option<&str>) -> AgentContext {
        AgentContext::new(
            self.clock,
            self.config.stock_code.clone(),
            self.config.reference_price,
            agent_id.to_string(),
            message_source.map(|s| s.to_string()),
        )
    }

    /// 处理 Agent 返回的动作列表
    ///
    /// 修复 M-DEF-4: 原返回 `Result<(), SimError>` 但实际永不返回 Err
    /// （所有内部调用 schedule_message / push 都不返回 Result）。
    /// 改为返回 ()，移除调用方的 `?` 死代码。
    fn process_actions(
        &mut self,
        source_id: &str,
        actions: &[AgentAction],
        current_time: SimTimestamp,
    ) {
        // 先获取 source_type（避免在 schedule_message 中持有 self.agents 的借用）
        let source_type: String = self
            .agents
            .get(source_id)
            .map(|e| e.agent_type.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        for action in actions {
            match action {
                AgentAction::SendMessage { target, body } => {
                    self.schedule_message(
                        source_id,
                        &source_type,
                        target,
                        body.clone(),
                        current_time,
                    );
                },
                AgentAction::Broadcast { targets, body } => {
                    for target in targets {
                        self.schedule_message(
                            source_id,
                            &source_type,
                            target,
                            body.clone(),
                            current_time,
                        );
                    }
                },
                AgentAction::WakeupAfter(delay_ns) => {
                    let deliver_at = current_time + delay_ns;
                    let event = SimEvent {
                        scheduled_at: deliver_at,
                        priority: 10, // 唤醒优先级略低
                        message: AgentMessage {
                            source: source_id.to_string(),
                            target: source_id.to_string(),
                            sent_at: current_time,
                            body: MessageBody::Wakeup,
                        },
                    };
                    self.event_queue.push(Reverse(event));
                },
            }
        }
    }

    /// 调度消息投递
    fn schedule_message(
        &mut self,
        source_id: &str,
        source_type: &str,
        target_id: &str,
        body: MessageBody,
        sent_at: SimTimestamp,
    ) {
        let target_entry = self.agents.get(target_id);
        let target_type = target_entry.map(|e| e.agent_type.as_str()).unwrap_or("unknown");

        let latency_ns = self.latency.get(
            source_id,
            target_id,
            source_type,
            target_type,
            self.config.default_latency_ns,
        );

        let deliver_at = sent_at + latency_ns;

        let event = SimEvent {
            scheduled_at: deliver_at,
            priority: 5, // 普通消息默认优先级
            message: AgentMessage {
                source: source_id.to_string(),
                target: target_id.to_string(),
                sent_at,
                body,
            },
        };

        self.event_queue.push(Reverse(event));
    }

    /// 从所有 Agent 收集交易结果
    ///
    /// 修复 P0-M1: 原实现硬编码 `return (Vec::new(), ...)`，导致
    /// stylized_facts / calibration 全部走"成交不足 < 20"分支得 999.0 分。
    /// 现在通过 `SimAgent::trade_history()` trait 方法从每个 Agent 聚合；
    /// 默认实现返回空切片，只有 ExchangeAgent override 后才有数据。
    ///
    /// 修复 C4.2: 原实现 exchange_stats 永远返回 (0, 0, 0)，导致 SimStats 中的
    /// total_orders/total_trades/total_volume 全为 0。现在通过 `SimAgent::exchange_stats()`
    /// trait 方法累加所有 ExchangeAgent 的真实统计。
    fn collect_results(&self) -> (Vec<TradeRecord>, Option<f64>, (u64, u64, Quantity)) {
        let mut all_trades: Vec<TradeRecord> = Vec::new();
        let mut total_orders: u64 = 0;
        let mut total_trades: u64 = 0;
        let mut total_volume: Quantity = 0;
        let mut final_mid: Option<f64> = None;
        for entry in self.agents.values() {
            // 修复 M-RES-14: 原实现从所有 Agent 收集 trade_history，
            // 但策略类 Agent（如 quant_bridge）的 trade_history 与 ExchangeAgent 重复，
            // 导致 all_trades 中有重复记录。改为只从 ExchangeAgent 收集。
            if entry.agent.agent_type() == AgentType::Exchange {
                all_trades.extend(entry.agent.trade_history().iter().cloned());
                // 修复 M-RES-12: 优先用 ExchangeAgent 的 OrderBook mid_price
                if final_mid.is_none() {
                    final_mid = entry.agent.final_mid_price();
                }
            }
            // 累加 ExchangeAgent 等的统计（其他 Agent 默认返回 (0, 0, 0)）
            let (o, t, v) = entry.agent.exchange_stats();
            total_orders += o;
            total_trades += t;
            total_volume += v;
        }
        // 修复 M-RES-12: 若 OrderBook mid_price 不可用（一侧为空），
        // 用最后一笔成交价兜底。
        let mid_price = final_mid.or_else(|| all_trades.last().map(|t| t.price as f64));
        (all_trades, mid_price, (total_orders, total_trades, total_volume))
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    /// 简单的回声 Agent：收到消息后回复一条固定消息
    struct EchoAgent {
        id: String,
        reply_to: Option<(String, MessageBody)>,
        messages_received: u64,
    }

    impl EchoAgent {
        fn new(id: &str) -> Self {
            Self { id: id.to_string(), reply_to: None, messages_received: 0 }
        }

        fn with_reply(mut self, target: &str, body: MessageBody) -> Self {
            self.reply_to = Some((target.to_string(), body));
            self
        }
    }

    impl SimAgent for EchoAgent {
        fn id(&self) -> &str {
            &self.id
        }

        fn agent_type(&self) -> AgentType {
            AgentType::Custom("echo".into())
        }

        fn on_message(&mut self, _msg: &MessageBody, _ctx: &mut AgentContext) -> Vec<AgentAction> {
            self.messages_received += 1;
            let mut actions = Vec::new();
            if let Some((ref target, ref body)) = self.reply_to {
                actions
                    .push(AgentAction::SendMessage { target: target.clone(), body: body.clone() });
            }
            actions
        }

        // 测试辅助：on_wakeup 也走与 on_message 一致的回复逻辑。
        // kernel 主循环将 MessageBody::Wakeup 路由到 on_wakeup（而非 on_message），
        // 若不覆写则走 trait 默认实现（返回空 Vec），EchoAgent 不会回复。
        fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
            self.on_message(&MessageBody::Wakeup, ctx)
        }
    }

    #[test]
    fn test_empty_kernel() {
        let config = SimConfig::default();
        let mut kernel = SimKernel::new(config);
        // 无 Agent 时 run 应返回错误
        assert!(kernel.run().is_err());
    }

    #[test]
    fn test_single_agent_bootstrap() {
        let config = SimConfig {
            max_time_ns: 1_000_000, // 1ms
            ..SimConfig::default()
        };
        let mut kernel = SimKernel::new(config);
        kernel.register(Box::new(EchoAgent::new("echo")));
        let result = kernel.run().unwrap();
        assert_eq!(result.stats.agent_count, 1);
        assert_eq!(result.total_events, 0); // 没有初始事件
    }

    #[test]
    fn test_two_agent_message() {
        let config = SimConfig {
            max_time_ns: 100_000_000, // 100ms
            ..SimConfig::default()
        };
        let mut kernel = SimKernel::new(config);

        // Agent A 收到任何消息后回复 "hello" 给 Agent B
        kernel.register(Box::new(
            EchoAgent::new("agent_a").with_reply("agent_b", MessageBody::Wakeup),
        ));
        // Agent B 只接收不回复
        kernel.register(Box::new(EchoAgent::new("agent_b")));

        // 手动注入初始事件：系统 → Agent A
        let init_event = SimEvent {
            scheduled_at: 0,
            priority: 0,
            message: AgentMessage {
                source: "kernel".into(),
                target: "agent_a".into(),
                sent_at: 0,
                body: MessageBody::Wakeup,
            },
        };
        kernel.event_queue.push(Reverse(init_event));

        let result = kernel.run().unwrap();
        // 2 个事件：kernel→A(唤醒) + A→B(hello)
        assert_eq!(result.total_events, 2);
    }

    #[test]
    fn test_full_market_simulation_few_events() {
        use crate::agent::{
            ExchangeAgent, MarketMakerAgent, MomentumAgent, NoiseAgent, ValueAgent,
        };

        let config = SimConfig {
            max_time_ns: 10_000_000, // 10ms
            reference_price: 1000,   // 10.00 元
            default_latency_ns: 100, // 100ns
            ..SimConfig::default()
        };

        let mut kernel = SimKernel::new(config);

        // 注册交易所
        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));

        // 注册做市商（价差 50bps，每档 500 股，库存上限 5000）
        kernel.register(Box::new(MarketMakerAgent::new("mm", 50, 500, 5000, 0.1, 200_000, 1000)));

        // 注册动量 Agent（5 窗口，0.5% 阈值，100 股/次，2000 上限）
        kernel.register(Box::new(MomentumAgent::new(
            "momentum", 5, 0.005, 100, 2000, 500_000, 1000.0,
        )));

        // 注册噪声 Agent（1ms 平均间隔，50% 概率，50 股上限，30bps 噪声）
        kernel.register(Box::new(NoiseAgent::new("noise_1", 500_000, 0.4, 50, 30, 1000, 42)));

        // 注册价值 Agent（参考价 1010，超过 20bps 时交易）
        kernel.register(Box::new(ValueAgent::new("value", 1010, 20, 200, 3000, 1_000_000)));

        // 运行模拟
        let result = kernel.run().unwrap();

        // 基本验证：事件被处理了
        assert!(result.total_events > 0, "应该有事件处理");
        assert_eq!(result.stats.agent_count, 5);
        // 修复 P0-5 后 ExchangeAgent 正确通知提交订单的 Agent，
        // 收尾阶段会处理队列中残留的消息事件（时间戳略超 max_time_ns，
        // 因为 deliver_at = sent_at + latency）。收尾阶段设计上不受
        // max_time_ns 限制（见 kernel.rs 收尾阶段注释），允许 1ms 容差。
        assert!(
            result.sim_time_ns <= 11_000_000,
            "sim_time_ns={} 应不超过 max_time_ns + 1ms 容差",
            result.sim_time_ns
        );
    }

    #[test]
    fn test_full_market_simulation_longer() {
        use crate::agent::{
            ExchangeAgent, MarketMakerAgent, MomentumAgent, NoiseAgent, ValueAgent,
        };

        let config = SimConfig {
            max_time_ns: 50_000_000, // 50ms
            reference_price: 1000,
            default_latency_ns: 100,
            ..SimConfig::default()
        };

        let mut kernel = SimKernel::new(config);

        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
        kernel.register(Box::new(MarketMakerAgent::new("mm", 30, 300, 5000, 0.1, 200_000, 1000)));
        kernel.register(Box::new(MomentumAgent::new(
            "momentum", 5, 0.003, 100, 2000, 500_000, 1000.0,
        )));
        kernel.register(Box::new(NoiseAgent::new("noise_1", 300_000, 0.3, 50, 30, 1000, 42)));
        kernel.register(Box::new(NoiseAgent::new("noise_2", 500_000, 0.5, 30, 20, 1000, 42)));
        kernel.register(Box::new(ValueAgent::new("value", 1020, 30, 200, 3000, 1_000_000)));

        let result = kernel.run().unwrap();

        assert!(result.total_events > 0, "应该有事件处理");
        assert_eq!(result.stats.agent_count, 6);
    }
}
