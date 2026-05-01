//! Shared Memory Pool - Agent间KV存储
//!
//! Features:
//! - Key-value storage for agent communication
//! - Namespace isolation
//! - TTL support
//! - Atomic operations
//! - Pub/sub notifications

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub namespace: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub ttl_secs: Option<u64>,
    pub owner_agent: Option<String>,
    pub readers: HashSet<String>,
    pub writers: HashSet<String>,
    /// Monotonically increasing version number for conflict detection.
    /// Incremented on every write. Agents can check version before overwriting.
    pub version: u64,
}

impl MemoryEntry {
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_secs {
            let now = current_timestamp();
            self.created_at + ttl < now
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedMemoryPool {
    entries: HashMap<String, MemoryEntry>,
    namespaces: HashMap<String, HashSet<String>>,
    subscribers: HashMap<String, Vec<String>>,
}

impl SharedMemoryPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(
        &mut self,
        key: &str,
        value: &str,
        namespace: &str,
        owner_agent: Option<&str>,
    ) -> Result<MemoryEntry, MemoryError> {
        let now = current_timestamp();
        let full_key = format!("{}:{}", namespace, key);

        // Check write permission if entry already exists and has writers set
        if let Some(existing) = self.entries.get(&full_key) {
            if !existing.writers.is_empty() {
                if let Some(agent) = owner_agent {
                    if !existing.writers.contains(agent) {
                        return Err(MemoryError::PermissionDenied);
                    }
                } else {
                    return Err(MemoryError::PermissionDenied);
                }
            }
        }

        // Determine version: increment if overwriting existing entry
        let next_version = self
            .entries
            .get(&full_key)
            .map(|e| e.version + 1)
            .unwrap_or(1);

        let entry = MemoryEntry {
            key: key.to_string(),
            value: value.to_string(),
            namespace: namespace.to_string(),
            created_at: now,
            updated_at: now,
            ttl_secs: None,
            owner_agent: owner_agent.map(String::from),
            readers: HashSet::new(),
            writers: HashSet::new(),
            version: next_version,
        };

        self.namespaces
            .entry(namespace.to_string())
            .or_default()
            .insert(full_key.clone());

        self.entries.insert(full_key, entry.clone());

        Ok(entry)
    }

    /// Write only if the current version matches `expected_version`.
    /// Returns Ok(entry) on success, Err(VersionConflict) if versions don't match.
    /// This prevents silent overwrites when multiple agents write the same key.
    pub fn set_if_version(
        &mut self,
        key: &str,
        value: &str,
        namespace: &str,
        expected_version: u64,
        owner_agent: Option<&str>,
    ) -> Result<MemoryEntry, MemoryError> {
        let full_key = format!("{}:{}", namespace, key);
        if let Some(existing) = self.entries.get(&full_key) {
            if existing.version != expected_version {
                return Err(MemoryError::VersionConflict {
                    expected: expected_version,
                    actual: existing.version,
                });
            }
        } else if expected_version != 0 {
            // Entry doesn't exist but expected_version != 0 means caller expected it to exist
            return Err(MemoryError::NotFound(full_key));
        }
        self.set(key, value, namespace, owner_agent)
    }

    pub fn set_with_ttl(
        &mut self,
        key: &str,
        value: &str,
        namespace: &str,
        ttl_secs: u64,
        owner_agent: Option<&str>,
    ) -> Result<MemoryEntry, MemoryError> {
        let _entry = self.set(key, value, namespace, owner_agent)?;

        let full_key = format!("{}:{}", namespace, key);

        if let Some(e) = self.entries.get_mut(&full_key) {
            e.ttl_secs = Some(ttl_secs);
        }

        Ok(self.entries.get(&full_key).unwrap().clone())
    }

    pub fn get(
        &self,
        key: &str,
        namespace: &str,
        reader_agent: Option<&str>,
    ) -> Result<MemoryEntry, MemoryError> {
        let full_key = format!("{}:{}", namespace, key);

        let entry = self
            .entries
            .get(&full_key)
            .ok_or_else(|| MemoryError::NotFound(full_key.clone()))?;

        if entry.is_expired() {
            return Err(MemoryError::Expired(full_key));
        }

        // Check read permission if readers set is non-empty
        if !entry.readers.is_empty() {
            if let Some(agent) = reader_agent {
                if !entry.readers.contains(agent) {
                    return Err(MemoryError::PermissionDenied);
                }
            } else {
                return Err(MemoryError::PermissionDenied);
            }
        }

        Ok(entry.clone())
    }

    pub fn delete(&mut self, key: &str, namespace: &str) -> Result<(), MemoryError> {
        let full_key = format!("{}:{}", namespace, key);

        if let Some(entry) = self.entries.remove(&full_key) {
            if let Some(ns) = self.namespaces.get_mut(&entry.namespace) {
                ns.remove(&full_key);
            }
        }

        Ok(())
    }

    pub fn list_namespace(&self, namespace: &str) -> Vec<MemoryEntry> {
        if let Some(keys) = self.namespaces.get(namespace) {
            keys.iter()
                .filter_map(|k| self.entries.get(k))
                .filter(|e| !e.is_expired())
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn list_all_namespaces(&self) -> Vec<String> {
        self.namespaces.keys().cloned().collect()
    }

    pub fn cleanup_expired(&mut self) -> usize {
        let expired_keys: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_expired())
            .map(|(k, _)| k.clone())
            .collect();

        for key in &expired_keys {
            if let Some(entry) = self.entries.remove(key) {
                if let Some(ns) = self.namespaces.get_mut(&entry.namespace) {
                    ns.remove(key);
                }
            }
        }

        expired_keys.len()
    }

    pub fn subscribe(&mut self, key_pattern: &str, agent_id: &str) {
        self.subscribers
            .entry(key_pattern.to_string())
            .or_default()
            .push(agent_id.to_string());
    }

    pub fn unsubscribe(&mut self, key_pattern: &str, agent_id: &str) {
        if let Some(subs) = self.subscribers.get_mut(key_pattern) {
            subs.retain(|a| a != agent_id);
        }
    }

    pub fn notify_subscribers(&self, key: &str, namespace: &str) -> Vec<String> {
        let full_key = format!("{}:{}", namespace, key);
        let mut notified = Vec::new();

        for (pattern, agents) in &self.subscribers {
            if full_key.contains(pattern) || pattern == "*" {
                notified.extend(agents.clone());
            }
        }

        notified
    }

    pub fn grant_read(
        &mut self,
        key: &str,
        namespace: &str,
        agent_id: &str,
    ) -> Result<(), MemoryError> {
        let full_key = format!("{}:{}", namespace, key);

        let entry = self
            .entries
            .get_mut(&full_key)
            .ok_or_else(|| MemoryError::NotFound(full_key.clone()))?;

        entry.readers.insert(agent_id.to_string());

        Ok(())
    }

    pub fn grant_write(
        &mut self,
        key: &str,
        namespace: &str,
        agent_id: &str,
    ) -> Result<(), MemoryError> {
        let full_key = format!("{}:{}", namespace, key);

        let entry = self
            .entries
            .get_mut(&full_key)
            .ok_or_else(|| MemoryError::NotFound(full_key.clone()))?;

        entry.writers.insert(agent_id.to_string());

        Ok(())
    }

    pub fn compare_and_set(
        &mut self,
        key: &str,
        namespace: &str,
        expected_value: &str,
        new_value: &str,
        writer_agent: Option<&str>,
    ) -> Result<bool, MemoryError> {
        let full_key = format!("{}:{}", namespace, key);

        let entry = self
            .entries
            .get_mut(&full_key)
            .ok_or_else(|| MemoryError::NotFound(full_key.clone()))?;

        // Check write permission if writers set is non-empty
        if !entry.writers.is_empty() {
            if let Some(agent) = writer_agent {
                if !entry.writers.contains(agent) {
                    return Err(MemoryError::PermissionDenied);
                }
            } else {
                return Err(MemoryError::PermissionDenied);
            }
        }

        if entry.value == expected_value {
            entry.value = new_value.to_string();
            entry.updated_at = current_timestamp();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn namespace_size(&self, namespace: &str) -> usize {
        self.namespaces.get(namespace).map(|s| s.len()).unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub enum MemoryError {
    NotFound(String),
    Expired(String),
    PermissionDenied,
    InvalidKey,
    VersionConflict { expected: u64, actual: u64 },
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(key) => write!(f, "Key not found: {}", key),
            Self::Expired(key) => write!(f, "Key expired: {}", key),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::InvalidKey => write!(f, "Invalid key"),
            Self::VersionConflict { expected, actual } => {
                write!(
                    f,
                    "Version conflict: expected {}, actual {}",
                    expected, actual
                )
            },
        }
    }
}

impl std::error::Error for MemoryError {}

pub struct SharedMemory {
    pool: Arc<RwLock<SharedMemoryPool>>,
    notification_tx: Arc<RwLock<Option<tokio::sync::mpsc::Sender<MemoryNotification>>>>,
}

#[derive(Debug, Clone)]
pub struct MemoryNotification {
    pub key: String,
    pub namespace: String,
    pub event: MemoryEvent,
    pub notified_agents: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum MemoryEvent {
    Created,
    Updated { old_value: String },
    Deleted,
    Expired,
}

impl SharedMemory {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(RwLock::new(SharedMemoryPool::new())),
            notification_tx: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_notification_channel(
        capacity: usize,
    ) -> (Self, tokio::sync::mpsc::Receiver<MemoryNotification>) {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);
        let shared = Self {
            pool: Arc::new(RwLock::new(SharedMemoryPool::new())),
            notification_tx: Arc::new(RwLock::new(Some(tx))),
        };
        (shared, rx)
    }

    pub fn set(&self, key: &str, value: &str, namespace: &str) -> Result<MemoryEntry, MemoryError> {
        let mut pool = self.pool.write().unwrap();
        // Lazy cleanup: remove expired entries on every write to prevent memory leak
        pool.cleanup_expired();
        let old_value = pool.get(key, namespace, None).ok().map(|e| e.value);
        let entry = pool.set(key, value, namespace, None)?;

        let notified = pool.notify_subscribers(key, namespace);
        if !notified.is_empty() {
            let event = if let Some(old) = old_value {
                MemoryEvent::Updated { old_value: old }
            } else {
                MemoryEvent::Created
            };
            let notification = MemoryNotification {
                key: key.to_string(),
                namespace: namespace.to_string(),
                event,
                notified_agents: notified,
            };
            let _ = self.send_notification(notification);
        }

        Ok(entry)
    }

    pub fn get(&self, key: &str, namespace: &str) -> Result<MemoryEntry, MemoryError> {
        let pool = self.pool.read().unwrap();
        pool.get(key, namespace, None)
    }

    pub fn get_with_agent(
        &self,
        key: &str,
        namespace: &str,
        agent_id: &str,
    ) -> Result<MemoryEntry, MemoryError> {
        let pool = self.pool.read().unwrap();
        pool.get(key, namespace, Some(agent_id))
    }

    pub fn delete(&self, key: &str, namespace: &str) -> Result<(), MemoryError> {
        let mut pool = self.pool.write().unwrap();
        let old_value = pool.get(key, namespace, None).ok().map(|e| e.value);
        pool.delete(key, namespace)?;

        if let Some(_value) = old_value {
            let notified = pool.notify_subscribers(key, namespace);
            if !notified.is_empty() {
                let notification = MemoryNotification {
                    key: key.to_string(),
                    namespace: namespace.to_string(),
                    event: MemoryEvent::Deleted,
                    notified_agents: notified,
                };
                drop(pool);
                let _ = self.send_notification(notification);
            }
        }
        Ok(())
    }

    pub fn list(&self, namespace: &str) -> Vec<MemoryEntry> {
        let pool = self.pool.read().unwrap();
        pool.list_namespace(namespace)
    }

    pub fn subscribe(&self, pattern: &str, agent_id: &str) {
        let mut pool = self.pool.write().unwrap();
        pool.subscribe(pattern, agent_id);
    }

    pub fn unsubscribe(&self, pattern: &str, agent_id: &str) {
        let mut pool = self.pool.write().unwrap();
        pool.unsubscribe(pattern, agent_id);
    }

    pub fn notify(&self, key: &str, namespace: &str) -> Vec<String> {
        let pool = self.pool.read().unwrap();
        pool.notify_subscribers(key, namespace)
    }

    pub fn cas(
        &self,
        key: &str,
        namespace: &str,
        expected: &str,
        new: &str,
    ) -> Result<bool, MemoryError> {
        let mut pool = self.pool.write().unwrap();
        pool.compare_and_set(key, namespace, expected, new, None)
    }

    pub fn cas_with_agent(
        &self,
        key: &str,
        namespace: &str,
        expected: &str,
        new: &str,
        agent_id: &str,
    ) -> Result<bool, MemoryError> {
        let mut pool = self.pool.write().unwrap();
        pool.compare_and_set(key, namespace, expected, new, Some(agent_id))
    }

    pub fn cleanup(&self) -> usize {
        let mut pool = self.pool.write().unwrap();
        pool.cleanup_expired()
    }

    fn send_notification(&self, notification: MemoryNotification) -> Result<(), MemoryError> {
        if let Ok(tx_guard) = self.notification_tx.read() {
            if let Some(ref tx) = *tx_guard {
                if let Err(e) = tx.try_send(notification) {
                    tracing::debug!("Notification channel full, dropping notification: {}", e);
                }
            }
        }
        Ok(())
    }

    pub fn stats(&self) -> MemoryStats {
        let pool = self.pool.read().unwrap();
        MemoryStats {
            total_entries: pool.size(),
            namespaces: pool.list_all_namespaces().len(),
        }
    }

    /// Persist all non-expired entries to a JSON file for crash recovery.
    pub fn save_snapshot(&self, path: &std::path::Path) -> Result<(), String> {
        let pool = self.pool.read().unwrap();
        // Clean up expired entries first
        let snapshot: Vec<&MemoryEntry> =
            pool.entries.values().filter(|e| !e.is_expired()).collect();
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| format!("Failed to serialize shared memory: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write shared memory snapshot: {}", e))
    }

    /// Load entries from a JSON snapshot file, merging into the current pool.
    pub fn load_snapshot(&self, path: &std::path::Path) -> Result<usize, String> {
        if !path.exists() {
            return Ok(0);
        }
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read shared memory snapshot: {}", e))?;
        let entries: Vec<MemoryEntry> = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize shared memory: {}", e))?;

        let mut pool = self.pool.write().unwrap();
        let mut loaded = 0;
        for entry in entries {
            if entry.is_expired() {
                continue;
            }
            let full_key = format!("{}:{}", entry.namespace, entry.key);
            pool.namespaces
                .entry(entry.namespace.clone())
                .or_default()
                .insert(full_key.clone());
            pool.entries.insert(full_key, entry);
            loaded += 1;
        }
        Ok(loaded)
    }
}

impl Default for SharedMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub namespaces: usize,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_set_get() {
        let mem = SharedMemory::new();

        mem.set("key1", "value1", "ns1").unwrap();

        let entry = mem.get("key1", "ns1").unwrap();
        assert_eq!(entry.value, "value1");
    }

    #[test]
    fn test_memory_not_found() {
        let mem = SharedMemory::new();

        let result = mem.get("nonexistent", "ns1");
        assert!(matches!(result, Err(MemoryError::NotFound(_))));
    }

    #[test]
    fn test_namespace_isolation() {
        let mem = SharedMemory::new();

        mem.set("key1", "value1", "ns1").unwrap();
        mem.set("key1", "value2", "ns2").unwrap();

        let v1 = mem.get("key1", "ns1").unwrap();
        let v2 = mem.get("key1", "ns2").unwrap();

        assert_eq!(v1.value, "value1");
        assert_eq!(v2.value, "value2");
    }

    #[test]
    fn test_cas() {
        let mem = SharedMemory::new();

        mem.set("key1", "original", "ns1").unwrap();

        let swapped = mem.cas("key1", "ns1", "wrong", "new").unwrap();
        assert!(!swapped);

        let swapped = mem.cas("key1", "ns1", "original", "new").unwrap();
        assert!(swapped);

        let entry = mem.get("key1", "ns1").unwrap();
        assert_eq!(entry.value, "new");
    }

    #[test]
    fn test_subscribe_notify() {
        let mem = SharedMemory::new();

        mem.subscribe("key*", "agent_1");

        let notified = mem.notify("key1", "ns1");
        assert!(notified.contains(&"agent_1".to_string()));
    }
}
