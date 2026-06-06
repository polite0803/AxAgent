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

    /// 严重度排序：Hard(3) > Graceful(2) > Soft(1)
    /// 数值越大代表中断意图越强，越不应被后续请求覆盖
    fn severity(self) -> u8 {
        match self {
            Self::Soft => 1,
            Self::Graceful => 2,
            Self::Hard => 3,
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
    /// 当前中断周期内的最高严重度级别；用于防止后续低级别请求将状态机降级
    max_level: Arc<RwLock<Option<InterruptLevel>>>,
    notify: Arc<Notify>,
    auto_recovery: bool,
}

impl InterruptManager {
    pub fn new(auto_recovery: bool) -> Self {
        Self {
            state: Arc::new(RwLock::new(InterruptState::None)),
            pending: Arc::new(RwLock::new(None)),
            max_level: Arc::new(RwLock::new(None)),
            notify: Arc::new(Notify::new()),
            auto_recovery,
        }
    }

    /// 提交一次中断请求
    ///
    /// 严重度规则：仅当新请求的级别严格高于当前周期内已记录的最高级别时才覆盖状态；
    /// 否则忽略低优先级请求，避免高级别中断意图被"降级"。
    /// 历史最高级别保留在 `max_level` 中，便于 audit。
    pub async fn request(&self, level: InterruptLevel, reason: Option<String>) {
        // 读取当前周期内已记录的最高级别（短锁后立即释放）
        let current_max = *self.max_level.read().await;
        let should_override = match current_max {
            Some(existing) => level.severity() > existing.severity(),
            None => true,
        };

        if !should_override {
            // 此时 current_max 必然是 Some(existing)
            let existing = current_max.expect("max_level should be Some when not overriding");
            // 低级别请求被忽略，但保留历史最高级别，便于 audit
            tracing::warn!(
                "忽略较低级别的中断请求：requested={}, current_max={}",
                level.as_str(),
                existing.as_str()
            );
            return;
        }

        let request = InterruptRequest {
            level,
            reason,
            timestamp: chrono::Utc::now(),
        };
        *self.pending.write().await = Some(request);
        *self.state.write().await = InterruptState::Pending(level);
        *self.max_level.write().await = Some(level);
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
        // 周期结束，重置最高级别记录
        *self.max_level.write().await = None;
    }

    pub async fn recover(&self) {
        *self.state.write().await = InterruptState::None;
        *self.pending.write().await = None;
        *self.max_level.write().await = None;
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

    /// 验证严重度防降级：高级别中断请求不可被后续低级别请求覆盖
    /// 修复缺陷 2.9：多次中断级别降级
    #[tokio::test]
    async fn test_higher_level_overrides_lower() {
        // 场景 1: Hard 在前，Soft 后续请求被忽略（防降级）
        let manager = InterruptManager::new(false);
        manager.hard_stop().await;
        manager.soft_stop().await;
        assert_eq!(manager.state().await, InterruptState::Pending(InterruptLevel::Hard));
        let req = manager.check().await.unwrap();
        assert_eq!(req.level, InterruptLevel::Hard);
        // Hard 不保留会话语义在防降级后仍生效
        assert!(!manager.should_preserve_session().await);
        // 任意级别 Pending 时均应停止当前 turn
        assert!(manager.should_stop_current_turn().await);

        // 场景 2: Graceful 在前，Soft 后续请求被忽略
        let manager2 = InterruptManager::new(false);
        manager2.graceful_stop().await;
        manager2.soft_stop().await;
        assert_eq!(manager2.state().await, InterruptState::Pending(InterruptLevel::Graceful));

        // 场景 3: Soft 在前，Hard 后续请求可升级
        let manager3 = InterruptManager::new(false);
        manager3.soft_stop().await;
        manager3.hard_stop().await;
        assert_eq!(manager3.state().await, InterruptState::Pending(InterruptLevel::Hard));

        // 场景 4: Graceful 在前，Hard 后续请求可升级
        let manager4 = InterruptManager::new(false);
        manager4.graceful_stop().await;
        manager4.hard_stop().await;
        assert_eq!(manager4.state().await, InterruptState::Pending(InterruptLevel::Hard));

        // 场景 5: 同级别（Hard -> Hard）不被覆盖为更低，且状态保持 Hard
        let manager5 = InterruptManager::new(false);
        manager5.hard_stop().await;
        manager5.soft_stop().await;
        manager5.graceful_stop().await;
        assert_eq!(manager5.state().await, InterruptState::Pending(InterruptLevel::Hard));

        // 场景 6: 周期结束后（complete）新周期从空开始，max_level 被重置
        let manager6 = InterruptManager::new(false);
        manager6.hard_stop().await;
        manager6.begin_processing().await;
        manager6.complete().await;
        // 周期结束后新请求应能正常进入新周期
        manager6.soft_stop().await;
        assert_eq!(manager6.state().await, InterruptState::Pending(InterruptLevel::Soft));
    }
}
