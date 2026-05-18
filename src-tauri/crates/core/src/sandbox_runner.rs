#[cfg(not(target_os = "android"))]
use anyhow::Result;
#[cfg(not(target_os = "android"))]
use serde::Serialize;
#[cfg(not(target_os = "android"))]
use std::process::Stdio;
#[cfg(not(target_os = "android"))]
use tokio::process::Command;

#[cfg(not(target_os = "android"))]
const SANDBOX_TIMEOUT_SECS: u64 = 30;

#[cfg(not(target_os = "android"))]
#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[cfg(not(target_os = "android"))]
pub struct SandboxRunner {
    node_path: String,
}

#[cfg(not(target_os = "android"))]
impl Default for SandboxRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "android"))]
impl SandboxRunner {
    pub fn new() -> Self {
        let node_path = std::env::var("NODE_PATH").unwrap_or_else(|_| "node".to_string());
        if node_path.contains('/') || node_path.contains('\\') {
            let resolved = std::fs::canonicalize(&node_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(node_path);
            let exe_name = std::path::Path::new(&resolved)
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !exe_name.contains("node") && !exe_name.contains("bun") {
                tracing::warn!(
                    "NODE_PATH does not appear to be a Node.js executable: {}",
                    resolved
                );
            }
            Self {
                node_path: resolved,
            }
        } else {
            Self { node_path }
        }
    }

    pub async fn execute(&self, code: &str, language: &str) -> Result<ExecutionResult> {
        let limits = crate::resource_limits::ResourceLimits::default_sandbox();
        if let Err(e) = limits.apply_to_current_process() {
            tracing::warn!("Failed to apply sandbox resource limits: {}", e);
        }

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
        Err(anyhow::anyhow!("Python execution handled by frontend Pyodide"))
    }
}

#[cfg(not(target_os = "android"))]
pub fn create_sandbox_runner() -> SandboxRunner {
    SandboxRunner::new()
}

#[cfg(target_os = "android")]
pub struct SandboxRunner;

#[cfg(target_os = "android")]
impl SandboxRunner {
    pub fn new() -> Self {
        Self
    }
    pub async fn execute(&self, _code: &str, _language: &str) -> anyhow::Result<ExecutionResult> {
        anyhow::bail!("Sandbox execution is not available on Android")
    }
}

#[cfg(target_os = "android")]
pub fn create_sandbox_runner() -> SandboxRunner {
    SandboxRunner::new()
}

#[cfg(target_os = "android")]
#[derive(Debug, serde::Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
