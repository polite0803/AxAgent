use crate::agent_roles::AgentRole;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub id: String,
    pub goal: String,
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
    LeaderDecides { leader_agent_id: String },
    WeightedVote { weights: HashMap<String, f32> },
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
    TaskAssign { task: String },
    ProgressReport { progress: f32, message: String },
    TaskResult { result: serde_json::Value },
    TaskError { error: String },
    TaskCancel,
    Data { content: serde_json::Value },
    ConsensusRequest { vote: serde_json::Value },
    ConsensusResponse { agreed: bool, value: serde_json::Value },
}

pub struct AgentOrchestrator {
    active_agents: Arc<RwLock<HashMap<String, ActiveAgent>>>,
    max_retries: u32,
    default_timeout: Duration,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            max_retries: 3,
            default_timeout: Duration::from_secs(300),
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

    pub async fn execute_plan(&self, plan: OrchestrationPlan) -> OrchestrationResult {
        let start_time = Instant::now();
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
                    .filter(|a| !completed.contains_key(&a.agent_id) && !failures.iter().any(|(id, _)| id == &a.agent_id))
                    .map(|a| a.agent_id.clone())
                    .collect();
                drop(agents);
                for agent_id in remaining {
                    failures.push((agent_id, "Timeout".to_string()));
                }
                break;
            }

            let ready_agents = self.get_ready_agents(&plan, &completed, max_concurrent).await;
            let has_ready = !ready_agents.is_empty();

            if !has_ready && pending_count > 0 {
                sleep(Duration::from_millis(100)).await;
                let agents = self.active_agents.read().await;
                pending_count = agents
                    .values()
                    .filter(|a| a.status == AgentRunStatus::Pending || a.status == AgentRunStatus::Starting || a.status == AgentRunStatus::Running)
                    .count();
                continue;
            }

            for agent_id in ready_agents {
                let assignment = plan
                    .agents
                    .iter()
                    .find(|a| a.agent_id == agent_id)
                    .cloned();

                if let Some(assignment) = assignment {
                    match self.run_agent(&assignment).await {
                        Ok(result) => {
                            let mut agents = self.active_agents.write().await;
                            if let Some(agent) = agents.get_mut(&agent_id) {
                                agent.status = AgentRunStatus::Completed;
                                agent.result = Some(result.clone());
                                agent.completed_at = Some(Instant::now());
                            }
                            completed.insert(agent_id.clone(), result);
                        }
                        Err(e) => {
                            let mut agents = self.active_agents.write().await;
                            if let Some(agent) = agents.get_mut(&agent_id) {
                                agent.status = AgentRunStatus::Failed;
                                agent.error = Some(e.clone());
                                agent.completed_at = Some(Instant::now());
                            }
                            failures.push((agent_id.clone(), e));
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
                    return Err(agent.error.clone().unwrap_or_else(|| "Unknown error".to_string()));
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
                results
                    .get(leader_agent_id)
                    .cloned()
                    .unwrap_or_else(|| results.values().next().cloned().unwrap_or(serde_json::json!(null)))
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
            ConsensusStrategy::FirstResponse => {
                results.values().next().cloned().unwrap_or(serde_json::json!(null))
            }
            ConsensusStrategy::Unanimous => {
                let unique_results: std::collections::HashSet<String> = results
                    .values()
                    .filter_map(|r| serde_json::to_string(r).ok())
                    .collect();
                if unique_results.len() == 1 {
                    results.values().next().cloned().unwrap_or(serde_json::json!(null))
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
            ConsensusStrategy::WeightedVote { .. } => {
                !results.is_empty()
            }
            ConsensusStrategy::FirstResponse => {
                !results.is_empty()
            }
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
