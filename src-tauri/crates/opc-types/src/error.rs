// SPDX-License-Identifier: AGPL-3.0-only

use thiserror::Error;

/// OPC 业务领域错误
#[derive(Error, Debug)]
pub enum OpcError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("duplicate: {0}")]
    Duplicate(String),

    #[error("invalid state transition: from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("database error: {0}")]
    Database(String),

    #[error("external service error: {0}")]
    ExternalService(String),
}

/// OPC 结果类型
pub type OpcResult<T> = Result<T, OpcError>;
