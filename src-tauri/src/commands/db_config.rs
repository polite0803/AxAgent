// SPDX-License-Identifier: AGPL-3.0-only

//! 数据库连接配置命令（DB 外持久化）。
//!
//! DbConfig 定义在 `axagent_dao::config`（init 与 commands 共享，避免重复定义）。
//! 密码字段使用 master.key（Aes256Gcm）加密后落盘，明文不写入文件。

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use sea_orm::{ConnectOptions, ConnectionTrait, Database};
use tauri::State;
use tauri::command;

use axagent_crypto::{decrypt_key, encrypt_key};

use axagent_dao::config::DbConfig;
use axagent_dao::migrations::SchemaMigrationStatus;

use axagent_agent_macro::agent_command;

use crate::AppState;

fn db_config_path() -> PathBuf {
    crate::paths::axagent_home().join("db_config.json")
}

/// 读取 master.key（用于密码加解密），复用 init::database 的加载逻辑。
fn load_master_key() -> Result<[u8; 32], String> {
    let app_dir = crate::paths::axagent_home();
    let key_path = app_dir.join("master.key");
    crate::init::database::load_or_create_master_key(&key_path, &app_dir)
}

#[agent_command(domain = "db_config", safety = Safe, call_mode = StateOnly, description = "获取数据库配置")]
#[command]
pub fn get_db_config() -> Result<DbConfig, String> {
    let path = db_config_path();
    if !path.exists() {
        return Ok(DbConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let mut cfg: DbConfig = serde_json::from_str(&content).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    // 解密密码供前端填充表单（pg_password_enc -> pg_password）
    if let Some(enc) = cfg.pg_password_enc.take() {
        if let Ok(key) = load_master_key() {
            if let Ok(plain) = decrypt_key(&enc, &key) {
                cfg.pg_password = Some(plain);
            }
        }
    }
    Ok(cfg)
}

#[agent_command(domain = "db_config", safety = Caution, call_mode = StateInput, description = "保存数据库配置")]
#[command]
pub fn save_db_config(config: DbConfig) -> Result<(), String> {
    let path = db_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }
    let mut to_save = config;
    // 若前端传回了明文密码，则加密落盘；空密码表示清除。
    if let Some(pw) = to_save.pg_password.take() {
        if pw.is_empty() {
            to_save.pg_password_enc = None;
        } else {
            let key = load_master_key()?;
            let enc = encrypt_key(&pw, &key).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            to_save.pg_password_enc = Some(enc);
        }
    }
    // 明文密码不落盘
    let content = serde_json::to_string_pretty(&to_save).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    fs::write(&path, content).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 测试数据库连接是否可用（不持久化）。
///
/// 直接用传入的 DbConfig 构建连接 URL 并打开一个最小连接，执行 `SELECT 1`
/// 验证连通性与凭据正确性。PostgreSQL 走 `build_db_url` 的密码解密逻辑。
#[agent_command(domain = "db_config", safety = Safe, call_mode = StateInput, description = "测试数据库连接")]
#[command]
pub async fn test_db_connection(config: DbConfig) -> Result<String, String> {
    let app_dir = crate::paths::axagent_home();
    let master_key = load_master_key()?;
    let (url, _is_sqlite) = crate::init::database::build_db_url(&config, &app_dir, &master_key)?;

    let mut opt = ConnectOptions::new(&url);
    opt.max_connections(1)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(10))
        .sqlx_logging(false);

    let conn = Database::connect(opt).await.map_err(|e| format!("连接失败: {}", e))?;
    conn.execute_unprepared("SELECT 1").await.map_err(|e| format!("查询验证失败: {}", e))?;
    Ok("连接成功".to_string())
}

/// P2-9: 查询当前 schema 迁移状态。
///
/// 返回已应用版本、最新版本、pending 数量和已应用迁移列表，
/// 供前端诊断「schema 滞后」类问题（如启动后迁移未跑完导致表缺失）。
#[agent_command(domain = "db_config", safety = Safe, call_mode = StateOnly, description = "获取数据库架构迁移状态")]
#[command]
pub async fn get_schema_status(
    state: State<'_, AppState>,
) -> Result<SchemaMigrationStatus, String> {
    let db = state.harness.db();
    axagent_dao::migrations::get_schema_status(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 修复数据库架构：无条件检查所有已知后加列，缺一补一。
/// 不影响运行中的会话数据，仅补全缺失的表结构。
#[agent_command(domain = "db_config", safety = Caution, call_mode = StateOnly, description = "修复数据库架构缺失列")]
#[command]
pub async fn repair_schema(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.harness.db();
    let (added, _total) = axagent_dao::migrations::repair_schema(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(format!("{}", added))
}
