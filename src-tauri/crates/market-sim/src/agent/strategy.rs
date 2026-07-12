//! 策略 Agent —— 根据股票分析决策参数执行交易，供蒙特卡洛验证决策鲁棒性。
//!
//! 与 MarketMaker/Momentum/Noise 等通用 Agent 不同，StrategyAgent 的行为由
//! 外部决策（action/targetPrice/stopLoss）直接控制。它模拟"如果我按这个决策
//! 操作，在给定的合成市场中会得到什么结果"。
//!
//! ## 行为逻辑
//!
//! | action | 初始行为 | 持仓后行为 |
//! |--------|----------|-----------|
//! | 买入/增持 | 市价买入 | 监控 targetPrice(止盈) + stopLoss(止损) |
//! | 持有 | 无 | 监控是否到达目标/止损 |
//! | 观望 | 无 | 什么都不做 |
//! | 减持/卖出 | 市价卖出 | 平仓离场 |

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::types::{OrderSide, *};

pub struct StrategyAgent {
    id: String,
    /// 决策动作
    action: String,
    /// 目标价（分）
    target_price: Price,
    /// 止损价（分）
    stop_loss: Price,
    /// 参考价（分）
    #[allow(dead_code)]
    reference_price: Price,
    /// 仓位大小（股）
    position_size: Quantity,
    /// 当前持仓（正=多, 负=空）
    current_position: i64,
    /// 是否已提交入场订单
    entry_submitted: bool,
    /// 入场成交价
    entry_price: Option<Price>,
    /// 是否已平仓离场
    exited: bool,
    /// 监控唤醒间隔（ns）
    wakeup_interval_ns: SimTimestamp,
    /// 最近一次查询到的市场价格
    last_price: Option<Price>,
    /// 自增 ID
    next_id: u64,
    /// 初始执行动作是否已记录
    action_taken: bool,
}

impl StrategyAgent {
    /// 创建策略 Agent
    ///
    /// - `action`: "买入"/"增持"/"持有"/"观望"/"减持"/"卖出"
    /// - `target_price`: 目标价（分）
    /// - `stop_loss`: 止损价（分）
    /// - `position_size`: 仓位（股）
    /// - `wakeup_interval_ns`: 市场监控频率（默认 1ms）
    pub fn new(
        id: impl Into<String>,
        action: impl Into<String>,
        target_price: Price,
        stop_loss: Price,
        reference_price: Price,
        position_size: Quantity,
        wakeup_interval_ns: SimTimestamp,
    ) -> Self {
        Self {
            id: id.into(),
            action: action.into(),
            target_price,
            stop_loss,
            reference_price,
            position_size,
            current_position: 0,
            entry_submitted: false,
            entry_price: None,
            exited: false,
            wakeup_interval_ns,
            last_price: None,
            next_id: 1,
            action_taken: false,
        }
    }

    fn gen_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// 检查是否需要止盈/止损
    fn check_exit(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        if self.exited || self.current_position == 0 {
            return Vec::new();
        }

        let price = match self.last_price.or(self.entry_price) {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut actions = Vec::new();

        // 止盈
        if self.current_position > 0 && price >= self.target_price {
            let order = MarketOrder {
                id: self.gen_id(),
                side: OrderSide::Sell,
                quantity: self.current_position as Quantity,
                agent_id: self.id.clone(),
                timestamp: ctx.current_time,
            };
            actions.push(AgentAction::SendMessage {
                target: "exchange".into(),
                body: MessageBody::SubmitMarket(order),
            });
            self.exited = true;
        }

        // 止损
        if self.current_position > 0 && price <= self.stop_loss {
            let order = MarketOrder {
                id: self.gen_id(),
                side: OrderSide::Sell,
                quantity: self.current_position as Quantity,
                agent_id: self.id.clone(),
                timestamp: ctx.current_time,
            };
            actions.push(AgentAction::SendMessage {
                target: "exchange".into(),
                body: MessageBody::SubmitMarket(order),
            });
            self.exited = true;
        }

        actions
    }
}

impl SimAgent for StrategyAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Strategy"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Strategy
    }

    fn on_init(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let mut actions = Vec::new();

        match self.action.as_str() {
            "买入" | "增持" => {
                let order = MarketOrder {
                    id: self.gen_id(),
                    side: OrderSide::Buy,
                    quantity: self.position_size,
                    agent_id: self.id.clone(),
                    timestamp: ctx.current_time,
                };
                actions.push(AgentAction::SendMessage {
                    target: "exchange".into(),
                    body: MessageBody::SubmitMarket(order),
                });
                self.entry_submitted = true;
                self.action_taken = true;
            },
            "减持" | "卖出" => {
                let order = MarketOrder {
                    id: self.gen_id(),
                    side: OrderSide::Sell,
                    quantity: self.position_size,
                    agent_id: self.id.clone(),
                    timestamp: ctx.current_time,
                };
                actions.push(AgentAction::SendMessage {
                    target: "exchange".into(),
                    body: MessageBody::SubmitMarket(order),
                });
                self.action_taken = true;
            },
            "持有" | "观望" => {
                // 不操作，仅监控
                self.action_taken = true;
            },
            _ => {
                tracing::warn!("StrategyAgent: 未知 action '{}'", self.action);
            },
        }

        // 订阅行情
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
                // 记录入场价（仅首次成交记录 entry_price）。
                // 修复 M-DS-6: 原代码只在 `entry_price.is_none()` 首次成交时更新
                // current_position，导致部分成交后续 fills 丢失持仓更新。
                // 现在每次 fill 都更新 current_position，但 entry_price 仅记录
                // 第一次成交价（用于止损止盈判定）。
                for trade in &fill.trades {
                    if trade.buyer_agent_id == self.id {
                        if self.entry_price.is_none() && self.entry_submitted {
                            self.entry_price = Some(trade.price);
                        }
                        self.current_position += trade.quantity as i64;
                    } else if trade.seller_agent_id == self.id {
                        if self.entry_price.is_none() && self.entry_submitted {
                            // 卖出首次成交也记录 entry_price
                            self.entry_price = Some(trade.price);
                        }
                        self.current_position -= trade.quantity as i64;
                    }
                }
                // 获取最新价格
                for trade in &fill.trades {
                    self.last_price = Some(trade.price);
                }
                Vec::new()
            },
            MessageBody::QuoteReply(snapshot) => {
                // 从快照中获取最新价格
                if let Some(last) = snapshot.last_trade_price {
                    self.last_price = Some(last);
                } else if let Some((bid, ask)) = snapshot.bids.first().zip(snapshot.asks.first()) {
                    self.last_price = Some((bid.price + ask.price) / 2);
                }
                Vec::new()
            },
            MessageBody::OrderCancelled { .. } => Vec::new(),
            MessageBody::OrderPlaced { .. } => Vec::new(),
            _ => Vec::new(),
        }
    }

    fn on_wakeup(&mut self, ctx: &mut AgentContext) -> Vec<AgentAction> {
        let mut actions = Vec::new();

        // 检查止盈/止损
        if !self.exited && self.current_position != 0 {
            actions.extend(self.check_exit(ctx));
        }

        // 持续监控市场
        actions.push(AgentAction::SendMessage {
            target: "exchange".into(),
            body: MessageBody::RequestQuote,
        });
        actions.push(AgentAction::WakeupAfter(self.wakeup_interval_ns));

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

    fn run_strategy_sim(
        action: &str,
        target: Price,
        stop: Price,
        ref_price: Price,
        duration_ns: SimTimestamp,
    ) -> SimResult {
        let mut kernel = SimKernel::new(SimConfig {
            max_time_ns: duration_ns,
            default_latency_ns: 1_000,
            reference_price: ref_price,
            ..Default::default()
        });
        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
        kernel.register(Box::new(MarketMakerAgent::new(
            "mm", 30, 500, 5000, 0.1, 500_000, ref_price,
        )));
        kernel.register(Box::new(NoiseAgent::new("noise", 500_000, 0.5, 100, 50, ref_price, 42)));
        kernel.register(Box::new(StrategyAgent::new(
            "strategy", action, target, stop, ref_price, 500, 1_000_000,
        )));
        kernel.run().unwrap()
    }

    #[test]
    fn test_strategy_buy_produces_trades() {
        let result = run_strategy_sim("买入", 1050, 950, 1000, 200_000_000);
        eprintln!("buy: events={} trades={}", result.total_events, result.trades.len());
        assert!(result.total_events > 50);
    }

    #[test]
    fn test_strategy_hold_produces_events() {
        let result = run_strategy_sim("持有", 1050, 950, 1000, 100_000_000);
        eprintln!("hold: events={} trades={}", result.total_events, result.trades.len());
        assert!(result.total_events > 20);
    }

    #[test]
    fn test_strategy_sell_produces_events() {
        let result = run_strategy_sim("卖出", 1050, 950, 1000, 100_000_000);
        eprintln!("sell: events={} trades={}", result.total_events, result.trades.len());
        assert!(result.total_events > 20);
    }

    #[test]
    fn test_strategy_wait_produces_events() {
        let result = run_strategy_sim("观望", 1050, 950, 1000, 100_000_000);
        eprintln!("wait: events={} trades={}", result.total_events, result.trades.len());
        assert!(result.total_events > 20);
    }
}
