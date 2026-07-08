// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt;

#[derive(Debug)]
pub enum TrajectoryError {
    Database(String),
    SeaOrm(sea_orm::DbErr),
    Rusqlite(rusqlite::Error),
    SerdeJson(serde_json::Error),
    Io(std::io::Error),
    Uuid(uuid::Error),
    Fts5(String),
    Storage(String),
    Batch(String),
    SubAgent(String),
    Execution(String),
    NotFound(String),
    InvalidConfig(String),
    Internal(String),
    TokioJoin(tokio::task::JoinError),
}

impl fmt::Display for TrajectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(msg) => write!(f, "Database error: {msg}"),
            Self::SeaOrm(e) => write!(f, "SeaORM error: {e}"),
            Self::Rusqlite(e) => write!(f, "Rusqlite error: {e}"),
            Self::SerdeJson(e) => write!(f, "JSON error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Uuid(e) => write!(f, "UUID error: {e}"),
            Self::Fts5(msg) => write!(f, "FTS5 error: {msg}"),
            Self::Storage(msg) => write!(f, "Storage error: {msg}"),
            Self::Batch(msg) => write!(f, "Batch error: {msg}"),
            Self::SubAgent(msg) => write!(f, "Sub-agent error: {msg}"),
            Self::Execution(msg) => write!(f, "Execution error: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::InvalidConfig(msg) => write!(f, "Invalid config: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
            Self::TokioJoin(e) => write!(f, "Tokio join error: {e}"),
        }
    }
}

impl std::error::Error for TrajectoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SeaOrm(e) => Some(e),
            Self::Rusqlite(e) => Some(e),
            Self::SerdeJson(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Uuid(e) => Some(e),
            Self::TokioJoin(e) => Some(e),
            _ => None,
        }
    }
}

impl From<sea_orm::DbErr> for TrajectoryError {
    fn from(e: sea_orm::DbErr) -> Self { Self::SeaOrm(e) }
}

impl From<rusqlite::Error> for TrajectoryError {
    fn from(e: rusqlite::Error) -> Self { Self::Rusqlite(e) }
}

impl From<serde_json::Error> for TrajectoryError {
    fn from(e: serde_json::Error) -> Self { Self::SerdeJson(e) }
}

impl From<std::io::Error> for TrajectoryError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}

impl From<uuid::Error> for TrajectoryError {
    fn from(e: uuid::Error) -> Self { Self::Uuid(e) }
}

impl From<tokio::task::JoinError> for TrajectoryError {
    fn from(e: tokio::task::JoinError) -> Self { Self::TokioJoin(e) }
}
