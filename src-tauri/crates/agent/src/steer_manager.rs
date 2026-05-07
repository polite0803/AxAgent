use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteerMessage {
    pub id: String,
    pub instruction: String,
    pub injected_at: chrono::DateTime<chrono::Utc>,
    pub consumed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SteerInjectionPoint {
    AfterToolCall,
    BeforeNextLlmCall,
    Immediate,
}

pub struct SteerManager {
    queue: Arc<RwLock<Vec<SteerMessage>>>,
    injection_point: Arc<RwLock<SteerInjectionPoint>>,
}

impl Default for SteerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SteerManager {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(Vec::new())),
            injection_point: Arc::new(RwLock::new(SteerInjectionPoint::AfterToolCall)),
        }
    }

    pub async fn push(&self, instruction: String) -> SteerMessage {
        let msg = SteerMessage {
            id: uuid::Uuid::new_v4().to_string(),
            instruction,
            injected_at: chrono::Utc::now(),
            consumed: false,
        };
        self.queue.write().await.push(msg.clone());
        tracing::info!("Steer message queued: {}", msg.id);
        msg
    }

    pub async fn drain_pending(&self) -> Vec<SteerMessage> {
        let mut queue = self.queue.write().await;
        let pending: Vec<SteerMessage> = queue.iter().filter(|m| !m.consumed).cloned().collect();
        for msg in queue.iter_mut() {
            msg.consumed = true;
        }
        queue.retain(|m| !m.consumed);
        pending
    }

    pub async fn format_steer_block(&self) -> Option<String> {
        let pending = self.drain_pending().await;
        if pending.is_empty() {
            return None;
        }
        let instructions: Vec<String> = pending
            .iter()
            .map(|m| format!("- [{}] {}", m.id, m.instruction))
            .collect();
        Some(format!(
            "<steer-instructions type=\"temporary\">\n{}\n</steer-instructions>",
            instructions.join("\n")
        ))
    }

    pub async fn has_pending(&self) -> bool {
        self.queue.read().await.iter().any(|m| !m.consumed)
    }

    pub async fn set_injection_point(&self, point: SteerInjectionPoint) {
        *self.injection_point.write().await = point;
    }

    pub async fn clear(&self) {
        self.queue.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_steer_manager_new() {
        let manager = SteerManager::new();
        assert!(!manager.has_pending().await);
    }

    #[tokio::test]
    async fn test_steer_manager_default() {
        let manager = SteerManager::default();
        assert!(!manager.has_pending().await);
    }

    #[tokio::test]
    async fn test_steer_manager_push() {
        let manager = SteerManager::new();
        let msg = manager.push("test instruction".to_string()).await;
        assert!(!msg.id.is_empty());
        assert_eq!(msg.instruction, "test instruction");
        assert!(!msg.consumed);
        assert!(manager.has_pending().await);
    }

    #[tokio::test]
    async fn test_steer_manager_drain_pending_empty() {
        let manager = SteerManager::new();
        let pending = manager.drain_pending().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_steer_manager_drain_pending_with_messages() {
        let manager = SteerManager::new();
        manager.push("instruction 1".to_string()).await;
        manager.push("instruction 2".to_string()).await;
        let pending = manager.drain_pending().await;
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].instruction, "instruction 1");
        assert_eq!(pending[1].instruction, "instruction 2");
    }

    #[tokio::test]
    async fn test_steer_manager_drain_pending_marks_consumed() {
        let manager = SteerManager::new();
        manager.push("instruction".to_string()).await;
        let _ = manager.drain_pending().await;
        assert!(!manager.has_pending().await);
    }

    #[tokio::test]
    async fn test_steer_manager_drain_pending_twice() {
        let manager = SteerManager::new();
        manager.push("instruction".to_string()).await;
        let first = manager.drain_pending().await;
        assert_eq!(first.len(), 1);
        let second = manager.drain_pending().await;
        assert!(second.is_empty());
    }

    #[tokio::test]
    async fn test_steer_manager_format_steer_block_empty() {
        let manager = SteerManager::new();
        let result = manager.format_steer_block().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_steer_manager_format_steer_block_with_messages() {
        let manager = SteerManager::new();
        manager.push("do something".to_string()).await;
        let result = manager.format_steer_block().await;
        assert!(result.is_some());
        let block = result.unwrap();
        assert!(block.contains("<steer-instructions"));
        assert!(block.contains("do something"));
        assert!(block.contains("</steer-instructions>"));
    }

    #[tokio::test]
    async fn test_steer_manager_format_steer_block_drains() {
        let manager = SteerManager::new();
        manager.push("instruction".to_string()).await;
        let _ = manager.format_steer_block().await;
        assert!(!manager.has_pending().await);
    }

    #[tokio::test]
    async fn test_steer_manager_has_pending_after_push() {
        let manager = SteerManager::new();
        assert!(!manager.has_pending().await);
        manager.push("test".to_string()).await;
        assert!(manager.has_pending().await);
    }

    #[tokio::test]
    async fn test_steer_manager_set_injection_point() {
        let manager = SteerManager::new();
        manager
            .set_injection_point(SteerInjectionPoint::Immediate)
            .await;
        manager
            .set_injection_point(SteerInjectionPoint::BeforeNextLlmCall)
            .await;
        manager
            .set_injection_point(SteerInjectionPoint::AfterToolCall)
            .await;
    }

    #[tokio::test]
    async fn test_steer_manager_clear() {
        let manager = SteerManager::new();
        manager.push("instruction 1".to_string()).await;
        manager.push("instruction 2".to_string()).await;
        assert!(manager.has_pending().await);
        manager.clear().await;
        assert!(!manager.has_pending().await);
    }

    #[tokio::test]
    async fn test_steer_message_serialization() {
        let msg = SteerMessage {
            id: "test-id".to_string(),
            instruction: "test instruction".to_string(),
            injected_at: chrono::Utc::now(),
            consumed: false,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SteerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.instruction, "test instruction");
        assert!(!deserialized.consumed);
    }

    #[test]
    fn test_steer_injection_point_serialization() {
        let point = SteerInjectionPoint::AfterToolCall;
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: SteerInjectionPoint = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, SteerInjectionPoint::AfterToolCall));

        let point = SteerInjectionPoint::BeforeNextLlmCall;
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: SteerInjectionPoint = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, SteerInjectionPoint::BeforeNextLlmCall));

        let point = SteerInjectionPoint::Immediate;
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: SteerInjectionPoint = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, SteerInjectionPoint::Immediate));
    }

    #[tokio::test]
    async fn test_steer_manager_push_id_uniqueness() {
        let manager = SteerManager::new();
        let msg1 = manager.push("instruction 1".to_string()).await;
        let msg2 = manager.push("instruction 2".to_string()).await;
        assert_ne!(msg1.id, msg2.id);
    }
}
