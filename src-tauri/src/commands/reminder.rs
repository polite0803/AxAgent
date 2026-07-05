// SPDX-License-Identifier: AGPL-3.0-only

//! Reminder manager — Tauri 命令层
//!
//! 提供提醒的增删改查、完成、贪睡、通知确认等操作。
//! ReminderManager 实例以 once_cell::sync::OnceLock 方式持有，线程安全。

use axagent_trajectory::{Reminder, ReminderManager, ReminderNotification};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::Mutex;

static REMINDER_MANAGER: OnceLock<Mutex<ReminderManager>> = OnceLock::new();

fn manager() -> &'static Mutex<ReminderManager> {
    REMINDER_MANAGER.get_or_init(|| Mutex::new(ReminderManager::new()))
}

// ── DTO ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ReminderListResult {
    pub active: Vec<ReminderItem>,
    pub completed: Vec<ReminderItem>,
    pub pending_notifications: Vec<ReminderNotificationItem>,
}

#[derive(Debug, Serialize)]
pub struct ReminderItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub scheduled_at: String,
    pub completed: bool,
    pub recurrence: Option<String>,
    pub created_at: String,
}

impl From<&Reminder> for ReminderItem {
    fn from(r: &Reminder) -> Self {
        Self {
            id: r.id.clone(),
            title: r.title.clone(),
            description: r.description.clone(),
            scheduled_at: r.scheduled_at.to_rfc3339(),
            completed: r.completed,
            recurrence: r
                .recurrence
                .as_ref()
                .map(|rec| format!("{:?}/{}", rec.frequency, rec.interval)),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ReminderNotificationItem {
    pub notification_id: String,
    pub reminder_id: String,
    pub reminder_title: String,
    pub triggered_at: String,
    pub acknowledged: bool,
}

impl From<&ReminderNotification> for ReminderNotificationItem {
    fn from(n: &ReminderNotification) -> Self {
        Self {
            notification_id: n.notification_id.clone(),
            reminder_id: n.reminder.id.clone(),
            reminder_title: n.reminder.title.clone(),
            triggered_at: n.triggered_at.to_rfc3339(),
            acknowledged: n.acknowledged,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateReminderInput {
    pub title: String,
    pub description: Option<String>,
    pub scheduled_at: String,
    pub recurrence_frequency: Option<String>, // "daily" | "weekly" | "monthly"
    pub recurrence_interval: Option<u32>,
}

// ── Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn reminder_create(input: CreateReminderInput) -> Result<ReminderItem, String> {
    let scheduled_at = DateTime::parse_from_rfc3339(&input.scheduled_at)
        .map_err(|e| format!("日期格式错误: {e}"))?
        .with_timezone(&Utc);

    let recurrence = match (input.recurrence_frequency.as_deref(), input.recurrence_interval) {
        (Some(freq), Some(interval)) if interval > 0 => {
            let frequency = match freq {
                "daily" => axagent_trajectory::RecurrenceFrequency::Daily,
                "weekly" => axagent_trajectory::RecurrenceFrequency::Weekly,
                "monthly" => axagent_trajectory::RecurrenceFrequency::Monthly,
                _ => return Err("recurrence_frequency 须为 daily/weekly/monthly".into()),
            };
            Some(axagent_trajectory::ReminderRecurrence {
                frequency,
                interval,
            })
        },
        _ => None,
    };

    let reminder = Reminder {
        id: axagent_trajectory::ProactiveAssistant::generate_reminder_id(),
        title: input.title,
        description: input.description.unwrap_or_default(),
        scheduled_at,
        completed: false,
        created_at: Utc::now(),
        recurrence,
    };

    let mut mgr = manager().lock().await;
    mgr.add_reminder(reminder.clone())
        .map_err(|e| format!("添加提醒失败: {e}"))?;

    Ok(ReminderItem::from(
        mgr.get_reminder(&reminder.id)
            .ok_or_else(|| String::from("添加后未找到提醒"))?,
    ))
}

#[tauri::command]
pub async fn reminder_list() -> Result<ReminderListResult, String> {
    let mgr = manager().lock().await;

    let active: Vec<ReminderItem> = mgr
        .get_active_reminders()
        .into_iter()
        .map(|r| r.into())
        .collect();
    let completed: Vec<ReminderItem> = mgr
        .get_completed_history()
        .iter()
        .map(|r| r.into())
        .collect();
    let pending_notifications: Vec<ReminderNotificationItem> = mgr
        .get_pending_notifications()
        .into_iter()
        .map(|n| n.into())
        .collect();

    Ok(ReminderListResult {
        active,
        completed,
        pending_notifications,
    })
}

#[tauri::command]
pub async fn reminder_complete(id: String) -> Result<ReminderItem, String> {
    let mut mgr = manager().lock().await;
    let r = mgr
        .complete_reminder(&id)
        .map_err(|e| format!("完成提醒失败: {e}"))?;
    Ok(ReminderItem::from(&r))
}

#[tauri::command]
pub async fn reminder_snooze(
    id: String,
    duration_minutes: Option<i64>,
) -> Result<ReminderItem, String> {
    let mut mgr = manager().lock().await;
    let r = mgr
        .snooze_reminder(&id, duration_minutes)
        .map_err(|e| format!("贪睡失败: {e}"))?;
    Ok(ReminderItem::from(&r))
}

#[tauri::command]
pub async fn reminder_delete(id: String) -> Result<(), String> {
    let mut mgr = manager().lock().await;
    mgr.delete_reminder(&id)
        .map_err(|e| format!("删除提醒失败: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn reminder_update(
    id: String,
    title: Option<String>,
    description: Option<String>,
    scheduled_at: Option<String>,
) -> Result<ReminderItem, String> {
    let scheduled = match scheduled_at {
        Some(s) => Some(
            DateTime::parse_from_rfc3339(&s)
                .map_err(|e| format!("日期格式错误: {e}"))?
                .with_timezone(&Utc),
        ),
        None => None,
    };

    let mut mgr = manager().lock().await;
    let r = mgr
        .update_reminder(&id, title, description, scheduled)
        .map_err(|e| format!("更新提醒失败: {e}"))?;
    Ok(ReminderItem::from(&r))
}

#[tauri::command]
pub async fn reminder_acknowledge(notification_id: String) -> Result<(), String> {
    let mut mgr = manager().lock().await;
    mgr.acknowledge_notification(&notification_id)
        .map_err(|e| format!("确认通知失败: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn reminder_cleanup() -> Result<u64, String> {
    let mut mgr = manager().lock().await;
    let before = mgr.get_completed_history().len();
    mgr.cleanup_completed();
    let after = mgr.get_completed_history().len();
    Ok((before - after) as u64)
}
