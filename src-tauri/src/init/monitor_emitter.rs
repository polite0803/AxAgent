// SPDX-License-Identifier: AGPL-3.0-only

//! P0: RealtimeMonitor → Tauri 前端 + DB 持久化 + 通知推送 桥接器
//!
//! 实现 `MonitorEventEmitter` trait，将 monitor.rs 的告警事件同时分发到三条通道：
//!   1. Tauri `app.emit("price-alert-triggered", ...)` — 前端 PriceAlertPanel 实时弹窗
//!   2. Tauri `app.emit("stock-monitor-alert", ...)` — 通用 monitor 事件（含 T+0 重跑请求）
//!   3. 写 `price_alerts` 表 `is_triggered=1` + `triggered_at=now` — 持久化触发历史
//!   4. 调用 `NotificationDispatcher::dispatch_alert` — 推送到 PushPlus/ServerChan/Ntfy/Gotify
//!
//! 设计要点：
//! - monitor.rs 的 `MonitorEventEmitter::emit(event, payload)` 是同步 trait，
//!   但 DB 写入和通知推送是异步操作。使用 `tokio::spawn` 在后台执行，避免阻塞 monitor 主循环。
//! - v203 后直接用 alert_type 匹配 price_alerts 表（6 类全部可持久化），
//!   替代旧的 6→2 降级映射 + 1% 容差匹配（change/volume 告警历史不再丢失）。
//! - emit 失败（如前端无监听器）不阻塞其他通道。

use std::sync::Arc;

use axagent_analysis_engine::monitor::MonitorEventEmitter;
use axagent_harness::AlertPayload;
use axagent_harness::AlertSeverity;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use serde_json::Value;
use tauri::Emitter;
use tracing::warn;

use axagent_entities::price_alerts;

/// 桥接 RealtimeMonitor 告警到 Tauri 前端 / DB / 通知推送
pub struct TauriMonitorEmitter {
    app: tauri::AppHandle,
    db: DatabaseConnection,
    notification_dispatcher: Arc<axagent_notification::NotificationDispatcher>,
}

impl TauriMonitorEmitter {
    pub fn new(
        app: tauri::AppHandle,
        db: DatabaseConnection,
        notification_dispatcher: Arc<axagent_notification::NotificationDispatcher>,
    ) -> Self {
        Self { app, db, notification_dispatcher }
    }
}

impl MonitorEventEmitter for TauriMonitorEmitter {
    fn emit(&self, event: &str, payload: Value) {
        let app = self.app.clone();
        let event_owned = event.to_string();

        // 即时 emit 到前端（非阻塞，失败仅 warn）
        if let Err(e) = app.emit(&event_owned, payload.clone()) {
            warn!("[monitor_emitter] emit {} 失败: {e}", event_owned);
        }

        // 兼容别名：stock-monitor-alert 同时 emit price-alert-triggered
        // 前端 PriceAlertPanel 监听 price-alert-triggered，monitor.rs emit 的是 stock-monitor-alert
        if event == "stock-monitor-alert" {
            if let Err(e) = app.emit("price-alert-triggered", payload.clone()) {
                warn!("[monitor_emitter] emit price-alert-triggered 别名失败: {e}");
            }

            // 告警事件：异步写 DB + 推送通知
            let db = self.db.clone();
            let dispatcher = self.notification_dispatcher.clone();
            let payload_clone = payload.clone();
            tokio::spawn(async move {
                handle_alert_event(&db, &dispatcher, &payload_clone).await;
            });
        }
    }
}

/// 处理 stock-monitor-alert 事件：
/// 1. 映射 alert_type → condition，更新 price_alerts 表的 is_triggered=1
/// 2. 调用 NotificationDispatcher 推送告警到已配置的渠道
async fn handle_alert_event(
    db: &DatabaseConnection,
    dispatcher: &axagent_notification::NotificationDispatcher,
    payload: &Value,
) {
    let stock_code = payload.get("stockCode").and_then(|v| v.as_str()).unwrap_or("");
    let stock_name = payload.get("stockName").and_then(|v| v.as_str()).unwrap_or("");
    let alert_type = payload.get("alertType").and_then(|v| v.as_str()).unwrap_or("");
    let alert_msg = payload.get("alertMessage").and_then(|v| v.as_str()).unwrap_or("");
    let suggested = payload.get("suggestedAction").and_then(|v| v.as_str());

    // ── 1. 更新 price_alerts 表 ──
    // v203 后直接用 alert_type 匹配，change/volume 类告警也能持久化。
    // 老 condition 字段不再用于查询，仅作为兼容回退。
    let now_ms = chrono::Utc::now().timestamp_millis();
    match price_alerts::Entity::find()
        .filter(price_alerts::Column::StockCode.eq(stock_code))
        .filter(price_alerts::Column::AlertType.eq(alert_type))
        .filter(price_alerts::Column::IsTriggered.eq(0))
        .all(db)
        .await
    {
        Ok(alerts) => {
            for alert in alerts {
                // v203 后 threshold 直接对应 alert_type 语义，无需 1% 容差匹配。
                // 任何匹配到的同 alert_type 未触发记录都标记为已触发。
                let mut active: price_alerts::ActiveModel = alert.into();
                active.is_triggered = Set(1);
                active.triggered_at = Set(Some(now_ms));
                active.updated_at = Set(now_ms);
                if let Err(e) = active.update(db).await {
                    warn!("[monitor_emitter] 更新 price_alerts 失败: {e}");
                }
            }
        },
        Err(e) => warn!("[monitor_emitter] 查询 price_alerts 失败: {e}"),
    }

    // ── 2. 推送通知 ──
    let severity = match alert_type {
        "stop_loss" | "support" => AlertSeverity::Critical,
        "take_profit" => AlertSeverity::Info,
        "resistance" => AlertSeverity::Info,
        "change" | "volume" => AlertSeverity::Warning,
        _ => AlertSeverity::Info,
    };

    let title = format!("告警 · {} ({})", stock_name, stock_code);
    let body = if let Some(s) = suggested {
        format!("{alert_msg}\n{s}")
    } else {
        alert_msg.to_string()
    };

    let alert_payload = AlertPayload {
        severity,
        title,
        body,
        stock_code: Some(stock_code.to_string()),
        generated_at: Utc::now(),
    };

    let _ = dispatcher.dispatch_alert(&alert_payload, Utc::now()).await;
}
