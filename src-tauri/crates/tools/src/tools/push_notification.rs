//! PushNotificationTool - 系统通知

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct PushNotificationTool;

#[async_trait]
impl Tool for PushNotificationTool {
    fn name(&self) -> &str {
        "PushNotification"
    }
    fn description(&self) -> &str {
        "向用户发送系统桌面通知。适用于后台任务完成提醒。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "通知标题" },
                "body": { "type": "string", "description": "通知正文" },
                "urgency": { "type": "string", "enum": ["low", "normal", "critical"], "default": "normal" }
            },
            "required": ["title", "body"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let title = input["title"].as_str().unwrap_or("AxAgent 通知");
        let body = input["body"].as_str().unwrap_or("");

        // 通过系统通知 API 发送
        #[cfg(target_os = "windows")]
        {
            // Windows 通知通过 Tauri notification API
            let _ = std::process::Command::new("powershell")
                .arg("-Command")
                .arg(format!("[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null; New-BurntToastNotification -Text '{}', '{}'", title, body))
                .output();
        }

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    r#"display notification "{}" with title "{}""#,
                    body, title
                ))
                .output();
        }

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("notify-send")
                .arg(title)
                .arg(body)
                .output();
        }

        Ok(ToolResult::success(format!("🔔 通知已发送: {}", title)))
    }
}
