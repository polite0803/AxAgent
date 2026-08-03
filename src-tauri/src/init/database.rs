// SPDX-License-Identifier: AGPL-3.0-only

use std::path::{Path, PathBuf};

/// 限制密钥文件权限为仅当前用户可访问。
/// - Unix: 0o600 (owner rw)
/// - Windows: icacls 移除继承权限，仅保留当前用户
pub fn restrict_file_permissions(path: &Path) -> Result<(), String> {
    #[cfg(all(unix, not(mobile)))]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("failed to set file permissions: {}", e))?;
    }
    #[cfg(windows)]
    {
        // Windows: 使用 icacls 移除继承权限，仅保留当前用户
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "SYSTEM".into());
        let mut scmd = std::process::Command::new("icacls");
        scmd.arg(path.as_os_str())
            .arg("/inheritance:r")
            .arg("/grant")
            .arg(format!("{}:(R,W)", username));
        #[cfg(windows)]
        axagent_kit::utils::hide_window(&mut scmd);
        let result = scmd.output().map_err(|e| format!("failed to run icacls: {}", e))?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            tracing::warn!(
                "icacls restricted permissions reported non-zero exit: stderr={}",
                stderr
            );
        }
    }
    #[cfg(mobile)]
    {
        let _ = path; // 移动端无文件权限细化需求
    }
    #[cfg(all(not(mobile), not(unix), not(windows)))]
    {
        let _ = path; // unsupported platform, skip
    }
    Ok(())
}

pub struct DatabaseInitResult {
    pub db_handle: axagent_dao::db::DbHandle,
    pub db_path: String,
    pub master_key: [u8; 32],
    pub app_dir: PathBuf,
}

/// 使用预先解析的 app_dir 初始化数据库。
///
/// Android：主线程已调用 `axagent_home()` + `create_dir_all()`，
/// 子线程中 `dirs::data_dir()` 因缺少 JNI 上下文不可用。
/// 此函数跳过路径解析，直接使用传入的目录。
/// 解析数据库连接配置（DB 外持久化于 `{app_dir}/db_config.json`）。
///
/// 返回 `(连接 URL, 是否 SQLite)`。未配置文件时回退到默认本地 SQLite。
fn resolve_db_url(app_dir: &Path, master_key: &[u8; 32]) -> Result<(String, bool), String> {
    let cfg_path = app_dir.join("db_config.json");
    if !cfg_path.exists() {
        return Ok((format!("sqlite:{}/axagent.db", app_dir.display()), true));
    }
    let content = std::fs::read_to_string(&cfg_path).map_err(|e| e.to_string())?;
    let cfg: axagent_dao::config::DbConfig =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;
    build_db_url(&cfg, app_dir, master_key)
}

/// 根据 DbConfig 解析数据库连接 URL 与是否 SQLite。
///
/// PostgreSQL 密码优先使用明文 `pg_password`（连接测试/前端回传），
/// 缺失时回退解密 `pg_password_enc`（启动时从盘读取）。
/// 供 `init_database_with_dir`（启动）与 `test_db_connection` 命令共用，
/// 避免解析逻辑重复。
pub(crate) fn build_db_url(
    cfg: &axagent_dao::config::DbConfig,
    app_dir: &Path,
    master_key: &[u8; 32],
) -> Result<(String, bool), String> {
    if cfg.db_type == "postgres" {
        let host = cfg.pg_host.clone().unwrap_or_else(|| "localhost".to_string());
        let port = cfg.pg_port.unwrap_or(5432);
        let database = cfg.pg_database.clone().unwrap_or_else(|| "axagent".to_string());
        let user = cfg.pg_user.clone().unwrap_or_else(|| "postgres".to_string());
        let password = match &cfg.pg_password {
            Some(pw) if !pw.is_empty() => pw.clone(),
            _ => match &cfg.pg_password_enc {
                Some(enc) => axagent_crypto::decrypt_key(enc, master_key)
                    .map_err(|e| format!("解密数据库密码失败: {}", e))?,
                None => String::new(),
            },
        };
        let sslmode = if cfg.use_ssl.unwrap_or(false) {
            "require"
        } else {
            "disable"
        };
        let mut url = format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            pg_url_encode(&user),
            pg_url_encode(&password),
            host,
            port,
            database,
            sslmode
        );
        if let Some(schema) = &cfg.pg_schema {
            if !schema.is_empty() {
                url.push_str(&format!("&search_path={}", pg_url_encode(schema)));
            }
        }
        Ok((url, false))
    } else {
        let path =
            cfg.sqlite_path.clone().unwrap_or_else(|| format!("{}/axagent.db", app_dir.display()));
        Ok((format!("sqlite:{}", path), true))
    }
}

/// 对 PostgreSQL 连接 URL 中的 user/password 做最小 percent-encode，
/// 转义 `@ : / % & # ?` 及空格，避免破坏 URL 结构。
fn pg_url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '@' | ':' | '/' | '%' | '&' | '#' | '?' | ' ' => {
                out.push_str(&format!("%{:02X}", c as u8));
            },
            _ => out.push(c),
        }
    }
    out
}

pub async fn init_database_with_dir(app_dir: PathBuf) -> Result<DatabaseInitResult, String> {
    axagent_storage::storage_paths::ensure_documents_dirs().unwrap_or_else(|e| {
        tracing::warn!(
            "Failed to create documents storage dirs (non-critical, will retry later): {}",
            e
        );
    });

    let key_path = app_dir.join("master.key");
    let master_key = load_or_create_master_key(&key_path, &app_dir)?;

    // 解析数据库连接配置（DB 外持久化于 {app_dir}/db_config.json）
    let (db_url, is_sqlite) = resolve_db_url(&app_dir, &master_key)?;

    // 仅 SQLite 注册 sqlite-vec 扩展。在 Android 上默认跳过（见 vector_store.rs），
    // 在桌面平台用 catch_unwind 防止 FFI 异常 panic。
    if is_sqlite {
        let vec_registration = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            axagent_search::vector_store::register_sqlite_vec_extension();
        }));
        if let Err(e) = vec_registration {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic payload".to_string()
            };
            tracing::error!(
                "sqlite-vec extension registration panicked: {} — vector search will be unavailable",
                msg
            );
        }
    }
    axagent_tools::global_state::set_db_path(&db_url);

    // 直接使用当前 tokio runtime，不再创建嵌套 Runtime
    let db_handle = axagent_dao::db::create_pool(&db_url)
        .await
        .map_err(|e| format!("database initialization failed: {}", e))?;

    // MCP 预设服务器播种（依赖 mcp_client，在 core 中）
    if let Err(e) = axagent_dao::repo::mcp_server::ensure_preset_servers(&db_handle.conn).await {
        tracing::warn!("[DB] MCP 预设服务器迁移失败: {e}");
    }

    // 硬编码路径 → 模板变量迁移（已迁入 storage，直接调用）
    // 注意：此函数已从 path_vars 移除，迁移逻辑由各模块自行处理
    // axagent_storage::path_vars::migrate_hardcoded_paths(&db_handle.conn).await;

    // 注册 SeaORM 连接
    axagent_tools::global_state::set_sea_db(std::sync::Arc::new(db_handle.conn.clone()));

    // 将 dao 实现的 agent / workflow 系列 repository 注册进 harness 全局服务注册表，
    // 供 consumer crate（rt-workflow 等）通过 trait 访问器获取，避免直接依赖 axagent-entities。
    axagent_dao::agent_repositories::register_repositories(&db_handle.conn);

    Ok(DatabaseInitResult { db_handle, db_path: db_url, master_key, app_dir })
}

pub(crate) fn load_or_create_master_key(
    key_path: &Path,
    app_dir: &Path,
) -> Result<[u8; 32], String> {
    if key_path.exists() {
        let mut bytes =
            std::fs::read(key_path).map_err(|e| format!("failed to read master key: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!(
                "master.key is corrupted: expected 32 bytes, got {}. Delete the file to regenerate.",
                bytes.len()
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        // Security: securely zero the temporary buffer before dropping.
        // Using a helper that inhibits compiler optimization of the clear.
        secure_zero(&mut bytes);
        // key is returned (copy), bytes is zeroed and dropped
        Ok(key)
    } else {
        let db_file = app_dir.join("axagent.db");
        if db_file.exists() {
            return Err(format!(
                "FATAL: axagent.db exists at '{}' but master.key is missing from '{}'.\n\
                 Generating a new master key would render all encrypted database \
                 contents permanently unrecoverable.\n\n\
                 Options:\n\
                 • Restore master.key from a backup and restart.\n\
                 • Remove axagent.db (and axagent.db-shm / axagent.db-wal if present) \
                   to start fresh — ALL DATA WILL BE LOST.",
                db_file.display(),
                key_path.display()
            ));
        }
        let key = axagent_crypto::generate_master_key();
        std::fs::write(key_path, key).map_err(|e| format!("failed to write master key: {}", e))?;
        restrict_file_permissions(key_path)?;
        Ok(key)
    }
}

/// Securely zero a byte buffer, inhibiting compiler optimization of the clear.
/// Uses volatile writes + compiler fence to ensure the memory is actually overwritten before drop.
#[inline(never)]
fn secure_zero(buf: &mut [u8]) {
    for byte in buf.iter_mut() {
        // SAFETY: byte is a valid mutable reference obtained from buf.iter_mut();
        // write_volatile is used to prevent compiler optimization from eliding the
        // zeroing of sensitive key material; this is the standard pattern for secure
        // memory clearing.
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }
    // SECURITY (C8): compiler_fence 防止编译器将上述 volatile 写入视为"死存储"而优化掉。
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
