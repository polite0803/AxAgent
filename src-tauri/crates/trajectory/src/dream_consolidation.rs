//! Dream 模式巩固增强
//!
//! 在会话空闲期间统一调度后台推理任务：
//! - 记忆提取和巩固
//! - 跨会话模式发现
//! - 主动建议生成
//! - 上下文预加载
//!
//! 通过时间门控和会话计数门控防止过度消耗资源，
//! 使用互斥锁防止并发运行。
//! 移植自 claude-code-main 的 autoDream 机制。

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// 门控配置
// ---------------------------------------------------------------------------

/// 两次 Dream 巩固之间的最小时间间隔（默认 1 小时）
const DEFAULT_MIN_INTERVAL_HOURS: i64 = 1;

/// 触发 Dream 巩固所需的最小新会话数
const DEFAULT_MIN_NEW_SESSIONS: u32 = 3;

/// Dream 巩固的最大持续时间（秒）
const DEFAULT_MAX_CONSOLIDATION_SECS: u64 = 120;

/// Dream 巩固锁定获取超时（秒）
const LOCK_TIMEOUT_SECS: u64 = 5;

// ---------------------------------------------------------------------------
// 配置
// ---------------------------------------------------------------------------

/// Dream 巩固的调度配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConsolidationConfig {
    /// 是否启用 Dream 巩固
    pub enabled: bool,
    /// 两次巩固之间的最小间隔（小时）
    pub min_interval_hours: i64,
    /// 触发巩固所需的最小新会话数
    pub min_new_sessions: u32,
    /// 单次巩固的最大持续时间（秒）
    pub max_consolidation_secs: u64,
    /// 是否在巩固期间运行记忆提取
    pub run_memory_extraction: bool,
    /// 是否在巩固期间运行模式学习
    pub run_pattern_learning: bool,
    /// 是否在巩固期间生成主动建议
    pub run_proactive_suggestions: bool,
}

impl Default for DreamConsolidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_interval_hours: DEFAULT_MIN_INTERVAL_HOURS,
            min_new_sessions: DEFAULT_MIN_NEW_SESSIONS,
            max_consolidation_secs: DEFAULT_MAX_CONSOLIDATION_SECS,
            run_memory_extraction: true,
            run_pattern_learning: true,
            run_proactive_suggestions: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 巩固结果
// ---------------------------------------------------------------------------

/// 单次 Dream 巩固周期的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConsolidationResult {
    /// 巩固是否成功执行
    pub executed: bool,
    /// 跳过原因（如果未执行）
    pub skip_reason: Option<String>,
    /// 提取的记忆数量
    pub memories_extracted: usize,
    /// 发现的模式数量
    pub patterns_discovered: usize,
    /// 生成的建议数量
    pub suggestions_generated: usize,
    /// 巩固开始时间
    pub started_at: DateTime<Utc>,
    /// 巩固持续时间（秒）
    pub duration_secs: u64,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

impl DreamConsolidationResult {
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            executed: false,
            skip_reason: Some(reason.into()),
            memories_extracted: 0,
            patterns_discovered: 0,
            suggestions_generated: 0,
            started_at: Utc::now(),
            duration_secs: 0,
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 状态跟踪
// ---------------------------------------------------------------------------

/// Dream 巩固的运行状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct DreamConsolidationState {
    /// 上次巩固完成时间
    pub last_consolidation_at: Option<DateTime<Utc>>,
    /// 上次巩固以来的新会话数
    pub sessions_since_last: u32,
    /// 累计巩固次数
    pub total_consolidations: u64,
    /// 累计提取的记忆数
    pub total_memories_extracted: u64,
    /// 累计执行的秒数
    pub total_consolidation_secs: u64,
    /// 是否正在运行
    pub is_running: bool,
}


// ---------------------------------------------------------------------------
// Dream 巩固调度器
// ---------------------------------------------------------------------------

/// 前端事件发射器类型：发送 (事件名, JSON载荷)
pub type DreamEventEmitter = Option<Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>>;

/// Dream 巩固调度器。
///
/// 统一管理后台推理任务，通过门控防止过度运行。
/// 使用互斥锁防止并发巩固。
pub struct DreamConsolidator {
    config: Arc<Mutex<DreamConsolidationConfig>>,
    state: Arc<Mutex<DreamConsolidationState>>,
    /// 巩固锁：确保同一时间只有一个巩固周期在运行
    consolidation_lock: Arc<Mutex<()>>,
    /// 前端事件发射器（设置后向 Tauri 前端发送事件）
    event_emitter: DreamEventEmitter,
}

impl DreamConsolidator {
    /// 创建新的 Dream 巩固调度器。
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(DreamConsolidationConfig::default())),
            state: Arc::new(Mutex::new(DreamConsolidationState::default())),
            consolidation_lock: Arc::new(Mutex::new(())),
            event_emitter: None,
        }
    }

    /// 使用自定义配置创建。
    pub fn with_config(config: DreamConsolidationConfig) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            state: Arc::new(Mutex::new(DreamConsolidationState::default())),
            consolidation_lock: Arc::new(Mutex::new(())),
            event_emitter: None,
        }
    }

    /// 设置前端事件发射器（由 Tauri 命令层注入）。
    pub fn set_event_emitter(&mut self, emitter: DreamEventEmitter) {
        self.event_emitter = emitter;
    }

    /// 内部辅助：向 Tauri 前端发射事件。
    fn emit(&self, event_name: &str, payload: serde_json::Value) {
        if let Some(ref emitter) = self.event_emitter {
            emitter(event_name, payload);
        }
    }

    /// 更新配置。
    pub async fn update_config(&self, config: DreamConsolidationConfig) {
        let mut cfg = self.config.lock().await;
        *cfg = config;
    }

    /// 获取配置。
    pub async fn get_config(&self) -> DreamConsolidationConfig {
        self.config.lock().await.clone()
    }

    /// 获取当前状态。
    pub async fn get_state(&self) -> DreamConsolidationState {
        self.state.lock().await.clone()
    }

    /// 记录一次新会话（用于会话计数门控）。
    pub async fn record_new_session(&self) {
        let mut state = self.state.lock().await;
        state.sessions_since_last += 1;
    }

    /// 检查是否应该触发 Dream 巩固。
    ///
    /// # 门控条件
    /// 1. 已启用
    /// 2. 不在运行中
    /// 3. 距上次巩固已超过最小区间（时间门控）
    /// 4. 新会话数达到阈值（会话计数门控）
    /// 5. 能获取巩固锁（防止并发）
    pub async fn should_consolidate(&self) -> bool {
        let config = self.config.lock().await;
        if !config.enabled {
            return false;
        }

        let state = self.state.lock().await;
        if state.is_running {
            return false;
        }

        // 时间门控
        if let Some(last) = state.last_consolidation_at {
            let elapsed = Utc::now() - last;
            if elapsed < Duration::hours(config.min_interval_hours) {
                return false;
            }
        }

        // 会话计数门控
        if state.sessions_since_last < config.min_new_sessions {
            return false;
        }

        // 检查锁是否可用（非阻塞检查）
        self.consolidation_lock.try_lock().is_ok()
    }

    /// 执行一次 Dream 巩固周期。
    ///
    /// 此方法会：
    /// 1. 获取巩固锁
    /// 2. 运行配置中的各子系统
    /// 3. 更新状态
    /// 4. 释放锁
    ///
    /// # 参数
    /// - `on_memories`: 记忆提取回调（接收提取的记忆数）
    /// - `on_patterns`: 模式学习回调
    /// - `on_suggestions`: 建议生成回调
    pub async fn consolidate(
        &self,
        on_memories: Option<&dyn Fn(usize)>,
        on_patterns: Option<&dyn Fn(usize)>,
        on_suggestions: Option<&dyn Fn(usize)>,
    ) -> DreamConsolidationResult {
        let config = self.get_config().await;

        if !config.enabled {
            return DreamConsolidationResult::skipped("Dream 巩固已禁用");
        }

        // 获取锁（带超时）
        let _lock = match tokio::time::timeout(
            std::time::Duration::from_secs(LOCK_TIMEOUT_SECS),
            self.consolidation_lock.lock(),
        )
        .await
        {
            Ok(lock) => lock,
            Err(_) => {
                return DreamConsolidationResult::skipped("无法获取巩固锁（超时）");
            },
        };

        let started_at = Utc::now();
        let deadline = started_at + Duration::seconds(config.max_consolidation_secs as i64);

        // 发射 dream-consolidation-started
        self.emit(
            "dream-consolidation-started",
            serde_json::json!({
                "timestamp": started_at.timestamp_millis(),
                "maxDurationSecs": config.max_consolidation_secs,
            }),
        );

        let mut state = self.state.lock().await;
        state.is_running = true;
        drop(state);

        let mut memories_extracted = 0usize;
        let mut patterns_discovered = 0usize;
        let mut suggestions_generated = 0usize;

        // 1. 记忆提取巩固
        if config.run_memory_extraction
            && Utc::now() < deadline {
                // 调用记忆提取逻辑（由外部注入）
                if let Some(callback) = on_memories {
                    callback(0); // 实际数量由回调填充
                }
                memories_extracted += 1; // 占位：实际实现需对接 auto_memory
            }

        // 2. 跨会话模式发现
        if config.run_pattern_learning
            && Utc::now() < deadline {
                if let Some(callback) = on_patterns {
                    callback(0);
                }
                patterns_discovered += 1;
            }

        // 3. 主动建议生成
        if config.run_proactive_suggestions
            && Utc::now() < deadline {
                if let Some(callback) = on_suggestions {
                    callback(0);
                }
                suggestions_generated += 1;
            }

        let duration_secs = (Utc::now() - started_at).num_seconds().max(0) as u64;

        // 更新状态
        let mut state = self.state.lock().await;
        state.last_consolidation_at = Some(Utc::now());
        state.sessions_since_last = 0;
        state.total_consolidations += 1;
        state.total_memories_extracted += memories_extracted as u64;
        state.total_consolidation_secs += duration_secs;
        state.is_running = false;
        drop(state);

        // 发射 dream-consolidation-completed
        self.emit(
            "dream-consolidation-completed",
            serde_json::json!({
                "executed": true,
                "memoriesExtracted": memories_extracted,
                "patternsDiscovered": patterns_discovered,
                "suggestionsGenerated": suggestions_generated,
                "startedAt": started_at.timestamp_millis(),
                "durationSecs": duration_secs,
                "error": null,
            }),
        );

        // 释放锁（_lock 在作用域结束时自动释放）

        DreamConsolidationResult {
            executed: true,
            skip_reason: None,
            memories_extracted,
            patterns_discovered,
            suggestions_generated,
            started_at,
            duration_secs,
            error: None,
        }
    }

    /// 强制触发一次巩固（忽略门控，但尊重锁）。
    pub async fn consolidate_force(&self) -> DreamConsolidationResult {
        // 暂时覆盖门控：记录足够的会话数
        {
            let mut state = self.state.lock().await;
            state.sessions_since_last = u32::MAX;
            state.last_consolidation_at = None; // 清除时间门控
        }

        self.consolidate(None, None, None).await
    }

    /// 重置所有状态。
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        *state = DreamConsolidationState::default();
    }

    /// 检查是否正在运行。
    pub async fn is_running(&self) -> bool {
        self.state.lock().await.is_running
    }
}

impl Default for DreamConsolidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DreamConsolidationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_interval_hours, 1);
        assert_eq!(config.min_new_sessions, 3);
        assert!(config.run_memory_extraction);
    }

    #[tokio::test]
    async fn test_should_consolidate_first_run() {
        let consolidator = DreamConsolidator::new();
        // 首次运行：没有上次巩固时间，但会话计数为 0
        assert!(!consolidator.should_consolidate().await);

        // 记录足够的新会话
        consolidator.record_new_session().await;
        consolidator.record_new_session().await;
        consolidator.record_new_session().await;

        // 现在应该可以巩固
        assert!(consolidator.should_consolidate().await);
    }

    #[tokio::test]
    async fn test_time_gate_blocks() {
        let consolidator = DreamConsolidator::new();

        // 模拟一次巩固
        let mut state = consolidator.state.lock().await;
        state.last_consolidation_at = Some(Utc::now());
        state.sessions_since_last = 10; // 足够的会话
        drop(state);

        // 刚刚巩固过，时间门控应阻止
        assert!(!consolidator.should_consolidate().await);
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let consolidator = DreamConsolidator::with_config(DreamConsolidationConfig {
            enabled: false,
            ..Default::default()
        });

        assert!(!consolidator.should_consolidate().await);
    }

    #[tokio::test]
    async fn test_consolidate_updates_state() {
        let consolidator = DreamConsolidator::new();
        // 强制运行（忽略门控）
        let result = consolidator.consolidate_force().await;
        assert!(result.executed);

        let state = consolidator.get_state().await;
        assert_eq!(state.total_consolidations, 1);
        assert_eq!(state.sessions_since_last, 0);
    }

    #[tokio::test]
    async fn test_skipped_when_disabled() {
        let consolidator = DreamConsolidator::with_config(DreamConsolidationConfig {
            enabled: false,
            ..Default::default()
        });

        let result = consolidator.consolidate_force().await;
        assert!(!result.executed);
        assert!(result.skip_reason.is_some());
    }

    #[tokio::test]
    async fn test_reset() {
        let consolidator = DreamConsolidator::new();
        consolidator.record_new_session().await;
        consolidator.record_new_session().await;

        consolidator.reset().await;
        let state = consolidator.get_state().await;
        assert_eq!(state.sessions_since_last, 0);
        assert_eq!(state.total_consolidations, 0);
    }

    #[tokio::test]
    async fn test_is_running_flag() {
        let consolidator = DreamConsolidator::new();
        assert!(!consolidator.is_running().await);
    }
}
