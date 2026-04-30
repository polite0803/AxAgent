use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct DatabaseInitResult {
    pub db_handle: axagent_core::db::DbHandle,
    pub db_path: String,
    pub master_key: [u8; 32],
    pub app_dir: PathBuf,
}

pub fn init_database() -> Result<DatabaseInitResult, String> {
    let app_dir = crate::paths::axagent_home();
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("failed to create AxAgent home dir: {}", e))?;

    axagent_core::storage_paths::ensure_documents_dirs()
        .map_err(|e| format!("failed to create documents storage dirs: {}", e))?;

    let db_path = format!("sqlite:{}/axagent.db", app_dir.display());

    let key_path = app_dir.join("master.key");
    let master_key = load_or_create_master_key(&key_path, &app_dir)?;

    axagent_core::vector_store::register_sqlite_vec_extension();
    axagent_core::builtin_tools_registry::set_global_db_path(&db_path);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let db_handle = rt
        .block_on(axagent_core::db::create_pool(&db_path))
        .map_err(|e| format!("database initialization failed: {}", e))?;

    Ok(DatabaseInitResult {
        db_handle,
        db_path,
        master_key,
        app_dir,
    })
}

fn load_or_create_master_key(key_path: &Path, app_dir: &Path) -> Result<[u8; 32], String> {
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
        let key = axagent_core::crypto::generate_master_key();
        std::fs::write(key_path, key).map_err(|e| format!("failed to write master key: {}", e))?;
        #[cfg(unix)]
        {
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(key_path, perms)
                .map_err(|e| format!("failed to set master.key permissions: {}", e))?;
        }
        Ok(key)
    }
}

/// Securely zero a byte buffer, inhibiting compiler optimization of the clear.
/// Uses volatile writes to ensure the memory is actually overwritten before drop.
#[inline(never)]
fn secure_zero(buf: &mut [u8]) {
    for byte in buf.iter_mut() {
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }
}
