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

    /// 修复 C4.2: 暴露 ExchangeAgent 的 (total_orders, total_trades, total_volume) 统计，
    /// 供 Kernel.collect_results 累加。原实现 Kernel 硬编码返回 (0, 0, 0)。
    fn exchange_stats(&self) -> (u64, u64, Quantity) {
        (self.total_orders, self.total_trades, self.total_volume)
    }

    /// 修复 M-RES-12: 暴露 OrderBook 的最终 mid price
    /// （best_bid + best_ask 的均值）。若订单簿一侧为空则返回 None。
    fn final_mid_price(&self) -> Option<f64> {
        self.orderbook.mid_price()
    }

    fn on_message(&mut self, msg: &MessageBody, ctx: &mut AgentContext) -> Vec<AgentAction> {
        match msg {
            // ── 限价单 ──
            MessageBody::SubmitLimit(order) => {
                self.total_orders += 1;
                self.orderbook.set_time(ctx.current_time);
                // 修复 P0-5: 通知目标应为提交订单的 Agent（order.agent_id），
                // 而非 ctx.agent_id()（其值为消息目标 "exchange"）。
                let source = order.agent_id.clone();

                match self.orderbook.submit_limit_order(order.clone()) {
                    Ok(result) => match result {
                        OrderResult::Placed { order_id } => {
                            // 修复 P0-6: OrderPlaced 携带 side 字段，
                            // 让做市商能区分 bid/ask 挂单并更新对应 order_id。
                            ctx.send(
                                &source,
                                MessageBody::OrderPlaced { order_id, side: order.side },
                            );
                        },
                        OrderResult::PartialFill { order_id, ref fill } => {
                            // 修复 P0-M1: 收集成交历史供 Kernel 提取
                            self.trade_history.extend(fill.trades.iter().cloned());
                            // 修复 H3.1: 限价单对手通知须区分提交者 side
                            // 原实现只通知 seller_agent_id != source，当提交者是卖方时不通知买方
                            for trade in &fill.trades {
                                let counterparty = match order.side {
                                    OrderSide::Buy => &trade.seller_agent_id,
                                    OrderSide::Sell => &trade.buyer_agent_id,
                                };
                                if counterparty != &source {
                                    let cp_order_id = if order.side == OrderSide::Buy {
                                        trade.seller_order_id
                                    } else {
                                        trade.buyer_order_id
                                    };
                                    ctx.send(
                                        counterparty,
                                        MessageBody::OrderFilled {
                                            order_id: cp_order_id,
                                            fill: fill.clone(),
                                        },
                                    );
                                }
                            }
                            // 通知提交者自己
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
                            // 修复 H3.1: 限价单对手通知须区分提交者 side
                            for trade in &fill.trades {
                                let counterparty = match order.side {
                                    OrderSide::Buy => &trade.seller_agent_id,
                                    OrderSide::Sell => &trade.buyer_agent_id,
                                };
                                if counterparty != &source {
                                    let cp_order_id = if order.side == OrderSide::Buy {
                                        trade.seller_order_id
                                    } else {
                                        trade.buyer_order_id
                                    };
                                    ctx.send(
                                        counterparty,
                                        MessageBody::OrderFilled {
                                            order_id: cp_order_id,
                                            fill: fill.clone(),
                                        },
                                    );
                                }
                            }
                            // 通知提交者自己
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
                // 修复 P0-5: 通知目标应为提交订单的 Agent（order.agent_id），
                // 而非 ctx.agent_id()（其值为消息目标 "exchange"）。
                let source = order.agent_id.clone();

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
                // 修复 P0-5: CancelOrder 消息体不含来源 ID，
                // 通过 ctx.message_source() 获取提交撤单请求的 Agent。
                let source = ctx.message_source().unwrap_or_default().to_string();
                match self.orderbook.cancel_order(*order_id) {
                    Ok(result) => {
                        if let OrderResult::Cancelled { order_id, remaining } = result {
                            // 仅在 source 非空时回复，避免向空目标发送通知
                            if !source.is_empty() {
                                ctx.send(
                                    &source,
                                    MessageBody::OrderCancelled { order_id, remaining },
                                );
                            }
                        }
                    },
                    Err(_) => {
                        // 订单不存在或已成交，静默忽略
                    },
                }
            },

            // ── 行情查询 ──
            MessageBody::RequestQuote => {
                // 修复 P0-5: RequestQuote 消息体不含来源 ID，
                // 通过 ctx.message_source() 获取查询行情的 Agent。
                let source = ctx.message_source().unwrap_or_default().to_string();
                let snapshot = self.orderbook.book_depth(10);
                if !source.is_empty() {
                    ctx.send(&source, MessageBody::QuoteReply(snapshot));
                }
            },

            // ── 不处理的消息 ──
            _ => {},
        }

        ctx.drain_actions()
    }
}
