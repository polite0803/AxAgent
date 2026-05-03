//! FileEditTool - 文件编辑工具
//!
//! 基于搜索替换的精确文件编辑，支持 replace_all 批量替换。

use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "FileEdit"
    }
    fn description(&self) -> &str {
        "精确编辑文件。通过 old_string/new_string 搜索替换。\
         若 old_string 匹配多次，需要设置 replace_all: true。\
         old_string 必须精确匹配文件内容（包括缩进、空行）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "要编辑的文件绝对路径"
                },
                "old_string": {
                    "type": "string",
                    "description": "要替换的文本（精确匹配）"
                },
                "new_string": {
                    "type": "string",
                    "description": "替换后的文本"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "是否替换所有匹配项（默认 false）",
                    "default": false
                }
            },
            "required": ["file_path", "old_string", "new_string"]
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
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 file_path"))?;

        if !Path::new(path).is_absolute() {
            return Err(ToolError::invalid_input_for(
                "FileEdit",
                "file_path 必须是绝对路径",
            ));
        }

        let old = input["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 old_string"))?;
        let new = input["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 new_string"))?;

        if old.is_empty() {
            return Err(ToolError::invalid_input_for(
                "FileEdit",
                "old_string 不能为空",
            ));
        }

        if old == new {
            return Err(ToolError::invalid_input(
                "old_string 和 new_string 相同，无需编辑",
            ));
        }

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if file_size > MAX_FILE_SIZE {
            return Err(ToolError::invalid_input_for(
                "FileEdit",
                format!(
                    "文件过大 ({} MB)，最大 {} MB",
                    file_size / 1024 / 1024,
                    MAX_FILE_SIZE / 1024 / 1024
                ),
            ));
        }

        if !ctx.allow_write {
            return Err(ToolError::permission_denied(
                "FileEdit",
                "当前上下文不允许写入操作",
            ));
        }

        Ok(())
    }

    fn check_permissions(&self, _input: &Value, _ctx: &ToolContext) -> PermissionResult {
        PermissionResult::Allow
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"].as_str().unwrap();
        let old_string = input["old_string"].as_str().unwrap();
        let new_string = input["new_string"].as_str().unwrap();
        let replace_all = input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let original = std::fs::read_to_string(file_path).map_err(|e| {
            ToolError::execution_failed_for("FileEdit", format!("读取文件失败: {}", e))
        })?;

        // 查找匹配次数
        let matches = original.matches(old_string).count();

        if matches == 0 {
            // 尝试规范化引号后重新匹配
            let normalized_old = normalize_quotes(old_string);
            if normalized_old != old_string {
                let matches_norm = original.matches(&normalized_old).count();
                if matches_norm == 0 {
                    return Err(ToolError::invalid_input_for("FileEdit", "在文件中未找到 old_string。请确认 old_string 与文件内容完全一致（包括空格和缩进）。\n已尝试引号规范化。".to_string()));
                }
                // 使用规范化后的字符串
                let new_content = if replace_all {
                    original.replace(&normalized_old, new_string)
                } else {
                    original.replacen(&normalized_old, new_string, 1)
                };
                return write_and_diff(file_path, &original, &new_content, matches_norm);
            }
            return Err(ToolError::invalid_input(
                "在文件中未找到 old_string。请确认 old_string 与文件内容完全一致（包括空格和缩进）。"
            ));
        }

        if matches > 1 && !replace_all {
            return Err(ToolError::invalid_input_for("FileEdit", format!(
                "old_string 匹配了 {} 次（非唯一匹配）。请设置 replace_all: true 替换所有匹配项，或提供更多上下文使匹配唯一。",
                matches
            )));
        }

        let new_content = if replace_all {
            original.replace(old_string, new_string)
        } else {
            original.replacen(old_string, new_string, 1)
        };

        write_and_diff(file_path, &original, &new_content, matches)
    }
}

fn normalize_quotes(s: &str) -> String {
    s.replace(['\u{2018}', '\u{2019}'], "'")
        .replace(['\u{201c}', '\u{201d}'], "\"")
}

fn write_and_diff(
    file_path: &str,
    original: &str,
    new: &str,
    match_count: usize,
) -> Result<ToolResult, ToolError> {
    std::fs::write(file_path, new)
        .map_err(|e| ToolError::execution_failed_for("FileEdit", format!("写入文件失败: {}", e)))?;

    let mut output = format!("✅ 已编辑文件: {}\n", file_path);
    output.push_str(&format!("替换了 {} 处匹配\n\n", match_count));

    // 生成 diff
    output.push_str("## 变更对比\n```diff\n");
    for diff in diff::lines(original, new) {
        match diff {
            diff::Result::Left(l) => output.push_str(&format!("-{}\n", l)),
            diff::Result::Right(r) => output.push_str(&format!("+{}\n", r)),
            diff::Result::Both(b, _) => output.push_str(&format!(" {}\n", b)),
        }
    }
    output.push_str("```\n");

    Ok(ToolResult::success(output))
}
