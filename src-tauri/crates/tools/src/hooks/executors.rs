//! Hook 执行器
//!
//! 实际执行 Hook 逻辑：启动子进程、发送 HTTP 请求、注入 Prompt。

use crate::hooks::{HookAction, HookConfig, HookExecutor, HookResult};
use tokio::process::Command;

/// 执行单个 Hook，超时控制
pub async fn execute_hook(hook: &HookConfig, tool_name: &str, input: &str) -> HookResult {
    let mut result = match &hook.executor {
        HookExecutor::Shell(shell) => execute_shell_hook(shell, tool_name, input).await,
        HookExecutor::Http(http) => execute_http_hook(http, tool_name, input).await,
        HookExecutor::Prompt(prompt) => {
            // Prompt hook 返回注入内容
            let context = prompt
                .template
                .replace("{{tool_name}}", tool_name)
                .replace("{{input}}", input);
            let mut r = HookResult::allowed();
            r.additional_context = Some(context);
            r
        },
    };

    result.hook_id = hook.id.clone();

    // 超时处理
    let timeout = std::time::Duration::from_secs(hook.timeout_secs);
    match tokio::time::timeout(timeout, async { &result }).await {
        Ok(_) => result,
        Err(_) => {
            let mut r = HookResult::allowed();
            r.hook_id = hook.id.clone();
            r.is_error = true;
            r.reason = Some(format!("Hook '{}' 超时", hook.id));
            r
        },
    }
}

/// 执行 Shell Hook
async fn execute_shell_hook(
    shell: &super::ShellHookExec,
    tool_name: &str,
    input: &str,
) -> HookResult {
    let mut cmd = Command::new(&shell.command);
    cmd.args(&shell.args);
    cmd.env("CLAUDE_HOOK_TOOL_NAME", tool_name);
    cmd.env("CLAUDE_HOOK_INPUT", input);

    if let Some(dir) = &shell.working_dir {
        cmd.current_dir(dir);
    }

    match cmd.output().await {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

            if !output.status.success() {
                let mut r = HookResult::allowed();
                r.is_error = true;
                r.reason = Some(format!(
                    "Hook 执行失败 (exit={}): {}",
                    output.status, stderr
                ));
                return r;
            }

            // 尝试解析 JSON 输出
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                let mut r = HookResult::allowed();
                if let Some(action) = json.get("action").and_then(|v| v.as_str()) {
                    r.action = match action {
                        "deny" => HookAction::Deny,
                        "allow" => HookAction::Allow,
                        "ask" => HookAction::Ask,
                        _ => HookAction::Allow,
                    };
                }
                if let Some(modified) = json.get("modified_input") {
                    r.modified_input = Some(modified.clone());
                }
                if let Some(ctx) = json.get("context") {
                    r.additional_context = Some(ctx.as_str().unwrap_or("").to_string());
                }
                r.reason = json
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                r
            } else {
                // 非 JSON 输出，作为附加上下文
                let mut r = HookResult::allowed();
                r.additional_context = Some(stdout);
                r
            }
        },
        Err(e) => {
            let mut r = HookResult::allowed();
            r.is_error = true;
            r.reason = Some(format!("Hook 启动失败: {}", e));
            r
        },
    }
}

/// 执行 HTTP Hook
async fn execute_http_hook(http: &super::HttpHookExec, tool_name: &str, input: &str) -> HookResult {
    let client = reqwest::Client::new();
    let method = match http.method.to_uppercase().as_str() {
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    let mut req = client
        .request(method, &http.url)
        .header("X-Hook-Tool-Name", tool_name)
        .timeout(std::time::Duration::from_secs(10));

    if let Some(headers) = &http.headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }

    // POST 请求默认发送 tool_name 和 input
    if http.method.to_uppercase() == "POST" {
        req = req.json(&serde_json::json!({
            "tool_name": tool_name,
            "input": input,
        }));
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(body) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        let mut r = HookResult::allowed();
                        if let Some(action) = json.get("action").and_then(|v| v.as_str()) {
                            r.action = match action {
                                "deny" => HookAction::Deny,
                                "ask" => HookAction::Ask,
                                _ => HookAction::Allow,
                            };
                        }
                        r.reason = json
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        r
                    } else if status.is_success() {
                        let mut r = HookResult::allowed();
                        r.additional_context = Some(body);
                        r
                    } else {
                        let mut r = HookResult::allowed();
                        r.is_error = true;
                        r.reason = Some(format!("HTTP {}: {}", status.as_u16(), body));
                        r
                    }
                },
                Err(e) => {
                    let mut r = HookResult::allowed();
                    r.is_error = true;
                    r.reason = Some(format!("HTTP 响应读取失败: {}", e));
                    r
                },
            }
        },
        Err(e) => {
            let mut r = HookResult::allowed();
            r.is_error = true;
            r.reason = Some(format!("HTTP 请求失败: {}", e));
            r
        },
    }
}
