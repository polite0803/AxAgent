//! 订单簿核心类型 — 自包含，不依赖 quant crate。
//!
//! 设计原则：
//! - 价格用 `Price` (i64, 分) 避免浮点精度问题（A 股以分为最小单位）
//! - 数量用 `Quantity` (u64, 股)
//! - 订单簿自主生成 OrderId，不依赖外部 ID 分配
//!
//! 与 `quant::types` 的关系：
//! - 本 crate 自包含，避免循环依赖
//! - Phase 2 会在兼容层做 `market_sim::Fill → quant::Fill` 桥接

use serde::{Deserialize, Serialize};

/// 价格（A 股以分为单位，整数避免浮点精度）
pub type Price = i64;

/// 数量（股）
pub type Quantity = u64;

/// 订单 ID（自动递增，全局唯一，用于快速撤单查找）
pub type OrderId = u64;

/// 模拟时间戳（纳秒，自模拟开始的偏移量）
pub type SimTimestamp = u64;

/// 订单方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// 限价单 —— 留在订单簿中等待匹配
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrder {
    pub id: OrderId,
    pub side: OrderSide,
    pub price: Price,
    pub quantity: Quantity,
    pub filled_quantity: Quantity,
    pub timestamp: SimTimestamp,
    pub agent_id: String,
}

impl LimitOrder {
    /// 剩余未成交数量
    pub fn remaining(&self) -> Quantity {
        self.quantity.saturating_sub(self.filled_quantity)
    }

    /// 是否已完全成交
    pub fn is_filled(&self) -> bool {
        self.remaining() == 0
    }
}

/// 市价单 —— 立即以最优对手价成交，不进入订单簿
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOrder {
    pub id: OrderId,
    pub side: OrderSide,
    pub quantity: Quantity,
    pub agent_id: String,
    pub timestamp: SimTimestamp,
}

/// 单笔成交记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub price: Price,
    pub quantity: Quantity,
    pub buyer_agent_id: String,
    pub seller_agent_id: String,
    pub buyer_order_id: OrderId,
    pub seller_order_id: OrderId,
    pub timestamp: SimTimestamp,
}

/// 订单簿档位快照（用于查询盘口深度）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: Price,
    pub total_quantity: Quantity,
    pub order_count: usize,
}

/// 订单簿快照（买盘 + 卖盘）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSnapshot {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    pub last_trade_price: Option<Price>,
    pub timestamp: SimTimestamp,
}

/// 市价单成交结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillResult {
    /// 各笔成交明细
    pub trades: Vec<TradeRecord>,
    /// 加权平均成交价
    pub vwap: f64,
    /// 已成交总数量
    pub filled_quantity: Quantity,
    /// 未成交数量（对手盘不足时）
    pub unfilled_quantity: Quantity,
    /// 冲击成本 = |vwap - 挂单前中间价| / 中间价
    pub market_impact_bps: f64,
    /// 消耗的对手盘档位数量
    pub levels_consumed: usize,
}

/// 订单簿统计指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookStats {
    pub mid_price: f64,
    pub spread: Price,
    pub spread_bps: f64,
    pub bid_depth: Quantity,
    pub ask_depth: Quantity,
    pub bid_levels: usize,
    pub ask_levels: usize,
    pub total_trades: usize,
    pub last_trade_price: Option<Price>,
}

/// 订单提交结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderResult {
    /// 限价单：已挂单（进入订单簿等待）
    Placed { order_id: OrderId },
    /// 限价单：立即部分成交
    PartialFill { order_id: OrderId, fill: FillResult },
    /// 限价单或市价单：完全成交
    FullFill { order_id: OrderId, fill: FillResult },
    /// 撤单成功
    Cancelled { order_id: OrderId, remaining: Quantity },
}
