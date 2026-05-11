use axagent_runtime_core::{
    ApiClient, ConversationRuntime, PermissionMode, PermissionPolicy, Session, ToolExecutor,
};
use tokio::sync::broadcast;

use crate::proactive_mode::ProactiveMode;

#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub response: String,
    pub iterations: usize,
    pub tool_call_count: usize,
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TurnStarted {
        iteration: usize,
    },
    TurnCompleted {
        iteration: usize,
    },
    ToolUse {
        tool_name: String,
        tool_use_id: String,
    },
    ToolResult {
        tool_use_id: String,
        is_error: bool,
    },
    Error {
        error: String,
    },
    /// 主动模式 tick 已注入
    ProactiveTick,
}

pub struct AgentRuntimeConfig {
    pub role: String,
    pub system_prompt: String,
    pub max_iterations: usize,
    pub timeout_secs: u64,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            role: "executor".to_string(),
            system_prompt: String::new(),
            max_iterations: 50,
            timeout_secs: 300,
        }
    }
}

pub struct AgentRuntime<C, T>
where
    C: ApiClient + Send,
    T: ToolExecutor + Send,
{
    session: Session,
    conversation_runtime: ConversationRuntime<C, T>,
    #[allow(dead_code)]
    config: AgentRuntimeConfig,
    event_sender: broadcast::Sender<AgentEvent>,
    /// 主动模式（可选）
    proactive: Option<ProactiveMode>,
}

impl<C, T> AgentRuntime<C, T>
where
    C: ApiClient + Send,
    T: ToolExecutor + Send,
{
    pub fn new(
        config: AgentRuntimeConfig,
        session: Session,
        api_client: C,
        tool_executor: T,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(100);

        let permission_policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite);

        // Fork session 重建：检查是否有父 agent 的 fork 上下文
        let mut forked_session = session.clone();
        let system_prompts = if let Some(fork_data) =
            axagent_runtime_core::fork_bridge::take_fork_session(&forked_session.session_id)
        {
            let mut sp = fork_data.parent_system_prompt;
            if let Some(child_sp) = fork_data.child_system_prompt {
                sp.push(child_sp);
            }
            if !fork_data.parent_messages_json.is_empty() {
                if let Ok(parent_msgs) = serde_json::from_str::<
                    Vec<axagent_runtime_core::ConversationMessage>,
                >(&fork_data.parent_messages_json)
                {
                    forked_session.messages = parent_msgs;
                }
            }
            sp
        } else if config.system_prompt.is_empty() {
            Vec::new()
        } else {
            vec![config.system_prompt.clone()]
        };

        let conversation_runtime = ConversationRuntime::new(
            forked_session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompts,
        )
        .with_max_iterations(config.max_iterations);

        // 根据 feature flag 启用主动模式
        let proactive = if ProactiveMode::is_enabled() {
            let mut pm = ProactiveMode::new();
            pm.activate();
            Some(pm)
        } else {
            None
        };

        Self {
            session,
            conversation_runtime,
            config,
            event_sender,
            proactive,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.event_sender.subscribe()
    }

    pub fn session_id(&self) -> &str {
        &self.session.session_id
    }

    /// 用户输入事件 — 暂停主动模式
    pub fn on_user_input(&mut self) {
        if let Some(ref mut proactive) = self.proactive {
            proactive.on_user_input();
        }
    }

    /// 获取主动模式引用
    pub fn proactive(&self) -> Option<&ProactiveMode> {
        self.proactive.as_ref()
    }

    /// 获取主动模式可变引用
    pub fn proactive_mut(&mut self) -> Option<&mut ProactiveMode> {
        self.proactive.as_mut()
    }

    pub fn run(&mut self, input: &str) -> Result<AgentOutput, AgentRuntimeError> {
        self.emit(AgentEvent::TurnStarted { iteration: 0 });

        // 主动模式：检查是否应该注入 tick
        let effective_input = if let Some(ref mut proactive) = self.proactive {
            if proactive.should_tick() {
                proactive.record_tick();
                let tick = proactive.build_tick_prompt();
                self.emit(AgentEvent::ProactiveTick);
                format!("{}\n{}", tick, input)
            } else {
                input.to_string()
            }
        } else {
            input.to_string()
        };

        let result = self.conversation_runtime.run_turn(&effective_input, None);

        // 恢复主动模式（如果之前因用户输入暂停）
        if let Some(ref mut proactive) = self.proactive {
            proactive.resume();
        }

        match result {
            Ok(summary) => {
                self.emit(AgentEvent::TurnCompleted {
                    iteration: summary.iterations,
                });

                let response = summary
                    .assistant_messages
                    .last()
                    .and_then(|msg| {
                        msg.blocks.iter().find_map(|block| {
                            if let axagent_runtime_core::ContentBlock::Text { text } = block {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_default();

                let tool_call_count = summary.tool_results.len();

                Ok(AgentOutput {
                    response,
                    iterations: summary.iterations,
                    tool_call_count,
                })
            },
            Err(e) => {
                // API 错误时暂停主动模式
                if let Some(ref mut proactive) = self.proactive {
                    proactive.on_api_error();
                }
                self.emit(AgentEvent::Error {
                    error: e.to_string(),
                });
                Err(AgentRuntimeError::RuntimeError(e.to_string()))
            },
        }
    }

    fn emit(&self, event: AgentEvent) {
        let _ = self.event_sender.send(event);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentRuntimeError {
    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Tool execution error: {0}")]
    ToolError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_output_creation() {
        let output = AgentOutput {
            response: "test response".to_string(),
            iterations: 5,
            tool_call_count: 3,
        };
        assert_eq!(output.response, "test response");
        assert_eq!(output.iterations, 5);
        assert_eq!(output.tool_call_count, 3);
    }

    #[test]
    fn test_agent_output_zero_values() {
        let output = AgentOutput {
            response: String::new(),
            iterations: 0,
            tool_call_count: 0,
        };
        assert!(output.response.is_empty());
        assert_eq!(output.iterations, 0);
        assert_eq!(output.tool_call_count, 0);
    }

    #[test]
    fn test_agent_runtime_config_default() {
        let config = AgentRuntimeConfig::default();
        assert_eq!(config.role, "executor");
        assert!(config.system_prompt.is_empty());
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_agent_runtime_config_custom() {
        let config = AgentRuntimeConfig {
            role: "planner".to_string(),
            system_prompt: "You are a planner".to_string(),
            max_iterations: 100,
            timeout_secs: 600,
        };
        assert_eq!(config.role, "planner");
        assert_eq!(config.system_prompt, "You are a planner");
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.timeout_secs, 600);
    }

    #[test]
    fn test_agent_event_turn_started() {
        let event = AgentEvent::TurnStarted { iteration: 0 };
        assert!(matches!(event, AgentEvent::TurnStarted { iteration: 0 }));
    }

    #[test]
    fn test_agent_event_turn_completed() {
        let event = AgentEvent::TurnCompleted { iteration: 3 };
        assert!(matches!(event, AgentEvent::TurnCompleted { iteration: 3 }));
    }

    #[test]
    fn test_agent_event_tool_use() {
        let event = AgentEvent::ToolUse {
            tool_name: "read_file".to_string(),
            tool_use_id: "id-1".to_string(),
        };
        match &event {
            AgentEvent::ToolUse {
                tool_name,
                tool_use_id,
            } => {
                assert_eq!(tool_name, "read_file");
                assert_eq!(tool_use_id, "id-1");
            },
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_agent_event_tool_result() {
        let event = AgentEvent::ToolResult {
            tool_use_id: "id-1".to_string(),
            is_error: false,
        };
        match &event {
            AgentEvent::ToolResult {
                tool_use_id,
                is_error,
            } => {
                assert_eq!(tool_use_id, "id-1");
                assert!(!is_error);
            },
            _ => panic!("Expected ToolResult"),
        }
    }

    #[test]
    fn test_agent_event_tool_result_error() {
        let event = AgentEvent::ToolResult {
            tool_use_id: "id-2".to_string(),
            is_error: true,
        };
        match &event {
            AgentEvent::ToolResult { is_error, .. } => {
                assert!(is_error);
            },
            _ => panic!("Expected ToolResult"),
        }
    }

    #[test]
    fn test_agent_event_error() {
        let event = AgentEvent::Error {
            error: "something failed".to_string(),
        };
        match &event {
            AgentEvent::Error { error } => {
                assert_eq!(error, "something failed");
            },
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_agent_event_proactive_tick() {
        let event = AgentEvent::ProactiveTick;
        assert!(matches!(event, AgentEvent::ProactiveTick));
    }

    #[test]
    fn test_agent_runtime_error_runtime() {
        let err = AgentRuntimeError::RuntimeError("runtime error".to_string());
        assert!(err.to_string().contains("runtime error"));
    }

    #[test]
    fn test_agent_runtime_error_session() {
        let err = AgentRuntimeError::SessionError("session error".to_string());
        assert!(err.to_string().contains("session error"));
    }

    #[test]
    fn test_agent_runtime_error_tool() {
        let err = AgentRuntimeError::ToolError("tool error".to_string());
        assert!(err.to_string().contains("tool error"));
    }

    #[test]
    fn test_agent_output_clone() {
        let output = AgentOutput {
            response: "clone me".to_string(),
            iterations: 2,
            tool_call_count: 1,
        };
        let cloned = output.clone();
        assert_eq!(cloned.response, "clone me");
        assert_eq!(cloned.iterations, 2);
        assert_eq!(cloned.tool_call_count, 1);
    }

    #[test]
    fn test_agent_output_debug() {
        let output = AgentOutput {
            response: "debug".to_string(),
            iterations: 1,
            tool_call_count: 0,
        };
        let debug_str = format!("{:?}", output);
        assert!(debug_str.contains("debug"));
    }

    #[test]
    fn test_agent_event_debug() {
        let event = AgentEvent::TurnStarted { iteration: 0 };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("TurnStarted"));
    }

    #[test]
    fn test_agent_runtime_error_debug() {
        let err = AgentRuntimeError::RuntimeError("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("RuntimeError"));
    }

    #[test]
    fn test_agent_runtime_config_default_role() {
        let config = AgentRuntimeConfig::default();
        assert_eq!(config.role, "executor");
    }

    #[test]
    fn test_agent_runtime_config_default_max_iterations() {
        let config = AgentRuntimeConfig::default();
        assert_eq!(config.max_iterations, 50);
    }

    #[test]
    fn test_agent_runtime_config_default_timeout() {
        let config = AgentRuntimeConfig::default();
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_agent_runtime_config_default_empty_system_prompt() {
        let config = AgentRuntimeConfig::default();
        assert!(config.system_prompt.is_empty());
    }

    #[test]
    fn test_agent_output_default_response() {
        let output = AgentOutput {
            response: String::new(),
            iterations: 0,
            tool_call_count: 0,
        };
        assert!(output.response.is_empty());
        assert_eq!(output.iterations, 0);
        assert_eq!(output.tool_call_count, 0);
    }

    #[test]
    fn test_agent_event_tool_use_fields() {
        let event = AgentEvent::ToolUse {
            tool_name: "write_file".to_string(),
            tool_use_id: "tool-123".to_string(),
        };
        if let AgentEvent::ToolUse {
            tool_name,
            tool_use_id,
        } = event
        {
            assert_eq!(tool_name, "write_file");
            assert_eq!(tool_use_id, "tool-123");
        }
    }

    #[test]
    fn test_agent_event_tool_result_fields() {
        let event = AgentEvent::ToolResult {
            tool_use_id: "tool-456".to_string(),
            is_error: true,
        };
        if let AgentEvent::ToolResult {
            tool_use_id,
            is_error,
        } = event
        {
            assert_eq!(tool_use_id, "tool-456");
            assert!(is_error);
        }
    }

    #[test]
    fn test_agent_event_error_field() {
        let event = AgentEvent::Error {
            error: "API timeout".to_string(),
        };
        if let AgentEvent::Error { error } = event {
            assert_eq!(error, "API timeout");
        }
    }

    #[test]
    fn test_agent_event_turn_started_iteration() {
        let event = AgentEvent::TurnStarted { iteration: 5 };
        if let AgentEvent::TurnStarted { iteration } = event {
            assert_eq!(iteration, 5);
        }
    }

    #[test]
    fn test_agent_event_turn_completed_iteration() {
        let event = AgentEvent::TurnCompleted { iteration: 10 };
        if let AgentEvent::TurnCompleted { iteration } = event {
            assert_eq!(iteration, 10);
        }
    }

    #[test]
    fn test_agent_runtime_error_session_message() {
        let err = AgentRuntimeError::SessionError("session expired".to_string());
        let msg = err.to_string();
        assert!(msg.contains("session expired"));
    }

    #[test]
    fn test_agent_runtime_error_tool_message() {
        let err = AgentRuntimeError::ToolError("tool crashed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("tool crashed"));
    }

    #[test]
    fn test_agent_output_clone_equality() {
        let output = AgentOutput {
            response: "hello".to_string(),
            iterations: 3,
            tool_call_count: 2,
        };
        let cloned = output.clone();
        assert_eq!(cloned.response, output.response);
        assert_eq!(cloned.iterations, output.iterations);
        assert_eq!(cloned.tool_call_count, output.tool_call_count);
    }

    #[test]
    fn test_agent_output_debug_format() {
        let output = AgentOutput {
            response: "test".to_string(),
            iterations: 1,
            tool_call_count: 0,
        };
        let debug = format!("{:?}", output);
        assert!(debug.contains("test"));
        assert!(debug.contains("response"));
    }

    #[test]
    fn test_agent_event_all_variants_clone() {
        let events = vec![
            AgentEvent::TurnStarted { iteration: 0 },
            AgentEvent::TurnCompleted { iteration: 1 },
            AgentEvent::ToolUse {
                tool_name: "t".to_string(),
                tool_use_id: "id".to_string(),
            },
            AgentEvent::ToolResult {
                tool_use_id: "id".to_string(),
                is_error: false,
            },
            AgentEvent::Error {
                error: "e".to_string(),
            },
            AgentEvent::ProactiveTick,
        ];
        let cloned = events.clone();
        assert_eq!(cloned.len(), 6);
    }

    #[test]
    fn test_agent_runtime_error_variants() {
        let runtime = AgentRuntimeError::RuntimeError("r".to_string());
        let session = AgentRuntimeError::SessionError("s".to_string());
        let tool = AgentRuntimeError::ToolError("t".to_string());
        assert!(runtime.to_string().contains("r"));
        assert!(session.to_string().contains("s"));
        assert!(tool.to_string().contains("t"));
    }

    #[test]
    fn test_agent_runtime_config_custom_values() {
        let config = AgentRuntimeConfig {
            role: "planner".to_string(),
            system_prompt: "Plan tasks".to_string(),
            max_iterations: 100,
            timeout_secs: 600,
        };
        assert_eq!(config.role, "planner");
        assert_eq!(config.system_prompt, "Plan tasks");
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.timeout_secs, 600);
    }

    #[test]
    fn test_agent_output_large_values() {
        let output = AgentOutput {
            response: "a".repeat(10000),
            iterations: usize::MAX,
            tool_call_count: 999,
        };
        assert_eq!(output.response.len(), 10000);
        assert_eq!(output.iterations, usize::MAX);
    }
}
