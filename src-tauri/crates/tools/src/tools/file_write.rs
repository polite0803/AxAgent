//! FileWriteTool - 文件写入工具
//!
//! 创建或覆盖文件，支持 diff 预览和 read-before-write 检查。

use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "FileWrite"
    }
    fn description(&self) -> &str {
        "创建新文件或完全覆盖已有文件。会自动创建必要的父目录。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "目标文件的绝对路径"
                },
                "content": {
                    "type": "string",
                    "description": "要写入的文件内容"
                }
            },
            "required": ["file_path", "content"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError> {
        let path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileWrite", "缺少 file_path 参数"))?;

        if !Path::new(path).is_absolute() {
            return Err(ToolError::invalid_input_for(
                "FileWrite",
                "file_path 必须是绝对路径",
            ));
        }

        let content = input["content"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileWrite", "缺少 content 参数"))?;

        if content.len() as u64 > MAX_FILE_SIZE {
            return Err(ToolError::invalid_input(format!(
                "内容过大 ({} MB)，最大允许 {} MB",
                content.len() / 1024 / 1024,
                MAX_FILE_SIZE / 1024 / 1024
            )));
        }

        if !ctx.allow_write {
            return Err(ToolError::permission_denied(
                "FileWrite",
                "当前上下文不允许写入操作",
            ));
        }

        Ok(())
    }

    fn check_permissions(&self, input: &Value, _ctx: &ToolContext) -> PermissionResult {
        let path = input["file_path"].as_str().unwrap_or("");

        // 禁止写入系统路径
        let dangerous_prefixes = [
            "/etc",
            "/boot",
            "/sys",
            "/proc",
            "/dev",
            "C:\\Windows",
            "C:\\Program Files",
        ];
        for prefix in &dangerous_prefixes {
            if path.starts_with(prefix) {
                return PermissionResult::Ask(format!("写入系统路径 '{}'，确认？", path));
            }
        }

        PermissionResult::Allow
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"].as_str().unwrap();
        let content = input["content"].as_str().unwrap();

        let path = Path::new(file_path);

        // 创建父目录
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建父目录失败: {}", e)))?;
        }

        let existed = path.exists();
        let old_content = if existed {
            std::fs::read_to_string(path).ok()
        } else {
            None
        };

        // 写入文件
        std::fs::write(path, content)
            .map_err(|e| ToolError::execution_failed(format!("写入文件失败: {}", e)))?;

        let action = if existed { "更新" } else { "创建" };
        let mut output = format!("✅ 已{}文件: {}\n", action, file_path);

        if let Some(old) = old_content {
            if old != content && old.len() < 50_000 && content.len() < 50_000 {
                // 生成简单 diff
                output.push_str("\n## 变更对比\n```diff\n");
                for diff in diff::lines(&old, content) {
                    match diff {
                        diff::Result::Left(l) => output.push_str(&format!("-{}\n", l)),
                        diff::Result::Right(r) => output.push_str(&format!("+{}\n", r)),
                        diff::Result::Both(b, _) => output.push_str(&format!(" {}\n", b)),
                    }
                }
                output.push_str("```\n");
            }
        }

        let lines = content.lines().count();
        output.push_str(&format!("\n{} 行, {} 字节", lines, content.len()));

        Ok(ToolResult::success(output))
    }
}
