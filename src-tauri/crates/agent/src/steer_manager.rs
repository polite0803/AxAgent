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
        let pending: Vec<SteerMessage> = queue
            .iter()
            .filter(|m| !m.consumed)
            .cloned()
            .collect();
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
