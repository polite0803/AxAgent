//! Parallel Execution Module - Multi-agent parallel task execution and result aggregation
//!
//! This module provides infrastructure for executing multiple independent tasks in parallel:
//! - Parallel task dispatch and execution
//! - Result aggregation and presentation
//! - Execution status tracking
//! - Timeout and error handling

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_prompt: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub agent_id: Option<String>,
    /// 任务超时时间（秒），None 表示不限制
    pub timeout_secs: Option<u64>,
    /// 输出 JSON Schema 校验字符串，None 表示不校验
    pub expected_output_schema: Option<String>,
}

impl ParallelTask {
    pub fn new(name: String, description: String, task_prompt: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            task_prompt,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            agent_id: None,
            timeout_secs: None,
            expected_output_schema: None,
        }
    }

    pub fn start(&mut self, agent_id: String) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
        self.agent_id = Some(agent_id);
    }

    pub fn complete(&mut self, result: String) {
        self.status = TaskStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
        self.progress = 1.0;
    }

    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.completed_at.and_then(|completed| {
            self.started_at
                .map(|started| (completed - started).num_milliseconds() as u64)
        })
    }

    /// 标记任务为超时
    pub fn mark_timeout(&mut self) {
        self.status = TaskStatus::Timeout;
        self.completed_at = Some(Utc::now());
        self.error = Some("任务执行超时".to_string());
    }

    /// 设置任务超时时间（秒）
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// 设置输出校验 Schema（JSON Schema 字符串）
    pub fn with_schema(mut self, schema: String) -> Self {
        self.expected_output_schema = Some(schema);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecution {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<ParallelTask>,
    pub status: ExecutionStatus,
    pub strategy: ExecutionStrategy,
    pub max_parallel: usize,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub aggregated_result: Option<String>,
}

impl ParallelExecution {
    pub fn new(
        name: String,
        description: String,
        strategy: ExecutionStrategy,
        max_parallel: usize,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            tasks: Vec::new(),
            status: ExecutionStatus::Pending,
            strategy,
            max_parallel,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            aggregated_result: None,
        }
    }

    pub fn add_task(&mut self, task: ParallelTask) {
        self.tasks.push(task);
    }

    pub fn add_tasks(&mut self, tasks: Vec<ParallelTask>) {
        self.tasks.extend(tasks);
    }

    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|t| {
            t.status == TaskStatus::Completed
                || t.status == TaskStatus::Failed
                || t.status == TaskStatus::Cancelled
                || t.status == TaskStatus::Timeout
        })
    }

    pub fn completed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count()
    }

    pub fn running_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count()
    }

    pub fn pending_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count()
    }

    pub fn overall_progress(&self) -> f32 {
        if self.tasks.is_empty() {
            return 0.0;
        }
        let total: f32 = self.tasks.iter().map(|t| t.progress).sum();
        total / self.tasks.len() as f32
    }

    pub fn aggregate_results(&mut self) -> String {
        let mut lines = vec![
            format!("# {} - 执行汇总\n", self.name),
            format!("总任务数: {}\n", self.tasks.len()),
            format!("成功: {}, 失败: {}\n", self.completed_count(), self.failed_count()),
            format!("执行时间: {} ms\n", self.duration_ms().unwrap_or(0)),
            "\n## 任务结果:\n".to_string(),
        ];

        for (i, task) in self.tasks.iter().enumerate() {
            lines.push(format!("\n### {}. {} [{}]", i + 1, task.name, format_status(&task.status)));

            if let Some(ref result) = task.result {
                lines.push(format!("\n结果:\n{}\n", result));
            }
            if let Some(ref error) = task.error {
                lines.push(format!("\n错误:\n{}\n", error));
            }
            if let Some(ms) = task.duration_ms() {
                lines.push(format!("耗时: {} ms\n", ms));
            }
        }

        let aggregated = lines.join("");
        self.aggregated_result = Some(aggregated.clone());
        aggregated
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.completed_at.and_then(|completed| {
            self.started_at
                .map(|started| (completed - started).num_milliseconds() as u64)
        })
    }
}

fn format_status(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "⏳ 等待中",
        TaskStatus::Running => "🔄 运行中",
        TaskStatus::Completed => "✅ 完成",
        TaskStatus::Failed => "❌ 失败",
        TaskStatus::Cancelled => "🚫 已取消",
        TaskStatus::Timeout => "⏱️ 超时",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ExecutionStrategy {
    Sequential,
    #[default]
    Parallel,
    PriorityBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub total_tasks: usize,
    pub completed: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub aggregated_summary: String,
    pub task_results: Vec<TaskResultSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultSummary {
    pub task_id: String,
    pub task_name: String,
    pub status: TaskStatus,
    pub result_preview: Option<String>,
    pub error_preview: Option<String>,
    pub duration_ms: Option<u64>,
}

impl From<&ParallelTask> for TaskResultSummary {
    fn from(task: &ParallelTask) -> Self {
        Self {
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            status: task.status,
            result_preview: task.result.as_ref().map(|r| {
                if r.len() > 200 {
                    format!("{}...", &r[..200])
                } else {
                    r.clone()
                }
            }),
            error_preview: task.error.as_ref().map(|e| {
                if e.len() > 200 {
                    format!("{}...", &e[..200])
                } else {
                    e.clone()
                }
            }),
            duration_ms: task.duration_ms(),
        }
    }
}

// ============================================================
// 并行执行验证
// ============================================================

/// 配置哪些验证检查项启用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// 启用输出完整性检查
    pub enable_completeness: bool,
    /// 启用 JSON Schema 校验
    pub enable_schema_validation: bool,
    /// 启用跨任务一致性检查
    pub enable_cross_validation: bool,
    /// 启用超时合规检查
    pub enable_timeout_check: bool,
    /// 启用错误率检查
    pub enable_error_rate: bool,
    /// 启用输出大小检查
    pub enable_output_size: bool,
    /// 单条结果最大字节数
    pub max_result_size_bytes: usize,
    /// 最大允许错误率（0.0-1.0）
    pub max_error_rate: f64,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            enable_completeness: true,
            enable_schema_validation: true,
            enable_cross_validation: true,
            enable_timeout_check: true,
            enable_error_rate: true,
            enable_output_size: true,
            max_result_size_bytes: 1_048_576,
            max_error_rate: 0.3,
        }
    }
}

/// 单项验证检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    /// 检查项名称
    pub name: String,
    /// 是否通过
    pub passed: bool,
    /// 详细描述
    pub detail: String,
    /// 该检查项的得分（0.0-1.0）
    pub score: f64,
}

/// 整体验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// 关联的执行 ID
    pub execution_id: String,
    /// 全部检查项是否通过
    pub is_valid: bool,
    /// 所有检查项列表
    pub checks: Vec<VerificationCheck>,
    /// 加权总分（0.0-1.0）
    pub overall_score: f64,
}

/// 并行执行验证器 —— 对已完成执行进行事后验证
pub struct ParallelExecutionVerifier {
    config: VerificationConfig,
}

impl ParallelExecutionVerifier {
    /// 使用指定配置创建验证器
    pub fn new(config: VerificationConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建验证器
    pub fn with_defaults() -> Self {
        Self::new(VerificationConfig::default())
    }

    /// 主入口：对一次并行执行进行全量验证
    pub fn verify(&self, execution: &ParallelExecution) -> VerificationResult {
        let mut checks = Vec::new();

        if self.config.enable_completeness {
            checks.push(self.check_output_completeness(execution));
        }
        if self.config.enable_schema_validation {
            checks.push(self.check_schema_validation(execution));
        }
        if self.config.enable_cross_validation {
            checks.push(self.check_cross_task_consistency(execution));
        }
        if self.config.enable_timeout_check {
            checks.push(self.check_timeout_compliance(execution));
        }
        if self.config.enable_error_rate {
            checks.push(self.check_error_rate(execution));
        }
        if self.config.enable_output_size {
            checks.push(self.check_output_size(execution));
        }

        let overall_score = Self::compute_weighted_score(&checks);

        VerificationResult {
            execution_id: execution.id.clone(),
            is_valid: checks.iter().all(|c| c.passed),
            checks,
            overall_score,
        }
    }

    // ── 检查 1：输出完整性 ──
    fn check_output_completeness(&self, execution: &ParallelExecution) -> VerificationCheck {
        let completable: Vec<&ParallelTask> = execution
            .tasks
            .iter()
            .filter(|t| {
                !matches!(
                    t.status,
                    TaskStatus::Cancelled | TaskStatus::Pending | TaskStatus::Running
                )
            })
            .collect();

        if completable.is_empty() {
            return VerificationCheck {
                name: "output_completeness".to_string(),
                passed: true,
                detail: "没有需要检查的任务".to_string(),
                score: 1.0,
            };
        }

        let with_results = completable.iter().filter(|t| t.result.is_some()).count();
        let missing: Vec<&str> = completable
            .iter()
            .filter(|t| t.result.is_none())
            .map(|t| t.name.as_str())
            .collect();

        let passed = missing.is_empty();
        let score = with_results as f64 / completable.len() as f64;

        VerificationCheck {
            name: "output_completeness".to_string(),
            passed,
            detail: if passed {
                format!("全部 {} 个已完成任务都有输出结果", with_results)
            } else {
                format!(
                    "{}/{} 个任务有输出结果，缺少结果的任务: [{}]",
                    with_results,
                    completable.len(),
                    missing.join(", ")
                )
            },
            score,
        }
    }

    // ── 检查 2：Schema 校验 ──
    fn check_schema_validation(&self, execution: &ParallelExecution) -> VerificationCheck {
        let tasks_with_schema: Vec<&ParallelTask> = execution
            .tasks
            .iter()
            .filter(|t| {
                t.expected_output_schema.is_some()
                    && t.status == TaskStatus::Completed
                    && t.result.is_some()
            })
            .collect();

        if tasks_with_schema.is_empty() {
            return VerificationCheck {
                name: "schema_validation".to_string(),
                passed: true,
                detail: "没有任务配置了 Schema 校验或都未完成".to_string(),
                score: 1.0,
            };
        }

        let mut passed_count = 0;
        let mut all_details = Vec::new();

        for task in &tasks_with_schema {
            let schema_str = task.expected_output_schema.as_ref().unwrap();
            let output = task.result.as_ref().unwrap();

            let schema: Result<serde_json::Value, _> = serde_json::from_str(schema_str);
            let output_value: Result<serde_json::Value, _> = serde_json::from_str(output);

            match (schema, output_value) {
                (Ok(schema), Ok(value)) => {
                    let (valid, errors) = axagent_core::validate_against_schema(&value, &schema);
                    if valid {
                        passed_count += 1;
                        all_details.push(format!("{}: 通过", task.name));
                    } else {
                        all_details.push(format!("{}: 失败 - {}", task.name, errors.join("; ")));
                    }
                },
                (Err(e), _) => {
                    all_details.push(format!("{}: Schema 解析失败 - {}", task.name, e));
                },
                (_, Err(e)) => {
                    all_details.push(format!("{}: 输出不是有效 JSON - {}", task.name, e));
                },
            }
        }

        let total = tasks_with_schema.len();
        let passed = passed_count == total;
        let score = passed_count as f64 / total as f64;

        VerificationCheck {
            name: "schema_validation".to_string(),
            passed,
            detail: format!(
                "{}/{} Schema 校验通过\n{}",
                passed_count,
                total,
                all_details.join("\n")
            ),
            score,
        }
    }

    // ── 检查 3：跨任务一致性 ──
    fn check_cross_task_consistency(&self, execution: &ParallelExecution) -> VerificationCheck {
        let completed_tasks: Vec<&ParallelTask> = execution
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed && t.result.is_some())
            .collect();

        if completed_tasks.len() < 2 {
            return VerificationCheck {
                name: "cross_task_consistency".to_string(),
                passed: true,
                detail: "不足 2 个已完成任务，无需检查一致性".to_string(),
                score: 1.0,
            };
        }

        // 将任务结果解析为 JSON 对象
        let parsed: Vec<(&str, serde_json::Map<String, serde_json::Value>)> = completed_tasks
            .iter()
            .filter_map(|t| {
                serde_json::from_str::<serde_json::Value>(t.result.as_ref().unwrap())
                    .ok()
                    .and_then(|v| v.as_object().cloned())
                    .map(|obj| (t.name.as_str(), obj))
            })
            .collect();

        if parsed.len() < 2 {
            return VerificationCheck {
                name: "cross_task_consistency".to_string(),
                passed: true,
                detail: "不足 2 个任务输出为 JSON 对象，跳过一致性检查".to_string(),
                score: 1.0,
            };
        }

        let mut conflicts = Vec::new();

        for i in 0..parsed.len() {
            for j in (i + 1)..parsed.len() {
                let (name_a, obj_a) = &parsed[i];
                let (name_b, obj_b) = &parsed[j];

                for key in obj_a.keys() {
                    if let (Some(val_a), Some(val_b)) = (obj_a.get(key), obj_b.get(key)) {
                        let is_scalar = |v: &serde_json::Value| {
                            v.is_string() || v.is_number() || v.is_boolean()
                        };
                        if is_scalar(val_a) && is_scalar(val_b) && val_a != val_b {
                            conflicts.push(format!(
                                "{}.{} = {:?} vs {}.{} = {:?}",
                                name_a, key, val_a, name_b, key, val_b
                            ));
                        }
                    }
                }
            }
        }

        let passed = conflicts.is_empty();
        let score = if parsed.len() <= 1 {
            1.0
        } else {
            let total_pairs = parsed.len() * (parsed.len() - 1) / 2;
            let max_conflicts = total_pairs * 10;
            if max_conflicts == 0 {
                1.0
            } else {
                (1.0 - conflicts.len() as f64 / max_conflicts as f64).max(0.0)
            }
        };

        VerificationCheck {
            name: "cross_task_consistency".to_string(),
            passed,
            detail: if passed {
                "所有并行任务输出一致，没有冲突字段".to_string()
            } else {
                format!("发现 {} 个字段值冲突: {}", conflicts.len(), conflicts.join("; "))
            },
            score,
        }
    }

    // ── 检查 4：超时合规 ──
    fn check_timeout_compliance(&self, execution: &ParallelExecution) -> VerificationCheck {
        let tasks_with_timeout: Vec<&ParallelTask> = execution
            .tasks
            .iter()
            .filter(|t| t.timeout_secs.is_some())
            .collect();

        if tasks_with_timeout.is_empty() {
            return VerificationCheck {
                name: "timeout_compliance".to_string(),
                passed: true,
                detail: "没有任务配置了超时限制".to_string(),
                score: 1.0,
            };
        }

        let mut compliant = 0;
        let mut violations = Vec::new();

        for task in &tasks_with_timeout {
            let limit_secs = task.timeout_secs.unwrap();
            let elapsed = task.duration_ms().unwrap_or(0) as f64 / 1000.0;

            if elapsed <= limit_secs as f64 {
                compliant += 1;
            } else {
                violations
                    .push(format!("{}: 实际 {:.1}s > 限制 {}s", task.name, elapsed, limit_secs));
            }
        }

        let total = tasks_with_timeout.len();
        let passed = violations.is_empty();
        let score = compliant as f64 / total as f64;

        VerificationCheck {
            name: "timeout_compliance".to_string(),
            passed,
            detail: if passed {
                format!("全部 {} 个任务在超时限制内完成", total)
            } else {
                format!("{}/{} 超时合规，违规: {}", compliant, total, violations.join("; "))
            },
            score,
        }
    }

    // ── 检查 5：错误率 ──
    fn check_error_rate(&self, execution: &ParallelExecution) -> VerificationCheck {
        let non_cancelled: Vec<&ParallelTask> = execution
            .tasks
            .iter()
            .filter(|t| {
                !matches!(
                    t.status,
                    TaskStatus::Cancelled | TaskStatus::Pending | TaskStatus::Running
                )
            })
            .collect();

        if non_cancelled.is_empty() {
            return VerificationCheck {
                name: "error_rate".to_string(),
                passed: true,
                detail: "没有已完成的任务".to_string(),
                score: 1.0,
            };
        }

        let failed = non_cancelled
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Failed | TaskStatus::Timeout))
            .count();
        let error_rate = failed as f64 / non_cancelled.len() as f64;
        let passed = error_rate <= self.config.max_error_rate;
        let score = (1.0 - error_rate).max(0.0);

        VerificationCheck {
            name: "error_rate".to_string(),
            passed,
            detail: format!(
                "错误率 {:.1}%（{}/{}），阈值 {:.1}%",
                error_rate * 100.0,
                failed,
                non_cancelled.len(),
                self.config.max_error_rate * 100.0
            ),
            score,
        }
    }

    // ── 检查 6：输出大小 ──
    fn check_output_size(&self, execution: &ParallelExecution) -> VerificationCheck {
        let tasks_with_results: Vec<&ParallelTask> = execution
            .tasks
            .iter()
            .filter(|t| t.result.is_some())
            .collect();

        if tasks_with_results.is_empty() {
            return VerificationCheck {
                name: "output_size".to_string(),
                passed: true,
                detail: "没有任务输出".to_string(),
                score: 1.0,
            };
        }

        let mut oversized = Vec::new();
        let total = tasks_with_results.len();

        for task in &tasks_with_results {
            let size = task.result.as_ref().unwrap().len();
            if size > self.config.max_result_size_bytes {
                oversized.push(format!(
                    "{}: {} bytes（上限 {} bytes）",
                    task.name, size, self.config.max_result_size_bytes
                ));
            }
        }

        let passed = oversized.is_empty();
        let compliant = total - oversized.len();
        let score = compliant as f64 / total as f64;

        VerificationCheck {
            name: "output_size".to_string(),
            passed,
            detail: if passed {
                format!("全部 {} 个输出均未超过大小上限", total)
            } else {
                format!("{}/{} 合规，超限: {}", compliant, total, oversized.join("; "))
            },
            score,
        }
    }

    // ── 加权评分（遵循 training_env.rs 的 RewardComputation 多维评分模式）──
    fn compute_weighted_score(checks: &[VerificationCheck]) -> f64 {
        if checks.is_empty() {
            return 1.0;
        }

        let weights: std::collections::HashMap<&str, f64> = [
            ("output_completeness", 0.25),
            ("schema_validation", 0.25),
            ("cross_task_consistency", 0.15),
            ("timeout_compliance", 0.15),
            ("error_rate", 0.10),
            ("output_size", 0.10),
        ]
        .into_iter()
        .collect();

        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        for check in checks {
            let weight = weights.get(check.name.as_str()).copied().unwrap_or(0.1);
            weighted_sum += check.score * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }
}

pub struct ParallelExecutionService {
    executions: Arc<RwLock<HashMap<String, ParallelExecution>>>,
    max_executions: usize,
}

impl Default for ParallelExecutionService {
    fn default() -> Self {
        Self::new(10)
    }
}

impl ParallelExecutionService {
    pub fn new(max_executions: usize) -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            max_executions,
        }
    }

    pub async fn create_execution(
        &self,
        name: String,
        description: String,
        tasks: Vec<(String, String, String)>,
        strategy: ExecutionStrategy,
        max_parallel: usize,
    ) -> Result<String> {
        let mut execution = ParallelExecution::new(name, description, strategy, max_parallel);

        for (task_name, task_desc, task_prompt) in tasks {
            let task = ParallelTask::new(task_name, task_desc, task_prompt);
            execution.add_task(task);
        }

        let exec_id = execution.id.clone();

        let mut executions = self.executions.write().unwrap();
        if executions.len() >= self.max_executions {
            if let Some(oldest) = executions.keys().next().cloned() {
                executions.remove(&oldest);
            }
        }
        executions.insert(exec_id.clone(), execution);

        Ok(exec_id)
    }

    pub async fn get_execution(&self, id: &str) -> Option<ParallelExecution> {
        let executions = self.executions.read().unwrap();
        executions.get(id).cloned()
    }

    pub async fn list_executions(&self) -> Vec<ParallelExecution> {
        let executions = self.executions.read().unwrap();
        executions.values().cloned().collect()
    }

    pub async fn get_next_pending_task(&self, execution_id: &str) -> Option<ParallelTask> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        let strategy = execution.strategy;
        let max_parallel = execution.max_parallel;
        let running = execution.running_count();

        if running >= max_parallel {
            return None;
        }

        match strategy {
            ExecutionStrategy::Sequential => execution
                .tasks
                .iter_mut()
                .find(|t| t.status == TaskStatus::Pending)
                .map(|t| {
                    t.start(Uuid::new_v4().to_string());
                    t.clone()
                }),
            ExecutionStrategy::Parallel => execution
                .tasks
                .iter_mut()
                .find(|t| t.status == TaskStatus::Pending)
                .map(|t| {
                    t.start(Uuid::new_v4().to_string());
                    t.clone()
                }),
            ExecutionStrategy::PriorityBased => execution
                .tasks
                .iter_mut()
                .find(|t| t.status == TaskStatus::Pending)
                .map(|t| {
                    t.start(Uuid::new_v4().to_string());
                    t.clone()
                }),
        }
    }

    pub async fn update_task_result(
        &self,
        execution_id: &str,
        task_id: &str,
        result: String,
    ) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        let task = execution.tasks.iter_mut().find(|t| t.id == task_id)?;
        task.complete(result);

        if execution.is_complete() {
            execution.status = if execution.failed_count() == 0 {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Failed
            };
            execution.completed_at = Some(Utc::now());
            execution.aggregate_results();
        }

        Some(())
    }

    pub async fn update_task_error(
        &self,
        execution_id: &str,
        task_id: &str,
        error: String,
    ) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        let task = execution.tasks.iter_mut().find(|t| t.id == task_id)?;
        task.fail(error);

        if execution.is_complete() {
            execution.status = ExecutionStatus::Failed;
            execution.completed_at = Some(Utc::now());
            execution.aggregate_results();
        }

        Some(())
    }

    pub async fn cancel_execution(&self, execution_id: &str) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        for task in &mut execution.tasks {
            if task.status == TaskStatus::Pending || task.status == TaskStatus::Running {
                task.status = TaskStatus::Cancelled;
                task.completed_at = Some(Utc::now());
            }
        }

        execution.status = ExecutionStatus::Cancelled;
        execution.completed_at = Some(Utc::now());
        execution.aggregate_results();

        Some(())
    }

    pub async fn get_execution_result(&self, execution_id: &str) -> Option<ExecutionResult> {
        let executions = self.executions.read().unwrap();
        let execution = executions.get(execution_id)?;

        Some(ExecutionResult {
            execution_id: execution.id.clone(),
            status: execution.status,
            total_tasks: execution.tasks.len(),
            completed: execution.completed_count(),
            failed: execution.failed_count(),
            duration_ms: execution.duration_ms().unwrap_or(0),
            aggregated_summary: execution.aggregated_result.clone().unwrap_or_default(),
            task_results: execution
                .tasks
                .iter()
                .map(TaskResultSummary::from)
                .collect(),
        })
    }

    pub async fn delete_execution(&self, execution_id: &str) -> bool {
        let mut executions = self.executions.write().unwrap();
        executions.remove(execution_id).is_some()
    }

    /// 扫描指定执行中所有运行中的任务，将超时的标记为 Timeout
    /// 返回被标记为超时的任务 ID 列表
    pub async fn check_and_apply_timeouts(&self, execution_id: &str) -> Vec<String> {
        let mut executions = self.executions.write().unwrap();
        let execution = match executions.get_mut(execution_id) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let now = Utc::now();
        let mut timed_out = Vec::new();

        for task in &mut execution.tasks {
            if task.status != TaskStatus::Running {
                continue;
            }
            if let (Some(started), Some(limit_secs)) = (task.started_at, task.timeout_secs) {
                let elapsed = (now - started).num_seconds();
                if elapsed >= limit_secs as i64 {
                    task.mark_timeout();
                    timed_out.push(task.id.clone());
                }
            }
        }

        // 如果所有任务已结束，更新执行状态
        if execution.is_complete() {
            let has_failure = execution.failed_count() > 0 || !timed_out.is_empty();
            execution.status = if has_failure {
                ExecutionStatus::Failed
            } else {
                ExecutionStatus::Completed
            };
            execution.completed_at = Some(now);
            execution.aggregate_results();
        }

        timed_out
    }

    pub async fn start_execution(&self, execution_id: &str) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;
        execution.start();
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_completed_task(
        name: &str,
        result: Option<&str>,
        schema: Option<&str>,
        timeout_secs: Option<u64>,
        duration_ms: u64,
    ) -> ParallelTask {
        let now = Utc::now();
        let started = now - chrono::Duration::milliseconds(duration_ms as i64);
        let mut task = ParallelTask::new(
            name.to_string(),
            format!("desc {}", name),
            format!("prompt {}", name),
        );
        task.status = TaskStatus::Completed;
        task.started_at = Some(started);
        task.completed_at = Some(now);
        task.progress = 1.0;
        if let Some(r) = result {
            task.result = Some(r.to_string());
        }
        task.expected_output_schema = schema.map(|s| s.to_string());
        task.timeout_secs = timeout_secs;
        task
    }

    fn make_failed_task(name: &str, error: &str) -> ParallelTask {
        let now = Utc::now();
        let started = now - chrono::Duration::milliseconds(500);
        let mut task = ParallelTask::new(
            name.to_string(),
            format!("desc {}", name),
            format!("prompt {}", name),
        );
        task.status = TaskStatus::Failed;
        task.error = Some(error.to_string());
        task.started_at = Some(started);
        task.completed_at = Some(now);
        task
    }

    fn make_running_task(name: &str, started_secs_ago: i64, timeout_secs: u64) -> ParallelTask {
        let now = Utc::now();
        let started = now - chrono::Duration::seconds(started_secs_ago);
        let mut task = ParallelTask::new(
            name.to_string(),
            format!("desc {}", name),
            format!("prompt {}", name),
        );
        task.status = TaskStatus::Running;
        task.started_at = Some(started);
        task.timeout_secs = Some(timeout_secs);
        task
    }

    fn make_execution(tasks: Vec<ParallelTask>) -> ParallelExecution {
        let mut exec = ParallelExecution::new(
            "test exec".to_string(),
            "for testing".to_string(),
            ExecutionStrategy::Parallel,
            3,
        );
        exec.tasks = tasks;
        exec
    }

    #[test]
    fn test_verification_config_defaults() {
        let config = VerificationConfig::default();
        assert!(config.enable_completeness);
        assert!(config.enable_schema_validation);
        assert!(config.enable_cross_validation);
        assert!(config.enable_timeout_check);
        assert!(config.enable_error_rate);
        assert!(config.enable_output_size);
        assert_eq!(config.max_result_size_bytes, 1_048_576);
        assert!((config.max_error_rate - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_output_completeness_all_have_results() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![
            make_completed_task("t1", Some("r1"), None, None, 100),
            make_completed_task("t2", Some("r2"), None, None, 200),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "output_completeness")
            .unwrap();
        assert!(check.passed);
        assert!((check.score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_output_completeness_missing_results() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![
            make_completed_task("t1", Some("r1"), None, None, 100),
            make_completed_task("t2", None, None, None, 200),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "output_completeness")
            .unwrap();
        assert!(!check.passed);
        assert!((check.score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_output_completeness_ignores_cancelled() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let mut cancelled = make_completed_task("c1", None, None, None, 100);
        cancelled.status = TaskStatus::Cancelled;
        let tasks = vec![
            make_completed_task("t1", Some("result"), None, None, 100),
            cancelled,
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "output_completeness")
            .unwrap();
        assert!(check.passed, "cancelled tasks should be ignored");
    }

    #[test]
    fn test_schema_validation_valid() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let schema =
            r#"{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}"#;
        let output = r#"{"name":"test","value":42}"#;
        let tasks = vec![make_completed_task(
            "t1",
            Some(output),
            Some(schema),
            None,
            100,
        )];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "schema_validation")
            .unwrap();
        assert!(check.passed);
    }

    #[test]
    fn test_schema_validation_missing_required() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let schema =
            r#"{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}"#;
        let output = r#"{"value":42}"#;
        let tasks = vec![make_completed_task(
            "t1",
            Some(output),
            Some(schema),
            None,
            100,
        )];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "schema_validation")
            .unwrap();
        assert!(!check.passed);
    }

    #[test]
    fn test_schema_validation_no_schemas() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![make_completed_task("t1", Some("data"), None, None, 100)];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "schema_validation")
            .unwrap();
        assert!(check.passed, "no schema should pass");
    }

    #[test]
    fn test_cross_task_consistency_no_conflicts() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![
            make_completed_task("t1", Some(r#"{"a":1,"b":"x"}"#), None, None, 100),
            make_completed_task("t2", Some(r#"{"c":2,"d":"y"}"#), None, None, 100),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "cross_task_consistency")
            .unwrap();
        assert!(check.passed);
    }

    #[test]
    fn test_cross_task_consistency_detects_conflict() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![
            make_completed_task("t1", Some(r#"{"a":1}"#), None, None, 100),
            make_completed_task("t2", Some(r#"{"a":2}"#), None, None, 100),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "cross_task_consistency")
            .unwrap();
        assert!(!check.passed);
    }

    #[test]
    fn test_cross_task_consistency_insufficient() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![make_completed_task(
            "t1",
            Some(r#"{"a":1}"#),
            None,
            None,
            100,
        )];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "cross_task_consistency")
            .unwrap();
        assert!(check.passed, "< 2 tasks should pass");
    }

    #[test]
    fn test_timeout_compliance_all_within() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![
            make_completed_task("t1", Some("r1"), None, Some(5), 1000),
            make_completed_task("t2", Some("r2"), None, Some(5), 2000),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "timeout_compliance")
            .unwrap();
        assert!(check.passed);
    }

    #[test]
    fn test_timeout_compliance_violation() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![make_completed_task("t1", Some("r1"), None, Some(1), 5000)];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "timeout_compliance")
            .unwrap();
        assert!(!check.passed);
    }

    #[test]
    fn test_error_rate_within_threshold() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![
            make_completed_task("t1", Some("r1"), None, None, 100),
            make_completed_task("t2", Some("r2"), None, None, 100),
            make_completed_task("t3", Some("r3"), None, None, 100),
            make_failed_task("t4", "err"),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "error_rate")
            .unwrap();
        assert!(check.passed); // 25% < 30%
    }

    #[test]
    fn test_error_rate_exceeds_threshold() {
        let mut config = VerificationConfig::default();
        config.max_error_rate = 0.2;
        let verifier = ParallelExecutionVerifier::new(config);
        let tasks = vec![
            make_completed_task("t1", Some("r1"), None, None, 100),
            make_failed_task("t2", "err"),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "error_rate")
            .unwrap();
        assert!(!check.passed); // 50% > 20%
    }

    #[test]
    fn test_output_size_within_limit() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![make_completed_task("t1", Some("small"), None, None, 100)];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "output_size")
            .unwrap();
        assert!(check.passed);
    }

    #[test]
    fn test_output_size_exceeds_limit() {
        let mut config = VerificationConfig::default();
        config.max_result_size_bytes = 5;
        let verifier = ParallelExecutionVerifier::new(config);
        let tasks = vec![make_completed_task("t1", Some("too long"), None, None, 100)];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "output_size")
            .unwrap();
        assert!(!check.passed);
    }

    #[test]
    fn test_overall_validation_all_pass() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let tasks = vec![
            make_completed_task("t1", Some(r#"{"a":1}"#), None, None, 100),
            make_completed_task("t2", Some(r#"{"b":2}"#), None, None, 200),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        assert!(result.is_valid);
        assert!(result.overall_score > 0.8);
    }

    #[test]
    fn test_overall_validation_some_fail() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let schema = r#"{"type":"object","required":["name"]}"#;
        let tasks = vec![
            make_completed_task("t1", Some(r#"{"no_name":1}"#), Some(schema), None, 100),
            make_completed_task("t2", None, None, None, 200),
        ];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        assert!(!result.is_valid);
        assert!(result.overall_score < 1.0);
    }

    #[test]
    fn test_disable_all_checks() {
        let config = VerificationConfig {
            enable_completeness: false,
            enable_schema_validation: false,
            enable_cross_validation: false,
            enable_timeout_check: false,
            enable_error_rate: false,
            enable_output_size: false,
            ..Default::default()
        };
        let verifier = ParallelExecutionVerifier::new(config);
        let tasks = vec![make_completed_task("t1", None, None, None, 100)];
        let exec = make_execution(tasks);
        let result = verifier.verify(&exec);
        assert!(result.checks.is_empty());
        assert!(result.is_valid);
    }

    #[test]
    fn test_verify_empty_execution() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let exec = make_execution(Vec::new());
        let result = verifier.verify(&exec);
        assert!(result.is_valid);
    }

    #[test]
    fn test_mark_timeout() {
        let mut task = make_running_task("t1", 60, 30);
        task.mark_timeout();
        assert_eq!(task.status, TaskStatus::Timeout);
        assert!(task.error.is_some());
    }

    #[tokio::test]
    async fn test_service_timeout_detection() {
        let service = ParallelExecutionService::new(10);
        let tasks = vec![("task1".to_string(), "desc".to_string(), "prompt".to_string())];
        let exec_id = service
            .create_execution(
                "test".to_string(),
                "desc".to_string(),
                tasks,
                ExecutionStrategy::Parallel,
                3,
            )
            .await
            .unwrap();

        {
            let mut executions = service.executions.write().unwrap();
            let exec = executions.get_mut(&exec_id).unwrap();
            let task = &mut exec.tasks[0];
            task.status = TaskStatus::Running;
            task.started_at = Some(Utc::now() - chrono::Duration::seconds(60));
            task.timeout_secs = Some(30);
        }

        let timed_out = service.check_and_apply_timeouts(&exec_id).await;
        assert_eq!(timed_out.len(), 1);

        let exec = service.get_execution(&exec_id).await.unwrap();
        assert_eq!(exec.tasks[0].status, TaskStatus::Timeout);
    }

    #[tokio::test]
    async fn test_service_timeout_no_violation() {
        let service = ParallelExecutionService::new(10);
        let tasks = vec![("task1".to_string(), "desc".to_string(), "prompt".to_string())];
        let exec_id = service
            .create_execution(
                "test".to_string(),
                "desc".to_string(),
                tasks,
                ExecutionStrategy::Parallel,
                3,
            )
            .await
            .unwrap();

        {
            let mut executions = service.executions.write().unwrap();
            let exec = executions.get_mut(&exec_id).unwrap();
            let task = &mut exec.tasks[0];
            task.status = TaskStatus::Running;
            task.started_at = Some(Utc::now() - chrono::Duration::seconds(5));
            task.timeout_secs = Some(300);
        }

        let timed_out = service.check_and_apply_timeouts(&exec_id).await;
        assert!(timed_out.is_empty());
    }

    #[test]
    fn test_task_builders() {
        let task = ParallelTask::new("t1".to_string(), "desc".to_string(), "prompt".to_string())
            .with_timeout(60)
            .with_schema(r#"{"type":"object"}"#.to_string());
        assert_eq!(task.timeout_secs, Some(60));
        assert_eq!(task.expected_output_schema, Some(r#"{"type":"object"}"#.to_string()));
    }

    #[test]
    fn test_verifier_with_defaults() {
        let verifier = ParallelExecutionVerifier::with_defaults();
        let exec = make_execution(Vec::new());
        let result = verifier.verify(&exec);
        assert!(!result.execution_id.is_empty());
        assert!(result.checks.len() >= 1);
    }
}
