// SPDX-License-Identifier: AGPL-3.0-only

//! price_alerts 表 ↔ RealtimeMonitor 双向转换（v203 数据模型对齐）
//!
//! ## 背景
//!
//! v203 迁移为 `price_alerts` 表新增 `alert_type` / `condition_type` / `threshold`
//! 三列，与 `RealtimeMonitor` 的 6 类 `alert_type` 对齐。本模块集中提供双向转换，
//! 替换 `monitor_emitter.rs` / `stock_analysis.rs` / `stock_workflow/core.rs`
//! 三处散落的硬编码 match（AGENTS.md Rule 12 禁止重复定义）。
//!
//! ## 权威模型
//!
//! 以 `RealtimeMonitor` 的 6 类 `alert_type` 为权威：
//! - `stop_loss` / `take_profit` / `resistance` / `support` → `condition_type=price`
//! - `change` → `condition_type=change_pct`
//! - `volume` → `condition_type=turnover_rate`

use crate::monitor::{MonitorAlert, MonitorConfig};

/// 6 类告警类型常量（与 RealtimeMonitor alert_type 字段对齐）
pub mod alert_types {
    pub const STOP_LOSS: &str = "stop_loss";
    pub const TAKE_PROFIT: &str = "take_profit";
    pub const RESISTANCE: &str = "resistance";
    pub const SUPPORT: &str = "support";
    pub const CHANGE: &str = "change";
    pub const VOLUME: &str = "volume";
}

/// 阈值语义类型
pub mod condition_types {
    pub const PRICE: &str = "price";
    pub const CHANGE_PCT: &str = "change_pct";
    pub const TURNOVER_RATE: &str = "turnover_rate";
}

/// 从 `MonitorAlert.alert_type` 推导 `condition_type`
///
/// - `stop_loss` / `take_profit` / `resistance` / `support` → `price`
/// - `change` → `change_pct`
/// - `volume` → `turnover_rate`
/// - 未知类型 → `price`（保守回退）
pub fn condition_type_for(alert_type: &str) -> &'static str {
    match alert_type {
        alert_types::CHANGE => condition_types::CHANGE_PCT,
        alert_types::VOLUME => condition_types::TURNOVER_RATE,
        _ => condition_types::PRICE,
    }
}

/// 从 `MonitorConfig` 提取已配置的告警类型 + 阈值列表
///
/// 返回 Vec<(alert_type, threshold)>，每个非 None 的 Option 字段对应一条
/// 用于在创建告警时一次性写入多行 price_alerts 记录（每种 alert_type 一行）。
pub fn extract_alerts_from_config(config: &MonitorConfig) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    if let Some(v) = config.stop_loss {
        out.push((alert_types::STOP_LOSS, v));
    }
    if let Some(v) = config.take_profit {
        out.push((alert_types::TAKE_PROFIT, v));
    }
    if let Some(v) = config.resistance_break {
        out.push((alert_types::RESISTANCE, v));
    }
    if let Some(v) = config.support_break {
        out.push((alert_types::SUPPORT, v));
    }
    if let Some(v) = config.change_pct_alert {
        out.push((alert_types::CHANGE, v));
    }
    if let Some(v) = config.turnover_rate_alert {
        out.push((alert_types::VOLUME, v));
    }
    out
}

/// 从 `MonitorAlert` 构造 price_alerts 的写入字段
///
/// 返回 `(alert_type, condition_type, threshold)` 三元组，用于构造 ActiveModel。
/// `threshold` 取 `current_price`（price 类）或 `change_pct`（change_pct 类），
/// 因为 MonitorAlert 不携带原始阈值，仅携带触发时的现价/涨跌幅。
///
/// **注意**：`threshold` 在此场景下是"触发时的值"，而非"用户设定的阈值"。
/// 调用方应优先用 `extract_alerts_from_config` 拿到原始阈值；本函数仅用于
/// monitor_emitter 在告警触发时回写 DB，标记"该告警在 X 价位被触发"。
pub fn alert_to_db_fields(alert: &MonitorAlert) -> (&'static str, &'static str, f64) {
    let cond_type = condition_type_for(&alert.alert_type);
    let threshold = match cond_type {
        condition_types::CHANGE_PCT => alert.change_pct,
        _ => alert.current_price,
    };
    // alert_type 已是 6 类标准值，直接 leak 到 'static
    // 安全性：6 类常量都是 'static，未知值保守视为 price 类
    let static_alert_type: &'static str = match alert.alert_type.as_str() {
        alert_types::STOP_LOSS => alert_types::STOP_LOSS,
        alert_types::TAKE_PROFIT => alert_types::TAKE_PROFIT,
        alert_types::RESISTANCE => alert_types::RESISTANCE,
        alert_types::SUPPORT => alert_types::SUPPORT,
        alert_types::CHANGE => alert_types::CHANGE,
        alert_types::VOLUME => alert_types::VOLUME,
        _ => alert_types::TAKE_PROFIT, // 未知类型保守回退
    };
    (static_alert_type, cond_type, threshold)
}

/// 老 condition 值 → 新 alert_type 的兼容映射
///
/// 用于读取老数据（condition 列有值、alert_type 列为 NULL 的迁移过渡期数据）。
/// v203 迁移已批量回填，但新代码仍应支持读老值兜底。
pub fn legacy_condition_to_alert_type(condition: &str) -> Option<&'static str> {
    match condition {
        "above" => Some(alert_types::TAKE_PROFIT),
        "below" => Some(alert_types::STOP_LOSS),
        "change_up" | "change_down" => Some(alert_types::CHANGE),
        "volume_spike" => Some(alert_types::VOLUME),
        _ => None,
    }
}

/// 从 price_alerts Model 反向构造 MonitorConfig 字段
///
/// 用于启动时从 DB 加载告警配置到 RealtimeMonitor。
/// 返回 `(field_name, value)`，调用方据此填充 MonitorConfig 的 6 个 Option 字段。
///
/// 优先读 `alert_type` + `threshold`；若 `alert_type` 为 NULL（老数据未回填），
/// 回退到 `legacy_condition_to_alert_type(condition)` + `target_price`。
pub fn db_model_to_config_field(
    alert_type: Option<&str>,
    condition_type: Option<&str>,
    threshold: Option<f64>,
    legacy_condition: &str,
    legacy_target_price: f64,
) -> Option<(&'static str, f64)> {
    // 优先走新字段
    if let Some(at) = alert_type {
        let static_at: &'static str = match at {
            alert_types::STOP_LOSS => alert_types::STOP_LOSS,
            alert_types::TAKE_PROFIT => alert_types::TAKE_PROFIT,
            alert_types::RESISTANCE => alert_types::RESISTANCE,
            alert_types::SUPPORT => alert_types::SUPPORT,
            alert_types::CHANGE => alert_types::CHANGE,
            alert_types::VOLUME => alert_types::VOLUME,
            _ => return None,
        };
        let value = threshold.unwrap_or(legacy_target_price);
        return Some((static_at, value));
    }
    // 回退到老 condition
    let static_at = legacy_condition_to_alert_type(legacy_condition)?;
    // 老数据只有 price 类阈值
    let _ = condition_type; // 老数据的 condition_type 为 NULL，忽略
    Some((static_at, legacy_target_price))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_type_for_returns_price_for_price_alerts() {
        assert_eq!(condition_type_for(alert_types::STOP_LOSS), condition_types::PRICE);
        assert_eq!(condition_type_for(alert_types::TAKE_PROFIT), condition_types::PRICE);
        assert_eq!(condition_type_for(alert_types::RESISTANCE), condition_types::PRICE);
        assert_eq!(condition_type_for(alert_types::SUPPORT), condition_types::PRICE);
    }

    #[test]
    fn condition_type_for_returns_change_pct_for_change() {
        assert_eq!(condition_type_for(alert_types::CHANGE), condition_types::CHANGE_PCT);
    }

    #[test]
    fn condition_type_for_returns_turnover_for_volume() {
        assert_eq!(condition_type_for(alert_types::VOLUME), condition_types::TURNOVER_RATE);
    }

    #[test]
    fn extract_alerts_from_config_returns_all_set_fields() {
        let config = MonitorConfig {
            stock_code: "000001".into(),
            stock_name: "test".into(),
            stop_loss: Some(9.0),
            take_profit: Some(11.0),
            resistance_break: None,
            support_break: Some(8.0),
            change_pct_alert: Some(3.0),
            turnover_rate_alert: None,
            enabled: true,
        };
        let alerts = extract_alerts_from_config(&config);
        assert_eq!(alerts.len(), 4);
        assert!(alerts.contains(&(alert_types::STOP_LOSS, 9.0)));
        assert!(alerts.contains(&(alert_types::TAKE_PROFIT, 11.0)));
        assert!(alerts.contains(&(alert_types::SUPPORT, 8.0)));
        assert!(alerts.contains(&(alert_types::CHANGE, 3.0)));
    }

    #[test]
    fn legacy_condition_to_alert_type_maps_above_below() {
        assert_eq!(legacy_condition_to_alert_type("above"), Some(alert_types::TAKE_PROFIT));
        assert_eq!(legacy_condition_to_alert_type("below"), Some(alert_types::STOP_LOSS));
        assert_eq!(legacy_condition_to_alert_type("change_up"), Some(alert_types::CHANGE));
        assert_eq!(legacy_condition_to_alert_type("volume_spike"), Some(alert_types::VOLUME));
        assert_eq!(legacy_condition_to_alert_type("unknown"), None);
    }

    #[test]
    fn db_model_to_config_field_prefers_new_fields() {
        // 新字段优先
        let r = db_model_to_config_field(
            Some(alert_types::RESISTANCE),
            Some(condition_types::PRICE),
            Some(15.5),
            "above",
            12.0,
        );
        assert_eq!(r, Some((alert_types::RESISTANCE, 15.5)));

        // 老字段回退
        let r = db_model_to_config_field(None, None, None, "below", 9.2);
        assert_eq!(r, Some((alert_types::STOP_LOSS, 9.2)));

        // 老字段未知值
        let r = db_model_to_config_field(None, None, None, "unknown", 1.0);
        assert_eq!(r, None);
    }
}
