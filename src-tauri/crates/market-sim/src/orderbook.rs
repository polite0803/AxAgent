//! 中央限价订单簿（LOB）—— ABIDES Phase 1 核心组件。
//!
//! ## 撮合规则
//!
//! - **价格-时间优先**：价格最优 → 同价先到先得
//! - **买盘降序、卖盘升序**：bids 用 `Reverse<Price>` 实现降序，asks 用默认升序
//! - **非自成交**：同一 agent_id 的订单不会互相成交
//!
//! ## 数据结构
//!
//! - `bids`: BTreeMap<Reverse<Price>, PriceLevel> — 买盘，最高价优先
//! - `asks`: BTreeMap<Price, PriceLevel> — 卖盘，最低价优先
//! - `order_index`: HashMap<OrderId, Locator> — O(1) 撤单查找
//! - 每个 PriceLevel 内部用 `VecDeque<LimitOrder>` 维护时间顺序
//!
//! ## A 股特性
//!
//! - `tick_size = 1`（1 分 = 最小价格单位）
//! - 涨跌停不在 LOB 层面处理（由上层 Agent 在发单前拦截）

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::error::SimError;
use crate::types::*;

/// 成交历史最大长度，防止无界增长导致 OOM
///
/// 修复 C4.5: 原 trade_history 在长时仿真中无限制累积，
/// 可能消耗 GB 级内存。这里限制为 100_000 条，超过时丢弃最旧记录。
const MAX_TRADE_HISTORY: usize = 100_000;

// ── 内部数据结构 ──

/// 价格档位：同一价位上的所有挂单（FIFO 时间优先队列）
#[derive(Debug, Clone)]
struct PriceLevel {
    #[allow(dead_code)]
    price: Price,
    orders: VecDeque<LimitOrder>,
    total_quantity: Quantity,
}

impl PriceLevel {
    fn new(price: Price) -> Self {
        Self { price, orders: VecDeque::new(), total_quantity: 0 }
    }

    fn push(&mut self, order: LimitOrder) {
        self.total_quantity += order.remaining();
        self.orders.push_back(order);
    }

    fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    fn order_count(&self) -> usize {
        self.orders.len()
    }
}

/// 快速定位一个挂单在订单簿中的位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderLocator {
    Bid(Price),
    Ask(Price),
}

// ── 公共结构 ──

/// 中央限价订单簿
///
/// # 使用示例
///
/// ```rust
/// use axagent_market_sim::{OrderBook, LimitOrder, OrderSide, FillResult};
///
/// let mut ob = OrderBook::new();
///
/// // 挂买单 @ 10.00 元 (1000 分) × 100 股
/// let buy = LimitOrder {
///     id: 1,
///     side: OrderSide::Buy,
///     price: 1000,
///     quantity: 100,
///     filled_quantity: 0,
///     timestamp: 0,
///     agent_id: "trader_a".into(),
/// };
/// ob.submit_limit_order(buy);
///
/// // 挂卖单 @ 10.01 元 × 50 股
/// let sell = LimitOrder {
///     id: 2,
///     side: OrderSide::Sell,
///     price: 1001,
///     quantity: 50,
///     filled_quantity: 0,
///     timestamp: 1,
///     agent_id: "trader_b".into(),
/// };
/// ob.submit_limit_order(sell);
///
/// assert!(ob.mid_price().is_some());
/// ```
#[derive(Debug, Clone)]
pub struct OrderBook {
    /// 买盘：最高价优先（Reverse 包装实现降序）
    bids: BTreeMap<Reverse<Price>, PriceLevel>,
    /// 卖盘：最低价优先（默认升序）
    asks: BTreeMap<Price, PriceLevel>,
    /// 订单快速查找表：OrderId → 位置
    order_index: HashMap<OrderId, OrderLocator>,
    /// 成交历史
    ///
    /// 修复 L-10: 此字段保留用于 OrderBook 内部查询（如 trade_count()、
    /// market_impact_estimate 等场景）。Kernel.collect_results 已改为
    /// 从 ExchangeAgent::trade_history() 聚合，二者数据一致但来源不同。
    /// 若未来确认 OrderBook 内部不再需要，可安全删除。
    trade_history: Vec<TradeRecord>,
    /// 最小价格变动单位（A 股 = 1 分）
    tick_size: Price,
    /// 自增 OrderId 生成器
    id_counter: OrderId,
    /// 当前模拟时间
    current_time: SimTimestamp,
}

impl OrderBook {
    /// 创建空订单簿
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            order_index: HashMap::new(),
            trade_history: Vec::new(),
            tick_size: 1,
            id_counter: 1,
            current_time: 0,
        }
    }

    /// 创建带自定义 tick_size 的订单簿
    pub fn with_tick_size(tick_size: Price) -> Self {
        let mut ob = Self::new();
        ob.tick_size = tick_size;
        ob
    }

    /// 设置当前模拟时间
    pub fn set_time(&mut self, time: SimTimestamp) {
        self.current_time = time;
    }

    /// 分配下一个 OrderId
    pub fn next_order_id(&mut self) -> OrderId {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    /// 修剪成交历史至不超过 MAX_TRADE_HISTORY 条
    ///
    /// 修复 C4.5: 在每次 extend 后调用，超过上限时丢弃最旧记录，防止 OOM。
    fn trim_trade_history(&mut self) {
        if self.trade_history.len() > MAX_TRADE_HISTORY {
            let overflow = self.trade_history.len() - MAX_TRADE_HISTORY;
            self.trade_history.drain(0..overflow);
        }
    }

    // ── 订单操作 ──

    /// 提交市价单：立即以最优对手价成交
    ///
    /// 返回成交结果。如果对手盘深度不足，部分成交，剩余数量记录在 `unfilled_quantity` 中。
    pub fn submit_market_order(&mut self, order: MarketOrder) -> Result<OrderResult, SimError> {
        if order.quantity == 0 {
            return Err(SimError::InvalidQuantity);
        }

        let mid_before = self.mid_price();
        let mut fill = FillResult {
            trades: Vec::new(),
            vwap: 0.0,
            filled_quantity: 0,
            unfilled_quantity: 0,
            market_impact_bps: 0.0,
            levels_consumed: 0,
        };

        match order.side {
            OrderSide::Buy => {
                self.match_against_asks(order.id, order.quantity, &order.agent_id, &mut fill);
            },
            OrderSide::Sell => {
                self.match_against_bids(order.id, order.quantity, &order.agent_id, &mut fill);
            },
        }

        fill.vwap = Self::compute_vwap(&fill);
        fill.unfilled_quantity = order.quantity.saturating_sub(fill.filled_quantity);
        self.trade_history.extend(fill.trades.iter().cloned());
        // 修复 C4.5: 修剪成交历史，防止无界增长
        self.trim_trade_history();

        // 计算冲击成本
        if let Some(mid) = mid_before
            && fill.vwap > 0.0
        {
            let impact = (fill.vwap - mid).abs() / mid * 10000.0;
            fill.market_impact_bps = (impact * 100.0).round() / 100.0;
        }

        if fill.filled_quantity > 0 && fill.unfilled_quantity > 0 {
            Ok(OrderResult::PartialFill { order_id: order.id, fill })
        } else if fill.filled_quantity > 0 {
            Ok(OrderResult::FullFill { order_id: order.id, fill })
        } else {
            // 无成交（对手盘为空）
            Ok(OrderResult::PartialFill {
                order_id: order.id,
                fill: FillResult {
                    trades: Vec::new(),
                    vwap: 0.0,
                    filled_quantity: 0,
                    unfilled_quantity: order.quantity,
                    market_impact_bps: 0.0,
                    levels_consumed: 0,
                },
            })
        }
    }

    /// 提交限价单：
    /// - 如果价格优于当前对手盘最优价（买价 >= 最低卖价 或 卖价 <= 最高买价），立即匹配
    /// - 否则挂单进入订单簿，等待对手匹配
    pub fn submit_limit_order(&mut self, order: LimitOrder) -> Result<OrderResult, SimError> {
        if order.price <= 0 {
            return Err(SimError::InvalidPrice(order.price));
        }
        if order.quantity == 0 {
            return Err(SimError::InvalidQuantity);
        }

        match order.side {
            OrderSide::Buy => {
                // 买价 >= 最低卖价 → 可立即成交
                if let Some((lowest_ask, _)) = self.asks.first_key_value()
                    && order.price >= *lowest_ask
                {
                    return self.execute_immediate_limit(order);
                }
            },
            OrderSide::Sell => {
                // 卖价 <= 最高买价 → 可立即成交
                if let Some((Reverse(highest_bid), _)) = self.bids.first_key_value()
                    && order.price <= *highest_bid
                {
                    return self.execute_immediate_limit(order);
                }
            },
        }

        // 无法立即成交 → 挂单
        self.place_order(order)
    }

    /// 撤单：从订单簿中移除指定挂单
    ///
    /// 返回撤单时剩余的未成交数量。已全部成交的订单返回错误。
    ///
    /// 修复 P0-M8: 原 `level.orders.remove(idx).unwrap()` 在 level 已被其他
    /// match 路径清空时会 panic。改为返回 `SimError::OrderNotFound`。
    ///
    /// 修复 H3.3: 原实现先 `order_index.remove` 再检查 bids/asks，错误路径
    /// 会导致 order_index 中已删除但订单仍挂在 bids/asks，撤单永久失败。
    /// 改为：先用 `get` 查找 locator，所有可能失败的操作（level 查找、position 查找、
    /// remove）全部成功后，最后才 `order_index.remove`。
    pub fn cancel_order(&mut self, order_id: OrderId) -> Result<OrderResult, SimError> {
        // 修复 H3.3: 用 get 而非 remove，避免错误路径下 order_index 状态不一致
        let locator =
            self.order_index.get(&order_id).copied().ok_or(SimError::OrderNotFound(order_id))?;

        let remaining = match locator {
            OrderLocator::Bid(price) => {
                let level =
                    self.bids.get_mut(&Reverse(price)).ok_or(SimError::OrderNotFound(order_id))?;
                let pos = level.orders.iter().position(|o| o.id == order_id);
                if let Some(idx) = pos {
                    let order =
                        level.orders.remove(idx).ok_or(SimError::OrderNotFound(order_id))?;
                    level.total_quantity = level.total_quantity.saturating_sub(order.remaining());
                    if level.is_empty() {
                        self.bids.remove(&Reverse(price));
                    }
                    order.remaining()
                } else {
                    return Err(SimError::OrderNotFound(order_id));
                }
            },
            OrderLocator::Ask(price) => {
                let level = self.asks.get_mut(&price).ok_or(SimError::OrderNotFound(order_id))?;
                let pos = level.orders.iter().position(|o| o.id == order_id);
                if let Some(idx) = pos {
                    let order =
                        level.orders.remove(idx).ok_or(SimError::OrderNotFound(order_id))?;
                    level.total_quantity = level.total_quantity.saturating_sub(order.remaining());
                    if level.is_empty() {
                        self.asks.remove(&price);
                    }
                    order.remaining()
                } else {
                    return Err(SimError::OrderNotFound(order_id));
                }
            },
        };

        // 所有可能失败的操作已完成，安全地移除 order_index 条目
        self.order_index.remove(&order_id);
        Ok(OrderResult::Cancelled { order_id, remaining })
    }

    // ── 查询方法 ──

    /// 中间价 = (最优买价 + 最优卖价) / 2
    /// 如果只有单边盘，返回 None
    pub fn mid_price(&self) -> Option<f64> {
        let best_bid = self.best_bid_price()?;
        let best_ask = self.best_ask_price()?;
        Some((best_bid as f64 + best_ask as f64) / 2.0)
    }

    /// 价差 = 最优卖价 - 最优买价（单位：分）
    pub fn spread(&self) -> Option<Price> {
        let bid = self.best_bid_price()?;
        let ask = self.best_ask_price()?;
        Some(ask - bid)
    }

    /// 价差基点 = spread / mid_price × 10000
    pub fn spread_bps(&self) -> Option<f64> {
        let spread = self.spread()?;
        let mid = self.mid_price()?;
        if mid > 0.0 {
            Some((spread as f64 / mid) * 10000.0)
        } else {
            None
        }
    }

    /// 最优买价
    pub fn best_bid_price(&self) -> Option<Price> {
        self.bids.first_key_value().map(|(Reverse(p), _)| *p)
    }

    /// 最优卖价
    pub fn best_ask_price(&self) -> Option<Price> {
        self.asks.first_key_value().map(|(p, _)| *p)
    }

    /// 盘口深度快照（前 N 档）
    pub fn book_depth(&self, levels: usize) -> BookSnapshot {
        let bids: Vec<BookLevel> = self
            .bids
            .iter()
            .take(levels)
            .map(|(Reverse(price), level)| BookLevel {
                price: *price,
                total_quantity: level.total_quantity,
                order_count: level.order_count(),
            })
            .collect();

        let asks: Vec<BookLevel> = self
            .asks
            .iter()
            .take(levels)
            .map(|(price, level)| BookLevel {
                price: *price,
                total_quantity: level.total_quantity,
                order_count: level.order_count(),
            })
            .collect();

        let last_trade_price = self.trade_history.last().map(|t| t.price);

        BookSnapshot { bids, asks, last_trade_price, timestamp: self.current_time }
    }

    /// 订单簿完整统计
    pub fn stats(&self) -> BookStats {
        let bid_depth: Quantity = self.bids.values().map(|l| l.total_quantity).sum();
        let ask_depth: Quantity = self.asks.values().map(|l| l.total_quantity).sum();

        BookStats {
            mid_price: self.mid_price().unwrap_or(0.0),
            spread: self.spread().unwrap_or(0),
            spread_bps: self.spread_bps().unwrap_or(0.0),
            bid_depth,
            ask_depth,
            bid_levels: self.bids.len(),
            ask_levels: self.asks.len(),
            total_trades: self.trade_history.len(),
            last_trade_price: self.trade_history.last().map(|t| t.price),
        }
    }

    /// 预计市价单对市场的冲击成本
    ///
    /// 模拟下单 `quantity` 股后的冲击成本（不影响实际订单簿状态）。
    /// 返回 (成交均价, 冲击成本 bps)。
    pub fn market_impact_estimate(
        &self,
        side: OrderSide,
        quantity: Quantity,
    ) -> Option<(f64, f64)> {
        let mid = self.mid_price()?;
        let vwap = self.simulate_impact(side, quantity)?;
        let impact = ((vwap - mid).abs() / mid) * 10000.0;
        Some((vwap, impact))
    }

    /// 订单簿中的订单总数
    pub fn order_count(&self) -> usize {
        self.order_index.len()
    }

    /// 成交历史数量
    pub fn trade_count(&self) -> usize {
        self.trade_history.len()
    }

    /// 成交历史引用
    pub fn trade_history(&self) -> &[TradeRecord] {
        &self.trade_history
    }

    // ── 内部方法 ──

    /// 市价买单：扫描卖盘从低到高直到买完或卖盘清空
    fn match_against_asks(
        &mut self,
        buyer_order_id: OrderId,
        mut remaining: Quantity,
        buyer_agent_id: &str,
        fill: &mut FillResult,
    ) {
        let mut consumed_levels = 0;
        let price_keys: Vec<Price> = self.asks.keys().copied().collect();

        for price in price_keys {
            if remaining == 0 {
                break;
            }
            let level = match self.asks.get_mut(&price) {
                Some(l) => l,
                None => continue,
            };

            consumed_levels += 1;
            let mut self_trade_skips = 0;

            while remaining > 0 && !level.is_empty() {
                // 修复 P0-M7: 自成交保护——跳过但不删除对方订单。
                // 修复 H3.2: 原 break 会跳过整个 level，丢失同 level 后续非自成交订单。
                // 改为：将队首自成交订单 rotate 到队尾（pop_front + push_back），
                // continue 继续匹配队首的下一张订单。整 level 全自成交时跳出。
                let is_self =
                    level.orders.front().map(|o| o.agent_id == buyer_agent_id).unwrap_or(false);
                if is_self {
                    if let Some(order) = level.orders.pop_front() {
                        level.orders.push_back(order);
                    }
                    self_trade_skips += 1;
                    if self_trade_skips >= level.orders.len() {
                        break;
                    }
                    continue;
                }
                self_trade_skips = 0;

                let seller_remaining = level.orders.front().unwrap().remaining();
                let trade_qty = remaining.min(seller_remaining);

                let (seller_agent, seller_order_id) = {
                    let s = level.orders.front().unwrap();
                    (s.agent_id.clone(), s.id)
                };

                fill.trades.push(TradeRecord {
                    price,
                    quantity: trade_qty,
                    buyer_agent_id: buyer_agent_id.to_string(),
                    seller_agent_id: seller_agent,
                    buyer_order_id,
                    seller_order_id,
                    timestamp: self.current_time,
                });

                fill.filled_quantity += trade_qty;
                // 修复 L-7: u64 下溢防护（trade_qty = remaining.min(...) 理论上不会下溢，但防御性处理）。
                remaining = remaining.saturating_sub(trade_qty);

                // 更新卖单（独立作用域，避免借用冲突）
                let is_filled = {
                    let seller = level.orders.front_mut().unwrap();
                    seller.filled_quantity += trade_qty;
                    // 修复 L-7: u64 下溢防护，避免 total_quantity 与订单 remaining 不一致时 panic。
                    level.total_quantity = level.total_quantity.saturating_sub(trade_qty);
                    seller.is_filled()
                };

                if is_filled && let Some(removed) = level.orders.pop_front() {
                    self.order_index.remove(&removed.id);
                }
            }

            if level.is_empty() {
                self.asks.remove(&price);
            }
        }

        fill.levels_consumed = consumed_levels;
        fill.vwap = Self::compute_vwap(fill);
    }

    /// 市价卖单：扫描买盘从高到低直到卖完或买盘清空
    fn match_against_bids(
        &mut self,
        seller_order_id: OrderId,
        mut remaining: Quantity,
        seller_agent_id: &str,
        fill: &mut FillResult,
    ) {
        let mut consumed_levels = 0;
        let price_keys: Vec<Price> = self.bids.keys().map(|Reverse(p)| *p).collect();

        for price in price_keys {
            if remaining == 0 {
                break;
            }
            let level = match self.bids.get_mut(&Reverse(price)) {
                Some(l) => l,
                None => continue,
            };

            consumed_levels += 1;
            let mut self_trade_skips = 0;

            while remaining > 0 && !level.is_empty() {
                // 修复 P0-M7: 自成交保护——跳过但不删除对方订单
                // 修复 H3.2: rotate 到队尾继续匹配，整 level 全自成交时跳出
                let is_self =
                    level.orders.front().map(|o| o.agent_id == seller_agent_id).unwrap_or(false);
                if is_self {
                    if let Some(order) = level.orders.pop_front() {
                        level.orders.push_back(order);
                    }
                    self_trade_skips += 1;
                    if self_trade_skips >= level.orders.len() {
                        break;
                    }
                    continue;
                }
                self_trade_skips = 0;

                let buyer_remaining = level.orders.front().unwrap().remaining();
                let trade_qty = remaining.min(buyer_remaining);

                let (buyer_agent, buyer_oid) = {
                    let b = level.orders.front().unwrap();
                    (b.agent_id.clone(), b.id)
                };

                fill.trades.push(TradeRecord {
                    price,
                    quantity: trade_qty,
                    buyer_agent_id: buyer_agent,
                    seller_agent_id: seller_agent_id.to_string(),
                    buyer_order_id: buyer_oid,
                    seller_order_id,
                    timestamp: self.current_time,
                });

                fill.filled_quantity += trade_qty;
                // 修复 L-7: u64 下溢防护（trade_qty = remaining.min(...) 理论上不会下溢，但防御性处理）。
                remaining = remaining.saturating_sub(trade_qty);

                // 更新买单
                let is_filled = {
                    let buyer = level.orders.front_mut().unwrap();
                    buyer.filled_quantity += trade_qty;
                    // 修复 L-7: u64 下溢防护，避免 total_quantity 与订单 remaining 不一致时 panic。
                    level.total_quantity = level.total_quantity.saturating_sub(trade_qty);
                    buyer.is_filled()
                };

                if is_filled && let Some(removed) = level.orders.pop_front() {
                    self.order_index.remove(&removed.id);
                }
            }

            if level.is_empty() {
                self.bids.remove(&Reverse(price));
            }
        }

        fill.levels_consumed = consumed_levels;
        fill.vwap = Self::compute_vwap(fill);
    }

    /// 限价单立即成交：价格优于当前对手盘最优价
    fn execute_immediate_limit(&mut self, order: LimitOrder) -> Result<OrderResult, SimError> {
        let mid_before = self.mid_price();
        let mut fill = FillResult {
            trades: Vec::new(),
            vwap: 0.0,
            filled_quantity: 0,
            unfilled_quantity: 0,
            market_impact_bps: 0.0,
            levels_consumed: 0,
        };

        match order.side {
            OrderSide::Buy => {
                self.match_limited_asks(
                    order.id,
                    order.quantity,
                    order.price,
                    &order.agent_id,
                    &mut fill,
                );
            },
            OrderSide::Sell => {
                self.match_limited_bids(
                    order.id,
                    order.quantity,
                    order.price,
                    &order.agent_id,
                    &mut fill,
                );
            },
        }

        fill.vwap = Self::compute_vwap(&fill);
        let unfilled = order.quantity.saturating_sub(fill.filled_quantity);
        self.trade_history.extend(fill.trades.iter().cloned());
        // 修复 C4.5: 修剪成交历史，防止无界增长
        self.trim_trade_history();

        if let Some(mid) = mid_before
            && fill.vwap > 0.0
        {
            let impact = (fill.vwap - mid).abs() / mid * 10000.0;
            fill.market_impact_bps = (impact * 100.0).round() / 100.0;
        }

        if unfilled > 0 {
            // 限价单未完全成交：剩余部分挂入订单簿。
            // 修复 C4.4: 原 place_order_internal 不分配新 ID，直接使用传入 order.id；
            // 现在改为内部分配新 ID 并返回，需在 PartialFill/Placed 中返回该新 ID，
            // 否则 caller 拿到的 order_id 不指向任何订单。
            // 同时修复双重挂单 bug: 原代码在 unfilled>0 时挂一次剩余订单后，
            // 又在 else 分支再次挂单（"理论上不应该走到这里"），导致订单簿中出现两条相同订单。
            let mut remaining_order = order.clone();
            remaining_order.filled_quantity = fill.filled_quantity;
            let new_id = self.place_order_internal(remaining_order);

            if fill.filled_quantity > 0 {
                Ok(OrderResult::PartialFill { order_id: new_id, fill })
            } else {
                Ok(OrderResult::Placed { order_id: new_id })
            }
        } else {
            // 完全成交
            Ok(OrderResult::FullFill { order_id: order.id, fill })
        }
    }

    /// 限价买单：只匹配价格 <= limit_price 的卖盘
    fn match_limited_asks(
        &mut self,
        buyer_order_id: OrderId,
        mut remaining: Quantity,
        limit_price: Price,
        buyer_agent_id: &str,
        fill: &mut FillResult,
    ) {
        let price_keys: Vec<Price> = self.asks.keys().copied().collect();
        let mut consumed = 0;

        for price in price_keys {
            if price > limit_price || remaining == 0 {
                break;
            }
            let level = match self.asks.get_mut(&price) {
                Some(l) => l,
                None => continue,
            };
            consumed += 1;
            let mut self_trade_skips = 0;

            while remaining > 0 && !level.is_empty() {
                // 修复 P0-M7: 自成交保护——跳过但不删除对方订单
                // 修复 H3.2: rotate 到队尾继续匹配，整 level 全自成交时跳出
                let is_self =
                    level.orders.front().map(|o| o.agent_id == buyer_agent_id).unwrap_or(false);
                if is_self {
                    if let Some(order) = level.orders.pop_front() {
                        level.orders.push_back(order);
                    }
                    self_trade_skips += 1;
                    if self_trade_skips >= level.orders.len() {
                        break;
                    }
                    continue;
                }
                self_trade_skips = 0;

                let seller_remaining = level.orders.front().unwrap().remaining();
                let trade_qty = remaining.min(seller_remaining);

                let (seller_agent, seller_oid) = {
                    let s = level.orders.front().unwrap();
                    (s.agent_id.clone(), s.id)
                };

                fill.trades.push(TradeRecord {
                    price,
                    quantity: trade_qty,
                    buyer_agent_id: buyer_agent_id.to_string(),
                    seller_agent_id: seller_agent,
                    buyer_order_id,
                    seller_order_id: seller_oid,
                    timestamp: self.current_time,
                });

                fill.filled_quantity += trade_qty;
                // 修复 L-7: u64 下溢防护（trade_qty = remaining.min(...) 理论上不会下溢，但防御性处理）。
                remaining = remaining.saturating_sub(trade_qty);

                let is_filled = {
                    let seller = level.orders.front_mut().unwrap();
                    seller.filled_quantity += trade_qty;
                    // 修复 L-7: u64 下溢防护，避免 total_quantity 与订单 remaining 不一致时 panic。
                    level.total_quantity = level.total_quantity.saturating_sub(trade_qty);
                    seller.is_filled()
                };

                if is_filled && let Some(removed) = level.orders.pop_front() {
                    self.order_index.remove(&removed.id);
                }
            }

            if level.is_empty() {
                self.asks.remove(&price);
            }
        }

        fill.levels_consumed = consumed;
    }

    /// 限价卖单：只匹配价格 >= limit_price 的买盘
    fn match_limited_bids(
        &mut self,
        seller_order_id: OrderId,
        mut remaining: Quantity,
        limit_price: Price,
        seller_agent_id: &str,
        fill: &mut FillResult,
    ) {
        let price_keys: Vec<Price> = self.bids.keys().map(|Reverse(p)| *p).collect();
        let mut consumed = 0;

        for price in price_keys {
            if price < limit_price || remaining == 0 {
                break;
            }
            let level = match self.bids.get_mut(&Reverse(price)) {
                Some(l) => l,
                None => continue,
            };
            consumed += 1;
            let mut self_trade_skips = 0;

            while remaining > 0 && !level.is_empty() {
                // 修复 P0-M7: 自成交保护——跳过但不删除对方订单
                // 修复 H3.2: rotate 到队尾继续匹配，整 level 全自成交时跳出
                let is_self =
                    level.orders.front().map(|o| o.agent_id == seller_agent_id).unwrap_or(false);
                if is_self {
                    if let Some(order) = level.orders.pop_front() {
                        level.orders.push_back(order);
                    }
                    self_trade_skips += 1;
                    if self_trade_skips >= level.orders.len() {
                        break;
                    }
                    continue;
                }
                self_trade_skips = 0;

                let buyer_remaining = level.orders.front().unwrap().remaining();
                let trade_qty = remaining.min(buyer_remaining);

                let (buyer_agent, buyer_oid) = {
                    let b = level.orders.front().unwrap();
                    (b.agent_id.clone(), b.id)
                };

                fill.trades.push(TradeRecord {
                    price,
                    quantity: trade_qty,
                    buyer_agent_id: buyer_agent,
                    seller_agent_id: seller_agent_id.to_string(),
                    buyer_order_id: buyer_oid,
                    seller_order_id,
                    timestamp: self.current_time,
                });

                fill.filled_quantity += trade_qty;
                // 修复 L-7: u64 下溢防护（trade_qty = remaining.min(...) 理论上不会下溢，但防御性处理）。
                remaining = remaining.saturating_sub(trade_qty);

                let is_filled = {
                    let buyer = level.orders.front_mut().unwrap();
                    buyer.filled_quantity += trade_qty;
                    // 修复 L-7: u64 下溢防护，避免 total_quantity 与订单 remaining 不一致时 panic。
                    level.total_quantity = level.total_quantity.saturating_sub(trade_qty);
                    buyer.is_filled()
                };

                if is_filled && let Some(removed) = level.orders.pop_front() {
                    self.order_index.remove(&removed.id);
                }
            }

            if level.is_empty() {
                self.bids.remove(&Reverse(price));
            }
        }

        fill.levels_consumed = consumed;
    }

    /// 挂单（无条件挂入，不检查是否可立即成交）
    fn place_order(&mut self, order: LimitOrder) -> Result<OrderResult, SimError> {
        let new_id = self.place_order_internal(order);
        Ok(OrderResult::Placed { order_id: new_id })
    }

    /// 内部挂单实现
    ///
    /// 修复 C4.4: 原实现直接使用传入的 order.id，并让 place_order 返回 `self.id_counter - 1`。
    /// 由于此处未调用 next_order_id()，id_counter 没有递增，导致返回的 order_id 完全错误
    /// （可能指向不存在的订单或上一个分配出去的订单）。
    /// 改为：内部分配新 ID 覆盖传入的 order.id，并返回真实的新 order_id。
    fn place_order_internal(&mut self, mut order: LimitOrder) -> OrderId {
        let new_id = self.next_order_id();
        order.id = new_id;
        let side = order.side;
        let price = order.price;

        match side {
            OrderSide::Buy => {
                let level =
                    self.bids.entry(Reverse(price)).or_insert_with(|| PriceLevel::new(price));
                level.push(order);
                self.order_index.insert(new_id, OrderLocator::Bid(price));
            },
            OrderSide::Sell => {
                let level = self.asks.entry(price).or_insert_with(|| PriceLevel::new(price));
                level.push(order);
                self.order_index.insert(new_id, OrderLocator::Ask(price));
            },
        }
        new_id
    }

    /// 模拟冲击成本（不影响订单簿状态）
    fn simulate_impact(&self, side: OrderSide, quantity: Quantity) -> Option<f64> {
        let mut remaining = quantity;
        let mut total_cost: f64 = 0.0;
        let mut total_qty: u64 = 0;

        match side {
            OrderSide::Buy => {
                for (price, level) in self.asks.iter() {
                    if remaining == 0 {
                        break;
                    }
                    let take = remaining.min(level.total_quantity);
                    total_cost += (*price as f64) * (take as f64);
                    total_qty += take;
                    // 修复 L-7: u64 下溢防护。
                    remaining = remaining.saturating_sub(take);
                }
            },
            OrderSide::Sell => {
                for (Reverse(price), level) in self.bids.iter() {
                    if remaining == 0 {
                        break;
                    }
                    let take = remaining.min(level.total_quantity);
                    total_cost += (*price as f64) * (take as f64);
                    total_qty += take;
                    // 修复 L-7: u64 下溢防护。
                    remaining = remaining.saturating_sub(take);
                }
            },
        }

        if total_qty > 0 {
            Some(total_cost / total_qty as f64)
        } else {
            None
        }
    }

    /// 计算加权平均成交价
    fn compute_vwap(fill: &FillResult) -> f64 {
        let total_qty: u64 = fill.trades.iter().map(|t| t.quantity).sum();
        if total_qty == 0 {
            return 0.0;
        }
        let total_value: f64 =
            fill.trades.iter().map(|t| (t.price as f64) * (t.quantity as f64)).sum();
        total_value / total_qty as f64
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn make_limit(
        id: OrderId,
        side: OrderSide,
        price: Price,
        qty: Quantity,
        agent: &str,
    ) -> LimitOrder {
        LimitOrder {
            id,
            side,
            price,
            quantity: qty,
            filled_quantity: 0,
            timestamp: 0,
            agent_id: agent.to_string(),
        }
    }

    fn make_market(id: OrderId, side: OrderSide, qty: Quantity, agent: &str) -> MarketOrder {
        MarketOrder { id, side, quantity: qty, agent_id: agent.to_string(), timestamp: 0 }
    }

    #[test]
    fn test_empty_book() {
        let ob = OrderBook::new();
        assert_eq!(ob.mid_price(), None);
        assert_eq!(ob.spread(), None);
        assert_eq!(ob.order_count(), 0);
    }

    #[test]
    fn test_submit_limit_buy() {
        let mut ob = OrderBook::new();
        // 挂买单 @ 1000 分 (10.00 元)
        let order = make_limit(1, OrderSide::Buy, 1000, 100, "trader_a");
        let result = ob.submit_limit_order(order).unwrap();
        assert!(matches!(result, OrderResult::Placed { .. }));
        assert_eq!(ob.order_count(), 1);
        assert_eq!(ob.best_bid_price(), Some(1000));
        assert_eq!(ob.best_ask_price(), None);
    }

    #[test]
    fn test_submit_limit_sell() {
        let mut ob = OrderBook::new();
        let order = make_limit(1, OrderSide::Sell, 1000, 100, "trader_a");
        let result = ob.submit_limit_order(order).unwrap();
        assert!(matches!(result, OrderResult::Placed { .. }));
        assert_eq!(ob.best_ask_price(), Some(1000));
        assert_eq!(ob.best_bid_price(), None);
    }

    #[test]
    fn test_simple_cross() {
        let mut ob = OrderBook::new();

        // 挂卖单 @ 1000
        ob.submit_limit_order(make_limit(1, OrderSide::Sell, 1000, 50, "trader_b")).unwrap();

        // 挂买单 @ 1001（优于最低卖价 1000 → 立即成交）
        let result =
            ob.submit_limit_order(make_limit(2, OrderSide::Buy, 1001, 50, "trader_a")).unwrap();

        assert!(matches!(result, OrderResult::FullFill { .. }));
        if let OrderResult::FullFill { ref fill, .. } = result {
            assert_eq!(fill.filled_quantity, 50);
            assert_eq!(fill.vwap, 1000.0);
        }
        assert_eq!(ob.order_count(), 0); // 双方都成交，无挂单
    }

    #[test]
    fn test_partial_fill() {
        let mut ob = OrderBook::new();

        // 挂卖单 50 股 @ 1000
        ob.submit_limit_order(make_limit(1, OrderSide::Sell, 1000, 50, "trader_b")).unwrap();

        // 市价买 100 股 → 50 成交 + 50 无对手
        let result =
            ob.submit_market_order(make_market(2, OrderSide::Buy, 100, "trader_a")).unwrap();

        assert!(matches!(result, OrderResult::PartialFill { .. }));
        if let OrderResult::PartialFill { ref fill, .. } = result {
            assert_eq!(fill.filled_quantity, 50);
            assert_eq!(fill.unfilled_quantity, 50);
            assert_eq!(fill.vwap, 1000.0);
        }
    }

    #[test]
    fn test_multi_level_cross() {
        let mut ob = OrderBook::new();

        // 挂 3 档卖盘
        ob.submit_limit_order(make_limit(1, OrderSide::Sell, 1000, 30, "mm")).unwrap();
        ob.submit_limit_order(make_limit(2, OrderSide::Sell, 1001, 40, "mm")).unwrap();
        ob.submit_limit_order(make_limit(3, OrderSide::Sell, 1002, 50, "mm")).unwrap();

        // 市价买 80 股 → 30+40+10=80，恰好完全成交
        let result = ob.submit_market_order(make_market(4, OrderSide::Buy, 80, "trader")).unwrap();

        if let OrderResult::FullFill { ref fill, .. } = result {
            assert_eq!(fill.filled_quantity, 80);
            // vwap = (30*1000 + 40*1001 + 10*1002) / 80 = 1000.75
            let expected = (30.0 * 1000.0 + 40.0 * 1001.0 + 10.0 * 1002.0) / 80.0;
            assert!((fill.vwap - expected).abs() < 0.01);
            assert_eq!(fill.levels_consumed, 3);
        } else {
            panic!("Expected FullFill, got {:?}", result);
        }

        // 验证卖盘：1002 档应剩余 40 股
        let snapshot = ob.book_depth(5);
        assert_eq!(snapshot.asks.len(), 1);
        assert_eq!(snapshot.asks[0].price, 1002);
        assert_eq!(snapshot.asks[0].total_quantity, 40);
    }

    #[test]
    fn test_no_self_trade() {
        let mut ob = OrderBook::new();

        // 同一 agent 挂双边：卖单先进入订单簿，买单优于卖单但自成交禁止
        ob.submit_limit_order(make_limit(1, OrderSide::Sell, 1000, 50, "trader")).unwrap();
        ob.submit_limit_order(make_limit(2, OrderSide::Buy, 1001, 50, "trader")).unwrap();

        // 自成交保护设计为 rotate+保留：卖单 rotate 到队尾保留在订单簿，
        // 买单无对手方可成交 → 挂入 bid 端。最终订单簿同时存在买卖双边，无成交。
        assert_eq!(ob.order_count(), 2);
        assert_eq!(ob.best_bid_price(), Some(1001));
        assert_eq!(ob.best_ask_price(), Some(1000));
        assert_eq!(ob.trade_count(), 0);
    }

    #[test]
    fn test_no_self_trade_different_agents() {
        let mut ob = OrderBook::new();

        // 不同 agent 挂双边 → 正常成交
        ob.submit_limit_order(make_limit(1, OrderSide::Sell, 1000, 50, "agent_a")).unwrap();
        let result =
            ob.submit_limit_order(make_limit(2, OrderSide::Buy, 1001, 30, "agent_b")).unwrap();

        assert!(matches!(result, OrderResult::FullFill { .. }));
        assert_eq!(ob.order_count(), 1); // agent_a 剩余 20 股
        assert_eq!(ob.trade_count(), 1);
    }

    #[test]
    fn test_cancel_order() {
        let mut ob = OrderBook::new();

        ob.submit_limit_order(make_limit(1, OrderSide::Buy, 1000, 100, "trader")).unwrap();
        assert_eq!(ob.order_count(), 1);

        let result = ob.cancel_order(1).unwrap();
        assert!(matches!(result, OrderResult::Cancelled { .. }));
        if let OrderResult::Cancelled { order_id, remaining } = result {
            assert_eq!(order_id, 1);
            assert_eq!(remaining, 100);
        }
        assert_eq!(ob.order_count(), 0);
    }

    #[test]
    fn test_cancel_nonexistent() {
        let mut ob = OrderBook::new();
        let result = ob.cancel_order(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_mid_price_and_spread() {
        let mut ob = OrderBook::new();

        ob.submit_limit_order(make_limit(1, OrderSide::Buy, 995, 100, "trader")).unwrap();
        ob.submit_limit_order(make_limit(2, OrderSide::Sell, 1005, 100, "trader")).unwrap();

        assert!((ob.mid_price().unwrap() - 1000.0).abs() < 0.01);
        assert_eq!(ob.spread(), Some(10)); // 10 分 = 0.10 元
        // spread_bps = 10 / 1000 * 10000 ≈ 100
        let bps = ob.spread_bps().unwrap();
        assert!((bps - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_market_impact_estimate() {
        let mut ob = OrderBook::new();

        // 2 档卖盘
        ob.submit_limit_order(make_limit(1, OrderSide::Sell, 1000, 50, "mm")).unwrap();
        ob.submit_limit_order(make_limit(2, OrderSide::Sell, 1002, 100, "mm")).unwrap();
        // 买单制造中间价
        ob.submit_limit_order(make_limit(3, OrderSide::Buy, 998, 50, "trader")).unwrap();

        // mid = (998 + 1000) / 2 = 999
        let (vwap, impact) = ob.market_impact_estimate(OrderSide::Buy, 80).unwrap();
        // vwap = (50*1000 + 30*1002) / 80 = 1000.75
        let expected = (50.0 * 1000.0 + 30.0 * 1002.0) / 80.0;
        assert!((vwap - expected).abs() < 0.01);
        // impact = |1000.75 - 999| / 999 * 10000 ≈ 17.5 bps
        assert!(impact > 10.0);
    }

    #[test]
    fn test_order_id_counter() {
        let mut ob = OrderBook::new();
        assert_eq!(ob.next_order_id(), 1);
        assert_eq!(ob.next_order_id(), 2);
        assert_eq!(ob.next_order_id(), 3);
    }

    #[test]
    fn test_book_stats() {
        let mut ob = OrderBook::new();
        ob.submit_limit_order(make_limit(1, OrderSide::Buy, 995, 100, "a")).unwrap();
        ob.submit_limit_order(make_limit(2, OrderSide::Buy, 994, 200, "b")).unwrap();
        ob.submit_limit_order(make_limit(3, OrderSide::Sell, 1005, 150, "c")).unwrap();

        let stats = ob.stats();
        assert_eq!(stats.bid_depth, 300);
        assert_eq!(stats.ask_depth, 150);
        assert_eq!(stats.bid_levels, 2);
        assert_eq!(stats.ask_levels, 1);
        assert_eq!(stats.total_trades, 0);
    }
}
