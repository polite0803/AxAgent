// SPDX-License-Identifier: AGPL-3.0-only

#![allow(dead_code)]

#[cfg(not(target_os = "android"))]
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "android"))]
use std::future::Future;
#[cfg(not(target_os = "android"))]
use std::pin::Pin;
#[cfg(not(target_os = "android"))]
use std::process::Stdio;
#[cfg(not(target_os = "android"))]
use std::time::Instant;
#[cfg(not(target_os = "android"))]
use tokio::process::Command;

#[cfg(not(target_os = "android"))]
use crate::skill_evolution::{
    ProcedureStep, SandboxExecutor, SandboxValidationResult, SkillGenome,
};

/// 命令最大长度（字符），防止超长命令导致解析 DoS 或绕过模式检测
#[cfg(not(target_os = "android"))]
const MAX_COMMAND_LEN: usize = 10_000;

/// 危险环境变量列表，子进程执行前必须清除，防止 LD_PRELOAD 等注入子进程
#[cfg(not(target_os = "android"))]
const DANGEROUS_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "LD_AUDIT",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "PERL5OPT",
    "PYTHONPATH",
    "RUBYOPT",
    "BASH_ENV",
    "ENV",
    "PS4",
    "NODE_OPTIONS",
    "JAVA_TOOL_OPTIONS",
];

/// 危险命令模式列表，用于基础过滤。
///
/// 注意：字符串包含检查可被简单混淆绕过（如 `rm -r"f /`、Base64+eval 等），
/// 仅作为第一道防线。真正的隔离依赖 env_clear、工作目录限制、rlimit 等机制。
#[cfg(not(target_os = "android"))]
const DANGEROUS_PATTERNS: &[&str] = &[
    // 删除/格式化磁盘
    "rm -rf /",
    "rm -rf /*",
    "rm -r -f /",
    "format c:",
    "del /s /q c:\\",
    "mkfs",
    // Windows 危险命令
    "rd /s /q",
    "rd /s/q",
    "powershell -enc",
    "pwsh -enc",
    "certutil -urlcache",
    "bitsadmin /transfer",
    "invoke-expression",
    "iex ",
    "net user ",
    "net localgroup ",
    "reg add ",
    "reg delete ",
    "schtasks /create",
    "wmic process call create",
    "start-process ",
    // 系统关机/重启
    "shutdown",
    "reboot",
    "halt",
    "poweroff",
    "init 0",
    "init 6",
    // fork bomb（多种变体，含无空格混淆形式）
    ":(){ :|:& };:",
    ":(){:|:&};:",
    ":() { :|: & };:",
    // 设备直接写入
    "dd if=/dev/zero of=",
    "dd if=",
    "> /dev/sd",
    "/dev/sda",
    "/dev/sdb",
    // 权限滥用
    "chmod 777 /",
    "chmod -R 777",
    "chown -R",
    // 远程脚本执行（管道下载并执行）
    "curl | sh",
    "curl | bash",
    "wget | bash",
    "wget | sh",
    // eval / base64 解码执行（常见混淆手段）
    "eval ",
    "base64 --decode",
    "base64 -d",
    // 提权
    "sudo ",
    "su -",
];

#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub allowed_tools: Vec<String>,
    pub max_steps: usize,
    pub timeout_secs: u64,
    pub max_output_bytes: u64,
}

#[cfg(not(target_os = "android"))]
impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            allowed_tools: vec![
                "read_file".into(),
                "write_file".into(),
                "list_dir".into(),
                "search".into(),
                "execute_bash".into(),
                "grep".into(),
            ],
            max_steps: 50,
            timeout_secs: 30,
            max_output_bytes: 1024 * 1024,
        }
    }
}

#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepValidationResult {
    pub step_order: usize,
    pub tool: Option<String>,
    pub allowed: bool,
    pub executed: bool,
    pub success: bool,
    pub execution_time_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub violations: Vec<String>,
}

#[cfg(not(target_os = "android"))]
pub struct SkillSandboxExecutor {
    policy: SandboxPolicy,
}

#[cfg(not(target_os = "android"))]
impl SkillSandboxExecutor {
    pub(crate) fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }

    pub fn with_default_policy() -> Self {
        Self::new(SandboxPolicy::default())
    }

    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    fn validate_step(&self, step: &ProcedureStep) -> StepValidationResult {
        let mut violations = Vec::new();

        if step.order >= self.policy.max_steps {
            violations.push(format!(
                "step order {} exceeds max_steps {}",
                step.order, self.policy.max_steps
            ));
        }

        if let Some(ref tool) = step.tool
            && !self.policy.allowed_tools.contains(tool)
        {
            violations.push(format!("tool '{}' is not in allowed list", tool));
        }

        if step.action.is_empty() {
            violations.push("step action is empty".into());
        }

        // SECURITY: 命令长度限制，防止超长命令导致 DoS 或绕过模式检测
        if step.action.len() > MAX_COMMAND_LEN {
            violations.push(format!(
                "command length {} exceeds max {} characters",
                step.action.len(),
                MAX_COMMAND_LEN
            ));
        }

        // SECURITY: 危险模式检测（仅第一道防线，可被混淆绕过）
        let action_lower = step.action.to_lowercase();
        for pattern in DANGEROUS_PATTERNS {
            if action_lower.contains(pattern) {
                violations.push(format!("dangerous pattern detected: '{}'", pattern));
            }
        }

        let allowed = violations.is_empty();

        StepValidationResult {
            step_order: step.order,
            tool: step.tool.clone(),
            allowed,
            executed: false,
            success: false,
            execution_time_ms: 0,
            stdout: String::new(),
            stderr: String::new(),
            violations,
        }
    }

    async fn execute_step(&self, step: &ProcedureStep) -> StepValidationResult {
        let mut result = self.validate_step(step);

        if !result.allowed {
            return result;
        }

        let action = step.action.trim();

        let command_str = if let Some(ref tool) = step.tool {
            match tool.as_str() {
                "execute_bash" | "bash" | "sh" => {
                    let cmd = action
                        .strip_prefix("Use execute_bash")
                        .or_else(|| action.strip_prefix("Use bash"))
                        .or_else(|| action.strip_prefix("Use sh"))
                        .unwrap_or(action);
                    let cmd = cmd.trim().trim_start_matches("with").trim();
                    let cmd = cmd.trim_start_matches("args").trim();
                    let cmd = cmd.trim().trim_start_matches(':').trim();
                    Some(cmd.to_string())
                },
                _ => None,
            }
        } else {
            None
        };

        if let Some(cmd) = command_str {
            if cmd.is_empty() {
                result.executed = true;
                result.success = true;
                result.stdout = "(no command to execute)".into();
                return result;
            }

            let start = Instant::now();

            let output_result =
                tokio::time::timeout(std::time::Duration::from_secs(self.policy.timeout_secs), {
                    #[cfg(target_family = "windows")]
                    {
                        let mut scmd = Command::new("cmd");
                        scmd.args(["/C", &cmd]).stdout(Stdio::piped()).stderr(Stdio::piped());
                        // SECURITY: 清除危险环境变量，防止 LD_PRELOAD 等注入子进程
                        for var in DANGEROUS_ENV_VARS {
                            scmd.env_remove(var);
                        }
                        // SECURITY: 设置工作目录到系统临时目录，仅作为 CWD 起点，不限制文件系统访问（子进程仍可通过绝对路径读写任何文件）
                        scmd.current_dir(std::env::temp_dir());
                        axagent_kit::utils::hide_window(scmd.as_std_mut());
                        scmd.output()
                    }
                    #[cfg(not(target_family = "windows"))]
                    {
                        let mut scmd = Command::new("sh");
                        scmd.args(["-c", &cmd]).stdout(Stdio::piped()).stderr(Stdio::piped());
                        // SECURITY: 清除危险环境变量，防止 LD_PRELOAD 等注入子进程
                        for var in DANGEROUS_ENV_VARS {
                            scmd.env_remove(var);
                        }
                        // SECURITY: 设置工作目录到系统临时目录，仅作为 CWD 起点，不限制文件系统访问（子进程仍可通过绝对路径读写任何文件）
                        scmd.current_dir(std::env::temp_dir());
                        scmd.output()
                    }
                })
                .await;

            result.execution_time_ms = start.elapsed().as_millis() as u64;

            match output_result {
                Ok(Ok(output)) => {
                    result.executed = true;
                    result.success = output.status.success();

                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    let max_bytes = self.policy.max_output_bytes as usize;
                    result.stdout = if stdout.len() > max_bytes {
                        stdout[..max_bytes].to_string()
                    } else {
                        stdout.into_owned()
                    };
                    result.stderr = if stderr.len() > max_bytes {
                        stderr[..max_bytes].to_string()
                    } else {
                        stderr.into_owned()
                    };
                },
                Ok(Err(e)) => {
                    result.executed = true;
                    result.success = false;
                    result.stderr = format!("execution error: {}", e);
                },
                Err(_) => {
                    result.executed = true;
                    result.success = false;
                    result.stderr =
                        format!("execution timed out after {}s", self.policy.timeout_secs);
                },
            }
        } else {
            result.executed = true;
            result.success = true;
            result.stdout = format!("(validated step: {})", step.action);
        }

        result
    }
}

#[cfg(not(target_os = "android"))]
impl SandboxExecutor for SkillSandboxExecutor {
    fn execute_skill<'a>(
        &'a self,
        genome: &'a SkillGenome,
        _test_input: &str,
    ) -> Pin<Box<dyn Future<Output = Result<SandboxValidationResult, String>> + Send + 'a>> {
        let steps = genome.steps.clone();
        let policy = self.policy.clone();
        Box::pin(async move {
            if steps.is_empty() {
                return Ok(SandboxValidationResult {
                    passed: false,
                    success_rate: 0.0,
                    execution_errors: vec!["genome has no steps".into()],
                    avg_execution_time_ms: 0,
                });
            }

            let executor = SkillSandboxExecutor::new(policy);
            let mut step_results = Vec::with_capacity(steps.len());
            let mut errors = Vec::new();
            let mut total_time_ms: u64 = 0;
            let mut success_count: usize = 0;

            for step in &steps {
                let result = executor.execute_step(step).await;
                total_time_ms += result.execution_time_ms;

                if !result.allowed {
                    errors.push(format!(
                        "step {} blocked: {}",
                        result.step_order,
                        result.violations.join(", ")
                    ));
                } else if !result.success {
                    errors.push(format!("step {} failed: {}", result.step_order, result.stderr));
                } else {
                    success_count += 1;
                }

                step_results.push(result);
            }

            let success_rate = success_count as f64 / steps.len() as f64;
            let avg_time = if !step_results.is_empty() {
                total_time_ms / step_results.len() as u64
            } else {
                0
            };

            let passed = success_rate >= 0.5 && errors.iter().all(|e| !e.contains("blocked"));

            Ok(SandboxValidationResult {
                passed,
                success_rate,
                execution_errors: errors,
                avg_execution_time_ms: avg_time,
            })
        })
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) struct DryRunSandboxExecutor {
    policy: SandboxPolicy,
}

#[cfg(not(target_os = "android"))]
impl DryRunSandboxExecutor {
    pub(crate) fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }

    pub(crate) fn with_default_policy() -> Self {
        Self::new(SandboxPolicy::default())
    }

    fn validate_step(&self, step: &ProcedureStep) -> StepValidationResult {
        let mut violations = Vec::new();

        if step.order >= self.policy.max_steps {
            violations.push(format!(
                "step order {} exceeds max_steps {}",
                step.order, self.policy.max_steps
            ));
        }

        if let Some(ref tool) = step.tool
            && !self.policy.allowed_tools.contains(tool)
        {
            violations.push(format!("tool '{}' is not in allowed list", tool));
        }

        if step.action.is_empty() {
            violations.push("step action is empty".into());
        }

        // SECURITY: 命令长度限制，防止超长命令导致 DoS 或绕过模式检测
        if step.action.len() > MAX_COMMAND_LEN {
            violations.push(format!(
                "command length {} exceeds max {} characters",
                step.action.len(),
                MAX_COMMAND_LEN
            ));
        }

        // SECURITY: 危险模式检测（仅第一道防线，可被混淆绕过）
        let action_lower = step.action.to_lowercase();
        for pattern in DANGEROUS_PATTERNS {
            if action_lower.contains(pattern) {
                violations.push(format!("dangerous pattern detected: '{}'", pattern));
            }
        }

        let allowed = violations.is_empty();

        StepValidationResult {
            step_order: step.order,
            tool: step.tool.clone(),
            allowed,
            executed: false,
            success: allowed,
            execution_time_ms: 0,
            stdout: String::new(),
            stderr: String::new(),
            violations,
        }
    }
}

#[cfg(not(target_os = "android"))]
impl SandboxExecutor for DryRunSandboxExecutor {
    fn execute_skill<'a>(
        &'a self,
        genome: &'a SkillGenome,
        _test_input: &str,
    ) -> Pin<Box<dyn Future<Output = Result<SandboxValidationResult, String>> + Send + 'a>> {
        let steps = genome.steps.clone();
        let policy = self.policy.clone();
        Box::pin(async move {
            if steps.is_empty() {
                return Ok(SandboxValidationResult {
                    passed: false,
                    success_rate: 0.0,
                    execution_errors: vec!["genome has no steps".into()],
                    avg_execution_time_ms: 0,
                });
            }

            let executor = DryRunSandboxExecutor::new(policy);
            let mut errors = Vec::new();
            let mut success_count: usize = 0;

            for step in &steps {
                let result = executor.validate_step(step);
                if !result.allowed {
                    errors.push(format!(
                        "step {} blocked: {}",
                        result.step_order,
                        result.violations.join(", ")
                    ));
                } else {
                    success_count += 1;
                }
            }

            let success_rate = success_count as f64 / steps.len() as f64;
            let passed = success_rate >= 0.5;

            Ok(SandboxValidationResult {
                passed,
                success_rate,
                execution_errors: errors,
                avg_execution_time_ms: 0,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_genome(steps: Vec<ProcedureStep>) -> SkillGenome {
        SkillGenome {
            skill_id: "test_skill".into(),
            content: "test content".into(),
            description: "test description".into(),
            steps,
            fitness: 0.5,
        }
    }

    fn make_step(order: usize, action: &str, tool: Option<&str>) -> ProcedureStep {
        ProcedureStep {
            order,
            action: action.into(),
            tool: tool.map(|t| t.into()),
            condition: None,
            error_handling: None,
        }
    }

    #[test]
    fn test_sandbox_policy_default() {
        let policy = SandboxPolicy::default();
        assert!(!policy.allowed_tools.is_empty());
        assert_eq!(policy.max_steps, 50);
        assert_eq!(policy.timeout_secs, 30);
    }

    #[test]
    fn test_validate_step_allowed_tool() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let step = make_step(0, "Use read_file with args", Some("read_file"));
        let result = executor.validate_step(&step);
        assert!(result.allowed);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_validate_step_denied_tool() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let step = make_step(0, "Use dangerous_tool with args", Some("dangerous_tool"));
        let result = executor.validate_step(&step);
        assert!(!result.allowed);
        assert!(result.violations.iter().any(|v| v.contains("not in allowed list")));
    }

    #[test]
    fn test_validate_step_exceeds_max_steps() {
        let policy = SandboxPolicy { max_steps: 2, ..SandboxPolicy::default() };
        let executor = SkillSandboxExecutor::new(policy);
        let step = make_step(5, "Use read_file", Some("read_file"));
        let result = executor.validate_step(&step);
        assert!(!result.allowed);
        assert!(result.violations.iter().any(|v| v.contains("exceeds max_steps")));
    }

    #[test]
    fn test_validate_step_dangerous_pattern() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let step = make_step(0, "Use execute_bash with rm -rf /", Some("execute_bash"));
        let result = executor.validate_step(&step);
        assert!(!result.allowed);
        assert!(result.violations.iter().any(|v| v.contains("dangerous pattern")));
    }

    #[test]
    fn test_validate_step_empty_action() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let step = make_step(0, "", Some("read_file"));
        let result = executor.validate_step(&step);
        assert!(!result.allowed);
        assert!(result.violations.iter().any(|v| v.contains("empty")));
    }

    #[tokio::test]
    async fn test_dry_run_executor_all_allowed() {
        let executor = DryRunSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![
            make_step(0, "Use read_file with args", Some("read_file")),
            make_step(1, "Use search with query", Some("search")),
        ]);
        let result =
            executor.execute_skill(&genome, "test input").await.expect("测试：异步操作应成功");
        assert!(result.passed);
        assert!((result.success_rate - 1.0).abs() < f64::EPSILON);
        assert!(result.execution_errors.is_empty());
    }

    #[tokio::test]
    async fn test_dry_run_executor_denied_tool() {
        let executor = DryRunSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![
            make_step(0, "Use read_file with args", Some("read_file")),
            make_step(1, "Use hack_tool", Some("hack_tool")),
            make_step(2, "Use exploit_tool", Some("exploit_tool")),
        ]);
        let result =
            executor.execute_skill(&genome, "test input").await.expect("测试：异步操作应成功");
        assert!(!result.passed);
        assert!(result.success_rate < 1.0);
        assert!(!result.execution_errors.is_empty());
    }

    #[tokio::test]
    async fn test_dry_run_executor_empty_genome() {
        let executor = DryRunSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![]);
        let result =
            executor.execute_skill(&genome, "test input").await.expect("测试：异步操作应成功");
        assert!(!result.passed);
        assert_eq!(result.success_rate, 0.0);
    }

    #[tokio::test]
    async fn test_skill_sandbox_executor_simple_command() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![make_step(
            0,
            "Use execute_bash with args: echo hello",
            Some("execute_bash"),
        )]);
        let result =
            executor.execute_skill(&genome, "test input").await.expect("测试：异步操作应成功");
        assert!(result.passed);
        assert!(result.avg_execution_time_ms > 0 || result.success_rate > 0.0);
    }

    #[tokio::test]
    async fn test_skill_sandbox_executor_blocked_step() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let genome =
            make_genome(vec![make_step(0, "Use execute_bash with rm -rf /", Some("execute_bash"))]);
        let result =
            executor.execute_skill(&genome, "test input").await.expect("测试：异步操作应成功");
        assert!(!result.passed);
        assert!(!result.execution_errors.is_empty());
    }

    #[tokio::test]
    async fn test_skill_sandbox_executor_non_executable_step() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![make_step(
            0,
            "Use read_file with args: /tmp/test.txt",
            Some("read_file"),
        )]);
        let result =
            executor.execute_skill(&genome, "test input").await.expect("测试：异步操作应成功");
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_skill_sandbox_executor_mixed_steps() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![
            make_step(0, "Use read_file with args", Some("read_file")),
            make_step(1, "Use hack_tool", Some("hack_tool")),
            make_step(2, "Use search with query", Some("search")),
        ]);
        let result =
            executor.execute_skill(&genome, "test input").await.expect("测试：异步操作应成功");
        assert!(!result.execution_errors.is_empty());
        assert!(result.success_rate > 0.0 && result.success_rate < 1.0);
    }

    #[test]
    fn test_step_validation_result_serialization() {
        let result = StepValidationResult {
            step_order: 0,
            tool: Some("read_file".into()),
            allowed: true,
            executed: false,
            success: true,
            execution_time_ms: 50,
            stdout: "output".into(),
            stderr: String::new(),
            violations: vec![],
        };
        let json = serde_json::to_string(&result).expect("测试：JSON序列化应成功");
        assert!(json.contains("read_file"));
        let deserialized: StepValidationResult =
            serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(deserialized.step_order, 0);
        assert!(deserialized.allowed);
    }

    #[test]
    fn test_sandbox_policy_custom() {
        let policy = SandboxPolicy {
            allowed_tools: vec!["custom_tool".into()],
            max_steps: 10,
            timeout_secs: 5,
            max_output_bytes: 512,
        };
        let executor = SkillSandboxExecutor::new(policy);
        let step = make_step(0, "Use custom_tool", Some("custom_tool"));
        let result = executor.validate_step(&step);
        assert!(result.allowed);

        let step2 = make_step(0, "Use read_file", Some("read_file"));
        let result2 = executor.validate_step(&step2);
        assert!(!result2.allowed);
    }
}

#[cfg(target_os = "android")]
pub struct SkillSandboxExecutor;

#[cfg(target_os = "android")]
impl SkillSandboxExecutor {
    pub fn with_default_policy() -> Self {
        Self
    }
}
