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

    #[error("data service not initialized")]
    DataServiceNotInitialized,

    #[error("industry adapter error: {0}")]
    IndustryAdapter(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type OpcResult<T> = Result<T, OpcError>;

impl From<sea_orm::DbErr> for OpcError {
    fn from(err: sea_orm::DbErr) -> Self {
        OpcError::Database(err.to_string())
    }
}

impl From<sea_orm::TransactionError<OpcError>> for OpcError {
    fn from(err: sea_orm::TransactionError<OpcError>) -> Self {
        match err {
            sea_orm::TransactionError::Connection(db_err) => OpcError::Database(db_err.to_string()),
            other => OpcError::Internal(format!("事务错误: {:?}", other)),
        }
    }
}

impl From<serde_json::Error> for OpcError {
    fn from(err: serde_json::Error) -> Self {
        OpcError::Internal(err.to_string())
    }
}

impl From<String> for OpcError {
    fn from(err: String) -> Self {
        OpcError::Internal(err)
    }
}

impl From<&str> for OpcError {
    fn from(err: &str) -> Self {
        OpcError::Internal(err.to_string())
    }
}
