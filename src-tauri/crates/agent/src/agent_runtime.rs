use axagent_runtime::{
    ApiClient, ConversationRuntime, PermissionMode, PermissionPolicy, Session, ToolExecutor,
};
use tokio::sync::broadcast;

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
        let system_prompts = if config.system_prompt.is_empty() {
            Vec::new()
        } else {
            vec![config.system_prompt.clone()]
        };

        let conversation_runtime = ConversationRuntime::new(
            session.clone(),
            api_client,
            tool_executor,
            permission_policy,
            system_prompts,
        )
        .with_max_iterations(config.max_iterations);

        Self {
            session,
            conversation_runtime,
            config,
            event_sender,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.event_sender.subscribe()
    }

    pub fn session_id(&self) -> &str {
        &self.session.session_id
    }

    pub fn run(&mut self, input: &str) -> Result<AgentOutput, AgentRuntimeError> {
        self.emit(AgentEvent::TurnStarted { iteration: 0 });

        let result = self.conversation_runtime.run_turn(input, None);

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
                            if let axagent_runtime::ContentBlock::Text { text } = block {
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
