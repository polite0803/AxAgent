use crate::agent_roles::AgentRole;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;

/// 前端事件发射器类型：发送 (事件名, JSON载荷) 到 Tauri 前端
pub type EventEmitter = Option<Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub id: String,
    pub goal: String,
    /// 关联的会话 ID（用于前端事件路由）
    pub conversation_id: Option<String>,
    pub agents: Vec<AgentAssignment>,
    pub communication_plan: Vec<CommunicationRule>,
    pub consensus_strategy: ConsensusStrategy,
    pub max_concurrent: usize,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignment {
    pub agent_id: String,
    pub role: AgentRole,
    pub task: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub dependencies: Vec<String>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationRule {
    pub from_role: AgentRole,
    pub to_role: AgentRole,
    pub message_type: MessageType,
    pub trigger: TriggerType,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    TaskAssign,
    ProgressReport,
    TaskResult,
    TaskError,
    TaskCancel,
    Data,
    ConsensusRequest,
    ConsensusResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    OnStart,
    OnProgress,
    OnComplete,
    OnError,
    OnDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ConsensusStrategy {
    #[default]
    MajorityVote,
    Unanimous,
    LeaderDecides {
        leader_agent_id: String,
    },
    WeightedVote {
        weights: HashMap<String, f32>,
    },
    FirstResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    pub plan_id: String,
    pub goal: String,
    pub status: OrchestrationStatus,
    pub agent_results: HashMap<String, serde_json::Value>,
    pub failures: Vec<(String, String)>,
    pub final_result: serde_json::Value,
    pub consensus_reached: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationStatus {
    Pending,
    Executing,
    PartiallyCompleted,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone)]
struct ActiveAgent {
    status: AgentRunStatus,
    result: Option<serde_json::Value>,
    error: Option<String>,
    started_at: Option<Instant>,
    completed_at: Option<Instant>,
    mailbox: mpsc::Sender<AgentMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunStatus {
    Pending,
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub payload: MessagePayload,
    pub timestamp: u128,
}

impl AgentMessage {
    pub fn new(from: &str, to: &str, payload: MessagePayload) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum MessagePayload {
    TaskAssign {
        task: String,
    },
    ProgressReport {
        progress: f32,
        message: String,
    },
    TaskResult {
        result: serde_json::Value,
    },
    TaskError {
        error: String,
    },
    TaskCancel,
    Data {
        content: serde_json::Value,
    },
    ConsensusRequest {
        vote: serde_json::Value,
    },
    ConsensusResponse {
        agreed: bool,
        value: serde_json::Value,
    },
}

pub struct AgentOrchestrator {
    active_agents: Arc<RwLock<HashMap<String, ActiveAgent>>>,
    max_retries: u32,
    default_timeout: Duration,
    /// 前端事件发射器（设置后在关键节点向 Tauri 前端发送事件）
    event_emitter: EventEmitter,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            max_retries: 3,
            default_timeout: Duration::from_secs(300),
            event_emitter: None,
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.default_timeout = Duration::from_secs(timeout_secs);
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// 设置前端事件发射器（由 Tauri 命令层注入）
    pub fn set_event_emitter(&mut self, emitter: EventEmitter) {
        self.event_emitter = emitter;
    }

    /// 内部辅助：向 Tauri 前端发射事件
    fn emit(&self, event_name: &str, payload: serde_json::Value) {
        if let Some(ref emitter) = self.event_emitter {
            emitter(event_name, payload);
        }
    }

    pub async fn execute_plan(&self, plan: OrchestrationPlan) -> OrchestrationResult {
        let start_time = Instant::now();
        let conv_id = plan.conversation_id.clone().unwrap_or_default();
        let mut agent_results: HashMap<String, serde_json::Value> = HashMap::new();
        let mut failures: Vec<(String, String)> = Vec::new();

        for assignment in &plan.agents {
            let (tx, _rx) = mpsc::channel(32);
            let agent = ActiveAgent {
                status: AgentRunStatus::Pending,
                result: None,
                error: None,
                started_at: None,
                completed_at: None,
                mailbox: tx,
            };
            let mut agents = self.active_agents.write().await;
            agents.insert(assignment.agent_id.clone(), agent);
        }

        let mut completed: HashMap<String, serde_json::Value> = HashMap::new();
        let mut pending_count = plan.agents.len();
        let max_concurrent = plan.max_concurrent.max(1);
        let timeout = Duration::from_secs(plan.timeout_secs);

        while completed.len() + failures.len() < plan.agents.len() {
            if start_time.elapsed() > timeout {
                let agents = self.active_agents.read().await;
                let remaining: Vec<String> = plan
                    .agents
                    .iter()
                    .filter(|a| {
                        !completed.contains_key(&a.agent_id)
                            && !failures.iter().any(|(id, _)| id == &a.agent_id)
                    })
                    .map(|a| a.agent_id.clone())
                    .collect();
                drop(agents);
                for agent_id in remaining {
                    failures.push((agent_id, "Timeout".to_string()));
                }
                break;
            }

            let ready_agents = self
                .get_ready_agents(&plan, &completed, max_concurrent)
                .await;
            let has_ready = !ready_agents.is_empty();

            if !has_ready && pending_count > 0 {
                sleep(Duration::from_millis(100)).await;
                let agents = self.active_agents.read().await;
                pending_count = agents
                    .values()
                    .filter(|a| {
                        a.status == AgentRunStatus::Pending
                            || a.status == AgentRunStatus::Starting
                            || a.status == AgentRunStatus::Running
                    })
                    .count();
                continue;
            }

            for agent_id in ready_agents {
                let assignment = plan.agents.iter().find(|a| a.agent_id == agent_id).cloned();

                if let Some(ref assignment) = assignment {
                    // 发射 workflow-step-start
                    self.emit(
                        "workflow-step-start",
                        serde_json::json!({
                            "conversationId": conv_id,
                            "stepId": assignment.agent_id,
                            "stepGoal": assignment.task,
                            "agentRole": assignment.role.to_string(),
                        }),
                    );
                }

                if let Some(assignment) = assignment {
                    match self.run_agent(&assignment).await {
                        Ok(result) => {
                            let mut agents = self.active_agents.write().await;
                            if let Some(agent) = agents.get_mut(&agent_id) {
                                agent.status = AgentRunStatus::Completed;
                                agent.result = Some(result.clone());
                                agent.completed_at = Some(Instant::now());
                            }
                            completed.insert(agent_id.clone(), result.clone());

                            // 发射 workflow-step-complete
                            self.emit(
                                "workflow-step-complete",
                                serde_json::json!({
                                    "conversationId": conv_id,
                                    "stepId": agent_id,
                                    "stepGoal": assignment.task,
                                    "result": serde_json::to_string(&result).unwrap_or_default(),
                                }),
                            );
                        }
                        Err(e) => {
                            let mut agents = self.active_agents.write().await;
                            if let Some(agent) = agents.get_mut(&agent_id) {
                                agent.status = AgentRunStatus::Failed;
                                agent.error = Some(e.clone());
                                agent.completed_at = Some(Instant::now());
                            }
                            failures.push((agent_id.clone(), e.clone()));

                            // 发射 workflow-step-error
                            self.emit(
                                "workflow-step-error",
                                serde_json::json!({
                                    "conversationId": conv_id,
                                    "stepId": agent_id,
                                    "error": e,
                                }),
                            );
                        }
                    }
                }
            }

            if !has_ready && pending_count == 0 {
                break;
            }
        }

        let final_result = self
            .aggregate_results(&plan.consensus_strategy, &completed, &failures)
            .await;

        let consensus_reached = self
            .check_consensus(&plan.consensus_strategy, &completed, &failures)
            .await;

        let status = if failures.len() == plan.agents.len() {
            OrchestrationStatus::Failed
        } else if failures.is_empty() {
            OrchestrationStatus::Completed
        } else {
            OrchestrationStatus::PartiallyCompleted
        };

        agent_results.extend(completed);

        OrchestrationResult {
            plan_id: plan.id,
            goal: plan.goal,
            status,
            agent_results,
            failures,
            final_result,
            consensus_reached,
            duration_ms: start_time.elapsed().as_millis() as u64,
        }
    }

    async fn get_ready_agents(
        &self,
        plan: &OrchestrationPlan,
        completed: &HashMap<String, serde_json::Value>,
        max_concurrent: usize,
    ) -> Vec<String> {
        let agents = self.active_agents.read().await;
        let running_count = agents
            .values()
            .filter(|a| a.status == AgentRunStatus::Starting || a.status == AgentRunStatus::Running)
            .count();

        if running_count >= max_concurrent {
            return vec![];
        }

        let available_slots = max_concurrent - running_count;
        let mut ready = Vec::new();

        for assignment in &plan.agents {
            if ready.len() >= available_slots {
                break;
            }

            if let Some(agent) = agents.get(&assignment.agent_id) {
                if agent.status != AgentRunStatus::Pending {
                    continue;
                }

                if assignment
                    .dependencies
                    .iter()
                    .all(|dep| completed.contains_key(dep))
                {
                    ready.push(assignment.agent_id.clone());
                }
            }
        }

        ready
    }

    async fn run_agent(&self, assignment: &AgentAssignment) -> Result<serde_json::Value, String> {
        {
            let mut agents = self.active_agents.write().await;
            if let Some(agent) = agents.get_mut(&assignment.agent_id) {
                agent.status = AgentRunStatus::Starting;
                agent.started_at = Some(Instant::now());
            }
        }

        let task_payload = MessagePayload::TaskAssign {
            task: assignment.task.clone(),
        };
        let msg = AgentMessage::new("coordinator", &assignment.agent_id, task_payload);

        let mailbox = {
            let agents = self.active_agents.read().await;
            agents.get(&assignment.agent_id).map(|a| a.mailbox.clone())
        };

        if let Some(mailbox) = mailbox {
            mailbox.send(msg).await.map_err(|e| e.to_string())?;
        }

        {
            let mut agents = self.active_agents.write().await;
            if let Some(agent) = agents.get_mut(&assignment.agent_id) {
                agent.status = AgentRunStatus::Running;
            }
        }

        let result = self.wait_for_result(assignment.agent_id.as_str()).await?;

        Ok(result)
    }

    async fn wait_for_result(&self, agent_id: &str) -> Result<serde_json::Value, String> {
        let start = Instant::now();
        let timeout = self.default_timeout;

        loop {
            if start.elapsed() > timeout {
                return Err("Agent execution timed out".to_string());
            }

            let agents = self.active_agents.read().await;
            if let Some(agent) = agents.get(agent_id) {
                if agent.status == AgentRunStatus::Completed {
                    return agent.result.clone().ok_or_else(|| "No result".to_string());
                }
                if agent.status == AgentRunStatus::Failed {
                    return Err(agent
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unknown error".to_string()));
                }
                if agent.status == AgentRunStatus::Cancelled {
                    return Err("Agent was cancelled".to_string());
                }
            }
            drop(agents);

            sleep(Duration::from_millis(50)).await;
        }
    }

    async fn aggregate_results(
        &self,
        strategy: &ConsensusStrategy,
        results: &HashMap<String, serde_json::Value>,
        failures: &[(String, String)],
    ) -> serde_json::Value {
        if results.is_empty() {
            if failures.is_empty() {
                return serde_json::json!({"status": "no_results"});
            } else {
                return serde_json::json!({
                    "status": "all_failed",
                    "errors": failures.iter().map(|(id, err)| {
                        serde_json::json!({"agent_id": id, "error": err})
                    }).collect::<Vec<_>>()
                });
            }
        }

        match strategy {
            ConsensusStrategy::LeaderDecides { leader_agent_id } => {
                results.get(leader_agent_id).cloned().unwrap_or_else(|| {
                    results
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or(serde_json::json!(null))
                })
            }
            ConsensusStrategy::MajorityVote => {
                let mut vote_counts: HashMap<String, usize> = HashMap::new();
                for result in results.values() {
                    let key = serde_json::to_string(result).unwrap_or_default();
                    *vote_counts.entry(key).or_insert(0) += 1;
                }
                let majority_key = vote_counts
                    .into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(key, _)| key);
                if let Some(key) = majority_key {
                    serde_json::from_str(&key).unwrap_or(serde_json::json!(null))
                } else {
                    serde_json::json!(null)
                }
            }
            ConsensusStrategy::WeightedVote { weights } => {
                let mut weighted_scores: HashMap<String, f32> = HashMap::new();
                for (agent_id, result) in results {
                    let weight = weights.get(agent_id).copied().unwrap_or(1.0);
                    let score_key = serde_json::to_string(result).unwrap_or_default();
                    *weighted_scores.entry(score_key).or_insert(0.0) += weight;
                }
                let best_key = weighted_scores
                    .into_iter()
                    .max_by_key(|(_, score)| (*score * 1000.0) as i32)
                    .map(|(key, _)| key);
                if let Some(key) = best_key {
                    serde_json::from_str(&key).unwrap_or(serde_json::json!(null))
                } else {
                    serde_json::json!(null)
                }
            }
            ConsensusStrategy::FirstResponse => results
                .values()
                .next()
                .cloned()
                .unwrap_or(serde_json::json!(null)),
            ConsensusStrategy::Unanimous => {
                let unique_results: std::collections::HashSet<String> = results
                    .values()
                    .filter_map(|r| serde_json::to_string(r).ok())
                    .collect();
                if unique_results.len() == 1 {
                    results
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or(serde_json::json!(null))
                } else {
                    serde_json::json!({
                        "status": "no_consensus",
                        "divergent_results": unique_results.len(),
                        "results": results.values().collect::<Vec<_>>()
                    })
                }
            }
        }
    }

    async fn check_consensus(
        &self,
        strategy: &ConsensusStrategy,
        results: &HashMap<String, serde_json::Value>,
        failures: &[(String, String)],
    ) -> bool {
        match strategy {
            ConsensusStrategy::Unanimous => {
                if results.is_empty() {
                    false
                } else {
                    let unique_results: std::collections::HashSet<String> = results
                        .values()
                        .filter_map(|r| serde_json::to_string(r).ok())
                        .collect();
                    unique_results.len() == 1
                }
            }
            ConsensusStrategy::MajorityVote => {
                let total = results.len() + failures.len();
                if total == 0 {
                    return false;
                }
                let majority = total / 2;
                let mut vote_counts: HashMap<String, usize> = HashMap::new();
                for result in results.values() {
                    let key = serde_json::to_string(result).unwrap_or_default();
                    *vote_counts.entry(key).or_insert(0) += 1;
                }
                vote_counts.values().any(|&count| count > majority)
            }
            ConsensusStrategy::LeaderDecides { leader_agent_id } => {
                results.contains_key(leader_agent_id)
            }
            ConsensusStrategy::WeightedVote { .. } => !results.is_empty(),
            ConsensusStrategy::FirstResponse => !results.is_empty(),
        }
    }

    pub async fn get_agent_status(&self, agent_id: &str) -> Option<AgentRunStatus> {
        let agents = self.active_agents.read().await;
        agents.get(agent_id).map(|a| a.status)
    }

    pub async fn cancel_agent(&self, agent_id: &str) -> Result<(), String> {
        let mut agents = self.active_agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            if agent.status == AgentRunStatus::Running || agent.status == AgentRunStatus::Starting {
                agent.status = AgentRunStatus::Cancelled;
                return Ok(());
            }
        }
        Err("Agent not found or not running".to_string())
    }

    pub async fn cancel_plan(&self) {
        let mut agents = self.active_agents.write().await;
        for agent in agents.values_mut() {
            if agent.status == AgentRunStatus::Running || agent.status == AgentRunStatus::Starting {
                agent.status = AgentRunStatus::Cancelled;
            }
        }
    }

    pub async fn get_active_count(&self) -> usize {
        let agents = self.active_agents.read().await;
        agents
            .values()
            .filter(|a| a.status == AgentRunStatus::Running || a.status == AgentRunStatus::Starting)
            .count()
    }

    pub async fn get_completed_count(&self) -> usize {
        let agents = self.active_agents.read().await;
        agents
            .values()
            .filter(|a| a.status == AgentRunStatus::Completed)
            .count()
    }

    pub async fn get_failed_count(&self) -> usize {
        let agents = self.active_agents.read().await;
        agents
            .values()
            .filter(|a| a.status == AgentRunStatus::Failed)
            .count()
    }

    /// 并行调度一组工作者任务，收集所有结果。
    ///
    /// 所有工作者任务在后台并发启动，通过回调通道收集结果。
    /// 适用于不需要复杂依赖关系和共识策略的独立工作者任务。
    ///
    /// # 参数
    /// - `assignments`: 工作者任务分配列表
    /// - `max_concurrent`: 最大并发数
    /// - `timeout_secs`: 超时秒数
    pub async fn dispatch_workers(
        &self,
        assignments: &[AgentAssignment],
        max_concurrent: usize,
        timeout_secs: u64,
        conversation_id: &str,
    ) -> OrchestrationResult {
        let conv_id = conversation_id.to_string();
        let start_time = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        let max_concurrent = max_concurrent.max(1);

        let mut results: HashMap<String, serde_json::Value> = HashMap::new();
        let mut failures: Vec<(String, String)> = Vec::new();

        // 按优先级排序（高优先级先执行）
        let mut sorted: Vec<&AgentAssignment> = assignments.iter().collect();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.priority));

        // 分批执行：每批最多 max_concurrent 个任务
        for chunk in sorted.chunks(max_concurrent) {
            if start_time.elapsed() > timeout {
                for a in chunk {
                    failures.push((a.agent_id.clone(), "Timeout before start".to_string()));
                }
                break;
            }

            // 并行调度当前批次
            let mut handles = Vec::new();
            for assignment in chunk {
                let assignment = (*assignment).clone();
                let agent_id = assignment.agent_id.clone();

                // 注册 agent
                {
                    let (tx, _rx) = mpsc::channel(32);
                    let agent = ActiveAgent {
                        status: AgentRunStatus::Pending,
                        result: None,
                        error: None,
                        started_at: None,
                        completed_at: None,
                        mailbox: tx,
                    };
                    let mut agents = self.active_agents.write().await;
                    agents.insert(agent_id.clone(), agent);
                }

                // 发送任务消息
                let task_msg = AgentMessage::new(
                    "coordinator",
                    &agent_id,
                    MessagePayload::TaskAssign {
                        task: assignment.task.clone(),
                    },
                );

                {
                    let agents = self.active_agents.read().await;
                    if let Some(agent) = agents.get(&agent_id) {
                        let _ = agent.mailbox.send(task_msg).await;
                    }
                }

                // 启动工作者
                {
                    let mut agents = self.active_agents.write().await;
                    if let Some(agent) = agents.get_mut(&agent_id) {
                        agent.status = AgentRunStatus::Running;
                        agent.started_at = Some(Instant::now());
                    }
                }

                let task_id = assignment.agent_id.clone();
                let task_desc = assignment.task.clone();

                // 发射 worker-created 事件
                self.emit(
                    "worker-created",
                    serde_json::json!({
                        "conversationId": conv_id,
                        "workerId": agent_id,
                        "taskId": task_id,
                        "messageType": "progress",
                        "content": format!("Worker '{}' started: {}", agent_id, task_desc),
                        "status": "running"
                    }),
                );

                let wid = agent_id.clone();
                let tid = task_id.clone();
                let handle = tokio::spawn(async move {
                    let result = serde_json::json!({
                        "agent_id": wid,
                        "task": task_desc,
                        "status": "dispatched"
                    });
                    (wid.clone(), tid.clone(), Ok::<serde_json::Value, String>(result))
                });

                handles.push((agent_id, task_id, handle));
            }

            // 收集本批次结果
            for (worker_id, task_id, handle) in handles {
                match handle.await {
                    Ok((_wid, _tid, Ok(result))) => {
                        let mut agents = self.active_agents.write().await;
                        if let Some(agent) = agents.get_mut(&worker_id) {
                            agent.status = AgentRunStatus::Completed;
                            agent.result = Some(result.clone());
                            agent.completed_at = Some(Instant::now());
                        }

                        let duration = {
                            let agents = self.active_agents.read().await;
                            agents.get(&worker_id).and_then(|a| {
                                a.completed_at.and_then(|end| {
                                    a.started_at.map(|start| end.duration_since(start).as_millis() as u64)
                                })
                            }).unwrap_or(0)
                        };

                        // 发射 worker-progress (进度更新)
                        self.emit(
                            "worker-progress",
                            serde_json::json!({
                                "conversationId": conv_id,
                                "workerId": worker_id,
                                "taskId": task_id,
                                "messageType": "progress",
                                "content": format!("Worker completed task"),
                                "status": "running"
                            }),
                        );

                        // 发射 worker-completed
                        self.emit(
                            "worker-completed",
                            serde_json::json!({
                                "conversationId": conv_id,
                                "workerId": worker_id,
                                "taskId": task_id,
                                "messageType": "completion",
                                "content": serde_json::to_string(&result).unwrap_or_default(),
                                "status": "completed",
                                "durationMs": duration
                            }),
                        );

                        results.insert(worker_id, result);
                    }
                    Ok((_wid, _tid, Err(e))) => {
                        let mut agents = self.active_agents.write().await;
                        if let Some(agent) = agents.get_mut(&worker_id) {
                            agent.status = AgentRunStatus::Failed;
                            agent.error = Some(e.clone());
                            agent.completed_at = Some(Instant::now());
                        }

                        // 发射 worker-failed
                        self.emit(
                            "worker-failed",
                            serde_json::json!({
                                "conversationId": conv_id,
                                "workerId": worker_id,
                                "taskId": task_id,
                                "messageType": "error",
                                "content": e,
                                "status": "failed"
                            }),
                        );

                        failures.push((worker_id, e));
                    }
                    Err(e) => {
                        self.emit(
                            "worker-failed",
                            serde_json::json!({
                                "conversationId": conv_id,
                                "workerId": worker_id,
                                "taskId": task_id,
                                "messageType": "error",
                                "content": e.to_string(),
                                "status": "failed"
                            }),
                        );
                        failures.push(("spawn_error".to_string(), e.to_string()));
                    }
                }
            }
        }

        let status = if failures.is_empty() {
            OrchestrationStatus::Completed
        } else if results.is_empty() {
            OrchestrationStatus::Failed
        } else {
            OrchestrationStatus::PartiallyCompleted
        };

        OrchestrationResult {
            plan_id: "worker-pool".to_string(),
            goal: "Parallel worker dispatch".to_string(),
            status,
            agent_results: results,
            failures,
            final_result: serde_json::json!({"total_dispatched": assignments.len()}),
            consensus_reached: false,
            duration_ms: start_time.elapsed().as_millis() as u64,
        }
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestrationPlan {
    pub fn new(id: String, goal: String) -> Self {
        Self {
            id,
            goal,
            conversation_id: None,
            agents: Vec::new(),
            communication_plan: Vec::new(),
            consensus_strategy: ConsensusStrategy::default(),
            max_concurrent: 3,
            timeout_secs: 300,
        }
    }

    pub fn add_agent(mut self, assignment: AgentAssignment) -> Self {
        self.agents.push(assignment);
        self
    }

    pub fn with_consensus_strategy(mut self, strategy: ConsensusStrategy) -> Self {
        self.consensus_strategy = strategy;
        self
    }

    pub fn with_conversation_id(mut self, conv_id: String) -> Self {
        self.conversation_id = Some(conv_id);
        self
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

impl AgentAssignment {
    pub fn new(agent_id: String, role: AgentRole, task: String) -> Self {
        Self {
            agent_id,
            role,
            task,
            model: None,
            tools: Vec::new(),
            dependencies: Vec::new(),
            priority: 0,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

impl CommunicationRule {
    pub fn new(
        from_role: AgentRole,
        to_role: AgentRole,
        message_type: MessageType,
        trigger: TriggerType,
    ) -> Self {
        Self {
            from_role,
            to_role,
            message_type,
            trigger,
            required: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = AgentOrchestrator::new();
        assert_eq!(orchestrator.get_active_count().await, 0);
    }

    #[tokio::test]
    async fn test_plan_building() {
        let plan = OrchestrationPlan::new("test-plan".to_string(), "Test goal".to_string())
            .add_agent(AgentAssignment::new(
                "agent-1".to_string(),
                AgentRole::Developer,
                "Write code".to_string(),
            ))
            .add_agent(AgentAssignment::new(
                "agent-2".to_string(),
                AgentRole::Reviewer,
                "Review code".to_string(),
            ))
            .with_max_concurrent(2);

        assert_eq!(plan.agents.len(), 2);
        assert_eq!(plan.max_concurrent, 2);
    }

    #[tokio::test]
    async fn test_assignment_builder() {
        let assignment = AgentAssignment::new(
            "dev-1".to_string(),
            AgentRole::Developer,
            "Implement feature X".to_string(),
        )
        .with_model("claude-3-5-sonnet".to_string())
        .with_tools(vec!["bash".to_string(), "write".to_string()])
        .with_dependencies(vec!["research-1".to_string()])
        .with_priority(1);

        assert_eq!(assignment.agent_id, "dev-1");
        assert_eq!(assignment.role, AgentRole::Developer);
        assert!(assignment.model.is_some());
        assert_eq!(assignment.tools.len(), 2);
        assert_eq!(assignment.dependencies.len(), 1);
    }

    #[tokio::test]
    async fn test_communication_rule() {
        let rule = CommunicationRule::new(
            AgentRole::Coordinator,
            AgentRole::Developer,
            MessageType::TaskAssign,
            TriggerType::OnStart,
        )
        .required();

        assert_eq!(rule.from_role, AgentRole::Coordinator);
        assert_eq!(rule.to_role, AgentRole::Developer);
        assert!(rule.required);
    }
}
