//! GrepTool - 内容搜索工具

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }
    fn description(&self) -> &str {
        "在文件中搜索匹配指定正则表达式的内容。支持 glob 文件过滤、上下文行数、大小写不敏感。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "正则表达式搜索模式"
                },
                "path": {
                    "type": "string",
                    "description": "搜索路径（目录或文件，默认当前工作目录）"
                },
                "glob": {
                    "type": "string",
                    "description": "文件过滤 glob，如 *.rs"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "是否忽略大小写",
                    "default": false
                },
                "context": {
                    "type": "integer",
                    "description": "上下文行数（前后各 N 行）",
                    "default": 0
                },
                "head_limit": {
                    "type": "integer",
                    "description": "最多返回多少个匹配",
                    "default": 250
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

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let pattern = input["pattern"].as_str().unwrap();
        let search_path = input["path"].as_str().unwrap_or(&ctx.working_dir);
        let case_insensitive = input
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let context = input.get("context").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
        let head_limit = input
            .get("head_limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(250) as usize;

        // 优先使用 rg (ripgrep)，回退到系统 grep
        let (mut cmd, prefer_rg) = if which::which("rg").is_ok() {
            (Command::new("rg"), true)
        } else if cfg!(target_os = "windows") {
            // Windows: 使用 findstr 或内建搜索
            return fallback_search(pattern, search_path, case_insensitive, context, head_limit);
        } else {
            (Command::new("grep"), false)
        };

        if prefer_rg {
            cmd.arg("--line-number");
            cmd.arg("--color=never");
            cmd.arg("--no-heading");
            if case_insensitive {
                cmd.arg("--ignore-case");
            }
            if context > 0 {
                cmd.arg(format!("-C{}", context));
            }
            // 排除常见忽略目录
            cmd.arg("--glob=!.git");
            cmd.arg("--glob=!node_modules");
            cmd.arg("--glob=!target");
            cmd.arg("--glob=!.venv");

            if let Some(glob) = input["glob"].as_str() {
                cmd.arg(format!("--glob={}", glob));
            }
        } else {
            cmd.arg("-rn");
            if case_insensitive {
                cmd.arg("-i");
            }
            if context > 0 {
                cmd.arg(format!("-C{}", context));
            }
            cmd.arg("--color=never");
            // 排除目录
            cmd.arg("--exclude-dir=.git");
            cmd.arg("--exclude-dir=node_modules");
        }

        cmd.arg(pattern);
        cmd.arg(search_path);

        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => return Err(ToolError::execution_failed(format!("grep 执行失败: {}", e))),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // grep 退出码 1 表示无匹配（不算错误）
        if !output.status.success() && output.status.code() != Some(1) {
            return Err(ToolError::execution_failed(format!(
                "grep 错误: {}",
                stderr
            )));
        }

        if stdout.trim().is_empty() {
            return Ok(ToolResult::success("未找到匹配项。"));
        }

        let lines: Vec<&str> = stdout.lines().collect();
        let total = lines.len();

        if total > head_limit {
            let selected = &lines[..head_limit];
            let mut result = String::new();
            result.push_str(&format!(
                "找到 {} 个匹配（显示前 {} 个）:\n\n",
                total, head_limit
            ));
            for line in selected {
                result.push_str(line);
                result.push('\n');
            }
            result.push_str(&format!("\n[结果已截断，共 {} 个匹配]", total));
            Ok(ToolResult::success(result))
        } else {
            let mut result = format!("找到 {} 个匹配:\n\n", total);
            for line in &lines {
                result.push_str(line);
                result.push('\n');
            }
            Ok(ToolResult::success(result))
        }
    }
}

/// 简单的文本搜索回退（无 rg/grep 时使用）
fn fallback_search(
    pattern: &str,
    search_path: &str,
    case_insensitive: bool,
    _context: u32,
    head_limit: usize,
) -> Result<ToolResult, ToolError> {
    let re = if case_insensitive {
        regex::RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
    } else {
        regex::Regex::new(pattern)
    }
    .map_err(|e| ToolError::invalid_input(format!("正则表达式无效: {}", e)))?;

    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(search_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        // 跳过隐藏目录和常见忽略
        let path_str = entry.path().to_string_lossy();
        if path_str.contains("/.git/")
            || path_str.contains("/node_modules/")
            || path_str.contains("/target/")
        {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    results.push(format!("{}:{}:{}", path_str, i + 1, line));
                    if results.len() >= head_limit {
                        let mut output = String::new();
                        output.push_str(&format!(
                            "搜索完成（已截断，匹配 > {} 个）:\n\n",
                            head_limit
                        ));
                        for r in &results {
                            output.push_str(r);
                            output.push('\n');
                        }
                        return Ok(ToolResult::success(output));
                    }
                }
            }
        }
    }

    if results.is_empty() {
        Ok(ToolResult::success("未找到匹配项。"))
    } else {
        let mut output = format!("找到 {} 个匹配:\n\n", results.len());
        for r in &results {
            output.push_str(r);
            output.push('\n');
        }
        Ok(ToolResult::success(output))
    }
}
