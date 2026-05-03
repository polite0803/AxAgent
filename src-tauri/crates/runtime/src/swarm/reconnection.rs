//! 断线重连逻辑
//! 队友断开后可自动重连

use std::time::{Duration, Instant};

/// 重连状态跟踪器
///
/// 记录重连尝试次数、上次尝试时间，并根据配置判断是否应该继续尝试重连。
#[derive(Debug, Clone)]
pub struct ReconnectionState {
    /// 已尝试次数
    pub attempts: u32,
    /// 最大尝试次数
    pub max_attempts: u32,
    /// 上次尝试的时间
    pub last_attempt: Option<Instant>,
    /// 重连间隔
    pub interval: Duration,
}

impl ReconnectionState {
    /// 使用默认常量创建新的重连状态
    pub fn new() -> Self {
        Self {
            attempts: 0,
            max_attempts: super::constants::MAX_RECONNECT_ATTEMPTS,
            last_attempt: None,
            interval: Duration::from_secs(super::constants::RECONNECT_INTERVAL_SECS),
        }
    }

    /// 是否应该尝试重连
    ///
    /// 检查两个条件：
    /// 1. 未超过最大重连次数
    /// 2. 距离上次尝试已超过重连间隔
    pub fn should_reconnect(&self) -> bool {
        if self.attempts >= self.max_attempts {
            return false;
        }
        match self.last_attempt {
            Some(last) => last.elapsed() >= self.interval,
            None => true,
        }
    }

    /// 记录一次重连尝试
    pub fn record_attempt(&mut self) {
        self.attempts += 1;
        self.last_attempt = Some(Instant::now());
    }

    /// 重置重连状态（成功连接后调用）
    pub fn reset(&mut self) {
        self.attempts = 0;
        self.last_attempt = None;
    }

    /// 是否已耗尽重连次数
    pub fn is_exhausted(&self) -> bool {
        self.attempts >= self.max_attempts
    }
}

impl Default for ReconnectionState {
    fn default() -> Self {
        Self::new()
    }
}
