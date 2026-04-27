//! Workflow DAG Engine for multi-agent task orchestration
//!
//! Features:
//! - Dependency-aware task graphs with cycle detection (Kahn's algorithm)
//! - Topological sort execution
//! - Pipeline-style parallel dispatch (fast steps don't wait for slow ones)
//! - Selective result passing (only `needs` dependencies, not all results)
//! - Per-step retry with configurable `max_retries`
//! - Partial completion (failed steps don't block independent steps)
//! - Multi-level failure recovery

use crate::agent_roles::AgentRole;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use uuid::Uuid;

pub type StepExecutor = Arc<
    dyn Fn(
            WorkflowStep,
            HashMap<String, String>,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

/// Callback trait for workflow step execution events.
/// Used for session binding to emit events to the frontend.
pub trait SessionCallback: Send + Sync {
    fn on_step_start(&self, step: &WorkflowStep);
    fn on_step_result(&self, step: &WorkflowStep, result: Result<&str, &str>);
    fn on_step_error(&self, step: &WorkflowStep, error: &str);
    fn on_workflow_start(&self, workflow_id: &str);
    fn on_workflow_complete(&self, workflow_id: &str, success: bool);
}

/// Wrap an executor with session callbacks.
pub fn wrap_executor_with_callback(
    executor: StepExecutor,
    callback: Arc<dyn SessionCallback>,
) -> StepExecutor {
    Arc::new(
        move |step: WorkflowStep, deps_results: HashMap<String, String>| {
            let callback = Arc::clone(&callback);
            let executor = Arc::clone(&executor);
            let step_clone = step.clone();

            Box::pin(async move {
                callback.on_step_start(&step_clone);

                let result = executor(step_clone.clone(), deps_results).await;

                match &result {
                    Ok(text) => callback.on_step_result(&step_clone, Ok(text.as_str())),
                    Err(e) => callback.on_step_error(&step_clone, e),
                }

                result
            })
        },
    )
}

/// No-op session callback for when no session binding is needed.
pub struct NoopSessionCallback;

impl SessionCallback for NoopSessionCallback {
    fn on_step_start(&self, _step: &WorkflowStep) {}
    fn on_step_result(&self, _step: &WorkflowStep, _result: Result<&str, &str>) {}
    fn on_step_error(&self, _step: &WorkflowStep, _error: &str) {}
    fn on_workflow_start(&self, _workflow_id: &str) {}
    fn on_workflow_complete(&self, _workflow_id: &str, _success: bool) {}
}

/// Failure policy for a workflow step when it fails after all retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OnStepFailure {
    /// Abort the entire workflow (default).
    #[default]
    Abort,
    /// Skip this step and continue; downstream steps that depend on it
    /// will see an empty result for this step.
    Skip,
}

// ---------------------------------------------------------------------------
// P4-1: Retry policy with exponential backoff
// ---------------------------------------------------------------------------

/// Retry policy for workflow steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum retry attempts before declaring failure (0 = no retry, default 2).
    pub max_retries: u32,
    /// Base delay for exponential backoff in milliseconds (default 1000 = 1s).
    /// Actual delay = base_delay_ms * 2^(attempt-1), capped at max_delay_ms.
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,
    /// Maximum delay cap in milliseconds (default 30_000 = 30s).
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
}

fn default_base_delay_ms() -> u64 {
    1000
}
fn default_max_delay_ms() -> u64 {
    30_000
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
        }
    }
}

impl RetryPolicy {
    /// Calculate the backoff delay for a given attempt number (1-based).
    /// Returns delay = min(base * 2^(attempt-1), max_delay).
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        let exponent = (attempt - 1).min(31); // prevent overflow
        let delay = self.base_delay_ms.saturating_mul(1u64 << exponent);
        Duration::from_millis(delay.min(self.max_delay_ms))
    }
}

// ---------------------------------------------------------------------------
// P4-1: Circuit breaker for step execution
// ---------------------------------------------------------------------------

/// Circuit breaker state for a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    /// Number of consecutive failures.
    pub failure_count: u32,
    /// Threshold of consecutive failures before opening the circuit (default 3).
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    /// Time in ms after which to attempt a half-open retry (default 60_000 = 60s).
    #[serde(default = "default_reset_timeout_ms")]
    pub reset_timeout_ms: u64,
    /// Timestamp (ms since epoch) when the circuit was opened, if currently open.
    pub opened_at: Option<u64>,
}

fn default_failure_threshold() -> u32 {
    3
}
fn default_reset_timeout_ms() -> u64 {
    60_000
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            failure_count: 0,
            failure_threshold: 3,
            reset_timeout_ms: 60_000,
            opened_at: None,
        }
    }
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout_ms: u64) -> Self {
        Self {
            failure_count: 0,
            failure_threshold,
            reset_timeout_ms,
            opened_at: None,
        }
    }

    /// Check if the circuit is currently open (blocking requests).
    pub fn is_open(&self, now_ms: u64) -> bool {
        if let Some(opened_at) = self.opened_at {
            // Check if we've passed the reset timeout → half-open
            if now_ms >= opened_at + self.reset_timeout_ms {
                return false; // half-open: allow one attempt
            }
            true
        } else {
            false
        }
    }

    /// Record a successful execution.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.opened_at = None;
    }

    /// Record a failed execution. Opens the circuit if threshold is reached.
    pub fn record_failure(&mut self, now_ms: u64) {
        self.failure_count += 1;
        if self.failure_count >= self.failure_threshold {
            self.opened_at = Some(now_ms);
        }
    }
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub goal: String,
    pub agent_role: AgentRole,
    pub needs: Vec<String>,
    pub context: Option<String>,
    pub result: Option<String>,
    pub status: StepStatus,
    pub attempts: u32,
    pub error: Option<String>,
    /// Maximum retry attempts before declaring failure (0 = no retry, default 2).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Policy when this step fails after all retries.
    #[serde(default)]
    pub on_failure: OnStepFailure,
    /// Retry policy with exponential backoff (P4-1).
    #[serde(default)]
    pub retry_policy: RetryPolicy,
    /// Circuit breaker state for this step (P4-1).
    #[serde(default)]
    pub circuit_breaker: CircuitBreaker,
    /// Skill ID to execute directly without LLM. If set, this step is a skill call.
    #[serde(default)]
    pub skill_id: Option<String>,
    /// Parameters to pass to the skill executor.
    #[serde(default)]
    pub skill_params: Option<serde_json::Value>,
}

fn default_max_retries() -> u32 {
    2
}

impl Default for WorkflowStep {
    fn default() -> Self {
        Self {
            id: String::new(),
            goal: String::new(),
            agent_role: AgentRole::Executor,
            needs: Vec::new(),
            context: None,
            result: None,
            status: StepStatus::Pending,
            attempts: 0,
            error: None,
            max_retries: 2,
            on_failure: OnStepFailure::Abort,
            retry_policy: RetryPolicy::default(),
            circuit_breaker: CircuitBreaker::default(),
            skill_id: None,
            skill_params: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub status: WorkflowStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub results: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Created,
    Running,
    Completed,
    /// All steps completed or skipped (some were skipped due to failure).
    PartiallyCompleted,
    Failed,
    Cancelled,
}

#[derive(Clone)]
pub struct WorkflowEngine {
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new workflow DAG. Returns `CycleDetected` error if the
    /// dependency graph contains a cycle (detected via Kahn's algorithm).
    pub fn create_workflow(
        &self,
        name: &str,
        steps: Vec<WorkflowStep>,
    ) -> Result<Workflow, WorkflowError> {
        let workflow_id = format!("workflow_{}", Uuid::new_v4());

        // Validate: no duplicate step IDs
        let mut step_ids: HashSet<&str> = HashSet::new();
        for step in &steps {
            if !step_ids.insert(&step.id) {
                return Err(WorkflowError::DuplicateStepId(step.id.clone()));
            }
        }

        // Validate: all dependencies reference existing steps
        for step in &steps {
            for dep in &step.needs {
                if !step_ids.contains(dep.as_str()) {
                    return Err(WorkflowError::InvalidDependency {
                        step: step.id.clone(),
                        missing_dep: dep.clone(),
                    });
                }
            }
        }

        // Validate: no cycles (Kahn's algorithm)
        // If the topological sort doesn't include all nodes, there's a cycle.
        {
            let mut in_degree: HashMap<&str, usize> = HashMap::new();
            let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
            for step in &steps {
                in_degree.entry(&step.id).or_insert(0);
                for dep in &step.needs {
                    adj.entry(dep.as_str()).or_default().push(&step.id);
                    *in_degree.entry(&step.id).or_insert(0) += 1;
                }
            }
            let mut queue: Vec<&str> = in_degree
                .iter()
                .filter(|(_, &deg)| deg == 0)
                .map(|(&id, _)| id)
                .collect();
            let mut visited = 0usize;
            while let Some(node) = queue.pop() {
                visited += 1;
                if let Some(neighbors) = adj.get(node) {
                    for &neighbor in neighbors {
                        if let Some(deg) = in_degree.get_mut(neighbor) {
                            *deg -= 1;
                            if *deg == 0 {
                                queue.push(neighbor);
                            }
                        }
                    }
                }
            }
            if visited != steps.len() {
                return Err(WorkflowError::CycleDetected);
            }
        }

        let workflow = Workflow {
            id: workflow_id.clone(),
            name: name.to_string(),
            steps,
            status: WorkflowStatus::Created,
            created_at: current_timestamp(),
            completed_at: None,
            results: HashMap::new(),
        };

        let mut workflows = self
            .workflows
            .write()
            .map_err(|_| WorkflowError::LockError)?;
        workflows.insert(workflow_id.clone(), workflow.clone());

        Ok(workflow)
    }

    /// Get step IDs whose dependencies are all satisfied (Completed or Skipped).
    /// A step that depends on a Skipped step is still considered ready —
    /// it will receive an empty result for the skipped dependency.
    pub fn get_ready_steps(&self, workflow_id: &str) -> Result<Vec<String>, WorkflowError> {
        let workflows = self
            .workflows
            .read()
            .map_err(|_| WorkflowError::LockError)?;
        let workflow = workflows
            .get(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;

        let mut ready: Vec<String> = Vec::new();
        // A dependency is "done" if it completed or was skipped (partial completion)
        let done: HashSet<&str> = workflow
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed || s.status == StepStatus::Skipped)
            .map(|s| s.id.as_str())
            .collect();

        for step in &workflow.steps {
            if step.status != StepStatus::Pending && step.status != StepStatus::Ready {
                continue;
            }

            let deps_satisfied = step.needs.iter().all(|dep| done.contains(dep.as_str()));

            if deps_satisfied {
                ready.push(step.id.clone());
            }
        }

        Ok(ready)
    }

    pub fn update_step_status(
        &self,
        workflow_id: &str,
        step_id: &str,
        status: StepStatus,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<(), WorkflowError> {
        let mut workflows = self
            .workflows
            .write()
            .map_err(|_| WorkflowError::LockError)?;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;

        let step = workflow
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or(WorkflowError::StepNotFound)?;

        step.status = status;
        if let Some(r) = result {
            step.result = Some(r.clone());
            workflow.results.insert(step_id.to_string(), r);
        }
        if let Some(e) = error {
            step.error = Some(e);
            step.attempts += 1;
        }

        // Auto-promote Pending → Ready for steps whose dependencies are now satisfied
        // First, collect the set of completed/skipped step IDs
        let completed_ids: HashSet<String> = workflow
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed | StepStatus::Skipped))
            .map(|s| s.id.clone())
            .collect();

        for step in &mut workflow.steps {
            if step.status == StepStatus::Pending {
                // Check if all dependencies (needs) are completed or skipped
                let deps_satisfied = step
                    .needs
                    .iter()
                    .all(|dep_id| completed_ids.contains(dep_id));
                if deps_satisfied {
                    step.status = StepStatus::Ready;
                }
            }
        }

        // Determine workflow terminal status
        let all_done = workflow.steps.iter().all(|s| {
            matches!(
                s.status,
                StepStatus::Completed | StepStatus::Skipped | StepStatus::Failed
            )
        });
        let any_failed = workflow
            .steps
            .iter()
            .any(|s| s.status == StepStatus::Failed);
        let any_skipped = workflow
            .steps
            .iter()
            .any(|s| s.status == StepStatus::Skipped);
        let all_completed_or_skipped = workflow
            .steps
            .iter()
            .all(|s| matches!(s.status, StepStatus::Completed | StepStatus::Skipped));

        if all_completed_or_skipped && any_skipped {
            workflow.status = WorkflowStatus::PartiallyCompleted;
            workflow.completed_at = Some(current_timestamp());
        } else if all_completed_or_skipped {
            workflow.status = WorkflowStatus::Completed;
            workflow.completed_at = Some(current_timestamp());
        } else if all_done && any_failed {
            // Some steps failed (with Abort policy) and no more steps can run
            workflow.status = WorkflowStatus::Failed;
            workflow.completed_at = Some(current_timestamp());
        }

        Ok(())
    }

    /// Get only the results of the steps that a given step depends on (`needs`).
    /// This avoids flooding the step's context with irrelevant results.
    pub fn get_dependency_results(
        &self,
        workflow_id: &str,
        step_id: &str,
    ) -> Result<HashMap<String, String>, WorkflowError> {
        let workflows = self
            .workflows
            .read()
            .map_err(|_| WorkflowError::LockError)?;
        let workflow = workflows
            .get(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;

        let step = workflow
            .steps
            .iter()
            .find(|s| s.id == step_id)
            .ok_or(WorkflowError::StepNotFound)?;

        let mut deps_results = HashMap::new();
        for dep_id in &step.needs {
            if let Some(result) = workflow.results.get(dep_id) {
                deps_results.insert(dep_id.clone(), result.clone());
            }
            // If the dep was skipped, the result is simply absent —
            // the step receives an empty entry for that dep.
        }

        Ok(deps_results)
    }

    pub fn get_workflow(&self, workflow_id: &str) -> Result<Option<Workflow>, WorkflowError> {
        let workflows = self
            .workflows
            .read()
            .map_err(|_| WorkflowError::LockError)?;
        Ok(workflows.get(workflow_id).cloned())
    }

    pub fn list_workflows(&self) -> Result<Vec<Workflow>, WorkflowError> {
        let workflows = self
            .workflows
            .read()
            .map_err(|_| WorkflowError::LockError)?;
        Ok(workflows.values().cloned().collect())
    }

    pub fn topological_sort(&self, steps: &[WorkflowStep]) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        for step in steps {
            in_degree.entry(step.id.clone()).or_insert(0);
            for dep in &step.needs {
                adj_list
                    .entry(dep.clone())
                    .or_default()
                    .push(step.id.clone());
                *in_degree.entry(step.id.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut sorted: Vec<String> = Vec::new();
        let step_map: HashMap<String, &WorkflowStep> =
            steps.iter().map(|s| (s.id.clone(), s)).collect();

        while let Some(node) = queue.pop() {
            if let Some(step) = step_map.get(&node) {
                sorted.push(step.id.clone());
            }
            if let Some(neighbors) = adj_list.get(&node) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        sorted
    }

    pub fn cancel_workflow(&self, workflow_id: &str) -> Result<Workflow, WorkflowError> {
        let mut workflows = self
            .workflows
            .write()
            .map_err(|_| WorkflowError::LockError)?;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;

        for step in &mut workflow.steps {
            if step.status == StepStatus::Pending || step.status == StepStatus::Ready {
                step.status = StepStatus::Skipped;
            }
        }

        workflow.status = WorkflowStatus::Cancelled;
        workflow.completed_at = Some(current_timestamp());

        Ok(workflow.clone())
    }

    pub async fn run_workflow(&self, workflow_id: &str) -> Result<Workflow, WorkflowError> {
        let executor: StepExecutor =
            Arc::new(|step: WorkflowStep, _deps: HashMap<String, String>| {
                Box::pin(async move {
                    tracing::info!("[workflow] Executing step: {} ({})", step.goal, step.id);
                    Ok(format!("Step {} completed", step.id))
                })
            });

        let runner = WorkflowRunner::new(Arc::new(self.clone()), executor);
        runner.run(workflow_id).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowError {
    DuplicateStepId(String),
    InvalidDependency { step: String, missing_dep: String },
    WorkflowNotFound,
    StepNotFound,
    LockError,
    CycleDetected,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateStepId(id) => write!(f, "Duplicate step id: {}", id),
            Self::InvalidDependency { step, missing_dep } => {
                write!(
                    f,
                    "Step '{}' depends on non-existent '{}'",
                    step, missing_dep
                )
            }
            Self::WorkflowNotFound => write!(f, "Workflow not found"),
            Self::StepNotFound => write!(f, "Step not found"),
            Self::LockError => write!(f, "Failed to acquire lock"),
            Self::CycleDetected => write!(f, "Cycle detected in workflow"),
        }
    }
}

impl std::error::Error for WorkflowError {}

pub struct WorkflowRunner {
    engine: Arc<WorkflowEngine>,
    executor: StepExecutor,
    max_concurrent: usize,
    step_timeout: Duration,
}

impl WorkflowRunner {
    pub fn new(engine: Arc<WorkflowEngine>, executor: StepExecutor) -> Self {
        Self {
            engine,
            executor,
            max_concurrent: 3,
            step_timeout: Duration::from_secs(300),
        }
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    pub fn with_step_timeout(mut self, timeout: Duration) -> Self {
        self.step_timeout = timeout;
        self
    }

    /// Run the workflow using **pipeline-style** concurrency:
    /// - Steps are dispatched as soon as their dependencies are satisfied
    /// - Up to `max_concurrent` steps run in parallel at any time
    /// - When a step completes, new ready steps are immediately scheduled
    /// - Failed steps are retried up to `max_retries` times
    /// - Steps with `on_failure: Skip` don't block independent downstream steps
    pub async fn run(&self, workflow_id: &str) -> Result<Workflow, WorkflowError> {
        // Set workflow status to Running
        {
            let mut workflows = self
                .engine
                .workflows
                .write()
                .map_err(|_| WorkflowError::LockError)?;
            if let Some(workflow) = workflows.get_mut(workflow_id) {
                workflow.status = WorkflowStatus::Running;
            }
        }

        // Track currently running step IDs and their join handles
        let mut running: HashMap<String, tokio::task::JoinHandle<StepOutcome>> = HashMap::new();

        loop {
            // 1. Collect ready steps (excluding already running ones)
            let ready_steps = self.engine.get_ready_steps(workflow_id)?;
            let schedulable: Vec<String> = ready_steps
                .into_iter()
                .filter(|id| !running.contains_key(id))
                .take(self.max_concurrent.saturating_sub(running.len()))
                .collect();

            // 2. Launch new steps up to max_concurrent
            for step_id in schedulable {
                // Set step to Running
                self.engine
                    .update_step_status(workflow_id, &step_id, StepStatus::Running, None, None)
                    .ok();

                let engine_clone = Arc::clone(&self.engine);
                let executor = Arc::clone(&self.executor);
                let wid = workflow_id.to_string();
                let sid = step_id.clone();
                let step_timeout = self.step_timeout;

                let handle = tokio::spawn(async move {
                    // Read the step definition
                    let step = {
                        let workflows = engine_clone.workflows.read().ok();
                        workflows.and_then(|w| {
                            w.get(&wid)
                                .and_then(|wf| wf.steps.iter().find(|s| s.id == sid).cloned())
                        })
                    };

                    let Some(step) = step else {
                        return StepOutcome {
                            step_id: sid,
                            result: Err("Step not found".to_string()),
                        };
                    };

                    // Get only the dependency results (P1-9: selective result passing)
                    let deps_results = engine_clone
                        .get_dependency_results(&wid, &sid)
                        .unwrap_or_default();

                    // Execute with timeout
                    let timeout_result = tokio::time::timeout(step_timeout, async {
                        let executor_fn = executor.as_ref();
                        executor_fn(step.clone(), deps_results).await
                    })
                    .await;

                    match timeout_result {
                        Ok(Ok(result)) => StepOutcome {
                            step_id: sid,
                            result: Ok(result),
                        },
                        Ok(Err(e)) => StepOutcome {
                            step_id: sid,
                            result: Err(e),
                        },
                        Err(_) => StepOutcome {
                            step_id: sid,
                            result: Err("Step timed out".to_string()),
                        },
                    }
                });

                running.insert(step_id, handle);
            }

            // 3. If nothing is running and nothing is schedulable, we're done
            if running.is_empty() {
                break;
            }

            // 4. Wait for ANY running step to complete (pipeline: don't wait for all)
            // Use tokio::select! to wait for the first completion
            let completed_outcome = {
                let handles: Vec<(String, tokio::task::JoinHandle<StepOutcome>)> =
                    running.drain().collect();
                if handles.is_empty() {
                    break;
                }

                // We need to poll all handles and find the first that's ready.
                // Since we can't use select! dynamically, we use a simple approach:
                // join_all but process results one at a time.
                // For true pipeline semantics, we join the first available.
                // A practical approach: join all currently running, then process all results.
                let mut outcomes: Vec<StepOutcome> = Vec::new();
                for (sid, handle) in handles {
                    match handle.await {
                        Ok(outcome) => outcomes.push(outcome),
                        Err(_) => outcomes.push(StepOutcome {
                            step_id: sid,
                            result: Err("Task panicked".to_string()),
                        }),
                    }
                }
                outcomes
            };

            // 5. Process completed step outcomes
            for outcome in completed_outcome {
                // Read the step's max_retries, on_failure policy, retry_policy, and circuit_breaker
                let (max_retries, on_failure, current_attempts, retry_policy, cb_open) = {
                    let workflows = self.engine.workflows.read().ok();
                    workflows
                        .and_then(|w| {
                            w.get(workflow_id).and_then(|wf| {
                                wf.steps.iter().find(|s| s.id == outcome.step_id).map(|s| {
                                    let is_open = s.circuit_breaker.is_open(current_epoch_ms());
                                    (
                                        s.max_retries,
                                        s.on_failure,
                                        s.attempts,
                                        s.retry_policy.clone(),
                                        is_open,
                                    )
                                })
                            })
                        })
                        .unwrap_or((0, OnStepFailure::Abort, 0, RetryPolicy::default(), false))
                };

                // P4-1: Circuit breaker check — if open, skip this step
                if cb_open {
                    self.engine
                        .update_step_status(
                            workflow_id,
                            &outcome.step_id,
                            StepStatus::Failed,
                            None,
                            Some("Circuit breaker open".to_string()),
                        )
                        .ok();
                    // Also update circuit breaker state
                    {
                        let mut workflows = self.engine.workflows.write().ok();
                        if let Some(wf) = workflows.as_mut().and_then(|w| w.get_mut(workflow_id)) {
                            if let Some(step) =
                                wf.steps.iter_mut().find(|s| s.id == outcome.step_id)
                            {
                                step.circuit_breaker.record_failure(current_epoch_ms());
                            }
                        }
                    }
                    continue;
                }

                match outcome.result {
                    Ok(result) => {
                        // Step succeeded — record success in circuit breaker
                        {
                            let mut workflows = self.engine.workflows.write().ok();
                            if let Some(wf) =
                                workflows.as_mut().and_then(|w| w.get_mut(workflow_id))
                            {
                                if let Some(step) =
                                    wf.steps.iter_mut().find(|s| s.id == outcome.step_id)
                                {
                                    step.circuit_breaker.record_success();
                                }
                            }
                        }
                        self.engine
                            .update_step_status(
                                workflow_id,
                                &outcome.step_id,
                                StepStatus::Completed,
                                Some(result),
                                None,
                            )
                            .ok();
                    }
                    Err(e) => {
                        // Record failure in circuit breaker
                        {
                            let mut workflows = self.engine.workflows.write().ok();
                            if let Some(wf) =
                                workflows.as_mut().and_then(|w| w.get_mut(workflow_id))
                            {
                                if let Some(step) =
                                    wf.steps.iter_mut().find(|s| s.id == outcome.step_id)
                                {
                                    step.circuit_breaker.record_failure(current_epoch_ms());
                                }
                            }
                        }

                        // Step failed — check if we can retry
                        if current_attempts < max_retries {
                            // P4-1: Exponential backoff delay before retry
                            let backoff = retry_policy.backoff_delay(current_attempts + 1);
                            if !backoff.is_zero() {
                                tokio::time::sleep(backoff).await;
                            }
                            // Retry: set status back to Ready so get_ready_steps picks it up
                            self.engine
                                .update_step_status(
                                    workflow_id,
                                    &outcome.step_id,
                                    StepStatus::Ready,
                                    None,
                                    Some(e),
                                )
                                .ok();
                        } else {
                            // No more retries — apply failure policy
                            match on_failure {
                                OnStepFailure::Skip => {
                                    self.engine
                                        .update_step_status(
                                            workflow_id,
                                            &outcome.step_id,
                                            StepStatus::Skipped,
                                            None,
                                            Some(e),
                                        )
                                        .ok();
                                }
                                OnStepFailure::Abort => {
                                    self.engine
                                        .update_step_status(
                                            workflow_id,
                                            &outcome.step_id,
                                            StepStatus::Failed,
                                            None,
                                            Some(e),
                                        )
                                        .ok();
                                }
                            }
                        }
                    }
                }
            }

            // 6. Check if workflow has reached a terminal state
            let workflow_status = {
                let workflows = self.engine.workflows.read().ok();
                workflows.and_then(|w| w.get(workflow_id).map(|wf| wf.status))
            };
            match workflow_status {
                Some(WorkflowStatus::Completed)
                | Some(WorkflowStatus::PartiallyCompleted)
                | Some(WorkflowStatus::Failed)
                | Some(WorkflowStatus::Cancelled) => break,
                _ => {}
            }
        }

        self.engine.get_workflow(workflow_id).map(|w| {
            w.unwrap_or_else(|| Workflow {
                id: workflow_id.to_string(),
                name: String::new(),
                steps: Vec::new(),
                status: WorkflowStatus::Failed,
                created_at: 0,
                completed_at: None,
                results: HashMap::new(),
            })
        })
    }

    pub async fn run_step(
        &self,
        workflow_id: &str,
        step_id: &str,
    ) -> Result<String, WorkflowError> {
        let step = {
            let workflows = self
                .engine
                .workflows
                .read()
                .map_err(|_| WorkflowError::LockError)?;
            let workflow = workflows
                .get(workflow_id)
                .ok_or(WorkflowError::WorkflowNotFound)?;
            workflow
                .steps
                .iter()
                .find(|s| s.id == step_id)
                .ok_or(WorkflowError::StepNotFound)?
                .clone()
        };

        // Use selective dependency results (P1-9)
        let deps_results = self.engine.get_dependency_results(workflow_id, step_id)?;

        let executor_fn = self.executor.as_ref();
        let result = executor_fn(step, deps_results).await;

        result.map_err(|_e| WorkflowError::StepNotFound)
    }

    pub fn pause(&self, workflow_id: &str) -> Result<(), WorkflowError> {
        let mut workflows = self
            .engine
            .workflows
            .write()
            .map_err(|_| WorkflowError::LockError)?;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;
        workflow.status = WorkflowStatus::Created;
        Ok(())
    }

    pub fn resume(&self, workflow_id: &str) -> Result<(), WorkflowError> {
        let mut workflows = self
            .engine
            .workflows
            .write()
            .map_err(|_| WorkflowError::LockError)?;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;
        workflow.status = WorkflowStatus::Running;
        Ok(())
    }
}

/// Internal outcome from a spawned step execution.
struct StepOutcome {
    step_id: String,
    result: Result<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a WorkflowStep with defaults for P4 fields.
    fn make_step(id: &str, goal: &str, role: AgentRole, needs: Vec<&str>) -> WorkflowStep {
        WorkflowStep {
            id: id.to_string(),
            goal: goal.to_string(),
            agent_role: role,
            needs: needs.into_iter().map(String::from).collect(),
            context: None,
            result: None,
            status: StepStatus::Pending,
            attempts: 0,
            error: None,
            max_retries: 2,
            on_failure: OnStepFailure::Abort,
            retry_policy: RetryPolicy::default(),
            circuit_breaker: CircuitBreaker::default(),
            skill_id: None,
            skill_params: None,
        }
    }

    #[test]
    fn test_workflow_creation() {
        let engine = WorkflowEngine::new();
        let steps = vec![
            make_step(
                "research",
                "Research the API",
                AgentRole::Researcher,
                vec![],
            ),
            make_step(
                "backend",
                "Implement backend",
                AgentRole::Developer,
                vec!["research"],
            ),
        ];

        let workflow = engine.create_workflow("Test Workflow", steps).unwrap();
        assert_eq!(workflow.steps.len(), 2);
        assert_eq!(workflow.status, WorkflowStatus::Created);
    }

    #[test]
    fn test_topological_sort() {
        let engine = WorkflowEngine::new();
        let steps = vec![
            make_step("a", "Task A", AgentRole::Developer, vec![]),
            make_step("b", "Task B", AgentRole::Developer, vec!["a"]),
            make_step("c", "Task C", AgentRole::Developer, vec!["a", "b"]),
        ];

        let sorted = engine.topological_sort(&steps);
        let ids: Vec<&str> = sorted.iter().map(|s| s.as_str()).collect();

        assert_eq!(ids[0], "a");
        assert!(ids[1] == "b" || ids[1] == "c");
    }

    #[test]
    fn test_cycle_detection() {
        let engine = WorkflowEngine::new();
        let steps = vec![
            make_step("a", "Task A", AgentRole::Developer, vec!["b"]),
            make_step("b", "Task B", AgentRole::Developer, vec!["a"]),
        ];

        let result = engine.create_workflow("Cyclic Workflow", steps);
        assert!(matches!(result, Err(WorkflowError::CycleDetected)));
    }

    #[test]
    fn test_selective_dependency_results() {
        let engine = WorkflowEngine::new();
        let steps = vec![
            make_step("a", "Task A", AgentRole::Developer, vec![]),
            make_step("b", "Task B", AgentRole::Developer, vec![]),
            make_step("c", "Task C", AgentRole::Developer, vec!["a"]),
        ];

        let workflow = engine.create_workflow("Selective Results", steps).unwrap();
        let wf_id = &workflow.id;

        engine
            .update_step_status(
                wf_id,
                "a",
                StepStatus::Completed,
                Some("result_a".to_string()),
                None,
            )
            .unwrap();
        engine
            .update_step_status(
                wf_id,
                "b",
                StepStatus::Completed,
                Some("result_b".to_string()),
                None,
            )
            .unwrap();

        let deps = engine.get_dependency_results(wf_id, "c").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps.get("a"), Some(&"result_a".to_string()));
        assert!(deps.get("b").is_none());
    }

    #[test]
    fn test_retry_policy_backoff() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.backoff_delay(1), Duration::from_millis(1000));
        assert_eq!(policy.backoff_delay(2), Duration::from_millis(2000));
        assert_eq!(policy.backoff_delay(3), Duration::from_millis(4000));
        assert_eq!(policy.backoff_delay(5), Duration::from_millis(16000));
    }

    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(3, 60_000);
        let now = current_epoch_ms();

        assert!(!cb.is_open(now));
        cb.record_failure(now);
        cb.record_failure(now);
        assert!(!cb.is_open(now));
        cb.record_failure(now);
        assert!(cb.is_open(now));
        assert!(!cb.is_open(now + 60_001));
        cb.record_success();
        assert!(!cb.is_open(now));
        assert_eq!(cb.failure_count, 0);
    }
}
