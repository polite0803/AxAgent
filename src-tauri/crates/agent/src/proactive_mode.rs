// SPDX-License-Identifier: AGPL-3.0-only

//! 主动模式 — 用户空闲时自动注入 tick 提示词
//! Feature flag: PROACTIVE_MODE

use axagent_harness::SharedFeatureFlagProvider;
use std::time::{Duration, Instant};

/// 主动模式管理器
pub struct ProactiveMode {
    /// 当前状态
    state: ProactiveState,
    /// tick 间隔
    tick_interval: Duration,
    /// 最后一次用户输入时间
    last_user_input: Instant,
    /// 最后一次 API 错误时间
    last_api_error: Option<Instant>,
    /// tick 计数
    tick_count: u64,
    /// Feature flag 提供者
    feature_flags: SharedFeatureFlagProvider,
}

/// 主动模式状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProactiveState {
    /// 未激活
    Inactive,
    /// 活跃中
    Active,
    /// 已暂停
    Paused {
        /// 暂停原因
        reason: PauseReason,
    },
}

/// 暂停原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseReason {
    /// 用户正在输入
    UserTyping,
    /// API 错误
    ApiError,
    /// 手动暂停
    ManualPause,
    /// 上下文被阻塞
    ContextBlocked,
}

impl ProactiveMode {
    /// 创建新的主动模式实例
    pub fn new(feature_flags: SharedFeatureFlagProvider) -> Self {
        let tick_interval = std::env::var("AXAGENT_PROACTIVE_TICK_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));

        Self {
            state: ProactiveState::Inactive,
            tick_interval,
            last_user_input: Instant::now(),
            last_api_error: None,
            tick_count: 0,
            feature_flags,
        }
    }

    /// 是否启用（检查 feature flag）— 无实例时的静态检查
    pub fn is_enabled_static(provider: &dyn axagent_harness::FeatureFlagProvider) -> bool {
        provider.is_enabled("PROACTIVE_MODE")
    }

    /// 是否启用（检查 feature flag）— 实例方法
    pub fn is_enabled(&self) -> bool {
        self.feature_flags.is_enabled("PROACTIVE_MODE")
    }

    /// 激活主动模式
    pub fn activate(&mut self) {
        if self.is_enabled() {
            self.state = ProactiveState::Active;
            self.last_user_input = Instant::now();
        }
    }

    /// 停用主动模式
    pub fn deactivate(&mut self) {
        self.state = ProactiveState::Inactive;
    }

    /// 暂停主动模式
    pub fn pause(&mut self, reason: PauseReason) {
        if self.state == ProactiveState::Active {
            self.state = ProactiveState::Paused { reason };
        }
    }

    /// 恢复主动模式
    pub fn resume(&mut self) {
        if matches!(self.state, ProactiveState::Paused { .. }) {
            self.state = ProactiveState::Active;
            self.last_user_input = Instant::now();
        }
    }

    /// 用户输入事件 — 自动暂停
    pub fn on_user_input(&mut self) {
        self.last_user_input = Instant::now();
        if self.state == ProactiveState::Active {
            self.pause(PauseReason::UserTyping);
        }
    }

    /// API 错误事件
    pub fn on_api_error(&mut self) {
        self.last_api_error = Some(Instant::now());
        self.pause(PauseReason::ApiError);
    }

    /// 检查是否应该发出 tick
    pub fn should_tick(&self) -> bool {
        if self.state != ProactiveState::Active {
            return false;
        }
        // 最近有 API 错误则跳过（5 分钟内）
        if let Some(err_time) = self.last_api_error
            && err_time.elapsed() < Duration::from_secs(300)
        {
            return false;
        }
        self.last_user_input.elapsed() >= self.tick_interval
    }

    /// 生成 tick 提示词
    pub fn build_tick_prompt(&self) -> String {
        let now = chrono::Local::now();
        format!("<tick>{}</tick>", now.format("%H:%M:%S"))
    }

    /// 获取当前状态
    pub fn state(&self) -> ProactiveState {
        self.state
    }

    /// 获取 tick 计数
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// 记录一次 tick
    pub fn record_tick(&mut self) {
        self.tick_count += 1;
    }

    /// 是否处于活跃状态
    pub fn is_active(&self) -> bool {
        self.state == ProactiveState::Active
    }
}

// ── 测试用 FeatureFlagProvider ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// 测试用 FeatureFlagProvider，通过 AtomicBool 控制开关状态。
    struct TestFeatureFlags {
        enabled: AtomicBool,
    }

    impl TestFeatureFlags {
        fn new(initial: bool) -> Self {
            Self {
                enabled: AtomicBool::new(initial),
            }
        }
    }

    impl axagent_harness::FeatureFlagProvider for TestFeatureFlags {
        fn is_enabled(&self, _name: &str) -> bool {
            self.enabled.load(Ordering::SeqCst)
        }

        fn enable(&self, _name: &str) {
            self.enabled.store(true, Ordering::SeqCst);
        }
    }

    fn make_flags(enabled: bool) -> SharedFeatureFlagProvider {
        Arc::new(TestFeatureFlags::new(enabled))
    }

    fn make_mode(enabled: bool) -> ProactiveMode {
        ProactiveMode::new(make_flags(enabled))
    }

    #[test]
    fn test_proactive_state_equality() {
        assert_eq!(ProactiveState::Inactive, ProactiveState::Inactive);
        assert_eq!(ProactiveState::Active, ProactiveState::Active);
        assert_ne!(ProactiveState::Active, ProactiveState::Inactive);
    }

    #[test]
    fn test_proactive_state_paused_equality() {
        let paused_typing = ProactiveState::Paused {
            reason: PauseReason::UserTyping,
        };
        let paused_api = ProactiveState::Paused {
            reason: PauseReason::ApiError,
        };
        assert_eq!(
            paused_typing,
            ProactiveState::Paused {
                reason: PauseReason::UserTyping
            }
        );
        assert_ne!(paused_typing, paused_api);
    }

    #[test]
    fn test_pause_reason_variants() {
        let reasons = [
            PauseReason::UserTyping,
            PauseReason::ApiError,
            PauseReason::ManualPause,
            PauseReason::ContextBlocked,
        ];
        assert_eq!(reasons.len(), 4);
    }

    #[test]
    fn test_new_proactive_mode() {
        let mode = make_mode(false);
        assert_eq!(mode.state(), ProactiveState::Inactive);
        assert_eq!(mode.tick_count(), 0);
        assert!(!mode.is_active());
    }

    #[test]
    fn test_activate() {
        let mut mode = make_mode(true);
        mode.activate();
        assert_eq!(mode.state(), ProactiveState::Active);
        assert!(mode.is_active());
    }

    #[test]
    fn test_deactivate() {
        let mut mode = make_mode(false);
        mode.activate();
        mode.deactivate();
        assert_eq!(mode.state(), ProactiveState::Inactive);
        assert!(!mode.is_active());
    }

    #[test]
    fn test_pause_from_active() {
        let mut mode = make_mode(true);
        mode.activate();
        mode.pause(PauseReason::ManualPause);
        assert_eq!(
            mode.state(),
            ProactiveState::Paused {
                reason: PauseReason::ManualPause
            }
        );
    }

    #[test]
    fn test_pause_from_inactive_no_effect() {
        let mut mode = make_mode(false);
        mode.pause(PauseReason::ManualPause);
        assert_eq!(mode.state(), ProactiveState::Inactive);
    }

    #[test]
    fn test_resume_from_paused() {
        let mut mode = make_mode(true);
        mode.activate();
        mode.pause(PauseReason::ApiError);
        mode.resume();
        assert_eq!(mode.state(), ProactiveState::Active);
    }

    #[test]
    fn test_resume_from_inactive_no_effect() {
        let mut mode = make_mode(false);
        mode.resume();
        assert_eq!(mode.state(), ProactiveState::Inactive);
    }

    #[test]
    fn test_on_user_input_pauses_active() {
        let mut mode = make_mode(true);
        mode.activate();
        mode.on_user_input();
        assert_eq!(
            mode.state(),
            ProactiveState::Paused {
                reason: PauseReason::UserTyping
            }
        );
    }

    #[test]
    fn test_on_user_input_inactive_no_effect() {
        let mut mode = make_mode(false);
        mode.on_user_input();
        assert_eq!(mode.state(), ProactiveState::Inactive);
    }

    #[test]
    fn test_on_api_error_pauses_active() {
        let mut mode = make_mode(true);
        mode.activate();
        mode.on_api_error();
        assert_eq!(
            mode.state(),
            ProactiveState::Paused {
                reason: PauseReason::ApiError
            }
        );
    }

    #[test]
    fn test_on_api_error_inactive_still_pauses() {
        let mut mode = make_mode(false);
        mode.on_api_error();
        assert_eq!(mode.state(), ProactiveState::Inactive);
    }

    #[test]
    fn test_should_tick_inactive() {
        let mode = make_mode(false);
        assert!(!mode.should_tick());
    }

    #[test]
    fn test_should_tick_active() {
        let mut mode = make_mode(true);
        mode.activate();
        assert!(!mode.should_tick());
    }

    #[test]
    fn test_should_tick_paused() {
        let mut mode = make_mode(true);
        mode.activate();
        mode.pause(PauseReason::ManualPause);
        assert!(!mode.should_tick());
    }

    #[test]
    fn test_record_tick() {
        let mut mode = make_mode(false);
        assert_eq!(mode.tick_count(), 0);
        mode.record_tick();
        assert_eq!(mode.tick_count(), 1);
        mode.record_tick();
        assert_eq!(mode.tick_count(), 2);
    }

    #[test]
    fn test_build_tick_prompt() {
        let mode = make_mode(false);
        let prompt = mode.build_tick_prompt();
        assert!(prompt.starts_with("<tick>"));
        assert!(prompt.ends_with("</tick>"));
    }

    #[test]
    fn test_default_behavior() {
        let mode = make_mode(false);
        assert_eq!(mode.state(), ProactiveState::Inactive);
    }
}
