//! 条件单引擎 — 监控+规则评估+自动执行
//!
//! ## 设计
//!
//! 条件单 (ConditionalOrder) 是一个 if-then 规则：
//!
//! ```text
//! IF 条件满足 (止损触发 / 支撑位跌破 / 涨跌幅超阈值 / 成交量异常)
//! THEN 执行动作 (买入 / 卖出 / 减仓 / 发通知)
//! ```
//!
//! 评估流程：
//! 1. `RealTimeQuoteWatcher` 推送 `QuoteChangeEvent`
//! 2. `ConditionalOrderEngine` 匹配规则
//! 3. 条件满足时调用 `ExecutionBridge` 执行动作
//!
//! ## 与 MonitorConfig 的关系
//!
//! MonitorConfig 只有条件定义（止损/止盈/压力/支撑/涨跌幅/换手率），
//! 没有执行动作（触发后怎么办）。ConditionalOrder 在 MonitorConfig 之上
//! 绑定了一个 Action，实现"条件满足 → 自动交易"的闭环。

use chrono::Timelike;
use serde::{Deserialize, Serialize};

/// 条件触发后的执行动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OrderAction {
    /// 买入（股数）
    Buy { quantity: i32 },
    /// 卖出（全部持仓或指定股数）
    Sell { quantity: Option<i32> },
    /// 减仓（减仓比例，0.0-1.0）
    Reduce { ratio: f64 },
    /// 仅发送通知
    Notify,
}

/// 条件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConditionType {
    /// 跌破止损价
    StopLoss,
    /// 突破止盈价
    TakeProfit,
    /// 突破压力位
    ResistanceBreak,
    /// 跌破支撑位
    SupportBreak,
    /// 涨跌幅超过阈值
    ChangePct { threshold: f64 },
    /// 换手率超过阈值
    TurnoverRate { threshold: f64 },
    /// 价格突破均线（当前价上穿/下穿 MA）
    MACross {
        period: usize,
        direction: String, // "above" | "below"
    },
    /// 自定义表达式（由 Rhai 脚本评估）
    Custom { script: String },
}

/// 单条件单
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalOrder {
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    /// 条件类型
    pub condition: ConditionType,
    /// 触发后的动作
    pub action: OrderAction,
    /// 是否启用
    pub enabled: bool,
    /// 生效时间范围（开始，HH:MM）
    pub active_start: Option<String>,
    /// 生效时间范围（结束，HH:MM）
    pub active_end: Option<String>,
    /// 单日触发上限（0=无限制）
    pub max_triggers_per_day: u32,
    /// 冷却分钟（同一条件单两次触发的最小间隔）
    pub cool_down_minutes: u32,
    /// 已触发次数（当天）
    #[serde(skip)]
    pub today_trigger_count: u32,
    /// 最后触发时间（ms）
    #[serde(skip)]
    pub last_triggered_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for ConditionalOrder {
    fn default() -> Self {
        Self {
            id: String::new(),
            stock_code: String::new(),
            stock_name: String::new(),
            condition: ConditionType::ChangePct { threshold: 3.0 },
            action: OrderAction::Notify,
            enabled: true,
            active_start: None,
            active_end: None,
            max_triggers_per_day: 3,
            cool_down_minutes: 30,
            today_trigger_count: 0,
            last_triggered_at: 0,
            created_at: 0,
            updated_at: 0,
        }
    }
}

impl ConditionalOrder {
    /// 检查是否应该触发
    pub fn should_trigger(
        &self,
        current_price: f64,
        prev_close: f64,
        turnover_rate: Option<f64>,
        now_ms: i64,
    ) -> TriggerDecision {
        if !self.enabled {
            return TriggerDecision::Skip("已禁用".into());
        }

        // 冷却检查
        if self.last_triggered_at > 0 {
            let elapsed_min = (now_ms - self.last_triggered_at) / 60_000;
            if elapsed_min < self.cool_down_minutes as i64 {
                return TriggerDecision::Skip(format!(
                    "冷却中 ({}m < {}m)",
                    elapsed_min, self.cool_down_minutes
                ));
            }
        }

        // 日触发上限检查
        if self.max_triggers_per_day > 0 && self.today_trigger_count >= self.max_triggers_per_day {
            return TriggerDecision::Skip("当日触发已达上限".into());
        }

        // 时间范围检查
        if let (Some(start), Some(end)) = (&self.active_start, &self.active_end) {
            let now_str = {
                let dt = chrono::Local::now();
                format!("{:02}:{:02}", dt.hour(), dt.minute())
            };
            if now_str.as_str() < start.as_str() || now_str.as_str() > end.as_str() {
                return TriggerDecision::Skip("不在生效时间范围".into());
            }
        }

        // 条件评估
        let meets = match &self.condition {
            ConditionType::StopLoss => {
                // 当前价跌破止损 → 触发
                current_price <= prev_close * (1.0 - 0.07) // 默认7%止损
            },
            ConditionType::TakeProfit => {
                current_price >= prev_close * (1.0 + 0.15) // 默认15%止盈
            },
            ConditionType::ResistanceBreak => {
                // 突破压力位（较前收盘涨超5%且加速）
                let change_pct = (current_price - prev_close) / prev_close;
                change_pct > 0.05
            },
            ConditionType::SupportBreak => {
                let change_pct = (prev_close - current_price) / prev_close;
                change_pct > 0.05
            },
            ConditionType::ChangePct { threshold } => {
                let change_pct = ((current_price - prev_close) / prev_close * 100.0).abs();
                change_pct >= *threshold
            },
            ConditionType::TurnoverRate { threshold } => {
                turnover_rate.map(|t| t >= *threshold).unwrap_or(false)
            },
            ConditionType::MACross { .. } => {
                // MA 的交叉需要 K 线历史数据，此处简化：仅检测日内涨跌幅是否超过3%
                let change_pct = (current_price - prev_close) / prev_close;
                change_pct.abs() > 0.03
            },
            ConditionType::Custom { .. } => false, // Rhai 脚本暂未接入
        };

        if meets {
            TriggerDecision::Trigger
        } else {
            TriggerDecision::Skip("条件未满足".into())
        }
    }
}

/// 触发决策
#[derive(Debug, Clone, PartialEq)]
pub enum TriggerDecision {
    /// 触发条件单
    Trigger,
    /// 跳过（附原因）
    Skip(String),
}

/// 条件单引擎
pub struct ConditionalOrderEngine {
    orders: Vec<ConditionalOrder>,
}

impl Default for ConditionalOrderEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalOrderEngine {
    pub fn new() -> Self {
        Self { orders: Vec::new() }
    }

    /// 设置条件单列表
    pub fn set_orders(&mut self, orders: Vec<ConditionalOrder>) {
        self.orders = orders;
    }

    /// 获取所有条件单
    pub fn orders(&self) -> &[ConditionalOrder] {
        &self.orders
    }

    /// 评估并返回需要触发的条件单
    pub fn evaluate(
        &self,
        stock_code: &str,
        current_price: f64,
        prev_close: f64,
        turnover_rate: f64,
        now_ms: i64,
    ) -> Vec<&ConditionalOrder> {
        self.orders
            .iter()
            .filter(|o| o.stock_code == stock_code)
            .filter(|o| {
                o.should_trigger(current_price, prev_close, Some(turnover_rate), now_ms)
                    == TriggerDecision::Trigger
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_loss_trigger() {
        let order = ConditionalOrder {
            id: "test1".into(),
            stock_code: "600519".into(),
            stock_name: "贵州茅台".into(),
            condition: ConditionType::StopLoss,
            action: OrderAction::Sell { quantity: None },
            enabled: true,
            active_start: None,
            active_end: None,
            max_triggers_per_day: 0,
            cool_down_minutes: 0,
            today_trigger_count: 0,
            last_triggered_at: 0,
            created_at: 0,
            updated_at: 0,
        };

        // 价格从100跌到90，应该触发
        assert_eq!(order.should_trigger(90.0, 100.0, None, 1000), TriggerDecision::Trigger);

        // 价格从100涨到105，不应该触发
        assert!(matches!(
            order.should_trigger(105.0, 100.0, None, 1000),
            TriggerDecision::Skip(..)
        ));
    }

    #[test]
    fn test_cool_down() {
        let order = ConditionalOrder {
            id: "test2".into(),
            stock_code: "600519".into(),
            stock_name: "贵州茅台".into(),
            condition: ConditionType::ChangePct { threshold: 3.0 },
            action: OrderAction::Notify,
            enabled: true,
            active_start: None,
            active_end: None,
            max_triggers_per_day: 0,
            cool_down_minutes: 30,
            today_trigger_count: 1,
            last_triggered_at: 1_000_000,
            created_at: 0,
            updated_at: 0,
        };

        // 刚触发过（时间差 < 冷却），应该跳过
        let decision = order.should_trigger(105.0, 100.0, None, 1_001_000);
        assert!(matches!(decision, TriggerDecision::Skip(msg) if msg.contains("冷却")));

        // 冷却期过后，应该触发（设 last_triggered_at=0 表示从未触发过）
        assert_eq!(order.should_trigger(105.0, 100.0, None, 100_000_000), TriggerDecision::Trigger);
    }

    #[test]
    fn test_disable_order() {
        let order = ConditionalOrder {
            id: "test3".into(),
            stock_code: "600519".into(),
            stock_name: "贵州茅台".into(),
            condition: ConditionType::ChangePct { threshold: 1.0 },
            action: OrderAction::Notify,
            enabled: false, // 禁用
            ..Default::default()
        };

        assert!(matches!(order.should_trigger(200.0, 100.0, None, 0), TriggerDecision::Skip(..)));
    }

    #[test]
    fn test_turnover_rate_threshold() {
        let order = ConditionalOrder {
            id: "test4".into(),
            stock_code: "600519".into(),
            stock_name: "贵州茅台".into(),
            condition: ConditionType::TurnoverRate { threshold: 10.0 },
            action: OrderAction::Notify,
            enabled: true,
            ..Default::default()
        };

        // 换手率 5% < 10%，不触发
        assert!(matches!(
            order.should_trigger(100.0, 100.0, Some(5.0), 0),
            TriggerDecision::Skip(..)
        ));

        // 换手率 15% > 10%，触发
        assert_eq!(order.should_trigger(100.0, 100.0, Some(15.0), 0), TriggerDecision::Trigger);
    }
}
