use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

use crate::skill_evolution::{
    ProcedureStep, SandboxExecutor, SandboxValidationResult, SkillGenome,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub allowed_tools: Vec<String>,
    pub max_steps: usize,
    pub timeout_secs: u64,
    pub max_output_bytes: u64,
}

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

pub struct SkillSandboxExecutor {
    policy: SandboxPolicy,
}

impl SkillSandboxExecutor {
    pub fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }

    pub fn with_default_policy() -> Self {
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

        if let Some(ref tool) = step.tool {
            if !self.policy.allowed_tools.contains(tool) {
                violations.push(format!("tool '{}' is not in allowed list", tool));
            }
        }

        if step.action.is_empty() {
            violations.push("step action is empty".into());
        }

        let dangerous_patterns = [
            "rm -rf /",
            "format c:",
            "del /s /q c:\\",
            "shutdown",
            ":(){ :|:& };:",
            "mkfs",
            "dd if=",
            "> /dev/sd",
            "chmod 777 /",
            "curl | sh",
            "wget | bash",
        ];
        let action_lower = step.action.to_lowercase();
        for pattern in &dangerous_patterns {
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

            let output_result = tokio::time::timeout(
                std::time::Duration::from_secs(self.policy.timeout_secs),
                Command::new("cmd")
                    .args(["/C", &cmd])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output(),
            )
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

pub struct DryRunSandboxExecutor {
    policy: SandboxPolicy,
}

impl DryRunSandboxExecutor {
    pub fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }

    pub fn with_default_policy() -> Self {
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

        if let Some(ref tool) = step.tool {
            if !self.policy.allowed_tools.contains(tool) {
                violations.push(format!("tool '{}' is not in allowed list", tool));
            }
        }

        if step.action.is_empty() {
            violations.push("step action is empty".into());
        }

        let dangerous_patterns = [
            "rm -rf /",
            "format c:",
            "del /s /q c:\\",
            "shutdown",
            ":(){ :|:& };:",
            "mkfs",
            "dd if=",
            "> /dev/sd",
            "chmod 777 /",
            "curl | sh",
            "wget | bash",
        ];
        let action_lower = step.action.to_lowercase();
        for pattern in &dangerous_patterns {
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
        assert!(result
            .violations
            .iter()
            .any(|v| v.contains("not in allowed list")));
    }

    #[test]
    fn test_validate_step_exceeds_max_steps() {
        let policy = SandboxPolicy {
            max_steps: 2,
            ..SandboxPolicy::default()
        };
        let executor = SkillSandboxExecutor::new(policy);
        let step = make_step(5, "Use read_file", Some("read_file"));
        let result = executor.validate_step(&step);
        assert!(!result.allowed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.contains("exceeds max_steps")));
    }

    #[test]
    fn test_validate_step_dangerous_pattern() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let step = make_step(0, "Use execute_bash with rm -rf /", Some("execute_bash"));
        let result = executor.validate_step(&step);
        assert!(!result.allowed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.contains("dangerous pattern")));
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
        let result = executor.execute_skill(&genome, "test input").await.unwrap();
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
        let result = executor.execute_skill(&genome, "test input").await.unwrap();
        assert!(!result.passed);
        assert!(result.success_rate < 1.0);
        assert!(!result.execution_errors.is_empty());
    }

    #[tokio::test]
    async fn test_dry_run_executor_empty_genome() {
        let executor = DryRunSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![]);
        let result = executor.execute_skill(&genome, "test input").await.unwrap();
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
        let result = executor.execute_skill(&genome, "test input").await.unwrap();
        assert!(result.passed);
        assert!(result.avg_execution_time_ms > 0 || result.success_rate > 0.0);
    }

    #[tokio::test]
    async fn test_skill_sandbox_executor_blocked_step() {
        let executor = SkillSandboxExecutor::with_default_policy();
        let genome = make_genome(vec![make_step(
            0,
            "Use execute_bash with rm -rf /",
            Some("execute_bash"),
        )]);
        let result = executor.execute_skill(&genome, "test input").await.unwrap();
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
        let result = executor.execute_skill(&genome, "test input").await.unwrap();
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
        let result = executor.execute_skill(&genome, "test input").await.unwrap();
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
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("read_file"));
        let deserialized: StepValidationResult = serde_json::from_str(&json).unwrap();
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
