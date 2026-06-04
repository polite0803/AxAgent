//! Error types and handling utilities for AxAgent
//!
//! This module provides a unified error hierarchy for the entire application,
//! with support for error propagation, context addition, and serialization.

use std::collections::HashMap;
use thiserror::Error;

/// Unified error type for AxAgent application
///
/// This enum represents all possible error conditions in the application,
/// organized by category for easier error handling and debugging.
///
/// # Variants
///
/// - `Database`: Database operation failures
/// - `Provider`: External provider errors (LLM, embedding, etc.)
/// - `Gateway`: Gateway-related errors
/// - `Crypto`: Cryptography operation errors
/// - `NotFound`: Resource not found errors
/// - `Validation`: Input validation errors
/// - `Io`: I/O operation errors
/// - `Config`: Configuration errors
/// - `Timeout`: Timeout errors
/// - `Workflow`: Workflow execution errors (with optional source)
/// - `Agent`: Agent-related errors (with optional source)
/// - `Execution`: Node execution errors (with optional source)
/// - `Internal`: Internal application errors
///
/// # Examples
///
/// ```
/// use axagent_core::error::{AxAgentError, Result};
///
/// fn example() -> Result<()> {
///     Err(AxAgentError::NotFound("User not found".to_string()))
/// }
/// ```
#[derive(Debug, Error)]
pub enum AxAgentError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Gateway error: {0}")]
    Gateway(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Workflow error: {context}")]
    Workflow {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: String,
    },

    #[error("Agent error: {context}")]
    Agent {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: String,
    },

    #[error("Execution error: {context}")]
    Execution {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        context: String,
    },

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Structured error: {message}")]
    StructuredError {
        context: ErrorContext,
        source: Box<dyn std::error::Error + Send + Sync>,
        message: String,
    },

    #[error("Model download error: {0}")]
    ModelDownload(String),

    #[error("Model integrity error: expected {expected}, got {actual}")]
    ModelIntegrity { expected: String, actual: String },

    #[error("Model inference error: {0}")]
    Inference(String),

    #[error("RAG error: {0}")]
    Rag(String),
}

impl AxAgentError {
    /// Creates a new workflow error with the given context message
    pub fn workflow<S: Into<String>>(context: S) -> Self {
        AxAgentError::Workflow {
            source: None,
            context: context.into(),
        }
    }

    /// Creates a new workflow error with an underlying source error
    pub fn workflow_with_source<E: Into<Box<dyn std::error::Error + Send + Sync>>>(
        context: String,
        source: E,
    ) -> Self {
        AxAgentError::Workflow {
            source: Some(source.into()),
            context,
        }
    }

    /// Creates a new agent error with the given context message
    pub fn agent<S: Into<String>>(context: S) -> Self {
        AxAgentError::Agent {
            source: None,
            context: context.into(),
        }
    }

    /// Creates a new execution error with the given context message
    pub fn execution<S: Into<String>>(context: S) -> Self {
        AxAgentError::Execution {
            source: None,
            context: context.into(),
        }
    }

    /// Creates a new internal error with the given context message
    pub fn internal<S: Into<String>>(context: S) -> Self {
        AxAgentError::Internal(context.into())
    }

    /// Creates a new configuration error with the given context message
    pub fn config<S: Into<String>>(context: S) -> Self {
        AxAgentError::Config(context.into())
    }

    /// Creates a new timeout error with the given context message
    pub fn timeout<S: Into<String>>(context: S) -> Self {
        AxAgentError::Timeout(context.into())
    }

    /// Creates a new provider error with the given context message
    pub fn provider<S: Into<String>>(context: S) -> Self {
        AxAgentError::Provider(context.into())
    }

    /// Adds context to an error, prepends the context string to the error message
    ///
    /// Only works for Workflow, Agent, and Execution error variants.
    /// Other variants are returned unchanged.
    pub fn add_context(self, ctx: String) -> Self {
        match self {
            AxAgentError::Workflow { source, context } => AxAgentError::Workflow {
                source,
                context: format!("{}: {}", ctx, context),
            },
            AxAgentError::Agent { source, context } => AxAgentError::Agent {
                source,
                context: format!("{}: {}", ctx, context),
            },
            AxAgentError::Execution { source, context } => AxAgentError::Execution {
                source,
                context: format!("{}: {}", ctx, context),
            },
            _ => self,
        }
    }

    /// Wraps the error with structured context
    pub fn with_context(self, context: ErrorContext) -> Self {
        let message = self.to_string();
        AxAgentError::StructuredError {
            context,
            source: Box::new(self),
            message,
        }
    }

    /// Returns the machine-readable error code for this error
    pub fn error_code(&self) -> ErrorCode {
        match self {
            AxAgentError::Database(_) => ErrorCode::InternalError,
            AxAgentError::Provider(_) => ErrorCode::LLMProviderError,
            AxAgentError::Gateway(_) => ErrorCode::NetworkError,
            AxAgentError::Crypto(_) => ErrorCode::InternalError,
            AxAgentError::NotFound(_) => ErrorCode::ValidationError,
            AxAgentError::Validation(_) => ErrorCode::ValidationError,
            AxAgentError::Io(_) => ErrorCode::InternalError,
            AxAgentError::Config(_) => ErrorCode::ConfigurationError,
            AxAgentError::Timeout(_) => ErrorCode::NetworkError,
            AxAgentError::Workflow { .. } => ErrorCode::AgentError,
            AxAgentError::Agent { .. } => ErrorCode::AgentError,
            AxAgentError::Execution { .. } => ErrorCode::ToolExecutionError,
            AxAgentError::Internal(_) => ErrorCode::InternalError,
            AxAgentError::StructuredError { context, .. } => {
                ErrorCode::from_component(&context.component)
            },
            AxAgentError::ModelDownload(_) => ErrorCode::NetworkError,
            AxAgentError::ModelIntegrity { .. } => ErrorCode::ValidationError,
            AxAgentError::Inference(_) => ErrorCode::AgentError,
            AxAgentError::Rag(_) => ErrorCode::ValidationError,
        }
    }

    /// Generates a serializable error report for telemetry/logging
    pub fn to_report(&self) -> ErrorReport {
        let mut source_chain = Vec::new();
        let mut current: Option<&dyn std::error::Error> = Some(self);

        while let Some(err) = current {
            source_chain.push(err.to_string());
            current = err.source();
        }

        let (context, message) = match self {
            AxAgentError::StructuredError {
                context, message, ..
            } => (context.clone(), message.clone()),
            _ => (
                ErrorContext::builder()
                    .component("unknown")
                    .operation("unknown")
                    .build(),
                self.to_string(),
            ),
        };

        ErrorReport {
            error_code: self.error_code(),
            message,
            context,
            source_chain,
            timestamp: chrono::Utc::now(),
            recoverable: self.is_recoverable(),
        }
    }

    /// Returns true if the error is potentially recoverable via retry
    pub fn is_recoverable(&self) -> bool {
        match self {
            AxAgentError::Timeout(_) => true,
            AxAgentError::Gateway(_) => true,
            AxAgentError::Provider(msg) => msg.contains("rate limit") || msg.contains("timeout"),
            AxAgentError::StructuredError { context, .. } => context.retry_count < 3,
            AxAgentError::ModelDownload(_) => true,
            AxAgentError::ModelIntegrity { .. } => false,
            AxAgentError::Inference(_) => false,
            AxAgentError::Rag(_) => false,
            _ => false,
        }
    }
}

impl serde::Serialize for AxAgentError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<sea_orm::TransactionError<sea_orm::DbErr>> for AxAgentError {
    fn from(err: sea_orm::TransactionError<sea_orm::DbErr>) -> Self {
        match err {
            sea_orm::TransactionError::Connection(e) => AxAgentError::Database(e),
            sea_orm::TransactionError::Transaction(e) => AxAgentError::Database(e),
        }
    }
}

impl From<String> for AxAgentError {
    fn from(s: String) -> Self {
        AxAgentError::Internal(s)
    }
}

impl From<&str> for AxAgentError {
    fn from(s: &str) -> Self {
        AxAgentError::Internal(s.to_string())
    }
}

impl From<serde_json::Error> for AxAgentError {
    fn from(err: serde_json::Error) -> Self {
        AxAgentError::Internal(format!("JSON serialization error: {}", err))
    }
}

/// Error type for health check operations
///
/// Distinguishes between transient errors (which may succeed on retry)
/// and permanent errors (which will always fail).
#[derive(Debug, thiserror::Error)]
pub enum HealthCheckError {
    #[error("Transient error: {0}")]
    Transient(String),
    #[error("Permanent error: {0}")]
    Permanent(String),
    #[error("Network error: {0}")]
    Network(String),
}

impl HealthCheckError {
    /// Returns true if the error is transient and may succeed on retry
    pub fn is_transient(&self) -> bool {
        matches!(self, HealthCheckError::Transient(_) | HealthCheckError::Network(_))
    }

    /// Creates a HealthCheckError from HTTP status code and response body
    ///
    /// Classifies errors based on HTTP status codes:
    /// - 4xx (except 429): Permanent errors
    /// - 429 (rate limit): Transient error
    /// - 5xx: Transient errors
    /// - Other: Transient error
    pub fn from_status(status: u16, body: &str) -> Self {
        match status {
            401 | 403 => HealthCheckError::Permanent(format!("Authentication failed: {}", body)),
            404 => HealthCheckError::Permanent(format!("Endpoint not found: {}", body)),
            429 => HealthCheckError::Transient(format!("Rate limited: {}", body)),
            500..=599 => HealthCheckError::Transient(format!("Server error {}: {}", status, body)),
            _ if (400..500).contains(&status) => {
                HealthCheckError::Permanent(format!("Client error {}: {}", status, body))
            },
            _ => HealthCheckError::Transient(format!("HTTP error {}: {}", status, body)),
        }
    }
}

/// Machine-readable error codes for categorizing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorCode {
    AgentError,
    ToolExecutionError,
    LLMProviderError,
    PlanGenerationError,
    StateTransitionError,
    ResourceExhaustionError,
    NetworkError,
    ValidationError,
    ConfigurationError,
    InternalError,
}

impl ErrorCode {
    fn from_component(component: &str) -> Self {
        match component.to_lowercase().as_str() {
            "agent" => ErrorCode::AgentError,
            "tool" | "runtime" => ErrorCode::ToolExecutionError,
            "llm" | "provider" => ErrorCode::LLMProviderError,
            "plan" => ErrorCode::PlanGenerationError,
            "state" => ErrorCode::StateTransitionError,
            "resource" => ErrorCode::ResourceExhaustionError,
            "network" | "gateway" => ErrorCode::NetworkError,
            "validation" => ErrorCode::ValidationError,
            "config" => ErrorCode::ConfigurationError,
            _ => ErrorCode::InternalError,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::AgentError => write!(f, "AGENT_ERROR"),
            ErrorCode::ToolExecutionError => write!(f, "TOOL_EXECUTION_ERROR"),
            ErrorCode::LLMProviderError => write!(f, "LLM_PROVIDER_ERROR"),
            ErrorCode::PlanGenerationError => write!(f, "PLAN_GENERATION_ERROR"),
            ErrorCode::StateTransitionError => write!(f, "STATE_TRANSITION_ERROR"),
            ErrorCode::ResourceExhaustionError => write!(f, "RESOURCE_EXHAUSTION_ERROR"),
            ErrorCode::NetworkError => write!(f, "NETWORK_ERROR"),
            ErrorCode::ValidationError => write!(f, "VALIDATION_ERROR"),
            ErrorCode::ConfigurationError => write!(f, "CONFIGURATION_ERROR"),
            ErrorCode::InternalError => write!(f, "INTERNAL_ERROR"),
        }
    }
}

/// Structured context for error reporting and telemetry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorContext {
    pub session_id: Option<String>,
    pub component: String,
    pub operation: String,
    pub retry_count: u32,
    pub metadata: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorContext {
    /// Creates a new ErrorContextBuilder for ergonomic construction
    pub fn builder() -> ErrorContextBuilder {
        ErrorContextBuilder::default()
    }
}

/// Builder for constructing ErrorContext instances
#[derive(Debug, Default)]
pub struct ErrorContextBuilder {
    session_id: Option<String>,
    component: Option<String>,
    operation: Option<String>,
    retry_count: u32,
    metadata: HashMap<String, String>,
}

impl ErrorContextBuilder {
    /// Sets the session ID for this error context
    pub fn session_id<S: Into<String>>(mut self, session_id: S) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Sets the component name (e.g., "agent", "runtime", "gateway")
    pub fn component<S: Into<String>>(mut self, component: S) -> Self {
        self.component = Some(component.into());
        self
    }

    /// Sets the operation name (e.g., "tool_execution", "llm_call", "plan_generation")
    pub fn operation<S: Into<String>>(mut self, operation: S) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Sets the retry count for this error
    pub fn retry_count(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }

    /// Sets the metadata hashmap for this error context
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Adds a single key-value pair to the metadata
    pub fn metadata_entry<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Builds the ErrorContext instance
    pub fn build(self) -> ErrorContext {
        ErrorContext {
            session_id: self.session_id,
            component: self.component.unwrap_or_else(|| "unknown".to_string()),
            operation: self.operation.unwrap_or_else(|| "unknown".to_string()),
            retry_count: self.retry_count,
            metadata: self.metadata,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Serializable error report for telemetry and logging
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorReport {
    pub error_code: ErrorCode,
    pub message: String,
    pub context: ErrorContext,
    pub source_chain: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub recoverable: bool,
}

pub type Result<T> = std::result::Result<T, AxAgentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let err = AxAgentError::Workflow {
            source: None,
            context: "User not found".to_string(),
        };
        let err_with_ctx = err.add_context("get_user".to_string());
        assert!(err_with_ctx.to_string().contains("get_user"));
    }

    #[test]
    fn test_workflow_error() {
        let err = AxAgentError::workflow("Invalid node configuration");
        assert!(matches!(err, AxAgentError::Workflow { .. }));
    }

    #[test]
    fn test_workflow_error_with_source() {
        let source = AxAgentError::NotFound("node not found".to_string());
        let err =
            AxAgentError::workflow_with_source("workflow execution failed".to_string(), source);
        match err {
            AxAgentError::Workflow {
                source: Some(_),
                context,
            } => {
                assert!(context.contains("workflow execution failed"));
            },
            _ => panic!("Expected Workflow error with source"),
        }
    }

    #[test]
    fn test_agent_error() {
        let err = AxAgentError::agent("Agent initialization failed");
        assert!(matches!(err, AxAgentError::Agent { .. }));
    }

    #[test]
    fn test_execution_error() {
        let err = AxAgentError::execution("Execution timeout");
        assert!(matches!(err, AxAgentError::Execution { .. }));
    }

    #[test]
    fn test_error_serialization() {
        let err = AxAgentError::Validation("Field is required".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Validation"));
    }

    #[test]
    fn test_error_display() {
        let err = AxAgentError::NotFound("User not found".to_string());
        let display = format!("{}", err);
        assert!(display.contains("User not found"));
    }

    #[test]
    fn test_error_from_string() {
        let err: AxAgentError = "some error".into();
        assert!(matches!(err, AxAgentError::Internal(_)));
    }
}
