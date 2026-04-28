use async_trait::async_trait;

use crate::coordinator::{AgentConfig, AgentError, AgentInput, AgentStatus, CoordinatorOutput};

#[async_trait]
pub trait Agent: Send + Sync {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError>;
    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError>;
    async fn pause(&mut self) -> Result<(), AgentError>;
    async fn resume(&mut self) -> Result<(), AgentError>;
    async fn cancel(&mut self) -> Result<(), AgentError>;
    fn status(&self) -> AgentStatus;
    fn agent_type(&self) -> &'static str;
}

#[async_trait]
impl<T: crate::coordinator::AgentImpl> Agent for T {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError> {
        <Self as crate::coordinator::AgentImpl>::initialize(self, config).await
    }

    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        <Self as crate::coordinator::AgentImpl>::execute(self, input).await
    }

    async fn pause(&mut self) -> Result<(), AgentError> {
        <Self as crate::coordinator::AgentImpl>::pause(self).await
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        <Self as crate::coordinator::AgentImpl>::resume(self).await
    }

    async fn cancel(&mut self) -> Result<(), AgentError> {
        <Self as crate::coordinator::AgentImpl>::cancel(self).await
    }

    fn status(&self) -> AgentStatus {
        <Self as crate::coordinator::AgentImpl>::status(self)
    }

    fn agent_type(&self) -> &'static str {
        <Self as crate::coordinator::AgentImpl>::agent_type(self)
    }
}
