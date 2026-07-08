//! A 股市场微观结构撮合器
//!
//! 实现 A 股交易的核心微观结构：
//! - **T+1 结算**: 当日买入次日才能卖出（T+0 不可回转）
//! - **涨跌停限制**: 涨停不可买入（仅卖单可成交），跌停不可卖出（仅买单可成交）
//! - **印花税**: 仅卖出收取，默认 0.05%（万 5）
//! - **佣金**: 买卖双向收取，默认万 2.5（最低 5 元）
//! - **滑点**: 市价/限价单按 `slippage_rate` 计算（默认 0.05%）
//! - **整手校验**: A 股 100 股一手
//!
//! ## 设计原则：matcher 是 pure function（无内部状态）
//!
//! - `Matcher` 内部不维护 positions/cash（避免与 ctx 双源）
//! - `match_order` 接收 order + bar + position snapshot + cash snapshot，返回 Fill
//! - Engine 拿到 Fill 后自行更新 ctx.positions / ctx.cash
//!
//! 这样：
//! - matcher 单元测试容易（无需 mock state）
//! - Engine 状态变更可审计
//! - 复盘模式与实盘模式走同一路径

use serde::{Deserialize, Serialize};

use crate::ctx::Position;
use crate::types::{Bar, Fill, Order, OrderType, Side};

/// 撮合器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatcherConfig {
    /// 佣金费率（万 2.5 = 0.00025）
    pub commission_rate: f64,
    /// 最低佣金（元）
    pub commission_min: f64,
    /// 印花税率（仅卖出，默认 0.0005 = 万 5）
    pub stamp_tax_rate: f64,
    /// 滑点率（市价/限价均按此计算，默认 0.0005 = 0.05%）
    pub slippage_rate: f64,
    /// 整手股数（A 股 100）
    pub lot_size: u64,
    /// 是否强制 T+1（默认 true）
    pub t1_enforced: bool,
    /// 是否启用涨跌停校验（默认 true）
    pub limit_check: bool,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            commission_rate: 0.00025,
            commission_min: 5.0,
            stamp_tax_rate: 0.0005,
            slippage_rate: 0.0005,
            lot_size: 100,
            t1_enforced: true,
            limit_check: true,
        }
    }
}

/// A 股撮合器（pure function）
pub struct Matcher {
    pub config: MatcherConfig,
}

impl Matcher {
    pub fn new(config: MatcherConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(MatcherConfig::default())
    }

    /// 撮合订单
    ///
    /// 完整流程：
    /// 1. 整手校验
    /// 2. 涨跌停校验
    /// 3. T+1 校验（卖出时）
    /// 4. 计算成交价（限价按限价，市价按 bar.open）
    /// 5. 计算滑点
    /// 6. 计算手续费 + 印花税
    /// 7. 资金校验（买入时）
    ///
    /// 返回 Fill：matched=true 表示成交，matched=false 表示拒单/未成交
    pub fn match_order(
        &self,
        order: Order,
        bar: &Bar,
        position: Option<&Position>,
        cash: f64,
    ) -> Fill {
        // 1. 整手校验
        if order.quantity == 0 {
            return self.reject(order, bar, "数量为 0");
        }
        if self.config.lot_size > 0 && !order.quantity.is_multiple_of(self.config.lot_size) {
            return self.reject(
                order,
                bar,
                &format!("非整手（必须为 {} 的倍数）", self.config.lot_size),
            );
        }

        // 2. 涨跌停校验
        if self.config.limit_check {
            if matches!(order.side, Side::Long) && bar.is_limit_up() {
                return self.reject(order, bar, "涨停不可买入");
            }
            if matches!(order.side, Side::Short) && bar.is_limit_down() {
                return self.reject(order, bar, "跌停不可卖出");
            }
        }

        // 3. T+1 + 持仓校验
        if matches!(order.side, Side::Short) {
            let pos = match position {
                Some(p) => p,
                None => return self.reject(order, bar, "无持仓可卖"),
            };
            if self.config.t1_enforced && pos.entry_date == order.timestamp {
                return self.reject(order, bar, "T+1 当日不可卖出");
            }
            if order.quantity > pos.quantity {
                let qty = order.quantity;
                let held = pos.quantity;
                return self.reject(order, bar, &format!("卖出数量 {} 超过持仓 {}", qty, held));
            }
        }

        // 4. 计算成交价
        let raw_price = match order.order_type {
            OrderType::Market => bar.open,
            OrderType::Limit { price } => {
                if matches!(order.side, Side::Long) {
                    if price < bar.low {
                        return self.reject(order, bar, "限价买入未触及");
                    }
                    price.min(bar.high)
                } else {
                    if price > bar.high {
                        return self.reject(order, bar, "限价卖出未触及");
                    }
                    price.max(bar.low)
                }
            },
        };

        // 5. 滑点
        let slippage_per_share = raw_price * self.config.slippage_rate;
        let fill_price = match order.side {
            Side::Long => raw_price + slippage_per_share,
            Side::Short => raw_price - slippage_per_share,
            Side::Flat => raw_price,
        };
        let total_slippage = slippage_per_share * order.quantity as f64;
        let fill_amount = fill_price * order.quantity as f64;

        // 6. 手续费 + 印花税
        let commission =
            (fill_amount * self.config.commission_rate).max(self.config.commission_min);
        let stamp_tax = if matches!(order.side, Side::Short) {
            fill_amount * self.config.stamp_tax_rate
        } else {
            0.0
        };

        // 7. 资金校验（仅买入）
        if matches!(order.side, Side::Long) {
            let cash_required = fill_amount + commission + stamp_tax;
            if cash_required > cash + 1e-6 {
                return self.reject(
                    order,
                    bar,
                    &format!("资金不足：需要 {:.2}，可用 {:.2}", cash_required, cash),
                );
            }
        }

        Fill {
            order,
            fill_price,
            fill_amount,
            commission,
            stamp_tax,
            slippage: total_slippage,
            timestamp: bar.date.clone(),
            matched: true,
            reject_reason: None,
        }
    }

    fn reject(&self, order: Order, bar: &Bar, reason: &str) -> Fill {
        Fill {
            order,
            fill_price: 0.0,
            fill_amount: 0.0,
            commission: 0.0,
            stamp_tax: 0.0,
            slippage: 0.0,
            timestamp: bar.date.clone(),
            matched: false,
            reject_reason: Some(reason.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar(code: &str, date: &str, open: f64, high: f64, low: f64, close: f64) -> Bar {
        Bar {
            date: date.to_string(),
            code: code.to_string(),
            open,
            high,
            low,
            close,
            volume: 1_000_000.0,
            amount: close * 1_000_000.0,
            turnover_rate: Some(1.0),
            adj_factor: Some(1.0),
            limit_up: Some((close * 1.10 * 100.0).round() / 100.0),
            limit_down: Some((close * 0.90 * 100.0).round() / 100.0),
            is_st: false,
        }
    }

    fn make_buy_order(code: &str, qty: u64, date: &str) -> Order {
        Order {
            code: code.to_string(),
            side: Side::Long,
            quantity: qty,
            order_type: OrderType::Market,
            timestamp: date.to_string(),
            reason: "test".to_string(),
        }
    }

    fn make_position(code: &str, qty: u64, cost: f64, entry_date: &str) -> Position {
        Position {
            code: code.to_string(),
            name: None,
            side: Side::Long,
            quantity: qty,
            cost_basis: cost,
            last_price: cost,
            market_value: cost * qty as f64,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            entry_date: entry_date.to_string(),
            entry_timestamp: entry_date.to_string(),
        }
    }

    #[test]
    fn test_basic_buy_market_order() {
        let m = Matcher::with_defaults();
        let bar = make_bar("600519", "2025-01-15", 100.0, 105.0, 99.0, 103.0);
        let order = make_buy_order("600519", 100, "2025-01-15");
        let fill = m.match_order(order, &bar, None, 1_000_000.0);
        assert!(fill.matched);
        assert!(fill.fill_price > 100.0); // 含滑点
        assert!(fill.commission >= 5.0);
        assert_eq!(fill.stamp_tax, 0.0); // 买入无印花税
    }

    #[test]
    fn test_lot_size_enforced() {
        let m = Matcher::with_defaults();
        let bar = make_bar("600519", "2025-01-15", 100.0, 105.0, 99.0, 103.0);
        let order = make_buy_order("600519", 50, "2025-01-15"); // 50 股非整手
        let fill = m.match_order(order, &bar, None, 1_000_000.0);
        assert!(!fill.matched);
        assert!(fill.reject_reason.unwrap().contains("整手"));
    }

    #[test]
    fn test_limit_up_blocks_buy() {
        let m = Matcher::with_defaults();
        let mut bar = make_bar("600519", "2025-01-15", 110.0, 110.0, 105.0, 110.0);
        bar.limit_up = Some(110.0);
        bar.is_st = false;
        let order = make_buy_order("600519", 100, "2025-01-15");
        let fill = m.match_order(order, &bar, None, 1_000_000.0);
        assert!(!fill.matched);
        assert!(fill.reject_reason.unwrap().contains("涨停"));
    }

    #[test]
    fn test_limit_down_blocks_sell() {
        let m = Matcher::with_defaults();
        let mut bar = make_bar("600519", "2025-01-15", 90.0, 95.0, 90.0, 90.0);
        bar.limit_down = Some(90.0);
        bar.is_st = false;
        let pos = make_position("600519", 1000, 95.0, "2025-01-10");
        let order = Order {
            code: "600519".to_string(),
            side: Side::Short,
            quantity: 100,
            order_type: OrderType::Market,
            timestamp: "2025-01-15".to_string(),
            reason: "test".to_string(),
        };
        let fill = m.match_order(order, &bar, Some(&pos), 1_000_000.0);
        assert!(!fill.matched);
        assert!(fill.reject_reason.unwrap().contains("跌停"));
    }

    #[test]
    fn test_t1_blocks_same_day_sell() {
        let m = Matcher::with_defaults();
        let bar = make_bar("600519", "2025-01-15", 100.0, 105.0, 99.0, 103.0);
        let pos = make_position("600519", 1000, 100.0, "2025-01-15"); // 当日建仓
        let order = Order {
            code: "600519".to_string(),
            side: Side::Short,
            quantity: 100,
            order_type: OrderType::Market,
            timestamp: "2025-01-15".to_string(),
            reason: "test".to_string(),
        };
        let fill = m.match_order(order, &bar, Some(&pos), 1_000_000.0);
        assert!(!fill.matched);
        assert!(fill.reject_reason.unwrap().contains("T+1"));
    }

    #[test]
    fn test_t1_allows_next_day_sell() {
        let m = Matcher::with_defaults();
        let bar = make_bar("600519", "2025-01-16", 105.0, 110.0, 104.0, 108.0);
        let pos = make_position("600519", 1000, 100.0, "2025-01-15");
        let order = Order {
            code: "600519".to_string(),
            side: Side::Short,
            quantity: 100,
            order_type: OrderType::Market,
            timestamp: "2025-01-16".to_string(),
            reason: "test".to_string(),
        };
        let fill = m.match_order(order, &bar, Some(&pos), 1_000_000.0);
        assert!(fill.matched, "Reason: {:?}", fill.reject_reason);
        assert!(fill.stamp_tax > 0.0); // 卖出有印花税
    }

    #[test]
    fn test_limit_buy_matched_at_limit_price() {
        let m = Matcher::with_defaults();
        let bar = make_bar("600519", "2025-01-15", 100.0, 105.0, 95.0, 102.0);
        // 限价 96，在 [low=95, high=105] 范围内，可成交
        let order = Order {
            code: "600519".to_string(),
            side: Side::Long,
            quantity: 100,
            order_type: OrderType::Limit { price: 96.0 },
            timestamp: "2025-01-15".to_string(),
            reason: "test".to_string(),
        };
        let fill = m.match_order(order, &bar, None, 1_000_000.0);
        assert!(fill.matched);
        // 成交价 = 限价 + 滑点 ≈ 96 + 0.05%
        assert!((fill.fill_price - 96.0).abs() < 0.5);
    }

    #[test]
    fn test_limit_buy_unmatched_when_below_low() {
        let m = Matcher::with_defaults();
        let bar = make_bar("600519", "2025-01-15", 100.0, 105.0, 99.0, 102.0);
        // 限价 95 < low=99, 当日不可能触及
        let order = Order {
            code: "600519".to_string(),
            side: Side::Long,
            quantity: 100,
            order_type: OrderType::Limit { price: 95.0 },
            timestamp: "2025-01-15".to_string(),
            reason: "test".to_string(),
        };
        let fill = m.match_order(order, &bar, None, 1_000_000.0);
        assert!(!fill.matched);
    }

    #[test]
    fn test_insufficient_cash() {
        let m = Matcher::with_defaults();
        let bar = make_bar("600519", "2025-01-15", 100.0, 105.0, 99.0, 103.0);
        let order = make_buy_order("600519", 100, "2025-01-15");
        // 资金 100 元，买 100 股 100 元以上，资金不足
        let fill = m.match_order(order, &bar, None, 100.0);
        assert!(!fill.matched);
        assert!(fill.reject_reason.unwrap().contains("资金"));
    }

    #[test]
    fn test_chinese_calendar_dates() {
        // YYYY-MM-DD 字典序 = 时间序（撮合器 T+1 校验依赖此性质）
        assert!("2025-01-15" < "2025-01-16");
        assert!("2025-02-01" < "2025-02-15");
    }
}
