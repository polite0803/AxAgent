use crate::hook_config::ShellHooksConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHookInput {
    pub event: String,
    pub tool_name: Option<String>,
    pub arguments: Option<serde_json::Value>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHookOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub veto: bool,
    pub reason: Option<String>,
    pub modified_input: Option<serde_json::Value>,
}

impl ShellHookOutput {
    pub fn from_raw(exit_code: i32, stdout: String, stderr: String) -> Self {
        let mut result = Self {
            exit_code,
            stdout,
            stderr,
            veto: false,
            reason: None,
            modified_input: None,
        };
        if exit_code != 0 {
            result.veto = true;
            result.reason = Some(format!("Hook exited with code {}", exit_code));
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result.stdout) {
            if let Some(veto) = parsed.get("veto").and_then(|v| v.as_bool()) {
                result.veto = veto;
            }
            if let Some(reason) = parsed.get("reason").and_then(|v| v.as_str()) {
                result.reason = Some(reason.to_string());
            }
            if let Some(modified) = parsed.get("modified_input") {
                result.modified_input = Some(modified.clone());
            }
        }
        result
    }
}

pub struct ShellHookExecutor {
    config: ShellHooksConfig,
}

impl ShellHookExecutor {
    pub fn from_dir(dir: &Path) -> Self {
        Self {
            config: ShellHooksConfig::load_from_dir(dir),
        }
    }

    pub fn from_default_dir() -> Self {
        Self::from_dir(&ShellHooksConfig::default_hooks_dir())
    }

    pub async fn execute(&self, input: ShellHookInput) -> Vec<ShellHookOutput> {
        let hooks = self.config.enabled_hooks_for(&input.event);
        let mut results = Vec::new();
        for hook in hooks {
            let json_input = serde_json::to_string(&input).unwrap_or_default();
            let result = match Command::new(&hook.command)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        if stdin.write_all(json_input.as_bytes()).await.is_err() {
                            drop(stdin);
                        }
                    }
                    match child.wait_with_output().await {
                        Ok(output) => ShellHookOutput::from_raw(
                            output.status.code().unwrap_or(-1),
                            String::from_utf8_lossy(&output.stdout).to_string(),
                            String::from_utf8_lossy(&output.stderr).to_string(),
                        ),
                        Err(e) => ShellHookOutput {
                            exit_code: -1,
                            stdout: String::new(),
                            stderr: e.to_string(),
                            veto: false,
                            reason: None,
                            modified_input: None,
                        },
                    }
                }
                Err(e) => ShellHookOutput {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    veto: false,
                    reason: None,
                    modified_input: None,
                },
            };
            if result.veto {
                results.push(result);
                return results;
            }
            results.push(result);
        }
        results
    }

    pub async fn should_veto(&self, input: ShellHookInput) -> Option<String> {
        let results = self.execute(input).await;
        results
            .into_iter()
            .find_map(|r| if r.veto { r.reason } else { None })
    }
}
