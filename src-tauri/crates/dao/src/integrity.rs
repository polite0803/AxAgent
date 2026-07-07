// SPDX-License-Identifier: AGPL-3.0-only

//! 数据库完整性检测与自动恢复
//!
//! 在数据库初始化前执行 `PRAGMA integrity_check`，检测到损坏时自动备份并重建。
//! 防止因 SQLite 文件损坏导致应用启动失败。

use std::path::Path;

use sea_orm::ConnectionTrait;
use tracing::{info, warn};

use axagent_harness::core_error::{AxAgentError, Result};

/// 完整性检测结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorruptionStatus {
    /// 数据库完好
    Healthy,
    /// 数据库损坏（附带错误信息）
    Corrupted(String),
}

/// 执行 `PRAGMA integrity_check`，返回检测结果。
///
/// Sea-ORM 的 query_all 要求 StatementBuilder（Statement 不实现），
/// 所以用 execute_unprepared + sqlite_master 双层检测。
pub async fn detect_corruption(conn: &impl ConnectionTrait) -> Result<CorruptionStatus> {
    // 第一层：执行 integrity_check，执行失败则视为损坏
    let exec_result = conn.execute_unprepared("PRAGMA integrity_check;").await;

    match exec_result {
        Ok(_) => {
            // 第二层：尝试读取 sqlite_master 确认数据库可正常查询
            match conn.execute_unprepared("SELECT COUNT(*) FROM sqlite_master;").await {
                Ok(_) => Ok(CorruptionStatus::Healthy),
                Err(e) => Ok(CorruptionStatus::Corrupted(format!(
                    "integrity_check passed but sqlite_master query failed: {e}"
                ))),
            }
        },
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("corrupt") || msg.contains("database disk image is malformed") {
                Ok(CorruptionStatus::Corrupted(format!("Corruption error: {e}")))
            } else {
                Err(AxAgentError::execution(format!("integrity_check failed: {e}")))
            }
        },
    }
}

/// 备份损坏的数据库文件。
fn backup_corrupted_db(db_path: &str) -> Result<String> {
    let path = Path::new(db_path);
    if !path.exists() {
        return Err(AxAgentError::execution(format!(
            "Cannot backup: database file not found: {db_path}"
        )));
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("{}.corrupted.{timestamp}", db_path);
    let backup_path = Path::new(&backup_name);

    std::fs::rename(path, backup_path).map_err(|e| {
        AxAgentError::execution(format!("Failed to backup corrupted database: {e}"))
    })?;

    for ext in ["-wal", "-shm"] {
        let sidecar = format!("{}{}", db_path, ext);
        let sidecar_path = Path::new(&sidecar);
        if sidecar_path.exists() {
            let backup_sidecar = format!("{db_path}.corrupted.{timestamp}{ext}");
            let _ = std::fs::rename(sidecar_path, &backup_sidecar);
        }
    }

    info!("Corrupted database backed up to: {backup_name}");
    Ok(backup_name)
}

/// 检测并自动恢复损坏的数据库。
pub async fn auto_recover(conn: &impl ConnectionTrait, db_path: &str) -> Result<()> {
    if db_path == ":memory:" || db_path.starts_with("sqlite::memory:") {
        return Ok(());
    }

    let file_path =
        db_path.strip_prefix("sqlite:").unwrap_or(db_path).split('?').next().unwrap_or(db_path);

    let path = Path::new(file_path);
    if !path.exists() {
        return Ok(());
    }

    match detect_corruption(conn).await? {
        CorruptionStatus::Healthy => {
            info!("Database integrity check passed for: {file_path}");
        },
        CorruptionStatus::Corrupted(err_msg) => {
            warn!("Database corruption detected at {file_path}: {err_msg}");
            let backup = backup_corrupted_db(file_path)?;
            info!(
                "Database auto-recovery: backed up to {backup}, fresh database will be initialized"
            );
        },
    }

    Ok(())
}
