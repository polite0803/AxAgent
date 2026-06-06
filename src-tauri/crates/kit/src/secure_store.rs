use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const SERVICE_NAME: &str = "axagent";

pub trait SecureStore: Send + Sync {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, String>;
    fn delete_secret(&self, key: &str) -> Result<(), String>;
    fn list_keys(&self) -> Result<Vec<String>, String>;
}

pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }
}

impl SecureStore for KeyringStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| format!("keyring entry creation failed: {}", e))?;
        entry
            .set_password(value)
            .map_err(|e| format!("keyring store failed: {}", e))
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| format!("keyring entry creation failed: {}", e))?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("keyring get failed: {}", e)),
        }
    }

    fn delete_secret(&self, key: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| format!("keyring entry creation failed: {}", e))?;
        match entry.delete_password() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("keyring delete failed: {}", e)),
        }
    }

    fn list_keys(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

pub struct FallbackEnvStore {
    env_path: PathBuf,
}

impl FallbackEnvStore {
    pub fn new(env_path: PathBuf) -> Self {
        Self { env_path }
    }

    fn read_env(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if !self.env_path.exists() {
            return map;
        }
        if let Ok(content) = fs::read_to_string(&self.env_path) {
            for line in content.lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().to_string();
                    if !key.is_empty() {
                        map.insert(key, value.trim().to_string());
                    }
                }
            }
        }
        map
    }

    fn write_env(&self, map: &HashMap<String, String>) -> Result<(), String> {
        if let Some(parent) = self.env_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let mut entries: Vec<_> = map.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        let content = entries
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&self.env_path, content)
            .map_err(|e| format!("Failed to write secrets file: {}", e))
    }
}

impl SecureStore for FallbackEnvStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
        let mut map = self.read_env();
        map.insert(key.to_string(), value.to_string());
        self.write_env(&map)
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.read_env().get(key).cloned())
    }

    fn delete_secret(&self, key: &str) -> Result<(), String> {
        let mut map = self.read_env();
        map.remove(key);
        self.write_env(&map)
    }

    fn list_keys(&self) -> Result<Vec<String>, String> {
        Ok(self.read_env().keys().cloned().collect())
    }
}

pub struct CombinedSecureStore {
    keyring: KeyringStore,
    fallback: FallbackEnvStore,
}

impl CombinedSecureStore {
    pub fn new(service: &str, env_path: PathBuf) -> Self {
        Self {
            keyring: KeyringStore::new(service),
            fallback: FallbackEnvStore::new(env_path),
        }
    }

    pub fn with_default_paths() -> Self {
        let env_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".axagent")
            .join(".secrets");
        Self::new(SERVICE_NAME, env_path)
    }
}

impl SecureStore for CombinedSecureStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
        match self.keyring.store_secret(key, value) {
            Ok(()) => {
                let _ = self.fallback.delete_secret(key);
                Ok(())
            },
            Err(e) => {
                // SECURITY (C8): 降级到明文文件必须显式可观测，且对真正敏感的 key 默认拒绝。
                tracing::error!(
                    target: "axagent.security",
                    "Keyring unavailable for key '{}', falling back to plaintext file at {}: {}",
                    key, self.fallback.env_path.display(), e
                );
                if is_secret_key(key) {
                    // 关键 key：必须显式 opt-in 才能降级
                    let allow = std::env::var("AXAGENT_ALLOW_PLAINTEXT_SECRETS")
                        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
                        .unwrap_or(false);
                    if !allow {
                        return Err(format!(
                            "refusing to store secret key '{}' in plaintext (set AXAGENT_ALLOW_PLAINTEXT_SECRETS=1 to override)",
                            key
                        ));
                    }
                    tracing::error!(
                        target: "axagent.security",
                        "AXAGENT_ALLOW_PLAINTEXT_SECRETS=1 — secret '{}' written to plaintext file",
                        key
                    );
                }
                self.fallback.store_secret(key, value)
            },
        }
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        match self.keyring.get_secret(key) {
            Ok(Some(v)) => Ok(Some(v)),
            Ok(None) => self.fallback.get_secret(key),
            Err(e) => {
                tracing::warn!(
                    target: "axagent.security",
                    "Keyring get failed for '{}', using fallback: {}",
                    key, e
                );
                self.fallback.get_secret(key)
            },
        }
    }

    fn delete_secret(&self, key: &str) -> Result<(), String> {
        let kr = self.keyring.delete_secret(key);
        let fb = self.fallback.delete_secret(key);
        kr.or(fb)
    }

    fn list_keys(&self) -> Result<Vec<String>, String> {
        let mut keys = self.keyring.list_keys()?;
        let fb_keys = self.fallback.list_keys()?;
        keys.extend(fb_keys);
        keys.sort();
        keys.dedup();
        Ok(keys)
    }
}

pub fn is_secret_key(key: &str) -> bool {
    let upper = key.to_uppercase();
    const PATTERNS: &[&str] = &[
        "KEY",
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "CREDENTIAL",
        "PRIVATE",
    ];
    PATTERNS.iter().any(|p| upper.contains(p))
}

pub fn migrate_secrets(
    store: &dyn SecureStore,
    secrets: HashMap<String, String>,
) -> Vec<(String, Result<(), String>)> {
    secrets
        .into_iter()
        .map(|(key, value)| {
            let result = store.store_secret(&key, &value);
            (key, result)
        })
        .collect()
}
