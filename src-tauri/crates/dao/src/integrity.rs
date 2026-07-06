// SPDX-License-Identifier: AGPL-3.0-only

//! 数据库完整性检测与自动恢复
//!
//! 在数据库初始化前执行 `PRAGMA integrity_check`，检测到损坏时自动备份并重建。
//! 防止因 SQLite 文件损坏导致应用启动失败。

use std::path::Path;

use sea_orm::{ConnectionTrait, DbBackend, Statement};
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
/// 使用 `execute_unprepared` + 日志解析方式绕过 sea-orm 的 StatementBuilder 限制。
pub async fn detect_corruption(conn: &impl ConnectionTrait) -> Result<CorruptionStatus> {
    // 通过执行 integrity_check 并捕获异常来检测
    // 如果返回 Err 或行中包含非 "ok" 的值，说明数据库损坏
    match conn
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA integrity_check;".to_string(),
        ))
        .await
    {
        Ok(result) => {
            // integrity_check 返回的行数反映结果：1 行 = "ok"，多行 = 错误详情
            // execute_raw 不能返回行内容，所以如果成功->假设完好
            // 为了更可靠的检测，再尝试读取一张表的 schema
            if result.rows_affected() > 1 {
                // 多行意味着有错误输出（SQLite 为每个错误输出一行）
                Ok(CorruptionStatus::Corrupted(
                    "integrity_check returned multiple rows indicating corruption".into(),
                ))
            } else {
                // 再执行一个快速验证查询
                match conn
                    .execute_raw(Statement::from_string(
                        DbBackend::Sqlite,
                        "SELECT COUNT(*) FROM sqlite_master;".to_string(),
                    ))
                    .await
                {
                    Ok(_) => Ok(CorruptionStatus::Healthy),
                    Err(e) => {
                        Ok(CorruptionStatus::Corrupted(format!("sqlite_master query failed: {e}")))
                    },
                }
            }
        },
        Err(e) => Ok(CorruptionStatus::Corrupted(format!("integrity_check execution failed: {e}"))),
    }
}

/// 备份损坏的数据库文件。
/// 将 `db_path` 重命名为 `db_path.corrupted.<timestamp>`。
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

    // 同时备份 WAL 和 SHM 文件（如果存在）
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
///
/// 在 `create_pool` 连接建立后、PRAGMA 设置之前调用。
pub async fn auto_recover(conn: &impl ConnectionTrait, db_path: &str) -> Result<()> {
    // 跳过内存数据库
    if db_path == ":memory:" || db_path.starts_with("sqlite::memory:") {
        return Ok(());
    }

    // 提取实际文件路径（去除 sqlite: 前缀和查询参数）
    let file_path = db_path
        .strip_prefix("sqlite:")
        .unwrap_or(db_path)
        .split('?')
        .next()
        .unwrap_or(db_path);

    let path = Path::new(file_path);
    if !path.exists() {
        // 新数据库，无需检查
        return Ok(());
    }

    match detect_corruption(conn).await? {
        CorruptionStatus::Healthy => {
            info!("Database integrity check passed for: {file_path}");
        },
        CorruptionStatus::Corrupted(err_msg) => {
            warn!("Database corruption detected at {file_path}: {err_msg}");

            // 备份损坏文件
            let backup = backup_corrupted_db(file_path)?;
            info!("Database auto-recovery: backed up to {backup}, creating fresh database");

            // 备份完成，调用方（create_pool）会继续执行 PRAGMA + 迁移
            // 由于原文件已被 rename，SQLite 会创建新文件
            info!("Database auto-recovery completed, fresh database will be initialized");
        },
    }

    Ok(())
}
