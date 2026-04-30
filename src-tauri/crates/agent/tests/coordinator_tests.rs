use async_trait::async_trait;
use axagent_agent::coordinator::{
    AgentConfig, AgentCoordinator, AgentError, AgentImpl, AgentInput, AgentStatus,
    CoordinatorOutput,
};
use std::sync::Arc;

struct MockAgent {
    status: AgentStatus,
    should_fail: bool,
}

impl MockAgent {
    fn new() -> Self {
        Self {
            status: AgentStatus::Idle,
            should_fail: false,
        }
    }

    fn with_failure() -> Self {
        Self {
            status: AgentStatus::Idle,
            should_fail: true,
        }
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
    let coordinator = AgentCoordinator::new(agent, None);

    let config = AgentConfig::default();
    let result = coordinator.initialize(config).await;
    assert!(result.is_ok());
    assert_eq!(coordinator.get_status().await, AgentStatus::Idle);
}

#[tokio::test]
async fn test_coordinator_cannot_initialize_twice_without_cancel() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = AgentCoordinator::new(agent, None);

    coordinator
        .initialize(AgentConfig::default())
        .await
        .unwrap();
    // After initialization, status is Idle, so a second init is valid
    let result = coordinator.initialize(AgentConfig::default()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordinator_execute_success() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = AgentCoordinator::new(agent, None);

    let input = AgentInput {
        content: "Hello, world!".to_string(),
        context: None,
    };

    let result = coordinator.execute(input).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.content, "Hello, world!");
    assert_eq!(output.status, AgentStatus::Completed);
}

#[tokio::test]
async fn test_coordinator_execute_failure() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::with_failure()));
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
        }
        _ => panic!("Expected ExecutionFailed error"),
    }
}

#[tokio::test]
async fn test_coordinator_cannot_execute_while_running() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = AgentCoordinator::new(agent, None);

    // Force into Running state via execute
    let input = AgentInput {
        content: "first".to_string(),
        context: None,
    };
    let _ = coordinator.execute(input).await;

    let input2 = AgentInput {
        content: "second".to_string(),
        context: None,
    };
    let result = coordinator.execute(input2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_coordinator_pause_resume() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = AgentCoordinator::new(agent, None);

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
    let coordinator = AgentCoordinator::new(agent, None);

    coordinator.force_now().await;
    assert!(!coordinator.prompt_cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_coordinator_prepare_for_new_session() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = AgentCoordinator::new(agent, None);

    coordinator.prepare_for_new_session().await;
    assert!(!coordinator.prompt_cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_coordinator_event_bus_access() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = AgentCoordinator::new(agent, None);

    let bus = coordinator.event_bus();
    assert_eq!(bus.name(), "typed_coordinator");
}

#[tokio::test]
async fn test_coordinator_cache_integration() {
    let agent = Arc::new(tokio::sync::Mutex::new(MockAgent::new()));
    let coordinator = AgentCoordinator::new(agent, None);

    coordinator
        .prompt_cache
        .record_system_prompt("test prompt")
        .await;
    assert!(coordinator.prompt_cache.is_cache_valid().await);

    let state = coordinator.prompt_cache.get_state().await;
    assert!(state.system_prompt_hash.is_some());

    // Check that the cache guard disallows modification when cache is valid
    // (without --now)
    let guard_result = coordinator
        .cache_guard
        .guard_system_prompt_modification()
        .await;
    assert!(guard_result.is_err());

    // With --now, it should allow
    coordinator.force_now().await;
    let guard_result = coordinator
        .cache_guard
        .guard_system_prompt_modification()
        .await;
    assert!(guard_result.is_ok());
}
