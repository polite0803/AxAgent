// SPDX-License-Identifier: AGPL-3.0-only

//! ComputerUseTool - 桌面控制和浏览器自动化工具

use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_core::computer_control;
use serde_json::Value;

pub struct ComputerUseTool;

#[async_trait]
impl Tool for ComputerUseTool {
    fn name(&self) -> &str {
        "ComputerUse"
    }
    fn description(&self) -> &str {
        "控制计算机桌面：截图、鼠标点击、键盘输入、滚动、鼠标移动。适用于 GUI 自动化和浏览器交互。"
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
                "text": { "type": "string", "description": "要输入的文本或按键名称" },
                "modifiers": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["ctrl", "alt", "shift", "meta"] },
                    "description": "修饰键列表（仅 key 操作使用）"
                },
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
        ToolCategory::Desktop
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn check_permissions(&self, _input: &Value, _ctx: &ToolContext) -> PermissionResult {
        PermissionResult::Ask("桌面控制需要用户确认。".into())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = self.name();
        let action = input["action"].as_str().unwrap_or("screenshot");

        let output = match action {
            "screenshot" => {
                let result = computer_control::screen_capture(None, None, None)
                    .await
                    .map_err(|e| ToolError::execution_failed_for(name, e.to_string()))?;
                let image_base64 = result["image_base64"].as_str().unwrap_or("");
                let width = result["width"].as_u64().unwrap_or(0);
                let height = result["height"].as_u64().unwrap_or(0);
                format!(
                    "## 屏幕截图\n({}x{})\n\n![screenshot](data:image/png;base64,{})",
                    width, height, image_base64
                )
            },
            "click" => {
                let coord = input["coordinate"].as_array().ok_or_else(|| {
                    ToolError::invalid_input_for(name, "click 需要 coordinate [x, y]")
                })?;
                let x = coord[0].as_f64().unwrap_or(0.0);
                let y = coord[1].as_f64().unwrap_or(0.0);
                let button = input["button"].as_str().unwrap_or("left");
                computer_control::mouse_click(x, y, Some(button.to_string()))
                    .await
                    .map_err(|e| ToolError::execution_failed_for(name, e.to_string()))?;
                format!("鼠标点击: ({}, {}) [{}]", x, y, button)
            },
            "type" => {
                let text = input["text"]
                    .as_str()
                    .ok_or_else(|| ToolError::invalid_input_for(name, "type 需要 text 参数"))?;
                let x = input["coordinate"]
                    .as_array()
                    .and_then(|c| c.first()?.as_f64());
                let y = input["coordinate"]
                    .as_array()
                    .and_then(|c| c.get(1)?.as_f64());
                computer_control::type_text(text.to_string(), x, y)
                    .await
                    .map_err(|e| ToolError::execution_failed_for(name, e.to_string()))?;
                match (x, y) {
                    (Some(cx), Some(cy)) => {
                        format!("在 ({}, {}) 输入文本: {}", cx, cy, text)
                    },
                    _ => format!("输入文本: {}", text),
                }
            },
            "key" => {
                let key = input["text"].as_str().ok_or_else(|| {
                    ToolError::invalid_input_for(name, "key 需要 text 参数（按键名称）")
                })?;
                let modifiers: Vec<String> = input["modifiers"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let desc = if modifiers.is_empty() {
                    format!("按键: {}", key)
                } else {
                    format!("组合键: {}+{}", modifiers.join("+"), key)
                };
                computer_control::press_key(key.to_string(), modifiers)
                    .await
                    .map_err(|e| ToolError::execution_failed_for(name, e.to_string()))?;
                desc
            },
            "scroll" => {
                let coord = input["coordinate"].as_array().ok_or_else(|| {
                    ToolError::invalid_input_for(name, "scroll 需要 coordinate [x, y]")
                })?;
                let x = coord[0].as_f64().unwrap_or(0.0);
                let y = coord[1].as_f64().unwrap_or(0.0);
                let dir = input["scroll_direction"].as_str().unwrap_or("down");
                let amount = input["scroll_amount"].as_f64().unwrap_or(3.0) as i32;
                let delta = match dir {
                    "up" => amount,
                    "down" => -amount,
                    _ => -amount,
                };
                computer_control::mouse_scroll(x, y, delta)
                    .await
                    .map_err(|e| ToolError::execution_failed_for(name, e.to_string()))?;
                format!("滚动: {} x{} (at {}, {})", dir, amount, x, y)
            },
            "move" => {
                let coord = input["coordinate"].as_array().ok_or_else(|| {
                    ToolError::invalid_input_for(name, "move 需要 coordinate [x, y]")
                })?;
                let x = coord[0].as_f64().unwrap_or(0.0);
                let y = coord[1].as_f64().unwrap_or(0.0);
                computer_control::mouse_move(x, y)
                    .await
                    .map_err(|e| ToolError::execution_failed_for(name, e.to_string()))?;
                format!("鼠标移动到: ({}, {})", x, y)
            },
            _ => {
                return Err(ToolError::invalid_input_for(
                    name,
                    format!(
                        "未知操作: {}。支持: screenshot, click, type, key, scroll, move",
                        action
                    ),
                ));
            },
        };

        Ok(ToolResult::success(output))
    }
}
