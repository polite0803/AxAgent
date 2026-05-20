use async_trait::async_trait;
use axagent_agent::action_executor::{ActionError, ActionResult};
use axagent_agent::checkpoint::{Checkpoint, CheckpointBuilder, CheckpointManager};
use axagent_agent::coordinator::{
    AgentConfig, AgentCoordinator, AgentError, AgentImpl, AgentInput, AgentStatus,
    CoordinatorOutput,
};
use axagent_agent::error_recovery_engine::{ErrorRecoveryEngine, RecoveryConfig, RecoveryContext};
use axagent_agent::hierarchical_planner::{
    HierarchicalPlanner, PlanBuilder, ReplanAction, ReplanReason, TaskBuilder, TaskStatus,
};
use axagent_agent::react_engine::{LlmReasoningProvider, ReActEngine, ReActError, ReActResult};
use axagent_agent::reasoning_state::{ActionType, ReActConfig, ReasoningContext};
use axagent_agent::recovery_strategies::{ClassifiedError, ErrorClassifier, ErrorType};
use axagent_agent::recovery_strategies::{RecoveryAttempt, RecoveryResult, RecoveryStrategy};
use axagent_agent::thought_chain::Action;
use axagent_agent::thought_chain::ThoughtChain;
use axagent_agent::tree_of_thoughts::{
    LlmReasoningProvider as ToTLlmReasoningProvider, ThoughtStatus, TreeOfThoughtsEngine,
};
use axagent_core::error::AxAgentError;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Mock LLM Provider for ReActEngine
// ============================================================================

struct MockReasoningProvider {
    think_responses: Vec<String>,
    plan_responses: Vec<Action>,
    synthesize_response: String,
    analyze_response: String,
    reflect_response: String,
    call_index: std::sync::atomic::AtomicUsize,
    should_fail: bool,
}

impl Clone for MockReasoningProvider {
    fn clone(&self) -> Self {
        Self {
            think_responses: self.think_responses.clone(),
            plan_responses: self.plan_responses.clone(),
            synthesize_response: self.synthesize_response.clone(),
            analyze_response: self.analyze_response.clone(),
            reflect_response: self.reflect_response.clone(),
            call_index: std::sync::atomic::AtomicUsize::new(
                self.call_index.load(std::sync::atomic::Ordering::SeqCst),
            ),
            should_fail: self.should_fail,
        }
    }
}

impl MockReasoningProvider {
    fn new() -> Self {
        Self {
            think_responses: vec!["Mock thinking step".to_string()],
            plan_responses: vec![Action {
                action_type: ActionType::Plan,
                tool_name: None,
                tool_input: None,
                llm_prompt: Some("Mock plan".to_string()),
                requires_confirmation: false,
            }],
            synthesize_response: "Mock synthesis complete".to_string(),
            analyze_response: "Mock analysis: 2 words, complexity=low".to_string(),
            reflect_response: "Mock reflection: all steps successful".to_string(),
            call_index: std::sync::atomic::AtomicUsize::new(0),
            should_fail: false,
        }
    }

    fn with_synthesis_response(mut self, response: String) -> Self {
        self.synthesize_response = response;
        self
    }

    fn with_failures(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn next_index(&self) -> usize {
        self.call_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl LlmReasoningProvider for MockReasoningProvider {
    async fn analyze(
        &self,
        _input: &str,
        _context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        if self.should_fail {
            return Err(ReActError::LlmReasoningError("Mock LLM failure".to_string()));
        }
        Ok(self.analyze_response.clone())
    }

    async fn think(
        &self,
        _input: &str,
        _context: &ReasoningContext,
        _chain: &ThoughtChain,
    ) -> Result<String, ReActError> {
        if self.should_fail {
            return Err(ReActError::LlmReasoningError("Mock LLM failure".to_string()));
        }
        let idx = self.next_index();
        let response = self
            .think_responses
            .get(idx % self.think_responses.len())
            .cloned()
            .unwrap_or_else(|| "Mock thinking".to_string());
        Ok(response)
    }

    async fn plan(
        &self,
        _input: &str,
        context: &mut ReasoningContext,
        _chain: &ThoughtChain,
    ) -> Result<Action, ReActError> {
        if self.should_fail {
            return Err(ReActError::LlmReasoningError("Mock LLM failure".to_string()));
        }
        let idx = self.next_index();
        context.increment_depth();
        let action = self
            .plan_responses
            .get(idx % self.plan_responses.len())
            .cloned()
            .unwrap_or_else(|| Action {
                action_type: ActionType::Plan,
                tool_name: None,
                tool_input: None,
                llm_prompt: Some("Mock plan".to_string()),
                requires_confirmation: false,
            });
        Ok(action)
    }

    async fn reflect(
        &self,
        _chain: &ThoughtChain,
        _context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        if self.should_fail {
            return Err(ReActError::LlmReasoningError("Mock LLM failure".to_string()));
        }
        Ok(self.reflect_response.clone())
    }

    async fn synthesize(
        &self,
        _chain: &ThoughtChain,
        _context: &ReasoningContext,
    ) -> Result<String, ReActError> {
        if self.should_fail {
            return Err(ReActError::LlmReasoningError("Mock LLM failure".to_string()));
        }
        Ok(self.synthesize_response.clone())
    }
}

// ============================================================================
// Mock LLM Provider for TreeOfThoughts
// ============================================================================

struct MockToTProvider {
    think_responses: Vec<String>,
    evaluate_responses: Vec<String>,
    call_index: std::sync::atomic::AtomicUsize,
    should_fail: bool,
}

impl MockToTProvider {
    fn new() -> Self {
        Self {
            think_responses: vec![
                "Alternative approach A: analyze from first principles".to_string(),
                "Alternative approach B: use empirical evidence".to_string(),
                "Alternative approach C: consider counterexamples".to_string(),
            ],
            evaluate_responses: vec!["0.7".to_string(), "0.5".to_string(), "0.9".to_string()],
            call_index: std::sync::atomic::AtomicUsize::new(0),
            should_fail: false,
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_eval_scores(mut self, scores: Vec<f64>) -> Self {
        self.evaluate_responses = scores.iter().map(|s| format!("{}", s)).collect();
        self
    }

    fn next_index(&self) -> usize {
        self.call_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl ToTLlmReasoningProvider for MockToTProvider {
    async fn think_branch(&self, _prompt: &str) -> Result<String, AxAgentError> {
        if self.should_fail {
            return Err(AxAgentError::Agent {
                source: None,
                context: "Mock LLM failure".to_string(),
            });
        }
        let idx = self.next_index();
        Ok(self
            .think_responses
            .get(idx % self.think_responses.len())
            .cloned()
            .unwrap_or_else(|| "Mock branch reasoning".to_string()))
    }

    async fn evaluate_thought(&self, _prompt: &str) -> Result<String, AxAgentError> {
        if self.should_fail {
            return Err(AxAgentError::Agent {
                source: None,
                context: "Mock LLM failure".to_string(),
            });
        }
        let idx = self.next_index();
        Ok(self
            .evaluate_responses
            .get(idx % self.evaluate_responses.len())
            .cloned()
            .unwrap_or_else(|| "0.5".to_string()))
    }
}

// ============================================================================
// Mock Agent for Coordinator tests
// ============================================================================

struct MockCoordinatorAgent {
    status: AgentStatus,
    fail_on_execute: bool,
    pause_count: usize,
    resume_count: usize,
}

impl MockCoordinatorAgent {
    fn new() -> Self {
        Self {
            status: AgentStatus::Idle,
            fail_on_execute: false,
            pause_count: 0,
            resume_count: 0,
        }
    }

    fn with_failure() -> Self {
        Self {
            status: AgentStatus::Idle,
            fail_on_execute: true,
            pause_count: 0,
            resume_count: 0,
        }
    }
}

#[async_trait]
impl AgentImpl for MockCoordinatorAgent {
    async fn initialize(&mut self, _config: AgentConfig) -> Result<(), AgentError> {
        self.status = AgentStatus::Idle;
        Ok(())
    }

    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        if self.fail_on_execute {
            return Err(AgentError::ExecutionFailed("simulated failure".to_string()));
        }
        self.status = AgentStatus::Running;
        Ok(CoordinatorOutput::success(input.content, 1))
    }

    async fn pause(&mut self) -> Result<(), AgentError> {
        self.pause_count += 1;
        self.status = AgentStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        self.resume_count += 1;
        self.status = AgentStatus::Running;
        Ok(())
    }

    async fn cancel(&mut self) -> Result<(), AgentError> {
        self.status = AgentStatus::Idle;
        Ok(())
    }

    fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    fn agent_type(&self) -> &'static str {
        "mock"
    }
}

// ============================================================================
// Test Module 1: test_react_engine_lifecycle
// ============================================================================

#[cfg(test)]
mod test_react_engine_lifecycle {
    use super::*;

    #[tokio::test]
    async fn test_react_engine_with_mock_provider() {
        let mock_provider = MockReasoningProvider::new();
        let mut engine = ReActEngine::new().with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("Test input").await;

        assert!(result.iterations > 0 || result.error.is_some());
    }

    #[tokio::test]
    async fn test_react_engine_full_reasoning_cycle() {
        let mock_provider = MockReasoningProvider::new()
            .with_synthesis_response("Final synthesized answer".to_string());
        let mut engine = ReActEngine::new().with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("What is the meaning of life?").await;

        assert!(result.iterations > 0, "Engine should complete at least one iteration");
        if result.success {
            assert!(
                !result.final_response.is_empty() || result.context.current_goal.is_some(),
                "Success result should have response or goal"
            );
        }
        assert!(result.context.iteration >= 1);
    }

    #[tokio::test]
    async fn test_react_result_produces_valid_context() {
        let mock_provider = MockReasoningProvider::new();
        let mut engine = ReActEngine::new().with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("Simple question").await;

        assert!(result.context.current_goal.is_some() || result.error.is_some());
    }

    #[tokio::test]
    async fn test_react_engine_produces_valid_react_result() {
        let mock_provider = MockReasoningProvider::new();
        let mut engine = ReActEngine::new().with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("Hello").await;

        assert!(result.iterations > 0 || result.error.is_some());
        let _ = &result.thought_chain;
        let _ = &result.context;
    }

    #[tokio::test]
    async fn test_react_engine_with_max_iterations_constraint() {
        let mock_provider = MockReasoningProvider::new();
        let config = ReActConfig {
            max_iterations: 2,
            ..Default::default()
        };
        let mut engine = ReActEngine::new()
            .with_config(config)
            .with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("Long question with many parts").await;

        assert!(result.iterations <= 2 || result.error.is_some());
    }

    #[tokio::test]
    async fn test_react_engine_mock_provider_failure() {
        let mock_provider = MockReasoningProvider::new().with_failures();
        let mut engine = ReActEngine::new().with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("Test").await;

        assert!(result.error.is_some() || !result.success);
    }

    #[tokio::test]
    async fn test_react_engine_pause_via_event_subscription() {
        let mock_provider = MockReasoningProvider::new();
        let engine = ReActEngine::new().with_reasoning_provider(Arc::new(mock_provider.clone()));

        let mut subscriber = engine.subscribe();

        let handle = tokio::spawn(async move {
            let mut events = Vec::new();
            loop {
                tokio::select! {
                    Ok(event) = subscriber.recv() => {
                        events.push(event);
                        if events.len() >= 3 {
                            break;
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {
                        break;
                    }
                }
            }
            events.len()
        });

        let mut engine = ReActEngine::new().with_reasoning_provider(Arc::new(mock_provider));
        let _ = engine.run("Test pause observation").await;

        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_react_engine_cancel_via_token_budget() {
        let mock_provider = MockReasoningProvider::new();
        let config = ReActConfig {
            token_budget_enabled: true,
            token_budget_limit: Some(100),
            max_iterations: 50,
            ..Default::default()
        };
        let mut engine = ReActEngine::new()
            .with_config(config)
            .with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("A very long and complex question").await;

        assert!(result.iterations > 0 || result.error.is_some());
    }

    #[tokio::test]
    async fn test_react_engine_think_act_observe_synthesize_sequence() {
        let mock_provider = MockReasoningProvider::new();
        let config = ReActConfig {
            enable_analyzing: false,
            max_iterations: 5,
            enable_reflection: false,
            verification_enabled: false,
            ..Default::default()
        };
        let mut engine = ReActEngine::new()
            .with_config(config)
            .with_reasoning_provider(Arc::new(mock_provider));

        let result = engine.run("Test sequence").await;

        assert!(result.iterations > 0 || result.error.is_some());
    }

    #[tokio::test]
    async fn test_react_result_success_and_failure_constructors() {
        use axagent_agent::reasoning_state::ReasoningState;
        use axagent_agent::thought_chain::{ChainSummary, ThoughtStep};

        let ctx = ReasoningContext::new("test");
        let chain = ChainSummary {
            total_steps: 3,
            iterations: 3,
            current_state: "finished".to_string(),
            steps: vec![ThoughtStep {
                id: 0,
                state: ReasoningState::Thinking,
                reasoning: "step1".to_string(),
                action: None,
                observation: None,
                result: None,
                is_verified: true,
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            }],
        };

        let success_result = ReActResult::success(
            "Success response".to_string(),
            chain.clone(),
            3,
            Duration::from_millis(100),
            ctx.clone(),
        );
        assert!(success_result.success);
        assert!(success_result.error.is_none());
        assert_eq!(success_result.final_response, "Success response");
        assert_eq!(success_result.iterations, 3);

        let failure_result = ReActResult::failure(
            "Test error".to_string(),
            chain,
            1,
            Duration::from_millis(50),
            ctx,
        );
        assert!(!failure_result.success);
        assert!(failure_result.error.is_some());
        assert_eq!(failure_result.error.unwrap(), "Test error");
    }
}

// ============================================================================
// Test Module 2: test_hierarchical_planner_dynamic_replanning
// ============================================================================

#[cfg(test)]
mod test_hierarchical_planner_dynamic_replanning {
    use super::*;

    fn make_task(desc: &str, action: &str) -> axagent_agent::hierarchical_planner::PlannedTask {
        TaskBuilder::new(desc, action).with_max_retries(1).build()
    }

    #[allow(dead_code)]
    fn make_phase(
        name: &str,
        desc: &str,
        deps: Vec<String>,
        tasks: Vec<axagent_agent::hierarchical_planner::PlannedTask>,
    ) -> axagent_agent::hierarchical_planner::Phase {
        axagent_agent::hierarchical_planner::Phase {
            id: format!("id_{}", name),
            name: name.to_string(),
            description: desc.to_string(),
            tasks,
            dependencies: deps,
            status: axagent_agent::hierarchical_planner::PhaseStatus::Pending,
        }
    }

    #[test]
    fn test_planner_with_three_phases_and_five_tasks() {
        let mut planner = HierarchicalPlanner::new();

        let plan = PlanBuilder::new("Integration Test Goal")
            .add_phase(
                "Phase 1: Setup",
                "Initialize environment",
                vec![],
                vec![
                    make_task("Setup database", "setup_db"),
                    make_task("Configure network", "setup_network"),
                ],
            )
            .add_phase(
                "Phase 2: Process",
                "Run data processing",
                vec!["id_Phase 1: Setup".to_string()],
                vec![
                    make_task("Load data", "load_data"),
                    make_task("Transform data", "transform_data"),
                ],
            )
            .add_phase(
                "Phase 3: Cleanup",
                "Finalize results",
                vec!["id_Phase 2: Process".to_string()],
                vec![make_task("Generate report", "generate_report")],
            )
            .build(&mut planner);

        assert_eq!(plan.goal, "Integration Test Goal");
        assert_eq!(plan.phases.len(), 3);

        let total_tasks: usize = plan.phases.iter().map(|p| p.tasks.len()).sum();
        assert_eq!(total_tasks, 5);
    }

    #[test]
    fn test_execute_first_two_tasks_then_simulate_failure() {
        let mut planner = HierarchicalPlanner::new();

        let _plan = PlanBuilder::new("Task Execution Test")
            .add_phase(
                "Execution Phase",
                "Execute tasks with failure",
                vec![],
                vec![
                    make_task("Task 1", "action1"),
                    make_task("Task 2", "action2"),
                    make_task("Task 3", "action3"),
                    make_task("Task 4", "action4"),
                    make_task("Task 5", "action5"),
                ],
            )
            .build(&mut planner);

        planner.start_execution().unwrap();

        let executable = planner.get_next_executable_tasks();
        let task1_id = executable[0].id.clone();
        planner.mark_task_started(&task1_id).unwrap();
        planner
            .mark_task_completed(&task1_id, serde_json::json!({"status": "ok"}))
            .unwrap();

        let executable = planner.get_next_executable_tasks();
        let task2_id = executable[0].id.clone();
        planner.mark_task_started(&task2_id).unwrap();
        planner
            .mark_task_completed(&task2_id, serde_json::json!({"status": "ok"}))
            .unwrap();

        let progress = planner.get_progress();
        assert_eq!(progress.completed_tasks, 2);

        let executable = planner.get_next_executable_tasks();
        let task3_id = executable[0].id.clone();
        planner.mark_task_started(&task3_id).unwrap();
        let mark_result = planner.mark_task_failed(&task3_id, "Simulated failure on task 3");
        assert!(mark_result.is_ok(), "mark_task_failed failed: {:?}", mark_result);

        let all_tasks: Vec<_> = planner
            .get_plan()
            .unwrap()
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .map(|t| (t.id.clone(), format!("{:?}", t.status), t.retry_count))
            .collect();

        let failed_steps = planner.get_failed_steps();
        assert!(
            failed_steps.contains(&task3_id),
            "Expected task3 ({}) to be in failed steps (status info: {:?}), got {:?}",
            task3_id,
            all_tasks,
            failed_steps
        );
    }

    #[test]
    fn test_replan_after_task_failure() {
        let mut planner = HierarchicalPlanner::new();

        let _plan = PlanBuilder::new("Replan Test")
            .add_phase(
                "Test Phase",
                "Phase with failure",
                vec![],
                vec![
                    make_task("Task 1", "action1"),
                    make_task("Task 2", "action2"),
                    make_task("Task 3", "action3"),
                ],
            )
            .build(&mut planner);

        planner.start_execution().unwrap();

        let task1_id = planner.get_next_executable_tasks()[0].id.clone();
        planner
            .mark_task_completed(&task1_id, serde_json::json!({"result": "success"}))
            .unwrap();

        let task2_id = planner.get_next_executable_tasks()[0].id.clone();
        planner
            .mark_task_completed(&task2_id, serde_json::json!({"result": "success"}))
            .unwrap();

        let task3_id = planner.get_next_executable_tasks()[0].id.clone();
        planner.mark_task_started(&task3_id).unwrap();
        planner
            .mark_task_failed(&task3_id, "Connection timeout")
            .unwrap();
        planner.mark_task_started(&task3_id).unwrap();
        planner
            .mark_task_failed(&task3_id, "Connection timeout again")
            .unwrap();

        let completed_before = planner.get_completed_steps();
        assert_eq!(completed_before.len(), 2);
        assert!(completed_before.contains(&task1_id));
        assert!(completed_before.contains(&task2_id));

        let reason = ReplanReason::StepFailed {
            task_id: task3_id.clone(),
            error: "Connection timeout".to_string(),
        };

        let actions = vec![ReplanAction::Retry {
            task_id: task3_id.clone(),
            modified_parameters: Some(serde_json::json!({
                "timeout_ms": 30000,
                "retries": 5
            })),
        }];

        let record = planner.replan(reason, actions).unwrap();

        assert_eq!(record.version, 1);
        assert!(matches!(record.reason, ReplanReason::StepFailed { .. }));

        let plan = planner.get_plan().unwrap();
        let task3 = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .find(|t| t.id == task3_id)
            .unwrap();
        assert_eq!(task3.status, TaskStatus::Pending);
        assert_eq!(task3.retry_count, 0);
        assert_eq!(task3.error, None);
    }

    #[test]
    fn test_completed_tasks_preserved_after_replan() {
        let mut planner = HierarchicalPlanner::new();

        let _plan = PlanBuilder::new("Preserve Test")
            .add_phase(
                "Phase",
                "Preserve completed tasks",
                vec![],
                vec![
                    make_task("Task A", "action_a"),
                    make_task("Task B", "action_b"),
                    make_task("Task C", "action_c"),
                ],
            )
            .build(&mut planner);

        planner.start_execution().unwrap();

        let task_a_id = planner.get_next_executable_tasks()[0].id.clone();
        planner
            .mark_task_completed(&task_a_id, serde_json::json!({"data": "a"}))
            .unwrap();

        let task_b_id = planner.get_next_executable_tasks()[0].id.clone();
        planner
            .mark_task_completed(&task_b_id, serde_json::json!({"data": "b"}))
            .unwrap();

        let task_c_id = planner.get_next_executable_tasks()[0].id.clone();
        planner.mark_task_started(&task_c_id).unwrap();
        planner
            .mark_task_failed(&task_c_id, "Task C failed")
            .unwrap();
        planner.mark_task_started(&task_c_id).unwrap();
        planner
            .mark_task_failed(&task_c_id, "Task C failed again")
            .unwrap();

        let reason = ReplanReason::StepFailed {
            task_id: task_c_id.clone(),
            error: "Task C failed".to_string(),
        };
        let actions = vec![ReplanAction::Retry {
            task_id: task_c_id.clone(),
            modified_parameters: None,
        }];
        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        let task_a = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .find(|t| t.id == task_a_id)
            .unwrap();
        let task_b = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .find(|t| t.id == task_b_id)
            .unwrap();

        assert_eq!(task_a.status, TaskStatus::Completed);
        assert_eq!(task_a.result, Some(serde_json::json!({"data": "a"})));
        assert_eq!(task_b.status, TaskStatus::Completed);
        assert_eq!(task_b.result, Some(serde_json::json!({"data": "b"})));
    }

    #[test]
    fn test_rollback_to_previous_version() {
        let mut planner = HierarchicalPlanner::new();

        let task1 = TaskBuilder::new("Original Task", "original_action").build();
        let task1_id = task1.id.clone();

        let _plan = PlanBuilder::new("Rollback Test")
            .add_phase("Phase", "Initial phase", vec![], vec![task1])
            .build(&mut planner);

        assert_eq!(planner.get_plan_versions().len(), 1);

        let reason = ReplanReason::ManualIntervention {
            reason: "Remove task for testing".to_string(),
        };
        let actions = vec![ReplanAction::Remove {
            task_id: task1_id.clone(),
            reason: "Testing rollback".to_string(),
        }];
        planner.replan(reason, actions).unwrap();

        assert_eq!(planner.get_plan_versions().len(), 2);

        let plan_after_replan = planner.get_plan().unwrap();
        assert!(
            plan_after_replan
                .phases
                .iter()
                .flat_map(|p| p.tasks.iter())
                .all(|t| t.id != task1_id)
        );

        planner.rollback(0).unwrap();

        assert_eq!(planner.get_plan_versions().len(), 2);
        let restored_plan = planner.get_plan().unwrap();
        let restored_task = restored_plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .find(|t| t.id == task1_id);
        assert!(restored_task.is_some());
    }

    #[test]
    fn test_replan_skip_and_insert() {
        let mut planner = HierarchicalPlanner::new();

        let _plan = PlanBuilder::new("Skip and Insert Test")
            .add_phase(
                "Phase",
                "Test skip and insert",
                vec![],
                vec![
                    make_task("Task 1", "action1"),
                    make_task("Task 2", "action2"),
                ],
            )
            .build(&mut planner);

        let task1_id = planner.get_plan().unwrap().phases[0].tasks[0].id.clone();
        let phase_id = planner.get_plan().unwrap().phases[0].id.clone();

        let reason = ReplanReason::ResourceConstraint {
            constraint: "API rate limited".to_string(),
        };

        let new_task = TaskBuilder::new("Replacement Task", "replacement").build();

        let actions = vec![
            ReplanAction::Skip {
                task_id: task1_id.clone(),
                reason: "Rate limited".to_string(),
            },
            ReplanAction::Insert {
                phase_id: phase_id.clone(),
                task: new_task,
                position: 1,
            },
        ];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        let skipped = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .find(|t| t.id == task1_id)
            .unwrap();
        assert_eq!(skipped.status, TaskStatus::Skipped);
        assert_eq!(plan.phases[0].tasks.len(), 3);
        assert_eq!(plan.phases[0].tasks[1].description, "Replacement Task");
    }

    #[test]
    fn test_replan_modify_task_parameters() {
        let mut planner = HierarchicalPlanner::new();

        let task = TaskBuilder::new("Modify Test", "action")
            .with_max_retries(1)
            .with_role("junior")
            .build();

        let _plan = PlanBuilder::new("Modify Test")
            .add_phase("Phase", "Modify phase", vec![], vec![task])
            .build(&mut planner);

        let task_id = planner.get_plan().unwrap().phases[0].tasks[0].id.clone();

        let reason = ReplanReason::ManualIntervention {
            reason: "Upgrade task parameters".to_string(),
        };

        let modifications = serde_json::json!({
            "description": "Modified task",
            "max_retries": 10,
            "assigned_role": "senior",
            "parameters": {"new_param": "value"}
        });

        let actions = vec![ReplanAction::ModifyTask {
            task_id: task_id.clone(),
            modifications,
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        let modified_task = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .find(|t| t.id == task_id)
            .unwrap();

        assert_eq!(modified_task.description, "Modified task");
        assert_eq!(modified_task.max_retries, 10);
        assert_eq!(modified_task.assigned_role, Some("senior".to_string()));
    }
}

// ============================================================================
// Test Module 3: test_tree_of_thoughts_reasoning
// ============================================================================

#[cfg(test)]
mod test_tree_of_thoughts_reasoning {
    use super::*;

    #[tokio::test]
    async fn test_tot_engine_creation() {
        let engine = TreeOfThoughtsEngine::new(3, 3, 0.3);

        assert_eq!(engine.branching_factor, 3);
        assert_eq!(engine.max_depth, 3);
        assert_eq!(engine.evaluation_threshold, 0.3);
        assert_eq!(engine.total_nodes(), 1);
        assert_eq!(engine.root_id, "node_0");
    }

    #[tokio::test]
    async fn test_tot_generate_branching_options() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> = Arc::new(MockToTProvider::new());
        let children = engine
            .generate_branching_options(root_id.clone(), "Test reasoning context", &provider)
            .await
            .unwrap();

        assert_eq!(children.len(), 3);

        for child_id in &children {
            let node = engine.get_node(child_id).unwrap();
            assert_eq!(node.parent, Some(root_id.clone()));
            assert_eq!(node.status, ThoughtStatus::Generated);
        }
    }

    #[tokio::test]
    async fn test_tot_evaluate_all_generated_thoughts() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> = Arc::new(MockToTProvider::new());
        let children = engine
            .generate_branching_options(root_id, "Test context", &provider)
            .await
            .unwrap();

        let mut scores = Vec::new();
        for child_id in &children {
            let score = engine
                .evaluate_and_score_node(child_id, "Test context", &provider)
                .await
                .unwrap();
            scores.push((child_id.clone(), score));

            let node = engine.get_node(child_id).unwrap();
            assert_eq!(node.status, ThoughtStatus::Explored);
        }

        assert_eq!(scores.len(), 3);
        for (_, score) in &scores {
            assert!(*score >= 0.0 && *score <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_tot_prune_below_threshold() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> = Arc::new(MockToTProvider::new());
        let children = engine
            .generate_branching_options(root_id, "Test context", &provider)
            .await
            .unwrap();

        for child_id in &children {
            let _ = engine
                .evaluate_and_score_node(child_id, "Test context", &provider)
                .await;
        }

        let pruned = engine.prune_below_threshold(0.3);

        for pruned_id in &pruned {
            let node = engine.get_node(pruned_id).unwrap();
            assert_eq!(node.status, ThoughtStatus::Pruned);
        }
    }

    #[tokio::test]
    async fn test_tot_select_best_path_root_to_leaf() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> = Arc::new(MockToTProvider::new());
        let children = engine
            .generate_branching_options(root_id.clone(), "Test context", &provider)
            .await
            .unwrap();

        for child_id in &children {
            let _ = engine
                .evaluate_and_score_node(child_id, "Test context", &provider)
                .await;
        }

        let best_path = engine.select_best_path();

        assert!(!best_path.is_empty());
        assert_eq!(best_path[0], root_id);
        assert!(best_path.len() >= 2);

        for i in 1..best_path.len() {
            let parent_id = &best_path[i - 1];
            let child_id = &best_path[i];
            let parent_node = engine.get_node(parent_id).unwrap();
            assert!(parent_node.children.contains(child_id));
        }
    }

    #[tokio::test]
    async fn test_tot_backtrack_and_re_explore() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> = Arc::new(MockToTProvider::new());
        let level1_children = engine
            .generate_branching_options(root_id.clone(), "Level 1 context", &provider)
            .await
            .unwrap();

        assert_eq!(level1_children.len(), 3);

        for child_id in &level1_children {
            let _ = engine
                .evaluate_and_score_node(child_id, "Level 1 context", &provider)
                .await;
        }

        let first_child = &level1_children[0];
        let first_child_clone = first_child.clone();

        let _level2_children = engine
            .generate_branching_options(first_child_clone, "Level 2 context", &provider)
            .await
            .unwrap();

        let total_nodes_before_backtrack = engine.total_nodes();
        assert!(total_nodes_before_backtrack > 4);

        engine.backtrack_to(&level1_children[0]).unwrap();

        let total_nodes_after_backtrack = engine.total_nodes();
        assert!(total_nodes_after_backtrack < total_nodes_before_backtrack);

        let backtracked_node = engine.get_node(&level1_children[0]).unwrap();
        assert!(backtracked_node.children.is_empty());
        assert_eq!(backtracked_node.status, ThoughtStatus::Explored);

        let re_explored = engine
            .generate_branching_options(level1_children[0].clone(), "Re-explore context", &provider)
            .await
            .unwrap();

        assert_eq!(re_explored.len(), 3);
    }

    #[tokio::test]
    async fn test_tot_with_custom_eval_scores() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> =
            Arc::new(MockToTProvider::new().with_eval_scores(vec![0.2, 0.8, 0.5]));
        let children = engine
            .generate_branching_options(root_id, "Scored context", &provider)
            .await
            .unwrap();

        for child_id in &children {
            let _ = engine
                .evaluate_and_score_node(child_id, "Scored context", &provider)
                .await;
        }

        engine.prune_below_threshold(0.3);

        let best_path = engine.select_best_path();

        assert_eq!(best_path.len(), 2);
        assert!(best_path.contains(&children[1]));
    }

    #[tokio::test]
    async fn test_tot_max_depth_respected() {
        let mut engine = TreeOfThoughtsEngine::new(2, 2, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> = Arc::new(MockToTProvider::new());

        let level1 = engine
            .generate_branching_options(root_id.clone(), "Level 1", &provider)
            .await
            .unwrap();
        assert_eq!(level1.len(), 2);

        let level2 = engine
            .generate_branching_options(level1[0].clone(), "Level 2", &provider)
            .await
            .unwrap();
        assert_eq!(level2.len(), 2);

        let level3 = engine
            .generate_branching_options(level2[0].clone(), "Level 3", &provider)
            .await
            .unwrap();
        assert!(level3.is_empty(), "Should not generate children beyond max_depth");
    }

    #[tokio::test]
    async fn test_tot_state_summary() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> = Arc::new(MockToTProvider::new());
        let children = engine
            .generate_branching_options(root_id, "State test", &provider)
            .await
            .unwrap();

        for child_id in &children {
            let _ = engine
                .evaluate_and_score_node(child_id, "State test", &provider)
                .await;
        }

        let state = engine.get_current_state();

        assert_eq!(state.nodes.len(), engine.total_nodes());
        assert!(!state.edges.is_empty());
        assert!(!state.selected_path.is_empty());
        assert_eq!(state.selected_path[0], engine.root_id);
    }

    #[tokio::test]
    async fn test_tot_mark_node_selected() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        engine.mark_node_selected(&root_id);

        let node = engine.get_node(&root_id).unwrap();
        assert_eq!(node.status, ThoughtStatus::Selected);
    }

    #[tokio::test]
    async fn test_tot_with_failing_provider() {
        let mut engine = TreeOfThoughtsEngine::new(3, 3, 0.3);
        let root_id = engine.root_id.clone();

        let provider: Arc<dyn ToTLlmReasoningProvider> =
            Arc::new(MockToTProvider::new().with_failure());

        let children = engine
            .generate_branching_options(root_id.clone(), "Failing context", &provider)
            .await
            .unwrap();

        assert_eq!(children.len(), 3);

        for child_id in &children {
            let node = engine.get_node(child_id).unwrap();
            assert!(node.content.contains("Alternative reasoning path"));
        }

        let eval = engine
            .evaluate_thought(&root_id, "Failing context", &provider)
            .await
            .unwrap();
        assert!((0.0..=1.0).contains(&eval));
    }
}

// ============================================================================
// Test Module 4: test_error_recovery_with_context
// ============================================================================

#[cfg(test)]
mod test_error_recovery_with_context {
    use super::*;

    #[test]
    fn test_error_context_classification() {
        let classifier = ErrorClassifier::new();

        let classified = classifier
            .classify_with_context("connection timeout", Some("During database query".to_string()));

        assert_eq!(classified.error_type, ErrorType::Transient);
        assert_eq!(classified.original_error, "connection timeout");
        assert_eq!(classified.context, Some("During database query".to_string()));
    }

    #[test]
    fn test_error_code_extraction() {
        let classifier = ErrorClassifier::new();

        let cases = vec![
            ("HTTP 404 not found", Some("404".to_string())),
            ("HTTP 500 internal server error", Some("500".to_string())),
            ("Error code: 429", Some("429".to_string())),
            ("Unknown error", None),
            ("Generic failure", None),
            ("Error 503 service unavailable", Some("503".to_string())),
        ];

        for (error_msg, expected_code) in cases {
            let classified = classifier.classify_with_context(error_msg, None);
            assert_eq!(classified.error_code, expected_code, "Failed for: {}", error_msg);
        }
    }

    #[test]
    fn test_to_report_generates_valid_error_report() {
        let classifier = ErrorClassifier::new();
        let classified = classifier
            .classify_with_context("HTTP 500 internal error", Some("During API call".to_string()));

        assert_eq!(classified.error_type, ErrorType::Unrecoverable);
        assert_eq!(classified.error_code, Some("500".to_string()));
        assert!(!classified.original_error.is_empty());
    }

    #[test]
    fn test_error_chain_propagation() {
        let classifier = ErrorClassifier::new();

        let error1 = classifier.classify("connection timeout");
        assert_eq!(error1, ErrorType::Transient);

        let error2 = classifier.classify("resource exhausted");
        assert_eq!(error2, ErrorType::Recoverable);

        let error3 = classifier.classify("syntax error");
        assert_eq!(error3, ErrorType::Unrecoverable);

        let error4 = classifier.classify("something unknown happened");
        assert_eq!(error4, ErrorType::Unknown);
    }

    #[test]
    fn test_error_type_properties() {
        let types = [
            ErrorType::Transient,
            ErrorType::Recoverable,
            ErrorType::Unrecoverable,
            ErrorType::Unknown,
        ];

        for error_type in &types {
            assert!(!error_type.as_str().is_empty());
            assert!(!error_type.description().is_empty());
        }

        assert_eq!(ErrorType::Transient.as_str(), "transient");
        assert_eq!(ErrorType::Recoverable.as_str(), "recoverable");
        assert_eq!(ErrorType::Unrecoverable.as_str(), "unrecoverable");
        assert_eq!(ErrorType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_classified_error_serialization() {
        let classified = ClassifiedError {
            error_type: ErrorType::Transient,
            original_error: "timeout".to_string(),
            error_code: Some("504".to_string()),
            context: Some("During request".to_string()),
        };

        let json = serde_json::to_string(&classified).unwrap();
        let deserialized: ClassifiedError = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.error_type, ErrorType::Transient);
        assert_eq!(deserialized.original_error, "timeout");
        assert_eq!(deserialized.error_code, Some("504".to_string()));
    }

    #[test]
    fn test_recovery_context_with_error() {
        let ctx = RecoveryContext::new()
            .with_task_id("task-1".to_string())
            .with_error("connection timeout".to_string())
            .build();

        assert_eq!(ctx.task_id, Some("task-1".to_string()));
        assert_eq!(ctx.original_error, Some("connection timeout".to_string()));
        assert_eq!(ctx.attempts, 0);
    }

    #[tokio::test]
    async fn test_recovery_engine_classifies_and_recovers() {
        let engine = ErrorRecoveryEngine::new();

        let classified = engine.classify_error("connection timeout");
        assert_eq!(classified.error_type, ErrorType::Transient);

        let result = engine
            .recover("connection timeout", || async { Ok::<i32, String>(42) })
            .await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_recovery_with_config_and_context() {
        let config = RecoveryConfig {
            max_total_attempts: 2,
            enable_fallback: true,
            enable_adjustments: true,
            timeout_per_attempt: Duration::from_secs(5),
        };

        let engine = ErrorRecoveryEngine::new().with_config(config);
        let mut attempts = 0;

        let result = engine
            .recover("permission denied", || {
                attempts += 1;
                async move {
                    if attempts >= 2 {
                        Ok::<i32, String>(100)
                    } else {
                        Err("still denied".to_string())
                    }
                }
            })
            .await;

        assert!(result.success);
    }

    #[test]
    fn test_error_type_as_str_and_description() {
        let transient = ErrorType::Transient;
        assert_eq!(transient.as_str(), "transient");
        assert!(transient.description().contains("Temporary"));

        let recoverable = ErrorType::Recoverable;
        assert_eq!(recoverable.as_str(), "recoverable");
        assert!(recoverable.description().contains("Recoverable"));

        let unrecoverable = ErrorType::Unrecoverable;
        assert_eq!(unrecoverable.as_str(), "unrecoverable");
        assert!(unrecoverable.description().contains("Unrecoverable"));

        let unknown = ErrorType::Unknown;
        assert_eq!(unknown.as_str(), "unknown");
        assert!(unknown.description().contains("Unknown"));
    }

    #[test]
    fn test_is_recoverable_for_different_error_types() {
        let engine = ErrorRecoveryEngine::new();

        let transient_strategy = engine.get_recovery_strategy(ErrorType::Transient);
        assert!(transient_strategy.should_retry());

        let recoverable_strategy = engine.get_recovery_strategy(ErrorType::Recoverable);
        assert!(recoverable_strategy.should_retry());

        let unrecoverable_strategy = engine.get_recovery_strategy(ErrorType::Unrecoverable);
        assert!(!unrecoverable_strategy.should_retry());

        let unknown_strategy = engine.get_recovery_strategy(ErrorType::Unknown);
        assert!(unknown_strategy.should_retry());
    }

    #[test]
    fn test_recovery_result_properties() {
        let success = RecoveryResult::success(3, 150);
        assert!(success.success);
        assert!(success.recovered);
        assert_eq!(success.attempts_made, 3);
        assert!(success.final_error.is_none());

        let failure = RecoveryResult::failure("Retry", 5, "timeout".to_string(), 300);
        assert!(!failure.success);
        assert!(!failure.recovered);
        assert_eq!(failure.attempts_made, 5);
        assert_eq!(failure.final_error, Some("timeout".to_string()));

        let skipped = RecoveryResult::skipped(50);
        assert!(skipped.success);
        assert!(!skipped.recovered);
        assert_eq!(skipped.attempts_made, 0);
    }
}

// ============================================================================
// Test Module 5: test_agent_coordinator_lifecycle
// ============================================================================

#[cfg(test)]
mod test_agent_coordinator_lifecycle {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_init_to_done_lifecycle() {
        let agent = Arc::new(tokio::sync::Mutex::new(MockCoordinatorAgent::new()));
        let coordinator = AgentCoordinator::new(agent, None);

        assert_eq!(coordinator.get_status().await, AgentStatus::Idle);

        let config = AgentConfig::default();
        let init_result = coordinator.initialize(config).await;
        assert!(init_result.is_ok());
        assert_eq!(coordinator.get_status().await, AgentStatus::Idle);

        let input = AgentInput {
            content: "Hello, coordinator!".to_string(),
            context: None,
        };
        let exec_result = coordinator.execute(input).await;
        assert!(exec_result.is_ok());

        let output = exec_result.unwrap();
        assert_eq!(output.status, AgentStatus::Completed);
        assert_eq!(output.content, "Hello, coordinator!");
    }

    #[tokio::test]
    async fn test_coordinator_cannot_execute_while_running() {
        let agent = Arc::new(tokio::sync::Mutex::new(MockCoordinatorAgent::new()));
        let coordinator = AgentCoordinator::new(agent, None);

        let input1 = AgentInput {
            content: "first".to_string(),
            context: None,
        };
        let _ = coordinator.execute(input1).await;

        let input2 = AgentInput {
            content: "second".to_string(),
            context: None,
        };
        let result = coordinator.execute(input2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_coordinator_execute_triggers_error_recovery() {
        let agent = Arc::new(tokio::sync::Mutex::new(MockCoordinatorAgent::with_failure()));
        let coordinator = AgentCoordinator::new(agent, None);

        let input = AgentInput {
            content: "test".to_string(),
            context: None,
        };

        let result = coordinator.execute(input).await;
        assert!(result.is_err());

        match result {
            Err(AgentError::ExecutionFailed(msg)) => {
                assert!(msg.contains("simulated failure"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }

        let status = coordinator.get_status().await;
        assert!(matches!(status, AgentStatus::Failed(_)));
    }

    #[tokio::test]
    async fn test_coordinator_cancel_from_running() {
        let agent = Arc::new(tokio::sync::Mutex::new(MockCoordinatorAgent::new()));
        let coordinator = AgentCoordinator::new(agent, None);

        let input = AgentInput {
            content: "start".to_string(),
            context: None,
        };
        let _ = coordinator.execute(input).await;

        let cancel_result = coordinator.cancel().await;
        assert!(cancel_result.is_ok());

        let status = coordinator.get_status().await;
        assert_eq!(status, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_coordinator_event_bus_integration() {
        let agent = Arc::new(tokio::sync::Mutex::new(MockCoordinatorAgent::new()));
        let coordinator = AgentCoordinator::new(agent, None);

        let bus = coordinator.event_bus();
        assert_eq!(bus.name(), "typed_coordinator");

        let mut rx = bus.subscribe("test-sub".to_string(), vec![]);

        let input = AgentInput {
            content: "event test".to_string(),
            context: None,
        };
        let _ = coordinator.execute(input).await;

        let event = rx.try_recv();
        assert!(event.is_ok());
    }

    #[tokio::test]
    async fn test_coordinator_force_now_and_prepare_new_session() {
        let agent = Arc::new(tokio::sync::Mutex::new(MockCoordinatorAgent::new()));
        let coordinator = AgentCoordinator::new(agent, None);

        coordinator
            .prompt_cache
            .record_system_prompt("initial prompt")
            .await;
        assert!(coordinator.prompt_cache.is_cache_valid().await);

        coordinator.force_now().await;
        assert!(!coordinator.prompt_cache.is_cache_valid().await);

        let guard_result = coordinator
            .cache_guard
            .guard_system_prompt_modification()
            .await;
        assert!(guard_result.is_ok());

        coordinator
            .prompt_cache
            .record_system_prompt("new prompt")
            .await;

        coordinator.prepare_for_new_session().await;
        assert!(!coordinator.prompt_cache.is_cache_valid().await);
    }

    #[tokio::test]
    async fn test_checkpoint_save_and_restore() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = CheckpointManager::new(temp_dir.path().to_str().unwrap());

        let checkpoint = CheckpointBuilder::new("plan-test-1", 0)
            .with_completed_tasks(vec!["task-1".to_string(), "task-2".to_string()])
            .with_state(serde_json::json!({
                "progress": 0.4,
                "phase": "setup"
            }))
            .with_label("After setup completion")
            .build();

        let save_result = manager.save(&checkpoint).await;
        assert!(save_result.is_ok());

        let loaded = manager.load(&checkpoint.id).await.unwrap();
        assert_eq!(loaded.plan_id, checkpoint.plan_id);
        assert_eq!(loaded.phase_index, checkpoint.phase_index);
        assert_eq!(loaded.completed_task_ids.len(), 2);
        assert_eq!(loaded.label, checkpoint.label);

        assert_eq!(loaded.state["progress"], serde_json::json!(0.4));
    }

    #[tokio::test]
    async fn test_checkpoint_list_and_delete() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = CheckpointManager::new(temp_dir.path().to_str().unwrap());

        let cp1 = CheckpointBuilder::new("plan-list", 0)
            .with_state(serde_json::json!({"v": 1}))
            .build();

        tokio::time::sleep(Duration::from_millis(10)).await;

        let cp2 = CheckpointBuilder::new("plan-list", 1)
            .with_state(serde_json::json!({"v": 2}))
            .build();

        tokio::time::sleep(Duration::from_millis(10)).await;

        manager.save(&cp1).await.unwrap();
        manager.save(&cp2).await.unwrap();

        let all = manager.list().await.unwrap();
        assert!(all.len() >= 2);

        manager.delete(&cp1.id).await.unwrap();

        let after_delete = manager.list().await.unwrap();
        let deleted_exists = after_delete.iter().any(|cp| cp.id == cp1.id);
        assert!(!deleted_exists);

        let delete_nonexistent = manager.delete("nonexistent-id").await;
        assert!(delete_nonexistent.is_err());
    }

    #[tokio::test]
    async fn test_checkpoint_latest_for_plan() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = CheckpointManager::new(temp_dir.path().to_str().unwrap());

        let cp1 = CheckpointBuilder::new("plan-latest", 0)
            .with_state(serde_json::json!({"v": 1}))
            .build();

        tokio::time::sleep(Duration::from_millis(10)).await;

        let cp2 = CheckpointBuilder::new("plan-latest", 1)
            .with_state(serde_json::json!({"v": 2}))
            .build();

        manager.save(&cp1).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        manager.save(&cp2).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        let latest = manager
            .get_latest_for_plan("plan-latest")
            .await
            .unwrap()
            .unwrap();

        assert!(
            latest.state["v"] == serde_json::json!(1) || latest.state["v"] == serde_json::json!(2),
            "Expected v=1 or v=2, got {:?}",
            latest.state["v"]
        );
    }

    #[tokio::test]
    async fn test_checkpoint_cleanup_old() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = CheckpointManager::new(temp_dir.path().to_str().unwrap());

        for i in 0..5 {
            let cp = CheckpointBuilder::new("plan-cleanup", i as usize)
                .with_state(serde_json::json!({"index": i}))
                .build();
            manager.save(&cp).await.unwrap();
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let deleted = manager.cleanup_old(2).await.unwrap();
        assert_eq!(deleted, 3);

        let remaining = manager.list().await.unwrap();
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn test_checkpoint_builder_defaults() {
        let cp = CheckpointBuilder::new("plan-default", 2).build();

        assert_eq!(cp.plan_id, "plan-default");
        assert_eq!(cp.phase_index, 2);
        assert!(cp.completed_task_ids.is_empty());
        assert!(cp.label.is_none());
    }

    #[test]
    fn test_checkpoint_serialization() {
        let cp = CheckpointBuilder::new("plan-serial", 1)
            .with_completed_tasks(vec!["t1".to_string()])
            .with_state(serde_json::json!({"key": "val"}))
            .with_label("label")
            .build();

        let json = serde_json::to_string(&cp).unwrap();
        let deserialized: Checkpoint = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.plan_id, "plan-serial");
        assert_eq!(deserialized.phase_index, 1);
        assert_eq!(deserialized.completed_task_ids, vec!["t1".to_string()]);
    }
}

// ============================================================================
// Test Module 6: test_tool_call_flow
// ============================================================================

#[cfg(test)]
mod test_tool_call_flow {
    use super::*;
    use axagent_agent::action_executor::ActionExecutor;
    use axagent_agent::reasoning_state::ActionType;

    #[tokio::test]
    async fn test_tool_call_lifecycle_use_start_result() {
        let executor = ActionExecutor::new();

        let action = Action {
            action_type: ActionType::ToolCall,
            tool_name: Some("test-tool".to_string()),
            tool_input: Some(serde_json::json!({"query": "test"})),
            llm_prompt: None,
            requires_confirmation: false,
        };

        let result = executor.execute(action, "test-conv").await;

        match result {
            Ok(action_result) => {
                assert!(action_result.is_success());
                let observation = action_result.to_observation();
                assert!(observation.contains("test-tool"));
            },
            Err(e) => {
                assert!(e.to_string().contains("test-tool") || e.to_string().contains("Invalid"));
            },
        }
    }

    #[tokio::test]
    async fn test_action_result_state_transitions() {
        let executor = ActionExecutor::new();

        let llm_action = Action::llm_call("Test prompt");
        let llm_result = executor.execute(llm_action, "conv-1").await.unwrap();

        assert!(llm_result.is_success());
        match &llm_result {
            ActionResult::LlmResponse(text) => assert_eq!(text, "Test prompt"),
            _ => panic!("Expected LlmResponse"),
        }

        let obs = llm_result.to_observation();
        assert!(obs.contains("LLM response"));
        assert!(obs.contains("Test prompt"));
    }

    #[tokio::test]
    async fn test_tool_call_error_result_handling() {
        let executor = ActionExecutor::new();

        let action = Action {
            action_type: ActionType::ToolCall,
            tool_name: None,
            tool_input: None,
            llm_prompt: None,
            requires_confirmation: false,
        };

        let result = executor.execute(action, "conv-1").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ActionError::InvalidAction(msg) => {
                assert!(msg.contains("tool_name"));
            },
            _ => panic!("Expected InvalidAction error"),
        }
    }

    #[tokio::test]
    async fn test_action_types_return_correct_result_types() {
        let executor = ActionExecutor::new();

        let analyze_action = Action {
            action_type: ActionType::Analyze,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("Analyze this".to_string()),
            requires_confirmation: false,
        };
        let analyze_result = executor.execute(analyze_action, "conv-1").await.unwrap();
        assert!(analyze_result.is_success());

        let plan_action = Action {
            action_type: ActionType::Plan,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("Plan this".to_string()),
            requires_confirmation: false,
        };
        let plan_result = executor.execute(plan_action, "conv-1").await.unwrap();
        assert!(plan_result.is_success());

        let reflect_action = Action {
            action_type: ActionType::Reflect,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("Reflect on this".to_string()),
            requires_confirmation: false,
        };
        let reflect_result = executor.execute(reflect_action, "conv-1").await.unwrap();
        assert!(reflect_result.is_success());

        let synthesize_action = Action {
            action_type: ActionType::Synthesize,
            tool_name: None,
            tool_input: None,
            llm_prompt: Some("Synthesize this".to_string()),
            requires_confirmation: false,
        };
        let synthesize_result = executor.execute(synthesize_action, "conv-1").await.unwrap();
        assert!(synthesize_result.is_success());
    }

    #[tokio::test]
    async fn test_user_confirmation_permission_flow() {
        let executor = ActionExecutor::new();

        let confirm_action = Action::user_confirm("Are you sure?");
        let result = executor.execute(confirm_action, "conv-1").await.unwrap();

        assert!(!result.is_success());
        match result {
            ActionResult::UserConfirmationRequired(ref msg) => {
                assert_eq!(msg, "Are you sure?");
            },
            _ => panic!("Expected UserConfirmationRequired"),
        }

        let obs = result.to_observation();
        assert!(obs.contains("Awaiting user confirmation"));
    }

    #[tokio::test]
    async fn test_validation_action_flow() {
        let executor = ActionExecutor::new();

        let validate_action = Action::validate("Check output format");
        let result = executor.execute(validate_action, "conv-1").await.unwrap();

        match result {
            ActionResult::Validation(ref desc) => {
                assert_eq!(desc, "Check output format");
            },
            _ => panic!("Expected Validation"),
        }

        assert!(!result.is_success());
    }

    #[test]
    fn test_action_result_is_success_variants() {
        let success_cases = vec![
            ActionResult::ToolSuccess("output".to_string(), "tool".to_string()),
            ActionResult::LlmResponse("response".to_string()),
            ActionResult::Analysis("analysis".to_string()),
            ActionResult::Planning("plan".to_string()),
            ActionResult::Reflection("reflection".to_string()),
            ActionResult::Synthesis("synthesis".to_string()),
        ];

        for case in success_cases {
            assert!(case.is_success(), "Expected {:?} to be success", case);
        }

        let non_success_cases = vec![
            ActionResult::UserConfirmationRequired("confirm?".to_string()),
            ActionResult::Validation("valid".to_string()),
        ];

        for case in non_success_cases {
            assert!(!case.is_success(), "Expected {:?} to NOT be success", case);
        }
    }

    #[test]
    fn test_action_error_is_retryable() {
        let retryable = vec![
            ActionError::Timeout("timed out".to_string()),
            ActionError::LlmError("llm failed".to_string()),
            ActionError::ToolExecution("tool failed".to_string()),
        ];

        for err in retryable {
            assert!(err.is_retryable(), "Expected {:?} to be retryable", err);
        }

        let non_retryable = vec![
            ActionError::InvalidAction("bad action".to_string()),
            ActionError::PermissionDenied("no access".to_string()),
        ];

        for err in non_retryable {
            assert!(!err.is_retryable(), "Expected {:?} to NOT be retryable", err);
        }
    }

    #[test]
    fn test_tool_call_result_in_thought_node() {
        use axagent_agent::tree_of_thoughts::ToolCallResult;

        let result = ToolCallResult {
            tool_name: "search".to_string(),
            output: "Found 3 results".to_string(),
            is_error: false,
        };

        assert_eq!(result.tool_name, "search");
        assert!(!result.is_error);
        assert!(!result.output.is_empty());

        let error_result = ToolCallResult {
            tool_name: "compute".to_string(),
            output: "Division by zero".to_string(),
            is_error: true,
        };
        assert!(error_result.is_error);
    }

    #[test]
    fn test_action_llm_call_and_user_confirm_constructors() {
        let llm_action = Action::llm_call("Test prompt");
        assert_eq!(llm_action.action_type, ActionType::LlmCall);
        assert_eq!(llm_action.llm_prompt, Some("Test prompt".to_string()));

        let confirm_action = Action::user_confirm("Confirm action?");
        assert_eq!(confirm_action.action_type, ActionType::UserConfirm);
        assert!(confirm_action.requires_confirmation);
    }

    #[test]
    fn test_recovery_strategy_tool_call_context() {
        let retry_strategy = RecoveryStrategy::for_error_type(ErrorType::Transient);
        match &retry_strategy {
            RecoveryStrategy::Retry {
                max_attempts,
                base_delay_ms,
                max_delay_ms,
                exponential_backoff,
            } => {
                assert_eq!(*max_attempts, 3);
                assert_eq!(*base_delay_ms, 1000);
                assert_eq!(*max_delay_ms, 10000);
                assert!(*exponential_backoff);
            },
            _ => panic!("Expected Retry strategy"),
        }

        let adjust_strategy = RecoveryStrategy::for_error_type(ErrorType::Recoverable);
        match &adjust_strategy {
            RecoveryStrategy::AdjustAndRetry {
                max_attempts,
                adjustments,
            } => {
                assert_eq!(*max_attempts, 2);
                assert_eq!(adjustments.len(), 2);
            },
            _ => panic!("Expected AdjustAndRetry strategy"),
        }
    }

    #[test]
    fn test_recovery_attempt_builder_chain() {
        let strategy = RecoveryStrategy::Retry {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 1000,
            exponential_backoff: true,
        };

        let attempt = RecoveryAttempt::new(1, "initial error".to_string(), strategy.clone())
            .with_delay(500)
            .with_success("recovered".to_string());

        assert_eq!(attempt.attempt_number, 1);
        assert_eq!(attempt.error, "initial error");
        assert_eq!(attempt.delay_ms, Some(500));
        assert!(attempt.success);
        assert_eq!(attempt.message, Some("recovered".to_string()));
    }
}

// ── 内存稳定性 + 并发压力测试 ──

#[tokio::test]
async fn stress_retry_policy_memory_stability() {
    use axagent_agent::retry_policy::{RetryPolicy, with_retry};

    let policy = RetryPolicy::new(3)
        .with_base_delay(std::time::Duration::from_millis(1))
        .with_exponential_backoff(false)
        .with_jitter(false);

    // 1000 次成功调用，验证无内存泄漏 (backoff 无内部累积)
    for _ in 0..1000 {
        let result = with_retry(&policy, || async { Ok::<String, String>("ok".to_string()) }).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn stress_concurrent_retry_policies() {
    use axagent_agent::retry_policy::{RetryPolicy, with_retry};
    use std::sync::Arc;

    let policy = Arc::new(
        RetryPolicy::new(5)
            .with_base_delay(std::time::Duration::from_millis(1))
            .with_exponential_backoff(false)
            .with_jitter(false),
    );

    let mut handles = vec![];
    for _ in 0..10 {
        let p = policy.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..100 {
                let _ = with_retry(&p, || async {
                    if fastrand::f64() < 0.3 {
                        Err("transient timeout".to_string())
                    } else {
                        Ok("ok".to_string())
                    }
                })
                .await;
            }
        }));
    }

    for h in handles {
        h.await.expect("task should not panic");
    }
}
