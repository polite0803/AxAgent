//! Exchange Agent — 中央交易所，维护订单簿并撮合交易。
//!
//! ExchangeAgent 是市场模拟中唯一的订单簿持有者。
//! 所有交易 Agent 通过消息与它交互（提交订单、撤单、查询行情）。
//! 成交后的通知由 ExchangeAgent 通过消息发回给相关 Agent。

use crate::agent::traits::{AgentAction, AgentContext, AgentType, MessageBody, SimAgent};
use crate::orderbook::OrderBook;
use crate::types::*;

/// 交易所 Agent
///
/// 维护中央限价订单簿，处理所有 Agent 的订单提交/撤单/查询请求。
pub struct ExchangeAgent {
    id: String,
    orderbook: OrderBook,
    /// 统计计数器
    total_orders: u64,
    total_trades: u64,
    total_volume: Quantity,
    /// 修复 P0-M1: 成交历史（按时间顺序），供 Kernel.collect_results 读取。
    /// 之前 Kernel 永远返回空 Vec，导致 stylized_facts / calibration 全部走
    /// "成交不足 < 20"分支得 999.0 分——整个仿真"产出 0"。
    trade_history: Vec<TradeRecord>,
}

impl ExchangeAgent {
    /// 创建交易所 Agent
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            orderbook: OrderBook::new(),
            total_orders: 0,
            total_trades: 0,
            total_volume: 0,
            trade_history: Vec::new(),
        }
    }

    /// 带 tick_size 的交易所
    pub fn with_tick_size(id: impl Into<String>, tick_size: Price) -> Self {
        Self {
            id: id.into(),
            orderbook: OrderBook::with_tick_size(tick_size),
            total_orders: 0,
            total_trades: 0,
            total_volume: 0,
            trade_history: Vec::new(),
        }
    }
}

impl SimAgent for ExchangeAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Exchange"
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Exchange
    }

    /// 修复 P0-M1: 实现 trade_history()，让 Kernel.collect_results 拿到真实成交
    fn trade_history(&self) -> &[TradeRecord] {
        &self.trade_history
    }

    fn on_message(&mut self, msg: &MessageBody, ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            // ── 限价单 ──
            MessageBody::SubmitLimit(order) => {
                self.total_orders += 1;
                self.orderbook.set_time(ctx.current_time);
                let source = ctx.agent_id().to_string();

                match self.orderbook.submit_limit_order(order.clone()) {
                    Ok(result) => match result {
                        OrderResult::Placed { order_id } => {
                            ctx.send(&source, MessageBody::OrderPlaced { order_id });
                        },
                        OrderResult::PartialFill { order_id, ref fill } => {
                            // 修复 P0-M1: 收集成交历史供 Kernel 提取
                            self.trade_history.extend(fill.trades.iter().cloned());
                            // 通知卖方
                            for trade in &fill.trades {
                                if trade.seller_agent_id != source {
                                    ctx.send(
                                        &trade.seller_agent_id,
                                        MessageBody::OrderFilled {
                                            order_id: trade.seller_order_id,
                                            fill: fill.clone(),
                                        },
                                    );
                                }
                            }
                            // 通知买方（自己）
                            ctx.send(
                                &source,
                                MessageBody::OrderFilled { order_id, fill: fill.clone() },
                            );
                            self.total_trades += fill.trades.len() as u64;
                            self.total_volume += fill.filled_quantity;
                        },
                        OrderResult::FullFill { order_id, ref fill } => {
                            // 修复 P0-M1: 收集成交历史供 Kernel 提取
                            self.trade_history.extend(fill.trades.iter().cloned());
                            // 通知对手方
                            for trade in &fill.trades {
                                if trade.seller_agent_id != source {
                                    ctx.send(
                                        &trade.seller_agent_id,
                                        MessageBody::OrderFilled {
                                            order_id: trade.seller_order_id,
                                            fill: fill.clone(),
                                        },
                                    );
                                }
                            }
                            // 通知自己
                            ctx.send(
                                &source,
                                MessageBody::OrderFilled { order_id, fill: fill.clone() },
                            );
                            self.total_trades += fill.trades.len() as u64;
                            self.total_volume += fill.filled_quantity;
                        },
                        OrderResult::Cancelled { .. } => {
                            // 不应出现
                        },
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Exchange: submit_limit_order failed: {} (order_id={})",
                            e,
                            order.id
                        );
                    },
                }
            },

            // ── 市价单 ──
            MessageBody::SubmitMarket(order) => {
                self.total_orders += 1;
                self.orderbook.set_time(ctx.current_time);
                let source = ctx.agent_id().to_string();

                match self.orderbook.submit_market_order(order.clone()) {
                    Ok(result) => match result {
                        OrderResult::PartialFill { order_id, ref fill }
                        | OrderResult::FullFill { order_id, ref fill } => {
                            // 修复 P0-M1: 收集成交历史供 Kernel 提取（市价单）
                            self.trade_history.extend(fill.trades.iter().cloned());
                            // 通知对手方
                            for trade in &fill.trades {
                                let counterparty = match order.side {
                                    OrderSide::Buy => &trade.seller_agent_id,
                                    OrderSide::Sell => &trade.buyer_agent_id,
                                };
                                if counterparty != &source {
                                    ctx.send(
                                        counterparty,
                                        MessageBody::OrderFilled {
                                            order_id: if order.side == OrderSide::Buy {
                                                trade.seller_order_id
                                            } else {
                                                trade.buyer_order_id
                                            },
                                            fill: fill.clone(),
                                        },
                                    );
                                }
                            }
                            // 通知自己
                            ctx.send(
                                &source,
                                MessageBody::OrderFilled { order_id, fill: fill.clone() },
                            );
                            self.total_trades += fill.trades.len() as u64;
                            self.total_volume += fill.filled_quantity;
                        },
                        _ => {},
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Exchange: submit_market_order failed: {} (order_id={})",
                            e,
                            order.id
                        );
                    },
                }
            },

            // ── 撤单 ──
            MessageBody::CancelOrder(order_id) => {
                let source = ctx.agent_id().to_string();
                match self.orderbook.cancel_order(*order_id) {
                    Ok(result) => {
                        if let OrderResult::Cancelled { order_id, remaining } = result {
                            ctx.send(&source, MessageBody::OrderCancelled { order_id, remaining });
                        }
                    },
                    Err(_) => {
                        // 订单不存在或已成交，静默忽略
                    },
                }
            },

            // ── 行情查询 ──
            MessageBody::RequestQuote => {
                let source = ctx.agent_id().to_string();
                let snapshot = self.orderbook.book_depth(10);
                ctx.send(&source, MessageBody::QuoteReply(snapshot));
            },

            // ── 不处理的消息 ──
            _ => {},
        }

        ctx.drain_actions()
    }
}
