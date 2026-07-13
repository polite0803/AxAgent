// SPDX-License-Identifier: AGPL-3.0-only

//! 数据库连接配置（DB 外持久化）。
//!
//! 应用自身的 settings 存于数据库内，因此「用哪个数据库」的配置不能也存进
//! 数据库。DbConfig 持久化在 `{axagent_home()}/db_config.json`，在
//! `init_database_with_dir` 之前读取。密码字段使用 master.key 加密后落盘。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    /// 数据库类型："sqlite" 或 "postgres"
    pub db_type: String,
    /// SQLite 自定义路径（仅 sqlite 类型；留空使用默认 app_dir/axagent.db）
    pub sqlite_path: Option<String>,
    // PostgreSQL 连接信息
    pub pg_host: Option<String>,
    pub pg_port: Option<u16>,
    pub pg_database: Option<String>,
    pub pg_user: Option<String>,
    /// 明文密码（仅在前端 <-> 后端传输时使用，不落盘）
    pub pg_password: Option<String>,
    /// 加密后的密码（Aes256Gcm + master.key，落盘存储）
    pub pg_password_enc: Option<String>,
    pub pg_schema: Option<String>,
    pub use_ssl: Option<bool>,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            db_type: "sqlite".to_string(),
            sqlite_path: None,
            pg_host: Some("localhost".to_string()),
            pg_port: Some(5432),
            pg_database: Some("axagent".to_string()),
            pg_user: Some("postgres".to_string()),
            pg_password: None,
            pg_password_enc: None,
            pg_schema: None,
            use_ssl: Some(false),
        }
    }
}
