// SPDX-License-Identifier: AGPL-3.0-only

//! Extended lifecycle tests for AgentCoordinator: cancel, pause/resume, state transitions.

use async_trait::async_trait;
use axagent_agent::coordinator::{
    AgentConfig, AgentCoordinator, AgentError, AgentImpl, AgentInput, AgentStatus,
    CoordinatorOutput,
};
use axagent_harness::cache_service::CacheService;
use axagent_harness::hook_service::HookService;
use std::sync::Arc;
use tokio::sync::Mutex;

struct NoopCacheService;
#[async_trait::async_trait]
impl CacheService for NoopCacheService {
    async fn is_cache_valid(&self) -> bool {
        false
    }
    async fn has_pending_changes(&self) -> bool {
        false
    }
    async fn invalidate(&self, _reason: &str) {}
    async fn invalidate_for_new_session(&self) {}
    async fn set_force_immediate(&self, _force: bool) {}
}

struct NoopHookService;
#[async_trait::async_trait]
impl HookService for NoopHookService {
    async fn register(&self, _hook: axagent_harness::plugin_hook::SharedHook) {}
    async fn unregister(&self, _name: &str) {}
    async fn list(&self) -> Vec<String> {
        vec![]
    }
    async fn execute_pre_tool_call(
        &self,
        _ctx: &axagent_harness::plugin_hook::ToolCallContext,
    ) -> Option<axagent_harness::plugin_hook::HookDecision> {
        None
    }
    async fn execute_post_tool_call(
        &self,
        _ctx: &axagent_harness::plugin_hook::ToolCallContext,
        _result: &axagent_harness::plugin_hook::ToolCallResult,
    ) {
    }
}

// ---------------------------------------------------------------------------
// Mock agent for lifecycle testing
// ---------------------------------------------------------------------------

struct LifecycleAgent {
    status: AgentStatus,
    delay_ms: u64,
}

impl LifecycleAgent {
    fn new(delay_ms: u64) -> Self {
        Self { status: AgentStatus::Idle, delay_ms }
    }
}

#[async_trait]
impl AgentImpl for LifecycleAgent {
    async fn initialize(&mut self, _config: AgentConfig) -> Result<(), AgentError> {
        self.status = AgentStatus::Initializing;
        Ok(())
    }

    async fn execute(&mut self, _input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        self.status = AgentStatus::Running;

        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        self.status = AgentStatus::Completed;
        Ok(CoordinatorOutput::success("done".to_string(), 1))
    }

    async fn pause(&mut self) -> Result<(), AgentError> {
        self.status = AgentStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        self.status = AgentStatus::Running;
        Ok(())
    }

    async fn cancel(&mut self) -> Result<(), AgentError> {
        self.status = AgentStatus::Failed("Cancelled".to_string());
        Ok(())
    }

    fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    fn agent_type(&self) -> &'static str {
        "lifecycle-test"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_coordinator_execute_simple() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(0)));
    let coordinator =
        AgentCoordinator::new(agent, None, Arc::new(NoopCacheService), Arc::new(NoopHookService));

    coordinator.initialize(AgentConfig::default()).await.expect("测试应成功");

    let result =
        coordinator.execute(AgentInput { content: "test".to_string(), context: None }).await;

    assert!(result.is_ok(), "Execution should succeed");
    let output = result.expect("测试应成功");
    assert_eq!(output.content, "done");
}

#[tokio::test]
async fn test_coordinator_cannot_execute_twice() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(300)));
    let coordinator =
        AgentCoordinator::new(agent, None, Arc::new(NoopCacheService), Arc::new(NoopHookService));

    coordinator.initialize(AgentConfig::default()).await.expect("测试应成功");

    // First execution
    let r1 = coordinator.execute(AgentInput { content: "first".to_string(), context: None }).await;
    assert!(r1.is_ok(), "First execution should succeed");

    // Second execution without prepare_for_new_session should fail
    let r2 = coordinator.execute(AgentInput { content: "second".to_string(), context: None }).await;
    assert!(r2.is_err(), "Second execution should fail");
    match r2.unwrap_err() {
        AgentError::AlreadyRunning | AgentError::InvalidState(_) => { /* expected */ },
        e => panic!("Unexpected error: {:?}", e),
    }
}

#[tokio::test]
async fn test_coordinator_initialize_starts_idle() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(0)));
    let coordinator =
        AgentCoordinator::new(agent, None, Arc::new(NoopCacheService), Arc::new(NoopHookService));

    // Before init, status is Idle
    let status = coordinator.get_status().await;
    assert!(matches!(status, AgentStatus::Idle), "Expected Idle before init, got {:?}", status);
}

#[tokio::test]
async fn test_coordinator_prepare_for_new_session() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(50)));
    let coordinator =
        AgentCoordinator::new(agent, None, Arc::new(NoopCacheService), Arc::new(NoopHookService));

    coordinator.initialize(AgentConfig::default()).await.expect("测试应成功");

    coordinator
        .execute(AgentInput { content: "test".to_string(), context: None })
        .await
        .expect("测试应成功");

    // Should be able to prepare for a new session
    coordinator.prepare_for_new_session().await;

    // After prepare, get_status should work (status may vary but no panic)
    let _status = coordinator.get_status().await;
}
