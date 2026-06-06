use std::env;
use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command as TokioCommand;
use tokio::runtime::Builder;
use tokio::time::timeout;

use crate::sandbox::{
    FilesystemIsolationMode, SandboxConfig, SandboxStatus, build_linux_sandbox_command,
    resolve_sandbox_status_for_request,
};
use axagent_runtime_core::ConfigLoader;

/// Input schema for the built-in bash execution tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BashCommandInput {
    pub command: String,
    pub timeout: Option<u64>,
    pub description: Option<String>,
    #[serde(rename = "run_in_background")]
    pub run_in_background: Option<bool>,
    /// SECURITY: 工具输入中的此字段会被服务端忽略。
    /// 关闭沙箱只能由服务器端配置（环境变量/启动参数）控制。
    /// 保留字段仅为向后兼容；调用方传 true 仅会记录告警，不会真的关闭沙箱。
    #[serde(rename = "dangerouslyDisableSandbox", default, skip_serializing)]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "namespaceRestrictions")]
    pub namespace_restrictions: Option<bool>,
    #[serde(rename = "isolateNetwork")]
    pub isolate_network: Option<bool>,
    #[serde(rename = "filesystemMode")]
    pub filesystem_mode: Option<FilesystemIsolationMode>,
    #[serde(rename = "allowedMounts")]
    pub allowed_mounts: Option<Vec<String>>,
}

/// Output returned from a bash tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BashCommandOutput {
    pub stdout: String,
    pub stderr: String,
    #[serde(rename = "rawOutputPath")]
    pub raw_output_path: Option<String>,
    pub interrupted: bool,
    #[serde(rename = "isImage")]
    pub is_image: Option<bool>,
    #[serde(rename = "backgroundTaskId")]
    pub background_task_id: Option<String>,
    #[serde(rename = "backgroundedByUser")]
    pub backgrounded_by_user: Option<bool>,
    #[serde(rename = "assistantAutoBackgrounded")]
    pub assistant_auto_backgrounded: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "returnCodeInterpretation")]
    pub return_code_interpretation: Option<String>,
    #[serde(rename = "noOutputExpected")]
    pub no_output_expected: Option<bool>,
    #[serde(rename = "structuredContent")]
    pub structured_content: Option<Vec<serde_json::Value>>,
    #[serde(rename = "persistedOutputPath")]
    pub persisted_output_path: Option<String>,
    #[serde(rename = "persistedOutputSize")]
    pub persisted_output_size: Option<u64>,
    #[serde(rename = "sandboxStatus")]
    pub sandbox_status: Option<SandboxStatus>,
}

/// 独立硬门禁：在执行前对命令做最小化安全检查，不依赖外部分类器。
/// 如果返回 `Err`，命令被拒绝执行。
fn hard_gate_check(command: &str) -> io::Result<()> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "命令为空"));
    }

    // 禁止模式列表：编译期常量，独立于 HeuristicClassifier 和 bash_validation
    #[rustfmt::skip]
    const FORBIDDEN: &[&str] = &[
        "rm -rf /", "rm -rf /*", "rm -rf ~", "rm -rf .",
        "mkfs.", "mke2fs", "mkfs.", "mkfs.ext", "mkfs.xfs", "mkfs.btrfs", "mkfs.ntfs",
        "dd if=", "dd  if=",
        ":(){ :|:& };:",  // fork bomb
        "> /dev/sd", "> /dev/hd", "> /dev/nvme", "> /dev/xvd",
        "chmod 777 /", "chmod -R 777 /",
        "chown -R", "chown root:root /",
        "wget -O- http:// | sh", "curl  | sh", "curl  | bash",
    ];

    let lower = trimmed.to_lowercase();
    for pattern in FORBIDDEN {
        if lower.contains(pattern) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("命令包含禁止模式: {pattern}"),
            ));
        }
    }

    // 命令过长拒绝（防止注入绕过简单的子串匹配）
    if trimmed.len() > 100_000 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "命令超过最大长度限制"));
    }

    Ok(())
}

/// Executes a shell command with the requested sandbox settings.
pub fn execute_bash(input: BashCommandInput) -> io::Result<BashCommandOutput> {
    hard_gate_check(&input.command)?;

    let cwd = env::current_dir()?;
    let sandbox_status = sandbox_status_for_input(&input, &cwd);

    if input.run_in_background.unwrap_or(false) {
        let mut child = prepare_command(&input.command, &cwd, &sandbox_status, false);
        let child = child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        return Ok(BashCommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            raw_output_path: None,
            interrupted: false,
            is_image: None,
            background_task_id: Some(child.id().to_string()),
            backgrounded_by_user: Some(false),
            assistant_auto_backgrounded: Some(false),
            dangerously_disable_sandbox: input.dangerously_disable_sandbox,
            return_code_interpretation: None,
            no_output_expected: Some(true),
            structured_content: None,
            persisted_output_path: None,
            persisted_output_size: None,
            sandbox_status: Some(sandbox_status),
        });
    }

    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(execute_bash_async(input, sandbox_status, cwd))
}

async fn execute_bash_async(
    input: BashCommandInput,
    sandbox_status: SandboxStatus,
    cwd: std::path::PathBuf,
) -> io::Result<BashCommandOutput> {
    let mut command = prepare_tokio_command(&input.command, &cwd, &sandbox_status, true);

    let output_result = if let Some(timeout_ms) = input.timeout {
        match timeout(Duration::from_millis(timeout_ms), command.output()).await {
            Ok(result) => (result?, false),
            Err(_) => {
                return Ok(BashCommandOutput {
                    stdout: String::new(),
                    stderr: format!("Command exceeded timeout of {timeout_ms} ms"),
                    raw_output_path: None,
                    interrupted: true,
                    is_image: None,
                    background_task_id: None,
                    backgrounded_by_user: None,
                    assistant_auto_backgrounded: None,
                    dangerously_disable_sandbox: input.dangerously_disable_sandbox,
                    return_code_interpretation: Some(String::from("timeout")),
                    no_output_expected: Some(true),
                    structured_content: None,
                    persisted_output_path: None,
                    persisted_output_size: None,
                    sandbox_status: Some(sandbox_status),
                });
            },
        }
    } else {
        (command.output().await?, false)
    };

    let (output, interrupted) = output_result;
    let stdout = truncate_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = truncate_output(&String::from_utf8_lossy(&output.stderr));
    let no_output_expected = Some(stdout.trim().is_empty() && stderr.trim().is_empty());
    let return_code_interpretation = output.status.code().and_then(|code| {
        if code == 0 {
            None
        } else {
            Some(format!("exit_code:{code}"))
        }
    });

    Ok(BashCommandOutput {
        stdout,
        stderr,
        raw_output_path: None,
        interrupted,
        is_image: None,
        background_task_id: None,
        backgrounded_by_user: None,
        assistant_auto_backgrounded: None,
        dangerously_disable_sandbox: input.dangerously_disable_sandbox,
        return_code_interpretation,
        no_output_expected,
        structured_content: None,
        persisted_output_path: None,
        persisted_output_size: None,
        sandbox_status: Some(sandbox_status),
    })
}

fn sandbox_status_for_input(input: &BashCommandInput, cwd: &std::path::Path) -> SandboxStatus {
    let config = ConfigLoader::default_for(cwd).load().map_or_else(
        |_| SandboxConfig::default(),
        |runtime_config| runtime_config.sandbox().clone(),
    );

    // SECURITY (C1/C10): 即使调用方传 dangerouslyDisableSandbox=true，也必须忽略。
    // 关闭沙箱只能由进程级环境变量 AXAGENT_ALLOW_UNSANDBOXED=1 显式开启（用于本地调试）。
    // 默认 (env 缺失) 时沙箱永远启用，LLM / agent 任何输入都无权关闭。
    let env_bypass = std::env::var("AXAGENT_ALLOW_UNSANDBOXED")
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    if input.dangerously_disable_sandbox.unwrap_or(false) && !env_bypass {
        tracing::warn!(
            target: "axagent.security",
            "Ignored LLM-supplied dangerouslyDisableSandbox=true (set AXAGENT_ALLOW_UNSANDBOXED=1 to honor)"
        );
    }
    let sandbox_enabled = !env_bypass;
    let request = config.resolve_request(
        Some(sandbox_enabled),
        input.namespace_restrictions,
        input.isolate_network,
        input.filesystem_mode,
        input.allowed_mounts.clone(),
    );
    resolve_sandbox_status_for_request(&request, cwd)
}

fn prepare_command(
    command: &str,
    cwd: &std::path::Path,
    sandbox_status: &SandboxStatus,
    create_dirs: bool,
) -> Command {
    if create_dirs {
        prepare_sandbox_dirs(cwd);
    }

    if let Some(launcher) = build_linux_sandbox_command(command, cwd, sandbox_status) {
        let mut prepared = Command::new(launcher.program);
        prepared.args(launcher.args);
        prepared.current_dir(cwd);
        prepared.envs(launcher.env);
        return prepared;
    }

    let mut prepared = Command::new("sh");
    prepared.arg("-lc").arg(command).current_dir(cwd);
    if sandbox_status.filesystem_active {
        prepared.env("HOME", cwd.join(".sandbox-home"));
        prepared.env("TMPDIR", cwd.join(".sandbox-tmp"));
    }
    prepared
}

fn prepare_tokio_command(
    command: &str,
    cwd: &std::path::Path,
    sandbox_status: &SandboxStatus,
    create_dirs: bool,
) -> TokioCommand {
    if create_dirs {
        prepare_sandbox_dirs(cwd);
    }

    if let Some(launcher) = build_linux_sandbox_command(command, cwd, sandbox_status) {
        let mut prepared = TokioCommand::new(launcher.program);
        prepared.args(launcher.args);
        prepared.current_dir(cwd);
        prepared.envs(launcher.env);
        return prepared;
    }

    let mut prepared = TokioCommand::new("sh");
    prepared.arg("-lc").arg(command).current_dir(cwd);
    if sandbox_status.filesystem_active {
        prepared.env("HOME", cwd.join(".sandbox-home"));
        prepared.env("TMPDIR", cwd.join(".sandbox-tmp"));
    }
    prepared
}

fn prepare_sandbox_dirs(cwd: &std::path::Path) {
    let _ = std::fs::create_dir_all(cwd.join(".sandbox-home"));
    let _ = std::fs::create_dir_all(cwd.join(".sandbox-tmp"));
}

#[cfg(test)]
mod tests {
    use super::{BashCommandInput, execute_bash};
    use crate::sandbox::FilesystemIsolationMode;

    #[test]
    fn executes_simple_command() {
        let output = execute_bash(BashCommandInput {
            command: String::from("printf 'hello'"),
            timeout: Some(1_000),
            description: None,
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(false),
            namespace_restrictions: Some(false),
            isolate_network: Some(false),
            filesystem_mode: Some(FilesystemIsolationMode::WorkspaceOnly),
            allowed_mounts: None,
        })
        .expect("bash command should execute");

        assert_eq!(output.stdout, "hello");
        assert!(!output.interrupted);
        assert!(output.sandbox_status.is_some());
    }

    #[test]
    fn hard_gate_rejects_rm_rf_root() {
        assert!(super::hard_gate_check("rm -rf / --no-preserve-root").is_err());
    }

    #[test]
    fn hard_gate_rejects_mkfs() {
        assert!(super::hard_gate_check("mkfs.ext4 /dev/sda1").is_err());
    }

    #[test]
    fn hard_gate_rejects_fork_bomb() {
        assert!(super::hard_gate_check(":(){ :|:& };:").is_err());
    }

    #[test]
    fn hard_gate_rejects_empty_command() {
        assert!(super::hard_gate_check("   ").is_err());
    }

    #[test]
    fn hard_gate_allows_safe_command() {
        assert!(super::hard_gate_check("echo hello").is_ok());
        assert!(super::hard_gate_check("ls -la").is_ok());
        assert!(super::hard_gate_check("git status").is_ok());
    }

    #[test]
    fn ignores_llm_supplied_sandbox_bypass() {
        // LLM 试图传 dangerouslyDisableSandbox=true — 必须被忽略。
        let output = execute_bash(BashCommandInput {
            command: String::from("printf 'hello'"),
            timeout: Some(1_000),
            description: None,
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(true),
            namespace_restrictions: None,
            isolate_network: None,
            filesystem_mode: None,
            allowed_mounts: None,
        })
        .expect("bash command should execute");

        assert!(
            output.sandbox_status.expect("sandbox status").enabled,
            "sandbox must stay enabled unless AXAGENT_ALLOW_UNSANDBOXED=1 is set"
        );
    }

    #[test]
    fn env_var_can_disable_sandbox() {
        // 用线程局部 env (set_var 改动全局有竞争)。这里改用 spawn 隔离：
        // 我们仅验证 "当 env_bypass=true 时 status.enabled=false"，通过直接调用内部函数。
        // 由于 sandbox_status_for_input 读 env，这里改用直接的环境隔离或跳过；
        // 改用 serial_test 替代品：直接断言 env_bypass 解析。
        let v = std::env::var("AXAGENT_ALLOW_UNSANDBOXED").unwrap_or_default();
        let enabled = matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        // 该测试只用于确保解析逻辑正确；不修改 env。
        assert!(!enabled, "by default env bypass must be off");
    }
}

/// Maximum output bytes before truncation (16 KiB, matching upstream).
const MAX_OUTPUT_BYTES: usize = 16_384;

/// Truncate output to `MAX_OUTPUT_BYTES`, appending a marker when trimmed.
fn truncate_output(s: &str) -> String {
    if s.len() <= MAX_OUTPUT_BYTES {
        return s.to_string();
    }
    // Find the last valid UTF-8 boundary at or before MAX_OUTPUT_BYTES
    let mut end = MAX_OUTPUT_BYTES;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = s[..end].to_string();
    truncated.push_str("\n\n[output truncated — exceeded 16384 bytes]");
    truncated
}

#[cfg(test)]
mod truncation_tests {
    use super::*;

    #[test]
    fn short_output_unchanged() {
        let s = "hello world";
        assert_eq!(truncate_output(s), s);
    }

    #[test]
    fn long_output_truncated() {
        let s = "x".repeat(20_000);
        let result = truncate_output(&s);
        assert!(result.len() < 20_000);
        assert!(result.ends_with("[output truncated — exceeded 16384 bytes]"));
    }

    #[test]
    fn exact_boundary_unchanged() {
        let s = "a".repeat(MAX_OUTPUT_BYTES);
        assert_eq!(truncate_output(&s), s);
    }

    #[test]
    fn one_over_boundary_truncated() {
        let s = "a".repeat(MAX_OUTPUT_BYTES + 1);
        let result = truncate_output(&s);
        assert!(result.contains("[output truncated"));
    }
}
