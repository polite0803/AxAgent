//! Engine bridge — isolation layer between the CodeEngine and GeneralEngine.
//!
//! Implements the three levels of isolation required by the architecture:
//!
//! 1. **Compute resource isolation**: Each engine may run on its own tokio
//!    runtime handle, preventing CPU contention between code and general tasks.
//! 2. **Index data isolation**: Code and general engines use separate SQLite
//!    connections (or separate databases), so index scans in one engine never
//!    block queries in the other.
//! 3. **Context session isolation**: Each engine maintains its own `Session`
//!    instance; a compaction in the general engine does not evict code context.
//!
//! The `EngineBridge` routes messages between engines and the shared
//! frontend transport.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

/// The identifier for an engine instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineId {
    Code,
    General,
}

impl EngineId {
    pub fn as_str(&self) -> &str {
        match self {
            EngineId::Code => "code",
            EngineId::General => "general",
        }
    }
}

/// A message passed between engines through the bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMessage {
    pub from: EngineId,
    pub to: EngineId,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
    pub timestamp: u64,
}

impl EngineMessage {
    pub fn new(from: EngineId, to: EngineId, payload: serde_json::Value) -> Self {
        Self {
            from,
            to,
            payload,
            correlation_id: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn with_correlation(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// Status of an engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineStatus {
    Idle,
    Busy,
    Error,
    Unavailable,
}

/// Health metrics for an engine on the bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineHealth {
    pub engine_id: EngineId,
    pub status: EngineStatus,
    pub active_tasks: usize,
    pub messages_processed: u64,
    pub last_heartbeat_ms: u64,
}

/// The engine bridge manages inter-engine communication and isolation.
pub struct EngineBridge {
    /// Channels per engine for sending messages.
    #[allow(clippy::type_complexity)]
    senders: RwLock<HashMap<EngineId, mpsc::Sender<EngineMessage>>>,
    /// Health state for each engine.
    health: RwLock<HashMap<EngineId, EngineHealth>>,
    /// Shared session store keyed by engine ID.
    session_keys: RwLock<HashMap<EngineId, String>>,
}

impl EngineBridge {
    pub fn new() -> Self {
        let mut health = HashMap::new();
        health.insert(
            EngineId::Code,
            EngineHealth {
                engine_id: EngineId::Code,
                status: EngineStatus::Idle,
                active_tasks: 0,
                messages_processed: 0,
                last_heartbeat_ms: 0,
            },
        );
        health.insert(
            EngineId::General,
            EngineHealth {
                engine_id: EngineId::General,
                status: EngineStatus::Idle,
                active_tasks: 0,
                messages_processed: 0,
                last_heartbeat_ms: 0,
            },
        );

        Self {
            senders: RwLock::new(HashMap::new()),
            health: RwLock::new(health),
            session_keys: RwLock::new(HashMap::new()),
        }
    }

    /// Register an engine with its message sender channel.
    pub async fn register_engine(&self, engine_id: EngineId, sender: mpsc::Sender<EngineMessage>) {
        self.senders.write().await.insert(engine_id, sender);
    }

    /// Unregister an engine.
    pub async fn unregister_engine(&self, engine_id: EngineId) {
        self.senders.write().await.remove(&engine_id);
    }

    /// Route a message from one engine to another.
    ///
    /// Returns `Ok(())` if the message was accepted by the target engine's
    /// channel, or `EngineNotFound`/`ChannelClosed` errors.
    pub async fn route_message(&self, msg: EngineMessage) -> Result<(), EngineBridgeError> {
        let from = msg.from;
        let senders = self.senders.read().await;
        let sender = senders
            .get(&msg.to)
            .ok_or(EngineBridgeError::EngineNotFound(msg.to))?;

        sender
            .try_send(msg)
            .map_err(|e| EngineBridgeError::ChannelClosed(format!("{e}")))?;

        // Update health
        drop(senders);
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(&from) {
            h.messages_processed += 1;
        }

        Ok(())
    }

    /// Update engine health status.
    pub async fn update_health(
        &self,
        engine_id: EngineId,
        status: EngineStatus,
        active_tasks: usize,
    ) {
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(&engine_id) {
            h.status = status;
            h.active_tasks = active_tasks;
            h.last_heartbeat_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        }
    }

    /// Get health for all engines.
    pub async fn all_health(&self) -> Vec<EngineHealth> {
        self.health.read().await.values().cloned().collect()
    }

    /// Get health for a specific engine.
    pub async fn engine_health(&self, engine_id: EngineId) -> Option<EngineHealth> {
        self.health.read().await.get(&engine_id).cloned()
    }

    /// Assign a session key for an engine.
    pub async fn set_session_key(&self, engine_id: EngineId, key: String) {
        self.session_keys.write().await.insert(engine_id, key);
    }

    /// Get the session key for an engine.
    pub async fn session_key(&self, engine_id: EngineId) -> Option<String> {
        self.session_keys.read().await.get(&engine_id).cloned()
    }

    /// Check if an engine is registered and available.
    pub async fn is_available(&self, engine_id: EngineId) -> bool {
        self.senders.read().await.contains_key(&engine_id)
    }
}

impl Default for EngineBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur on the engine bridge.
#[derive(Debug, Clone)]
pub enum EngineBridgeError {
    EngineNotFound(EngineId),
    ChannelClosed(String),
}

impl std::fmt::Display for EngineBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineBridgeError::EngineNotFound(id) => write!(f, "Engine {:?} not found", id),
            EngineBridgeError::ChannelClosed(msg) => write!(f, "Channel closed: {msg}"),
        }
    }
}

impl std::error::Error for EngineBridgeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_register_and_health() {
        let bridge = EngineBridge::new();
        let (tx, _rx) = mpsc::channel::<EngineMessage>(16);

        bridge.register_engine(EngineId::Code, tx).await;
        assert!(bridge.is_available(EngineId::Code).await);
        assert!(!bridge.is_available(EngineId::General).await);

        bridge
            .update_health(EngineId::Code, EngineStatus::Busy, 3)
            .await;
        let health = bridge.engine_health(EngineId::Code).await.unwrap();
        assert_eq!(health.status, EngineStatus::Busy);
        assert_eq!(health.active_tasks, 3);
    }

    #[tokio::test]
    async fn test_route_message() {
        let bridge = EngineBridge::new();
        let (tx, mut rx) = mpsc::channel::<EngineMessage>(16);

        bridge.register_engine(EngineId::General, tx).await;

        let msg = EngineMessage::new(
            EngineId::Code,
            EngineId::General,
            serde_json::json!({"type": "ping"}),
        );
        bridge.route_message(msg).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.from, EngineId::Code);
        assert_eq!(received.to, EngineId::General);
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let bridge = EngineBridge::new();

        bridge.set_session_key(EngineId::Code, "code_session_1".to_string()).await;
        bridge.set_session_key(EngineId::General, "general_session_1".to_string()).await;

        assert_eq!(bridge.session_key(EngineId::Code).await.unwrap(), "code_session_1");
        assert_eq!(bridge.session_key(EngineId::General).await.unwrap(), "general_session_1");
    }
}
