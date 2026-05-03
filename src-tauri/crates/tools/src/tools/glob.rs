//! GlobTool - 文件模式搜索工具

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }
    fn description(&self) -> &str {
        "使用 glob 模式搜索文件（如 \"**/*.rs\", \"src/**/*.ts\"）。返回匹配的文件路径列表，按修改时间排序。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob 模式，如 **/*.rs"
                },
                "path": {
                    "type": "string",
                    "description": "搜索起始目录（默认为当前工作目录）"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多返回多少个结果（默认 100）",
                    "default": 100
                }
            },
            "required": ["pattern"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ToolError> {
        let _pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("Glob", "缺少 pattern 参数"))?;
        if let Some(path) = input["path"].as_str() {
            if !Path::new(path).is_absolute() {
                return Err(ToolError::invalid_input_for("Glob", "path 必须是绝对路径"));
            }
        }
        Ok(())
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let pattern = input["pattern"].as_str().unwrap();
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
        let search_path = input["path"].as_str().unwrap_or(&ctx.working_dir);

        let full_pattern = if search_path.ends_with('/') || search_path.ends_with('\\') {
            format!("{}{}", search_path, pattern)
        } else {
            format!("{}/{}", search_path, pattern)
        };

        let mut paths: Vec<PathBuf> = match glob::glob(&full_pattern) {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                return Err(ToolError::invalid_input_for(
                    "Glob",
                    format!("Glob 模式无效: {}", e),
                ))
            },
        };

        // 按修改时间排序
        paths.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        paths.reverse(); // 最新的在前

        let total = paths.len();
        let truncated = total > limit;
        if truncated {
            paths.truncate(limit);
        }

        let mut output = if truncated {
            format!("找到 {} 个匹配文件（显示前 {} 个）:\n", total, limit)
        } else {
            format!("找到 {} 个匹配文件:\n", total)
        };

        for p in &paths {
            let meta = std::fs::metadata(p).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let display = p.strip_prefix(search_path).unwrap_or(p);
            output.push_str(&format!(
                "  {}  ({:.1} KB)\n",
                display.display(),
                size as f64 / 1024.0
            ));
        }

        Ok(ToolResult::success(output))
    }
}
