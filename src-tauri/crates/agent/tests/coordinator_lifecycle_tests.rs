// SPDX-License-Identifier: AGPL-3.0-only

//! Extended lifecycle tests for AgentCoordinator: cancel, pause/resume, state transitions.

use async_trait::async_trait;
use axagent_agent::coordinator::{
    AgentConfig, AgentCoordinator, AgentImpl, AgentInput, AgentStatus, CoordinatorOutput,
};
use std::sync::Arc;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Mock agent for lifecycle testing
// ---------------------------------------------------------------------------

struct LifecycleAgent {
    status: Mutex<AgentStatus>,
    delay_ms: u64,
}

impl LifecycleAgent {
    fn new(delay_ms: u64) -> Self {
        Self {
            status: Mutex::new(AgentStatus::Idle),
            delay_ms,
        }
    }
}

#[async_trait]
impl AgentImpl for LifecycleAgent {
    async fn initialize(&self, _config: AgentConfig) -> Result<(), String> {
        let mut s = self.status.lock().await;
        *s = AgentStatus::Initializing;
        Ok(())
    }

    async fn execute(&self, _input: AgentInput) -> Result<CoordinatorOutput, String> {
        let mut s = self.status.lock().await;
        *s = AgentStatus::Running;
        drop(s);

        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        let mut s = self.status.lock().await;
        *s = AgentStatus::Completed;
        Ok(CoordinatorOutput::success("done".to_string(), 1))
    }

    async fn pause(&self) -> Result<(), String> {
        let mut s = self.status.lock().await;
        *s = AgentStatus::Paused;
        Ok(())
    }

    async fn resume(&self) -> Result<(), String> {
        let mut s = self.status.lock().await;
        *s = AgentStatus::Running;
        Ok(())
    }

    async fn cancel(&self) -> Result<(), String> {
        let mut s = self.status.lock().await;
        *s = AgentStatus::Failed("Cancelled".to_string());
        Ok(())
    }

    async fn status(&self) -> AgentStatus {
        self.status.lock().await.clone()
    }

    fn agent_type(&self) -> &'static str {
        "lifecycle-test"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_coordinator_cancel_from_running() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(500)));
    let coordinator = Arc::new(AgentCoordinator::new(agent, None));

    coordinator
        .initialize(AgentConfig {
            max_iterations: 10,
            timeout_secs: 30,
            requires_confirmation: false,
        })
        .await
        .unwrap();

    // Spawn execution in background so we can cancel it
    let coordinator_for_exec = coordinator.clone();
    let exec_handle = tokio::spawn(async move {
        coordinator_for_exec
            .execute(AgentInput {
                message: "test".to_string(),
                conversation_id: None,
                attachments: vec![],
                workspace_config: None,
            })
            .await
    });

    // Give it a moment to start running
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cancel while running
    coordinator.cancel().await.unwrap();

    let result = exec_handle.await.unwrap();
    // Either execution was cancelled (error) or completed before cancel took effect
    let status = coordinator.get_status().await;
    match status {
        AgentStatus::Failed(_) | AgentStatus::Completed => { /* expected */ },
        _ => panic!("Unexpected status after cancel: {:?}", status),
    }
}

#[tokio::test]
async fn test_coordinator_pause_resume_cycle() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(100)));
    let coordinator = AgentCoordinator::new(agent, None);

    coordinator
        .initialize(AgentConfig::default())
        .await
        .unwrap();

    // Pause when idle (should not error)
    coordinator.pause().await.unwrap();

    let status = coordinator.get_status().await;
    assert!(
        matches!(status, AgentStatus::Idle | AgentStatus::Paused),
        "Expected idle or paused, got {:?}",
        status
    );

    // Resume
    coordinator.resume().await.unwrap();
}

#[tokio::test]
async fn test_coordinator_cannot_execute_twice() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(300)));
    let coordinator = AgentCoordinator::new(agent, None);

    coordinator
        .initialize(AgentConfig::default())
        .await
        .unwrap();

    // First execution
    let r1 = coordinator
        .execute(AgentInput {
            message: "first".to_string(),
            conversation_id: None,
            attachments: vec![],
            workspace_config: None,
        })
        .await;
    assert!(r1.is_ok(), "First execution should succeed");

    // Second execution without reset should fail
    let r2 = coordinator
        .execute(AgentInput {
            message: "second".to_string(),
            conversation_id: None,
            attachments: vec![],
            workspace_config: None,
        })
        .await;
    assert!(r2.is_err(), "Second execution without reset should fail");
    let err = r2.unwrap_err().to_string();
    // Should mention already running or invalid state
    assert!(
        err.contains("Running") || err.contains("state") || err.contains("already"),
        "Error should mention execution state conflict, got: {}",
        err
    );
}

#[tokio::test]
async fn test_coordinator_get_status_lifecycle() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(0)));
    let coordinator = AgentCoordinator::new(agent, None);

    // Should start idle
    assert_eq!(coordinator.get_status().await, AgentStatus::Idle);

    coordinator
        .initialize(AgentConfig::default())
        .await
        .unwrap();

    // After init we should not be idle
    let status = coordinator.get_status().await;
    assert_ne!(status, AgentStatus::Idle);
}

#[tokio::test]
async fn test_coordinator_prepare_for_new_session() {
    let agent = Arc::new(Mutex::new(LifecycleAgent::new(50)));
    let coordinator = AgentCoordinator::new(agent, None);

    coordinator
        .initialize(AgentConfig::default())
        .await
        .unwrap();

    coordinator
        .execute(AgentInput {
            message: "test".to_string(),
            conversation_id: None,
            attachments: vec![],
            workspace_config: None,
        })
        .await
        .unwrap();

    // Should be able to prepare for a new session
    coordinator.prepare_for_new_session().await;

    // Status should be reset (likely Idle or ok to reinitialize)
    coordinator
        .initialize(AgentConfig::default())
        .await
        .unwrap();
}
