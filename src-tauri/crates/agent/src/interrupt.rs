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
        self.request(
            InterruptLevel::Soft,
            Some("User requested soft stop".to_string()),
        )
        .await;
    }

    pub async fn hard_stop(&self) {
        self.request(
            InterruptLevel::Hard,
            Some("User requested hard stop".to_string()),
        )
        .await;
    }

    pub async fn graceful_stop(&self) {
        self.request(
            InterruptLevel::Graceful,
            Some("User requested graceful stop".to_string()),
        )
        .await;
    }
}
