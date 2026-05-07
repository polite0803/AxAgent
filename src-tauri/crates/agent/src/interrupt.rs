use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptLevel {
    Soft,
    Hard,
    Graceful,
}

impl InterruptLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Hard => "hard",
            Self::Graceful => "graceful",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptRequest {
    pub level: InterruptLevel,
    pub reason: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptState {
    None,
    Pending(InterruptLevel),
    Processing,
    Completed,
    Recovering,
}

pub struct InterruptManager {
    state: Arc<RwLock<InterruptState>>,
    pending: Arc<RwLock<Option<InterruptRequest>>>,
    notify: Arc<Notify>,
    auto_recovery: bool,
}

impl InterruptManager {
    pub fn new(auto_recovery: bool) -> Self {
        Self {
            state: Arc::new(RwLock::new(InterruptState::None)),
            pending: Arc::new(RwLock::new(None)),
            notify: Arc::new(Notify::new()),
            auto_recovery,
        }
    }

    pub async fn request(&self, level: InterruptLevel, reason: Option<String>) {
        let request = InterruptRequest {
            level,
            reason,
            timestamp: chrono::Utc::now(),
        };
        *self.pending.write().await = Some(request);
        *self.state.write().await = InterruptState::Pending(level);
        self.notify.notify_one();
        tracing::info!("Interrupt requested: level={}", level.as_str());
    }

    pub async fn check(&self) -> Option<InterruptRequest> {
        self.pending.read().await.clone()
    }

    pub async fn should_stop_current_turn(&self) -> bool {
        let state = self.state.read().await;
        matches!(
            *state,
            InterruptState::Pending(InterruptLevel::Soft)
                | InterruptState::Pending(InterruptLevel::Hard)
                | InterruptState::Pending(InterruptLevel::Graceful)
        )
    }

    pub async fn should_preserve_session(&self) -> bool {
        let pending = self.pending.read().await;
        matches!(
            pending.as_ref().map(|p| p.level),
            Some(InterruptLevel::Soft) | Some(InterruptLevel::Graceful)
        )
    }

    pub async fn begin_processing(&self) {
        *self.state.write().await = InterruptState::Processing;
    }

    pub async fn complete(&self) {
        if self.auto_recovery {
            *self.state.write().await = InterruptState::Recovering;
            tracing::info!("Interrupt completed, auto-recovery enabled");
        } else {
            *self.state.write().await = InterruptState::Completed;
        }
        *self.pending.write().await = None;
    }

    pub async fn recover(&self) {
        *self.state.write().await = InterruptState::None;
        *self.pending.write().await = None;
        tracing::info!("Interrupt recovery completed");
    }

    pub async fn state(&self) -> InterruptState {
        *self.state.read().await
    }

    pub fn notified(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    pub async fn soft_stop(&self) {
        self.request(InterruptLevel::Soft, Some("User requested soft stop".to_string()))
            .await;
    }

    pub async fn hard_stop(&self) {
        self.request(InterruptLevel::Hard, Some("User requested hard stop".to_string()))
            .await;
    }

    pub async fn graceful_stop(&self) {
        self.request(InterruptLevel::Graceful, Some("User requested graceful stop".to_string()))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_level_as_str() {
        assert_eq!(InterruptLevel::Soft.as_str(), "soft");
        assert_eq!(InterruptLevel::Hard.as_str(), "hard");
        assert_eq!(InterruptLevel::Graceful.as_str(), "graceful");
    }

    #[test]
    fn test_interrupt_level_equality() {
        assert_eq!(InterruptLevel::Soft, InterruptLevel::Soft);
        assert_ne!(InterruptLevel::Soft, InterruptLevel::Hard);
        assert_ne!(InterruptLevel::Hard, InterruptLevel::Graceful);
    }

    #[test]
    fn test_interrupt_state_equality() {
        assert_eq!(InterruptState::None, InterruptState::None);
        assert_eq!(InterruptState::Processing, InterruptState::Processing);
        assert_eq!(InterruptState::Completed, InterruptState::Completed);
        assert_eq!(InterruptState::Recovering, InterruptState::Recovering);
        assert_eq!(
            InterruptState::Pending(InterruptLevel::Soft),
            InterruptState::Pending(InterruptLevel::Soft)
        );
        assert_ne!(
            InterruptState::Pending(InterruptLevel::Soft),
            InterruptState::Pending(InterruptLevel::Hard)
        );
    }

    #[test]
    fn test_interrupt_request_serialization() {
        let req = InterruptRequest {
            level: InterruptLevel::Soft,
            reason: Some("test".to_string()),
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: InterruptRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.level, InterruptLevel::Soft);
        assert_eq!(deserialized.reason, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_interrupt_manager_new() {
        let manager = InterruptManager::new(true);
        assert_eq!(manager.state().await, InterruptState::None);
        assert!(manager.check().await.is_none());
    }

    #[tokio::test]
    async fn test_interrupt_manager_request_soft() {
        let manager = InterruptManager::new(false);
        manager
            .request(InterruptLevel::Soft, Some("test".to_string()))
            .await;
        assert_eq!(manager.state().await, InterruptState::Pending(InterruptLevel::Soft));
        let req = manager.check().await.unwrap();
        assert_eq!(req.level, InterruptLevel::Soft);
        assert_eq!(req.reason, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_interrupt_manager_request_hard() {
        let manager = InterruptManager::new(false);
        manager.hard_stop().await;
        assert_eq!(manager.state().await, InterruptState::Pending(InterruptLevel::Hard));
    }

    #[tokio::test]
    async fn test_interrupt_manager_request_graceful() {
        let manager = InterruptManager::new(false);
        manager.graceful_stop().await;
        assert_eq!(manager.state().await, InterruptState::Pending(InterruptLevel::Graceful));
    }

    #[tokio::test]
    async fn test_interrupt_manager_should_stop_current_turn() {
        let manager = InterruptManager::new(false);
        assert!(!manager.should_stop_current_turn().await);
        manager.soft_stop().await;
        assert!(manager.should_stop_current_turn().await);
    }

    #[tokio::test]
    async fn test_interrupt_manager_should_preserve_session() {
        let manager = InterruptManager::new(false);
        manager.soft_stop().await;
        assert!(manager.should_preserve_session().await);
    }

    #[tokio::test]
    async fn test_interrupt_manager_should_not_preserve_session_on_hard() {
        let manager = InterruptManager::new(false);
        manager.hard_stop().await;
        assert!(!manager.should_preserve_session().await);
    }

    #[tokio::test]
    async fn test_interrupt_manager_begin_processing() {
        let manager = InterruptManager::new(false);
        manager.soft_stop().await;
        manager.begin_processing().await;
        assert_eq!(manager.state().await, InterruptState::Processing);
    }

    #[tokio::test]
    async fn test_interrupt_manager_complete_no_auto_recovery() {
        let manager = InterruptManager::new(false);
        manager.soft_stop().await;
        manager.begin_processing().await;
        manager.complete().await;
        assert_eq!(manager.state().await, InterruptState::Completed);
        assert!(manager.check().await.is_none());
    }

    #[tokio::test]
    async fn test_interrupt_manager_complete_with_auto_recovery() {
        let manager = InterruptManager::new(true);
        manager.soft_stop().await;
        manager.begin_processing().await;
        manager.complete().await;
        assert_eq!(manager.state().await, InterruptState::Recovering);
    }

    #[tokio::test]
    async fn test_interrupt_manager_recover() {
        let manager = InterruptManager::new(true);
        manager.soft_stop().await;
        manager.begin_processing().await;
        manager.complete().await;
        manager.recover().await;
        assert_eq!(manager.state().await, InterruptState::None);
        assert!(manager.check().await.is_none());
    }

    #[tokio::test]
    async fn test_interrupt_manager_soft_stop() {
        let manager = InterruptManager::new(false);
        manager.soft_stop().await;
        let req = manager.check().await.unwrap();
        assert_eq!(req.level, InterruptLevel::Soft);
        assert!(req.reason.is_some());
    }
}
