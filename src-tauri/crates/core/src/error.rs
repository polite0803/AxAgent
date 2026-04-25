use thiserror::Error;

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
}

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
    pub fn is_transient(&self) -> bool {
        matches!(self, HealthCheckError::Transient(_) | HealthCheckError::Network(_))
    }

    pub fn from_status(status: u16, body: &str) -> Self {
        match status {
            401 | 403 => HealthCheckError::Permanent(format!("Authentication failed: {}", body)),
            404 => HealthCheckError::Permanent(format!("Endpoint not found: {}", body)),
            429 => HealthCheckError::Transient(format!("Rate limited: {}", body)),
            500..=599 => HealthCheckError::Transient(format!("Server error {}: {}", status, body)),
            _ if (400..500).contains(&status) => HealthCheckError::Permanent(format!("Client error {}: {}", status, body)),
            _ => HealthCheckError::Transient(format!("HTTP error {}: {}", status, body)),
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

pub type Result<T> = std::result::Result<T, AxAgentError>;
