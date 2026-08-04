// SPDX-License-Identifier: AGPL-3.0-only

//! Reminder manager — Tauri 命令层
//!
//! 提供提醒的增删改查、完成、贪睡、通知确认等操作。
//! ReminderManager 实例以 once_cell::sync::OnceLock 方式持有，线程安全。
//! 数据持久化到 JSON 文件，应用重启后自动恢复。

use agent_macro::agent_command;

use axagent_trajectory::{Reminder, ReminderManager, ReminderNotification, ReminderRecurrence};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::Mutex;

static REMINDER_MANAGER: OnceLock<Mutex<ReminderManager>> = OnceLock::new();

/// 获取提醒数据文件路径
fn data_file_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".axagent").join("data").join("reminders.json")
}

async fn init_manager() -> &'static Mutex<ReminderManager> {
    REMINDER_MANAGER.get_or_init(|| {
        let path = data_file_path();
        let mgr = ReminderManager::load_from_file(&path).unwrap_or_else(|e| {
            tracing::warn!("加载提醒数据失败，使用空状态: {e}");
            ReminderManager::new()
        });
        Mutex::new(mgr)
    })
}

async fn manager() -> &'static Mutex<ReminderManager> {
    init_manager().await
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
    pub recurrence: Option<ReminderRecurrence>,
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
            recurrence: r.recurrence.clone(),
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

#[agent_command(domain = reminder, safety = Caution, call_mode = StateInput, description = "创建新提醒")]
#[tauri::command]
pub async fn reminder_create(input: CreateReminderInput) -> Result<ReminderItem, String> {
    // 输入验证
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(String::from(crate::commands::error::ErrorResponse::from_error(
            "提醒标题不能为空".to_string(),
            crate::commands::error::ErrorCategory::Validation,
        )));
    }
    if title.len() > 200 {
        return Err(String::from(crate::commands::error::ErrorResponse::from_error(
            "提醒标题不能超过 200 个字符".to_string(),
            crate::commands::error::ErrorCategory::Validation,
        )));
    }

    let scheduled_at = DateTime::parse_from_rfc3339(&input.scheduled_at)
        .map_err(|e| format!("日期格式错误: {e}"))?
        .with_timezone(&Utc);

    // 允许过去时间（作为立即提醒），但警告
    if scheduled_at < Utc::now() {
        tracing::warn!(
            title = %title,
            scheduled_at = %scheduled_at,
            "提醒时间已过期，将立即触发"
        );
    }

    let recurrence = match (input.recurrence_frequency.as_deref(), input.recurrence_interval) {
        (Some(freq), Some(interval)) if interval > 0 => {
            let frequency = match freq {
                "daily" => axagent_trajectory::RecurrenceFrequency::Daily,
                "weekly" => axagent_trajectory::RecurrenceFrequency::Weekly,
                "monthly" => axagent_trajectory::RecurrenceFrequency::Monthly,
                _ => {
                    return Err(String::from(crate::commands::error::ErrorResponse::from_error(
                        "recurrence_frequency 须为 daily/weekly/monthly".to_string(),
                        crate::commands::error::ErrorCategory::Validation,
                    )));
                },
            };
            Some(axagent_trajectory::ReminderRecurrence { frequency, interval })
        },
        _ => None,
    };

    let reminder = Reminder {
        id: axagent_trajectory::ProactiveAssistant::generate_reminder_id(),
        title,
        description: input.description.unwrap_or_default().trim().to_string(),
        scheduled_at,
        completed: false,
        created_at: Utc::now(),
        recurrence,
    };

    let mut mgr = manager().await.lock().await;
    mgr.add_reminder(reminder.clone()).map_err(|e| format!("添加提醒失败: {e}"))?;

    let result = ReminderItem::from(
        mgr.get_reminder(&reminder.id).ok_or_else(|| String::from("添加后未找到提醒"))?,
    );
    let _ = mgr.save_to_file(&data_file_path());
    Ok(result)
}

#[agent_command(domain = reminder, safety = Safe, call_mode = StateOnly, description = "列出所有提醒")]
#[tauri::command]
pub async fn reminder_list() -> Result<ReminderListResult, String> {
    let mgr = manager().await.lock().await;

    let active: Vec<ReminderItem> =
        mgr.get_active_reminders().into_iter().map(|r| r.into()).collect();
    let completed: Vec<ReminderItem> =
        mgr.get_completed_history().iter().map(|r| r.into()).collect();
    let pending_notifications: Vec<ReminderNotificationItem> =
        mgr.get_pending_notifications().into_iter().map(|n| n.into()).collect();

    Ok(ReminderListResult { active, completed, pending_notifications })
}

#[agent_command(domain = reminder, safety = Caution, call_mode = StateInput, description = "标记提醒为已完成")]
#[tauri::command]
pub async fn reminder_complete(id: String) -> Result<ReminderItem, String> {
    let mut mgr = manager().await.lock().await;
    let r = mgr.complete_reminder(&id).map_err(|e| format!("完成提醒失败: {e}"))?;
    let _ = mgr.save_to_file(&data_file_path());
    Ok(ReminderItem::from(&r))
}

#[agent_command(domain = reminder, safety = Caution, call_mode = StateInput, description = "推迟提醒时间（贪睡）")]
#[tauri::command]
pub async fn reminder_snooze(
    id: String,
    duration_minutes: Option<i64>,
) -> Result<ReminderItem, String> {
    let mut mgr = manager().await.lock().await;
    let r = mgr.snooze_reminder(&id, duration_minutes).map_err(|e| format!("贪睡失败: {e}"))?;
    let _ = mgr.save_to_file(&data_file_path());
    Ok(ReminderItem::from(&r))
}

#[agent_command(domain = reminder, safety = Dangerous, call_mode = StateInput, description = "删除指定提醒")]
#[tauri::command]
pub async fn reminder_delete(id: String) -> Result<(), String> {
    let mut mgr = manager().await.lock().await;
    mgr.delete_reminder(&id).map_err(|e| format!("删除提醒失败: {e}"))?;
    let _ = mgr.save_to_file(&data_file_path());
    Ok(())
}

#[agent_command(domain = reminder, safety = Caution, call_mode = StateInput, description = "更新提醒内容")]
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

    let mut mgr = manager().await.lock().await;
    let r = mgr
        .update_reminder(&id, title, description, scheduled)
        .map_err(|e| format!("更新提醒失败: {e}"))?;
    let _ = mgr.save_to_file(&data_file_path());
    Ok(ReminderItem::from(&r))
}

#[derive(Debug, Serialize)]
pub struct AcknowledgeResult {
    pub acknowledged: bool,
    pub notification_id: String,
}

#[agent_command(domain = reminder, safety = Caution, call_mode = StateInput, description = "确认通知已读")]
#[tauri::command]
pub async fn reminder_acknowledge(notification_id: String) -> Result<AcknowledgeResult, String> {
    let mut mgr = manager().await.lock().await;
    mgr.acknowledge_notification(&notification_id).map_err(|e| format!("确认通知失败: {e}"))?;
    let _ = mgr.save_to_file(&data_file_path());
    Ok(AcknowledgeResult { acknowledged: true, notification_id })
}

#[agent_command(domain = reminder, safety = Caution, call_mode = StateOnly, description = "清理已完成的提醒历史")]
#[tauri::command]
pub async fn reminder_cleanup() -> Result<u64, String> {
    let mut mgr = manager().await.lock().await;
    let before = mgr.get_completed_history().len();
    mgr.cleanup_completed();
    let after = mgr.get_completed_history().len();
    let _ = mgr.save_to_file(&data_file_path());
    Ok((before - after) as u64)
}
