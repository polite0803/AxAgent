// SPDX-License-Identifier: AGPL-3.0-only

//! Credential error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<String> for CredentialError {
    fn from(s: String) -> Self {
        CredentialError::Internal(s)
    }
}

pub type Result<T> = std::result::Result<T, CredentialError>;
