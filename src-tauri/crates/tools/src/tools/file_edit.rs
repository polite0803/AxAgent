use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "FileEdit"
    }
    fn description(&self) -> &str {
        "精确编辑文件（字符串替换）。适用：修改代码行、修改变量名、修复 bug。\
         不适用：新建文件(FileWrite)、删除文件(DeleteFile)。\
         必须先用 FileRead 读取文件获取精确原始文本（含缩进/空行）。\
         old_string 在文件中必须唯一（否则需 replace_all: true）。"
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
            return Err(ToolError::invalid_input_for("FileEdit", "file_path 必须是绝对路径"));
        }

        if path.contains("..") || path.starts_with('~') {
            return Err(ToolError::permission_denied(
                "FileEdit",
                "file_path 包含禁止的路径遍历模式 (.. 或 ~)",
            ));
        }

        let old = input["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 old_string"))?;
        let new = input["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 new_string"))?;

        if old.is_empty() {
            return Err(ToolError::invalid_input_for("FileEdit", "old_string 不能为空"));
        }

        if old == new {
            return Err(ToolError::invalid_input("old_string 和 new_string 相同，无需编辑"));
        }

        let file_size = tokio::fs::metadata(path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
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
            return Err(ToolError::permission_denied("FileEdit", "当前上下文不允许写入操作"));
        }

        Ok(())
    }

    fn check_permissions(&self, input: &Value, _ctx: &ToolContext) -> PermissionResult {
        let path = input["file_path"].as_str().unwrap_or("");
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
                return PermissionResult::Ask(format!("编辑系统路径 '{}'，确认？", path));
            }
        }
        PermissionResult::Allow
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 file_path 参数"))?;
        let old_string = input["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 old_string 参数"))?;
        let new_string = input["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileEdit", "缺少 new_string 参数"))?;
        let replace_all = input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let original = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            ToolError::execution_failed_for("FileEdit", format!("读取文件失败: {}", e))
        })?;

        let matches = original.matches(old_string).count();

        if matches == 0 {
            let normalized_old = normalize_quotes(old_string);
            if normalized_old != old_string {
                let matches_norm = original.matches(&normalized_old).count();
                if matches_norm == 0 {
                    return Err(ToolError::invalid_input_for("FileEdit", "在文件中未找到 old_string。请确认 old_string 与文件内容完全一致（包括空格和缩进）。\n已尝试引号规范化。".to_string()));
                }
                let new_content = if replace_all {
                    original.replace(&normalized_old, new_string)
                } else {
                    original.replacen(&normalized_old, new_string, 1)
                };
                return write_and_diff(file_path, &original, &new_content, matches_norm).await;
            }
            return Err(ToolError::invalid_input(
                "在文件中未找到 old_string。请确认 old_string 与文件内容完全一致（包括空格和缩进）。",
            ));
        }

        if matches > 1 && !replace_all {
            return Err(ToolError::invalid_input_for(
                "FileEdit",
                format!(
                    "old_string 匹配了 {} 次（非唯一匹配）。请设置 replace_all: true 替换所有匹配项，或提供更多上下文使匹配唯一。",
                    matches
                ),
            ));
        }

        let new_content = if replace_all {
            original.replace(old_string, new_string)
        } else {
            original.replacen(old_string, new_string, 1)
        };

        write_and_diff(file_path, &original, &new_content, matches).await
    }
}

fn normalize_quotes(s: &str) -> String {
    s.replace(['\u{2018}', '\u{2019}'], "'")
        .replace(['\u{201c}', '\u{201d}'], "\"")
}

async fn write_and_diff(
    file_path: &str,
    original: &str,
    new: &str,
    match_count: usize,
) -> Result<ToolResult, ToolError> {
    tokio::fs::write(file_path, new)
        .await
        .map_err(|e| ToolError::execution_failed_for("FileEdit", format!("写入文件失败: {}", e)))?;

    let mut output = format!("✅ 已编辑文件: {}\n", file_path);
    output.push_str(&format!("替换了 {} 处匹配\n\n", match_count));

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
