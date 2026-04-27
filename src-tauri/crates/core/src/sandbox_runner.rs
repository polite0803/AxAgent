use anyhow::Result;
use serde::Serialize;
use std::process::Stdio;
use tokio::process::Command;

const SANDBOX_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct SandboxRunner {
    node_path: String,
}

impl Default for SandboxRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxRunner {
    pub fn new() -> Self {
        Self {
            node_path: std::env::var("NODE_PATH").unwrap_or_else(|_| "node".to_string()),
        }
    }

    pub async fn execute(&self, code: &str, language: &str) -> Result<ExecutionResult> {
        match language {
            "javascript" | "js" | "typescript" | "ts" => self.execute_js(code).await,
            "python" | "py" => self.execute_python(code).await,
            _ => Err(anyhow::anyhow!("Unsupported language: {}", language)),
        }
    }

    async fn execute_js(&self, code: &str) -> Result<ExecutionResult> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join(format!("axagent_sandbox_{}.js", uuid::Uuid::new_v4()));

        tokio::fs::write(&script_path, code).await?;

        let output = Command::new(&self.node_path)
            .arg(&script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output();

        let result =
            tokio::time::timeout(std::time::Duration::from_secs(SANDBOX_TIMEOUT_SECS), output)
                .await
                .map_err(|_| anyhow::anyhow!("Execution timeout"))??;

        let _ = tokio::fs::remove_file(&script_path).await;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        Ok(ExecutionResult {
            stdout,
            stderr,
            exit_code: result.status.code().unwrap_or(-1),
        })
    }

    async fn execute_python(&self, _code: &str) -> Result<ExecutionResult> {
        Err(anyhow::anyhow!(
            "Python execution handled by frontend Pyodide"
        ))
    }
}

pub fn create_sandbox_runner() -> SandboxRunner {
    SandboxRunner::new()
}
