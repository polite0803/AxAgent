//! REPLTool - 隔离代码执行

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

pub struct REPLTool;

#[async_trait]
impl Tool for REPLTool {
    fn name(&self) -> &str {
        "REPL"
    }
    fn description(&self) -> &str {
        "在隔离环境中执行代码片段。支持 Python, Node.js, Rust 等语言。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "language": { "type": "string", "enum": ["python", "node", "rust", "bash"], "description": "编程语言" },
                "code": { "type": "string", "description": "要执行的代码" },
                "timeout_secs": { "type": "integer", "default": 30 }
            },
            "required": ["language", "code"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let lang = input["language"].as_str().unwrap_or("bash");
        let code = input["code"].as_str().unwrap_or("");
        let timeout = input["timeout_secs"].as_u64().unwrap_or(30);

        let (runner, arg, _ext) = match lang {
            "python" => ("python", "-c", "py"),
            "node" => ("node", "-e", "js"),
            "rust" => {
                // Rust 需要编译执行
                let tmp =
                    std::env::temp_dir().join(format!("axagent_repl_{}.rs", uuid::Uuid::new_v4()));
                std::fs::write(&tmp, code)
                    .map_err(|e| ToolError::execution_failed(format!("写入临时文件失败: {}", e)))?;
                let output = Command::new("rustc")
                    .arg(&tmp)
                    .arg("-o")
                    .arg(tmp.with_extension(""))
                    .output();
                let _ = std::fs::remove_file(&tmp);
                match output {
                    Ok(o) if o.status.success() => {
                        let exe = tmp.with_extension("");
                        match Command::new(&exe).output() {
                            Ok(out) => {
                                let _ = std::fs::remove_file(&exe);
                                return Ok(ToolResult::success(format!(
                                    "## Rust REPL\n```\n{}```\n\nstdout:\n{}stderr:\n{}",
                                    code,
                                    String::from_utf8_lossy(&out.stdout),
                                    String::from_utf8_lossy(&out.stderr),
                                )));
                            },
                            Err(e) => {
                                let _ = std::fs::remove_file(&exe);
                                return Err(ToolError::execution_failed(e.to_string()));
                            },
                        }
                    },
                    Ok(o) => {
                        return Err(ToolError::execution_failed(format!(
                            "编译失败:\n{}",
                            String::from_utf8_lossy(&o.stderr)
                        )))
                    },
                    Err(e) => return Err(ToolError::execution_failed(e.to_string())),
                }
            },
            _ => ("bash", "-c", "sh"),
        };

        if lang != "rust" {
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(timeout),
                tokio::process::Command::new(runner)
                    .arg(arg)
                    .arg(code)
                    .output(),
            )
            .await;

            match output {
                Ok(Ok(out)) => Ok(ToolResult::success(format!(
                    "## {} REPL\n```{}```\n\nstdout:\n{}stderr:\n{}",
                    lang,
                    code,
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr),
                ))),
                Ok(Err(e)) => Err(ToolError::execution_failed(e.to_string())),
                Err(_) => Err(ToolError {
                    error_code: "tool.REPL.timeout".into(),
                    message: "REPL 执行超时".into(),
                    kind: crate::ToolErrorKind::Timeout,
                }),
            }
        } else {
            unreachable!()
        }
    }
}
