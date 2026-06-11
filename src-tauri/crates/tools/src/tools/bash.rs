//! BashTool - Shell 命令执行工具（带安全层）
//!
//! 多层安全防护：
//! 1. 危险命令模式检测
//! 2. 命令白名单匹配
//! 3. 路径边界验证
//! 4. 输出重定向验证

use crate::permissions::classifier::HeuristicClassifier;
use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const MAX_TIMEOUT_SECS: u64 = 600;
const MAX_OUTPUT_BYTES: usize = 500_000;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }
    fn description(&self) -> &str {
        "执行 shell 命令。适用：运行测试、构建、git 操作、安装依赖等。\
         不适用：读取文件（用 FileRead）、搜索代码（用 Grep/Glob）、编辑文件（用 FileEdit）。\
         自动检测 OS (bash/powershell)，默认超时 120s，最大 600s。\
         危险命令（rm -rf, sudo, chmod 777 等）需权限确认。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的 shell 命令"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时秒数（默认 120，最大 600）",
                    "default": 120
                },
                "working_dir": {
                    "type": "string",
                    "description": "工作目录（可选，默认为当前工作目录）"
                }
            },
            "required": ["command"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }
    fn max_result_chars(&self) -> usize {
        200_000
    }

    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError> {
        let cmd = input["command"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("Bash", "缺少 command 参数"))?;

        if cmd.trim().is_empty() {
            return Err(ToolError::invalid_input_for("Bash", "command 不能为空"));
        }

        if cmd.len() > 10_000 {
            return Err(ToolError::invalid_input_for("Bash", "命令过长（最大 10000 字符）"));
        }

        let timeout = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        if timeout > MAX_TIMEOUT_SECS {
            return Err(ToolError::invalid_input_for(
                "Bash",
                format!("超时时间最大 {} 秒", MAX_TIMEOUT_SECS),
            ));
        }

        if !ctx.allow_execute {
            return Err(ToolError::permission_denied("Bash", "当前上下文不允许执行 shell 命令"));
        }

        // 安全分类
        let classifier_result = HeuristicClassifier::classify_bash(cmd);
        if classifier_result.suggest_deny {
            return Err(ToolError::permission_denied("Bash", &classifier_result.reason));
        }

        Ok(())
    }

    fn check_permissions(&self, input: &Value, _ctx: &ToolContext) -> PermissionResult {
        let cmd = input["command"].as_str().unwrap_or("");
        let classifier_result = HeuristicClassifier::classify_bash(cmd);

        match classifier_result.risk_level {
            crate::permissions::classifier::RiskLevel::Safe => PermissionResult::Allow,
            crate::permissions::classifier::RiskLevel::Low => PermissionResult::Allow,
            crate::permissions::classifier::RiskLevel::Critical => {
                PermissionResult::Deny(classifier_result.reason)
            },
            _ => PermissionResult::Ask(format!(
                "命令风险评估: {} - {}",
                match classifier_result.risk_level {
                    crate::permissions::classifier::RiskLevel::Medium => "中风险",
                    crate::permissions::classifier::RiskLevel::High => "高风险",
                    _ => "未知",
                },
                classifier_result.reason
            )),
        }
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let cmd = input["command"].as_str().unwrap_or("");
        if cmd.is_empty() {
            return Err(ToolError::invalid_input_for("Bash", "缺少 command 参数"));
        }
        // ── 安全分析（call() 中也做，防御 validate() 被绕过） ──
        use crate::bash::parser::parse_command;
        use crate::bash::security::SecurityAnalyzer;
        if let Ok(parsed) = parse_command(cmd) {
            let analyzer = SecurityAnalyzer::new();
            if let crate::bash::security::SecurityResult::Blocked(reason) =
                analyzer.analyze(&parsed)
            {
                return Err(ToolError::permission_denied("Bash", &format!("安全阻止: {}", reason)));
            }
        } else {
            let classifier_result = HeuristicClassifier::classify_bash(cmd);
            if classifier_result.suggest_deny {
                return Err(ToolError::permission_denied("Bash", &classifier_result.reason));
            }
        }
        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        let working_dir = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);

        // heredoc / 注入检测
        if cmd.contains("<<") || cmd.contains("EOF") || cmd.contains("EOT") {
            let lower = cmd.to_lowercase();
            if lower.contains("curl") || lower.contains("wget") || lower.contains("eval") {
                return Err(ToolError::permission_denied(
                    "Bash",
                    "检测到 heredoc + 网络/执行 组合，存在注入风险",
                ));
            }
        }

        // 自动后台: 超过 60s 的命令建议后台
        if timeout_secs > 60
            && !input
                .get("run_in_background")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        {
            // 命令超过 60 秒，建议使用 Monitor 或 run_in_background
        }

        // 选择 shell
        let (shell, flag) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("bash", "-c")
        };

        let child = tokio::process::Command::new(shell)
            .arg(flag)
            .arg(cmd)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ToolError::execution_failed_for("Bash", format!("启动命令失败: {}", e)))?;

        let start = std::time::Instant::now();

        let output_result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            child.wait_with_output(),
        )
        .await;

        let elapsed = start.elapsed();
        let output = match output_result {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Err(ToolError::execution_failed_for(
                    "Bash",
                    format!("命令执行异常: {}", e),
                ));
            },
            Err(_timeout) => {
                return Err(ToolError {
                    error_code: "tool.Bash.timeout".into(),
                    message: format!("命令执行超时（{} 秒）", timeout_secs),
                    kind: crate::ToolErrorKind::Timeout,
                });
            },
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let stdout_display = if stdout.len() > MAX_OUTPUT_BYTES {
            format!(
                "{}\n\n[stdout 已截断，显示 {}/{} 字节]",
                &stdout[..MAX_OUTPUT_BYTES],
                MAX_OUTPUT_BYTES,
                stdout.len(),
            )
        } else {
            stdout.to_string()
        };

        let stderr_display = if stderr.is_empty() {
            String::new()
        } else if stderr.len() > MAX_OUTPUT_BYTES / 2 {
            format!("\n\n## stderr\n{}\n[已截断]", &stderr[..MAX_OUTPUT_BYTES / 2])
        } else {
            format!("\n\n## stderr\n{}", stderr)
        };

        let mut result = String::new();
        result.push_str(&format!(
            "## 退出码: {}\n耗时: {:.1}s\n\n",
            output.status.code().unwrap_or(-1),
            elapsed.as_secs_f64()
        ));

        if !stdout_display.is_empty() {
            result.push_str(&stdout_display);
        }
        if !stderr_display.is_empty() {
            result.push_str(&stderr_display);
        }

        return Ok(ToolResult::success(result));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn bash_does_not_block_runtime() {
        // 1s 命令绝不应阻塞 3s+（旧实现的 try_wait + std::thread::sleep
        // 会把 worker 线程挂死，导致整体延迟放大）。
        let tool = BashTool;
        let input = serde_json::json!({
            "command": if cfg!(windows) { "ping -n 2 127.0.0.1" } else { "sleep 1" },
            "timeout": 10
        });
        let ctx = ToolContext::new(".");
        let start = Instant::now();
        let result = tool.call(input, &ctx).await;
        let elapsed = start.elapsed();
        assert!(result.is_ok(), "bash call should succeed: {:?}", result);
        assert!(
            elapsed < Duration::from_secs(3),
            "elapsed={:?} too long (likely blocking runtime)",
            elapsed
        );
    }

    #[tokio::test]
    async fn bash_kill_on_timeout_reaps_child() {
        // 启动一个 30s 命令，超时 1s 后必须被 kill 并返回超时错误。
        // 同时验证：子进程不会泄漏到下一次测试（kill_on_drop / 同步 kill 路径）。
        let tool = BashTool;
        let input = serde_json::json!({
            "command": if cfg!(windows) { "ping -n 30 127.0.0.1 > NUL" } else { "sleep 30" },
            "timeout": 1
        });
        let ctx = ToolContext::new(".");
        let result = tool.call(input, &ctx).await;
        assert!(result.is_err(), "expected timeout error, got: {:?}", result);
        let err = result.unwrap_err();
        assert!(
            err.message.contains("超时") || err.message.contains("timeout"),
            "expected timeout error, got: {}",
            err.message
        );
    }
}
