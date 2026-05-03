//! ComputerUseTool - 桌面控制和浏览器自动化工具

use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct ComputerUseTool;

#[async_trait]
impl Tool for ComputerUseTool {
    fn name(&self) -> &str {
        "ComputerUse"
    }
    fn description(&self) -> &str {
        "控制计算机桌面：截图、鼠标点击、键盘输入、滚动。适用于 GUI 自动化和浏览器交互。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["screenshot", "click", "type", "key", "scroll", "move"],
                    "description": "要执行的操作"
                },
                "coordinate": {
                    "type": "array",
                    "items": { "type": "number" },
                    "description": "坐标 [x, y]"
                },
                "text": { "type": "string", "description": "要输入的文本或按键" },
                "scroll_direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"]
                },
                "scroll_amount": { "type": "number", "default": 3 }
            },
            "required": ["action"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn check_permissions(&self, _input: &Value, _ctx: &ToolContext) -> PermissionResult {
        PermissionResult::Ask("桌面控制需要用户确认。".into())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input["action"].as_str().unwrap();

        let mut output = match action {
            "screenshot" => {
                // 调用屏幕截图
                capture_screenshot().await?
            },
            "click" => {
                let coord = input["coordinate"].as_array().ok_or_else(|| {
                    ToolError::invalid_input_for("ComputerUse", "click 需要 coordinate [x, y]")
                })?;
                let x = coord[0].as_f64().unwrap_or(0.0) as i32;
                let y = coord[1].as_f64().unwrap_or(0.0) as i32;
                format!("🖱️ 点击坐标: ({}, {})", x, y)
            },
            "type" => {
                let text = input["text"].as_str().unwrap_or("");
                format!("⌨️ 输入文本: {}", text)
            },
            "scroll" => {
                let dir = input["scroll_direction"].as_str().unwrap_or("down");
                let amount = input["scroll_amount"].as_f64().unwrap_or(3.0) as i32;
                format!("📜 滚动: {} x{}", dir, amount)
            },
            _ => {
                return Err(ToolError::invalid_input_for(
                    "ComputerUse",
                    format!("未知操作: {}", action),
                ))
            },
        };

        output.push_str("\n\n[桌面控制功能通过 AxAgent 后端执行]");
        Ok(ToolResult::success(output))
    }
}

async fn capture_screenshot() -> Result<String, ToolError> {
    #[cfg(target_os = "windows")]
    {
        use xcap::Monitor;
        let monitors = Monitor::all()
            .map_err(|e| ToolError::execution_failed_for("X", format!("获取显示器失败: {}", e)))?;
        if let Some(monitor) = monitors.first() {
            let image = monitor
                .capture_image()
                .map_err(|e| ToolError::execution_failed_for("X", format!("截图失败: {}", e)))?;
            let width = image.width();
            let height = image.height();

            // 编码为 PNG bytes
            let mut buf = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| {
                    ToolError::execution_failed_for("X", format!("编码截图失败: {}", e))
                })?;
            let bytes = buf.into_inner();

            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

            Ok(format!(
                "## 屏幕截图\n({}x{})\n\n![screenshot](data:image/png;base64,{})",
                width, height, b64
            ))
        } else {
            Err(ToolError::execution_failed_for(
                "ComputerUse",
                "未找到显示器",
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // macOS/Linux: 使用系统命令
        use std::process::Command;
        let tmp = std::env::temp_dir().join("axagent_screenshot.png");
        let tmp_str = tmp.to_string_lossy().to_string();

        let status = if cfg!(target_os = "macos") {
            Command::new("screencapture")
                .arg("-x")
                .arg(&tmp_str)
                .status()
        } else {
            Command::new("import")
                .arg("-window")
                .arg("root")
                .arg(&tmp_str)
                .status()
        };

        match status {
            Ok(s) if s.success() => {
                let bytes = std::fs::read(&tmp_str).map_err(|e| {
                    ToolError::execution_failed_for("X", format!("读取截图失败: {}", e))
                })?;
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let _ = std::fs::remove_file(&tmp_str);
                Ok(format!(
                    "## 屏幕截图\n\n![screenshot](data:image/png;base64,{})",
                    b64
                ))
            },
            _ => Err(ToolError::execution_failed_for(
                "ComputerUse",
                "截图命令执行失败",
            )),
        }
    }
}
