use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

const BROADCAST_CAPACITY: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum EntryPriority {
    #[default]
    Normal,
    Low,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardEntry {
    pub id: String,
    pub key: String,
    pub value: serde_json::Value,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ttl_secs: Option<u64>,
    pub tags: Vec<String>,
    pub priority: EntryPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlackboardEvent {
    Written { key: String, author: String },
    Updated { key: String, author: String },
    Deleted { key: String },
    Expired { key: String },
}

pub struct Blackboard {
    name: String,
    entries: Arc<RwLock<HashMap<String, BlackboardEntry>>>,
    event_sender: broadcast::Sender<BlackboardEvent>,
}

impl Blackboard {
    pub fn new(name: &str) -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            name: name.to_string(),
            entries: Arc::new(RwLock::new(HashMap::new())),
            event_sender: tx,
        }
    }

    pub fn write(&self, author: &str, key: &str, value: serde_json::Value) -> BlackboardEntry {
        let entry = BlackboardEntry {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            value,
            author: author.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ttl_secs: None,
            tags: Vec::new(),
            priority: EntryPriority::Normal,
        };
        let event = BlackboardEvent::Written {
            key: key.to_string(),
            author: author.to_string(),
        };
        {
            let mut map = self.entries.write().unwrap();
            map.insert(key.to_string(), entry.clone());
        }
        let _ = self.event_sender.send(event);
        entry
    }

    pub fn read(&self, key: &str) -> Option<BlackboardEntry> {
        let map = self.entries.read().unwrap();
        map.get(key).cloned()
    }

    pub fn read_all(&self) -> Vec<BlackboardEntry> {
        let map = self.entries.read().unwrap();
        map.values().cloned().collect()
    }

    pub fn read_by_tag(&self, tag: &str) -> Vec<BlackboardEntry> {
        let map = self.entries.read().unwrap();
        map.values()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    pub fn read_by_author(&self, author: &str) -> Vec<BlackboardEntry> {
        let map = self.entries.read().unwrap();
        map.values()
            .filter(|e| e.author == author)
            .cloned()
            .collect()
    }

    pub fn update(&self, key: &str, value: serde_json::Value) -> Option<BlackboardEntry> {
        let mut map = self.entries.write().unwrap();
        if let Some(entry) = map.get_mut(key) {
            entry.value = value;
            entry.updated_at = Utc::now();
            let updated = entry.clone();
            let event = BlackboardEvent::Updated {
                key: key.to_string(),
                author: entry.author.clone(),
            };
            drop(map);
            let _ = self.event_sender.send(event);
            Some(updated)
        } else {
            None
        }
    }

    pub fn delete(&self, key: &str) -> Option<BlackboardEntry> {
        let mut map = self.entries.write().unwrap();
        let removed = map.remove(key);
        if removed.is_some() {
            let event = BlackboardEvent::Deleted {
                key: key.to_string(),
            };
            drop(map);
            let _ = self.event_sender.send(event);
        }
        removed
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BlackboardEvent> {
        self.event_sender.subscribe()
    }

    pub fn cleanup_expired(&self) -> usize {
        let mut map = self.entries.write().unwrap();
        let now = Utc::now();
        let expired_keys: Vec<String> = map
            .iter()
            .filter(|(_, entry)| {
                if let Some(ttl) = entry.ttl_secs {
                    let elapsed = now.signed_duration_since(entry.created_at);
                    elapsed.num_seconds() as u64 > ttl
                } else {
                    false
                }
            })
            .map(|(k, _)| k.clone())
            .collect();
        let count = expired_keys.len();
        for key in &expired_keys {
            map.remove(key);
            let event = BlackboardEvent::Expired { key: key.clone() };
            let _ = self.event_sender.send(event);
        }
        count
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct BlackboardManager {
    blackboards: Arc<RwLock<HashMap<String, Arc<Blackboard>>>>,
}

impl BlackboardManager {
    pub fn new() -> Self {
        Self {
            blackboards: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_blackboard(&self, name: &str) -> Arc<Blackboard> {
        let bb = Arc::new(Blackboard::new(name));
        let mut map = self.blackboards.write().unwrap();
        map.insert(name.to_string(), bb.clone());
        bb
    }

    pub fn get_blackboard(&self, name: &str) -> Option<Arc<Blackboard>> {
        let map = self.blackboards.read().unwrap();
        map.get(name).cloned()
    }

    pub fn list_blackboards(&self) -> Vec<String> {
        let map = self.blackboards.read().unwrap();
        map.keys().cloned().collect()
    }

    pub fn delete_blackboard(&self, name: &str) -> bool {
        let mut map = self.blackboards.write().unwrap();
        map.remove(name).is_some()
    }
}

impl Default for BlackboardManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_new() {
        let bb = Blackboard::new("test-board");
        assert_eq!(bb.name(), "test-board");
    }

    #[test]
    fn test_write_and_read() {
        let bb = Blackboard::new("test");
        bb.write("agent-1", "key1", serde_json::json!("value1"));
        let entry = bb.read("key1");
        assert!(entry.is_some());
        let e = entry.unwrap();
        assert_eq!(e.key, "key1");
        assert_eq!(e.value, serde_json::json!("value1"));
        assert_eq!(e.author, "agent-1");
    }

    #[test]
    fn test_read_nonexistent() {
        let bb = Blackboard::new("test");
        assert!(bb.read("missing").is_none());
    }

    #[test]
    fn test_write_overwrites() {
        let bb = Blackboard::new("test");
        bb.write("agent-1", "key1", serde_json::json!("v1"));
        bb.write("agent-2", "key1", serde_json::json!("v2"));
        let entry = bb.read("key1").unwrap();
        assert_eq!(entry.value, serde_json::json!("v2"));
        assert_eq!(entry.author, "agent-2");
    }

    #[test]
    fn test_update() {
        let bb = Blackboard::new("test");
        bb.write("agent-1", "key1", serde_json::json!("old"));
        let updated = bb.update("key1", serde_json::json!("new"));
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().value, serde_json::json!("new"));
        assert_eq!(bb.read("key1").unwrap().value, serde_json::json!("new"));
    }

    #[test]
    fn test_update_nonexistent() {
        let bb = Blackboard::new("test");
        assert!(bb.update("missing", serde_json::json!("x")).is_none());
    }

    #[test]
    fn test_delete() {
        let bb = Blackboard::new("test");
        bb.write("agent-1", "key1", serde_json::json!("val"));
        let deleted = bb.delete("key1");
        assert!(deleted.is_some());
        assert!(bb.read("key1").is_none());
    }

    #[test]
    fn test_delete_nonexistent() {
        let bb = Blackboard::new("test");
        assert!(bb.delete("missing").is_none());
    }

    #[test]
    fn test_read_all() {
        let bb = Blackboard::new("test");
        bb.write("a", "k1", serde_json::json!(1));
        bb.write("a", "k2", serde_json::json!(2));
        bb.write("a", "k3", serde_json::json!(3));
        let all = bb.read_all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_read_by_tag() {
        let bb = Blackboard::new("test");
        bb.write("a", "k1", serde_json::json!("v1"));
        let mut map = bb.entries.write().unwrap();
        if let Some(e) = map.get_mut("k1") {
            e.tags = vec!["important".to_string(), "urgent".to_string()];
        }
        drop(map);
        bb.write("a", "k2", serde_json::json!("v2"));
        let mut map2 = bb.entries.write().unwrap();
        if let Some(e) = map2.get_mut("k2") {
            e.tags = vec!["low-priority".to_string()];
        }
        drop(map2);
        let important = bb.read_by_tag("important");
        assert_eq!(important.len(), 1);
        assert_eq!(important[0].key, "k1");
    }

    #[test]
    fn test_read_by_author() {
        let bb = Blackboard::new("test");
        bb.write("alice", "k1", serde_json::json!(1));
        bb.write("bob", "k2", serde_json::json!(2));
        bb.write("alice", "k3", serde_json::json!(3));
        let alice_entries = bb.read_by_author("alice");
        assert_eq!(alice_entries.len(), 2);
        let bob_entries = bb.read_by_author("bob");
        assert_eq!(bob_entries.len(), 1);
    }

    #[test]
    fn test_cleanup_expired() {
        let bb = Blackboard::new("test");
        bb.write("a", "fresh", serde_json::json!("v"));
        let mut map = bb.entries.write().unwrap();
        if let Some(e) = map.get_mut("fresh") {
            e.ttl_secs = Some(3600);
        }
        if let Some(e) = map.get_mut("fresh") {
            e.created_at = Utc::now();
        }
        drop(map);

        bb.write("a", "expired", serde_json::json!("v2"));
        let mut map2 = bb.entries.write().unwrap();
        if let Some(e) = map2.get_mut("expired") {
            e.ttl_secs = Some(1);
        }
        if let Some(e) = map2.get_mut("expired") {
            e.created_at = Utc::now() - chrono::Duration::seconds(10);
        }
        drop(map2);

        let count = bb.cleanup_expired();
        assert_eq!(count, 1);
        assert!(bb.read("fresh").is_some());
        assert!(bb.read("expired").is_none());
    }

    #[test]
    fn test_cleanup_no_ttl_not_expired() {
        let bb = Blackboard::new("test");
        bb.write("a", "no-ttl", serde_json::json!("v"));
        let count = bb.cleanup_expired();
        assert_eq!(count, 0);
        assert!(bb.read("no-ttl").is_some());
    }

    #[test]
    fn test_subscribe_write_event() {
        let bb = Blackboard::new("test");
        let mut rx = bb.subscribe();
        bb.write("agent", "key1", serde_json::json!("val"));
        let event = rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            BlackboardEvent::Written { key, author } => {
                assert_eq!(key, "key1");
                assert_eq!(author, "agent");
            },
            _ => panic!("Expected Written event"),
        }
    }

    #[test]
    fn test_subscribe_update_event() {
        let bb = Blackboard::new("test");
        bb.write("agent", "key1", serde_json::json!("old"));
        let mut rx = bb.subscribe();
        bb.update("key1", serde_json::json!("new"));
        let event = rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            BlackboardEvent::Updated { key, author } => {
                assert_eq!(key, "key1");
                assert_eq!(author, "agent");
            },
            _ => panic!("Expected Updated event"),
        }
    }

    #[test]
    fn test_subscribe_delete_event() {
        let bb = Blackboard::new("test");
        bb.write("agent", "key1", serde_json::json!("val"));
        let mut rx = bb.subscribe();
        bb.delete("key1");
        let event = rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            BlackboardEvent::Deleted { key } => {
                assert_eq!(key, "key1");
            },
            _ => panic!("Expected Deleted event"),
        }
    }

    #[test]
    fn test_subscribe_expired_event() {
        let bb = Blackboard::new("test");
        bb.write("a", "exp-key", serde_json::json!("v"));
        let mut map = bb.entries.write().unwrap();
        if let Some(e) = map.get_mut("exp-key") {
            e.ttl_secs = Some(1);
            e.created_at = Utc::now() - chrono::Duration::seconds(10);
        }
        drop(map);
        let mut rx = bb.subscribe();
        bb.cleanup_expired();
        let event = rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            BlackboardEvent::Expired { key } => {
                assert_eq!(key, "exp-key");
            },
            _ => panic!("Expected Expired event"),
        }
    }

    #[test]
    fn test_entry_priority_default() {
        assert_eq!(EntryPriority::default(), EntryPriority::Normal);
    }

    #[test]
    fn test_blackboard_manager_create() {
        let mgr = BlackboardManager::new();
        let bb = mgr.create_blackboard("board-1");
        assert_eq!(bb.name(), "board-1");
    }

    #[test]
    fn test_blackboard_manager_get() {
        let mgr = BlackboardManager::new();
        mgr.create_blackboard("board-1");
        let bb = mgr.get_blackboard("board-1");
        assert!(bb.is_some());
        assert_eq!(bb.unwrap().name(), "board-1");
    }

    #[test]
    fn test_blackboard_manager_get_nonexistent() {
        let mgr = BlackboardManager::new();
        assert!(mgr.get_blackboard("nope").is_none());
    }

    #[test]
    fn test_blackboard_manager_list() {
        let mgr = BlackboardManager::new();
        mgr.create_blackboard("a");
        mgr.create_blackboard("b");
        mgr.create_blackboard("c");
        let list = mgr.list_blackboards();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_blackboard_manager_delete() {
        let mgr = BlackboardManager::new();
        mgr.create_blackboard("del-me");
        assert!(mgr.delete_blackboard("del-me"));
        assert!(mgr.get_blackboard("del-me").is_none());
    }

    #[test]
    fn test_blackboard_manager_delete_nonexistent() {
        let mgr = BlackboardManager::new();
        assert!(!mgr.delete_blackboard("nope"));
    }

    #[test]
    fn test_blackboard_manager_create_overwrites() {
        let mgr = BlackboardManager::new();
        mgr.create_blackboard("dup");
        mgr.create_blackboard("dup");
        let list = mgr.list_blackboards();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_entry_has_uuid() {
        let bb = Blackboard::new("test");
        let e = bb.write("a", "k", serde_json::json!(1));
        assert!(!e.id.is_empty());
        assert!(uuid::Uuid::parse_str(&e.id).is_ok());
    }
}
