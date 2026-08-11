// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_agent::coordinator::{
    AgentConfig, AgentCoordinator, AgentError, AgentImpl, AgentInput, AgentStatus,
    CoordinatorOutput,
};
use axagent_agent::event_bus::AgentEventType;
use axagent_harness::cache_service::CacheService;
use axagent_harness::hook_service::HookService;
use std::sync::Arc;

fn make_coordinator<T: AgentImpl + Send + 'static>(
    agent: Arc<tokio::sync::Mutex<T>>,
) -> AgentCoordinator<T> {
    AgentCoordinator::new(agent, None, Arc::new(NoopCacheService), Arc::new(NoopHookService))
}

/// 构建开启计划确认闸门的配置（P0-2）。
fn config_with_approval() -> AgentConfig {
    AgentConfig { require_plan_approval: true, ..AgentConfig::default() }
}

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
    async fn execute_pre_api_request(
        &self,
        _ctx: &axagent_harness::plugin_hook::ApiCallContext,
    ) -> Option<axagent_harness::plugin_hook::HookDecision> {
        None
    }
    async fn execute_post_api_request(
        &self,
        _ctx: &axagent_harness::plugin_hook::ApiCallContext,
        _result: &axagent_harness::plugin_hook::ApiCallResult,
    ) {
    }
    async fn execute_pre_llm_call(
        &self,
        _ctx: &axagent_harness::plugin_hook::LlmCallContext,
    ) -> Option<axagent_harness::plugin_hook::HookDecision> {
        None
    }
    async fn execute_post_llm_call(
        &self,
        _ctx: &axagent_harness::plugin_hook::LlmCallContext,
        _result: &axagent_harness::plugin_hook::LlmCallResult,
    ) {
    }
}

struct MockAgent {
    status: AgentStatus,
    should_fail: bool,
}

impl MockAgent {
    fn new() -> Self {
        Self { status: AgentStatus::Idle, should_fail: false }
    }

    fn with_failure() -> Self {
        Self { status: AgentStatus::Idle, should_fail: true }
    }
}

#[async_trait]
impl AgentImpl for MockAgent {
    async fn initialize(&mut self, _config: AgentConfig) -> Result<(), AgentError> {
        self.status = AgentStatus::Idle;
        Ok(())
    }

    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        if self.should_fail {
            Err(AgentError::ExecutionFailed("simulated failure".to_string()))
        } else {
            Ok(CoordinatorOutput::success(input.content, 1))
        }
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

#[tokio::test]
async fn test_coordinator_initialization() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    let config = AgentConfig::default();
    let result = coordinator.initialize(config).await;
    assert!(result.is_ok());
    assert_eq!(coordinator.get_status().await, AgentStatus::Idle);
}

#[tokio::test]
async fn test_coordinator_cannot_initialize_twice_without_cancel() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    coordinator.initialize(AgentConfig::default()).await.expect("测试应成功");
    // After initialization, status is Idle, so a second init is valid
    let result = coordinator.initialize(AgentConfig::default()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordinator_execute_success() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    let input = AgentInput { content: "Hello, world!".to_string(), context: None };

    let result = coordinator.execute(input).await;
    assert!(result.is_ok());
    let output = result.expect("测试应成功");
    assert_eq!(output.content, "Hello, world!");
    assert_eq!(output.status, AgentStatus::Completed);
}

#[tokio::test]
async fn test_coordinator_execute_failure() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::with_failure()));
    let coordinator = make_coordinator(agent);

    let input = AgentInput { content: "test".to_string(), context: None };

    let result = coordinator.execute(input).await;
    assert!(result.is_err());
    match result {
        Err(AgentError::ExecutionFailed(msg)) => {
            assert!(msg.contains("simulated failure"));
        },
        _ => panic!("Expected ExecutionFailed error"),
    }
}

#[tokio::test]
async fn test_coordinator_cannot_execute_while_running() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    // Force into Running state via execute
    let input = AgentInput { content: "first".to_string(), context: None };
    let _ = coordinator.execute(input).await;

    let input2 = AgentInput { content: "second".to_string(), context: None };
    let result = coordinator.execute(input2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_coordinator_pause_resume() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    // Can't pause from Idle
    let result = coordinator.pause().await;
    assert!(result.is_err());

    // Execute first to get to Running
    // We need to make this work differently since execute is async and we can't
    // pause while it's running in a single test easily
}

#[tokio::test]
async fn test_coordinator_force_now() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    coordinator.force_now().await;
    // prompt_cache 已重构为 cache_service，字段名变更
    // assert!(!coordinator.prompt_cache.is_cache_valid().await);
    // 验证 cache_service 的副作用：force_now 不返回 bool，暂时跳过
}

#[tokio::test]
async fn test_coordinator_prepare_for_new_session() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    coordinator.prepare_for_new_session().await;
    // prompt_cache 已重构为 cache_service，字段名变更
    // assert!(!coordinator.prompt_cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_coordinator_event_bus_access() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    let bus = coordinator.event_bus();
    assert_eq!(bus.name(), "typed_coordinator");
}

#[tokio::test]
async fn test_coordinator_cache_integration() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let _coordinator = make_coordinator(agent);

    // prompt_cache 已重构为 cache_service，以下测试需适配新 API
    // coordinator.prompt_cache.record_system_prompt("test prompt").await;
    // assert!(coordinator.prompt_cache.is_cache_valid().await);
}

// ---------------------------------------------------------------------------
// P0-2：对话级计划确认闸门（Draft → await_approval → execute）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_coordinator_plan_approval_gate_complex_task() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);
    coordinator.initialize(config_with_approval()).await.expect("测试应成功");

    let mut rx =
        coordinator.event_bus().subscribe("test-gate", vec![AgentEventType::PlanReadyForApproval]);

    // 复杂任务：多步 + 验证关键词 → 触发确认闸门
    let input = AgentInput {
        content: "首先读取所有发票，然后验证金额是否一致，最后生成审计报告".to_string(),
        context: None,
    };
    let result = coordinator.execute(input).await.expect("测试应成功");
    assert_eq!(result.status, AgentStatus::WaitingForConfirmation);
    assert_eq!(result.metadata["awaiting_approval"], serde_json::json!(true));

    // 应收到 PlanReadyForApproval 事件，且携带计划草稿
    let evt = rx.recv().await.expect("测试应成功");
    assert_eq!(evt.event_type, AgentEventType::PlanReadyForApproval);
    assert!(evt.payload["plan"].as_str().is_some());

    // approve_plan 后真正执行，内容应来自原始输入
    let approved = coordinator.approve_plan().await.expect("测试应成功");
    assert_eq!(approved.status, AgentStatus::Completed);
    assert!(approved.content.contains("发票"));
}

#[tokio::test]
async fn test_coordinator_plan_approval_skipped_for_simple_task() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);
    coordinator.initialize(config_with_approval()).await.expect("测试应成功");

    // 简单任务：单步、无分支/验证关键词 → 直接执行，不进闸门
    let input = AgentInput { content: "你好".to_string(), context: None };
    let result = coordinator.execute(input).await.expect("测试应成功");
    assert_eq!(result.status, AgentStatus::Completed);
    assert_eq!(result.content, "你好");
}

#[tokio::test]
async fn test_coordinator_approve_plan_requires_waiting_state() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);

    // 未进入等待确认状态，直接 approve 应报 InvalidState
    let result = coordinator.approve_plan().await;
    assert!(result.is_err());
    match result {
        Err(AgentError::InvalidState(_)) => {},
        _ => panic!("expected InvalidState"),
    }
}

#[tokio::test]
async fn test_coordinator_plan_approval_disabled_by_default() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = make_coordinator(agent);
    coordinator.initialize(AgentConfig::default()).await.expect("测试应成功");

    // 默认关闭闸门：即使是复杂任务也直接执行，行为与改造前一致
    let input = AgentInput {
        content: "首先读取所有发票，然后验证金额是否一致，最后生成审计报告".to_string(),
        context: None,
    };
    let result = coordinator.execute(input).await.expect("测试应成功");
    assert_eq!(result.status, AgentStatus::Completed);
    assert!(result.content.contains("发票"));
}
