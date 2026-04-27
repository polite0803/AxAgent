//! Error types and handling utilities for AxAgent
//!
//! This module provides a unified error hierarchy for the entire application,
//! with support for error propagation, context addition, and serialization.

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
        source: Option<Box<AxAgentError>>,
        context: String,
    },

    #[error("Agent error: {context}")]
    Agent {
        #[source]
        source: Option<Box<AxAgentError>>,
        context: String,
    },

    #[error("Execution error: {context}")]
    Execution {
        #[source]
        source: Option<Box<AxAgentError>>,
        context: String,
    },

    #[error("Internal error: {0}")]
    Internal(String),
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
    pub fn workflow_with_source<E: Into<AxAgentError>>(context: String, source: E) -> Self {
        AxAgentError::Workflow {
            source: Some(Box::new(source.into())),
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
        matches!(
            self,
            HealthCheckError::Transient(_) | HealthCheckError::Network(_)
        )
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
            }
            _ => HealthCheckError::Transient(format!("HTTP error {}: {}", status, body)),
        }
    }
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
            }
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
