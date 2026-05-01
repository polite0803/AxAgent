use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// 缓存断点检测
// 移植自 claude-code-main 的 prompt cache break detection
// ---------------------------------------------------------------------------

/// 缓存断点检测的阈值：实际缓存读取 token 低于预期的比例触发告警
const CACHE_BREAK_RATIO_THRESHOLD: f64 = 0.5;

/// 缓存断点检测的最小预期 token 数（低于此值不触发检测）
const MIN_EXPECTED_TOKENS_FOR_DETECTION: u64 = 100;

/// 记录单次缓存读取事件，用于断点分析。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheReadEvent {
    /// 事件发生时间
    pub timestamp_ms: u64,
    /// 预期的缓存读取 token 数
    pub expected_tokens: u64,
    /// API 报告的实际缓存读取 token 数
    pub actual_tokens: u64,
    /// 我们是否主动失效了缓存（如压缩、编辑）
    pub self_invalidated: bool,
    /// 失效原因（如果 self_invalidated）
    pub invalidation_reason: Option<String>,
    /// 是否检测为异常断点
    pub is_anomaly: bool,
}

impl CacheReadEvent {
    /// 判断是否为缓存异常断点。
    ///
    /// 条件：
    /// - 预期 token 数 >= 最小检测阈值
    /// - 非主动失效
    /// - 实际 token 数 < 预期的 50%
    pub fn is_cache_break(&self) -> bool {
        if self.self_invalidated {
            return false;
        }
        if self.expected_tokens < MIN_EXPECTED_TOKENS_FOR_DETECTION {
            return false;
        }
        if self.expected_tokens == 0 {
            return false;
        }
        let ratio = self.actual_tokens as f64 / self.expected_tokens as f64;
        ratio < CACHE_BREAK_RATIO_THRESHOLD
    }
}

/// 缓存断点分析摘要。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheBreakSummary {
    /// 总缓存读取事件数
    pub total_reads: u64,
    /// 异常的缓存断点次数
    pub anomaly_count: u64,
    /// 主动失效次数（正常）
    pub self_invalidation_count: u64,
    /// 最近一次异常事件
    pub last_anomaly: Option<CacheReadEvent>,
    /// 累计节省的 token 估计
    pub total_tokens_saved: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptCacheState {
    pub system_prompt_hash: Option<String>,
    pub tools_hash: Option<String>,
    pub memory_hash: Option<String>,
    pub context_files_hash: Option<String>,
    pub cache_valid: bool,
    pub last_invalidation_reason: Option<String>,
    pub pending_changes: Vec<PendingChange>,
    pub tokens_saved_estimate: u64,
    pub cache_hits: u64,
    // 缓存断点检测字段
    /// 预期的缓存读取 token 数（基于 prompt 结构估算）
    pub expected_cache_read_tokens: u64,
    /// 最近 N 次缓存读取事件的环形缓冲区
    pub recent_read_events: Vec<CacheReadEvent>,
    /// 断点分析摘要
    pub break_summary: CacheBreakSummary,
    /// 是否启用了缓存断点检测
    pub break_detection_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    pub component: String,
    pub description: String,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PromptCache {
    state: Arc<RwLock<PromptCacheState>>,
}

impl PromptCache {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(PromptCacheState::default())),
        }
    }

    pub async fn record_system_prompt(&self, content: &str) {
        let hash = compute_hash(content);
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.system_prompt_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "system_prompt".to_string(),
                description: "System prompt modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
            state.cache_valid = false;
            state.last_invalidation_reason = Some("System prompt changed".to_string());
        }

        if state.system_prompt_hash.is_none() {
            state.cache_valid = true;
        }

        state.system_prompt_hash = Some(hash);
    }

    pub async fn record_tools(&self, tool_names: &[String]) {
        let hash = compute_hash(&tool_names.join(","));
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.tools_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "tools".to_string(),
                description: "Tool set modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
            state.cache_valid = false;
            state.last_invalidation_reason = Some("Tool set changed".to_string());
        }

        state.tools_hash = Some(hash);
    }

    pub async fn record_memory(&self, content: &str) {
        let hash = compute_hash(content);
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.memory_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "memory".to_string(),
                description: "Memory content modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
        }

        state.memory_hash = Some(hash);
    }

    pub async fn record_context_files(&self, content: &str) {
        let hash = compute_hash(content);
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.context_files_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "context_files".to_string(),
                description: "Context files modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
        }

        state.context_files_hash = Some(hash);
    }

    pub async fn is_cache_valid(&self) -> bool {
        self.state.read().await.cache_valid
    }

    pub async fn get_state(&self) -> PromptCacheState {
        self.state.read().await.clone()
    }

    pub async fn mark_cache_hit(&self, token_count: u64) {
        let mut state = self.state.write().await;
        state.cache_hits += 1;
        state.tokens_saved_estimate += token_count;
    }

    pub async fn invalidate(&self, reason: &str) {
        let mut state = self.state.write().await;
        state.cache_valid = false;
        state.last_invalidation_reason = Some(reason.to_string());
    }

    pub async fn invalidate_for_new_session(&self) {
        let mut state = self.state.write().await;
        state.system_prompt_hash = None;
        state.tools_hash = None;
        state.memory_hash = None;
        state.context_files_hash = None;
        state.cache_valid = false;
        state.last_invalidation_reason = None;
        state.pending_changes.clear();
    }

    pub async fn apply_pending_changes(&self) -> Vec<PendingChange> {
        let mut state = self.state.write().await;
        let changes = state.pending_changes.clone();
        state.pending_changes.clear();
        state.cache_valid = true;
        changes
    }

    pub async fn has_pending_changes(&self) -> bool {
        !self.state.read().await.pending_changes.is_empty()
    }

    /// Export the current cache state for persistence.
    pub async fn export_state(&self) -> PromptCacheState {
        self.state.read().await.clone()
    }

    /// Restore the cache state from a persisted snapshot.
    pub async fn restore_state(&self, state: PromptCacheState) {
        let mut current = self.state.write().await;
        *current = state;
    }

    pub async fn total_tokens_saved(&self) -> u64 {
        self.state.read().await.tokens_saved_estimate
    }

    pub async fn reset_stats(&self) {
        let mut state = self.state.write().await;
        state.cache_hits = 0;
        state.tokens_saved_estimate = 0;
    }

    // -----------------------------------------------------------------------
    // 缓存断点检测方法
    // -----------------------------------------------------------------------

    /// 启用或禁用缓存断点检测。
    pub async fn set_break_detection(&self, enabled: bool) {
        let mut state = self.state.write().await;
        state.break_detection_enabled = enabled;
    }

    /// 设置预期的缓存读取 token 数。
    ///
    /// 在发送 API 请求前调用，根据 prompt 结构估算应该被缓存的 token 数。
    pub async fn set_expected_cache_tokens(&self, tokens: u64) {
        let mut state = self.state.write().await;
        state.expected_cache_read_tokens = tokens;
    }

    /// 记录一次缓存读取事件并进行断点分析。
    ///
    /// 在收到 API 响应后调用，对比预期和实际的缓存读取 token 数。
    ///
    /// # 参数
    /// - `actual_cache_read_tokens`: API 报告的 cache_read_input_tokens
    ///
    /// # 返回
    /// - `Some(event)`: 如果检测到异常断点
    /// - `None`: 正常或检测被禁用
    pub async fn record_cache_read(&self, actual_cache_read_tokens: u64) -> Option<CacheReadEvent> {
        let mut state = self.state.write().await;

        if !state.break_detection_enabled {
            return None;
        }

        let expected = state.expected_cache_read_tokens;
        // 仅当显式调用了 invalidate() 且不是新建状态时，才标记为"主动失效"
        // cache_valid 在新建时默认为 false，但这不代表我们主动失效了缓存
        let was_explicitly_invalidated = state.last_invalidation_reason.is_some() && !state.cache_valid;
        let reason = state.last_invalidation_reason.clone();

        let event = CacheReadEvent {
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            expected_tokens: expected,
            actual_tokens: actual_cache_read_tokens,
            self_invalidated: was_explicitly_invalidated,
            invalidation_reason: reason,
            is_anomaly: false, // 下面计算
        };

        let is_break = event.is_cache_break();
        let mut event = event;
        event.is_anomaly = is_break;

        // 更新断点摘要
        state.break_summary.total_reads += 1;
        if is_break {
            state.break_summary.anomaly_count += 1;
            state.break_summary.last_anomaly = Some(event.clone());
        }
        if was_explicitly_invalidated {
            state.break_summary.self_invalidation_count += 1;
        }
        state.break_summary.total_tokens_saved += actual_cache_read_tokens;

        // 维护最近事件的环形缓冲区（最多保留 20 条）
        const MAX_EVENTS: usize = 20;
        state.recent_read_events.push(event.clone());
        if state.recent_read_events.len() > MAX_EVENTS {
            state.recent_read_events.remove(0);
        }

        // 重置预期值（每次请求后清零）
        state.expected_cache_read_tokens = 0;

        if is_break {
            Some(event)
        } else {
            None
        }
    }

    /// 获取缓存断点分析摘要。
    pub async fn get_break_summary(&self) -> CacheBreakSummary {
        self.state.read().await.break_summary.clone()
    }

    /// 获取最近的缓存读取事件。
    pub async fn get_recent_events(&self) -> Vec<CacheReadEvent> {
        self.state.read().await.recent_read_events.clone()
    }

    /// 重置断点统计数据。
    pub async fn reset_break_stats(&self) {
        let mut state = self.state.write().await;
        state.break_summary = CacheBreakSummary::default();
        state.recent_read_events.clear();
        state.expected_cache_read_tokens = 0;
    }

    /// 估算可缓存的 prompt 前缀 token 数。
    ///
    /// 基于 Anthropic prompt caching 规则：system prompt + tools + 前面的消息
    /// 可以被缓存。此函数提供一个粗略估算。
    ///
    /// # 估算公式
    /// - System prompt: ~4 chars/token
    /// - Tools: ~3 chars/token (结构化 JSON)
    /// - Messages: 前 N 条消息的 token 估算
    pub async fn estimate_cacheable_tokens(
        &self,
        system_prompt: &str,
        tool_names: &[String],
        prefix_message_count: usize,
        avg_chars_per_message: usize,
    ) -> u64 {
        let system_tokens = (system_prompt.len() / 4) as u64;
        let tools_chars: usize = tool_names.iter().map(|n| n.len() + 10).sum(); // +10 for JSON overhead
        let tools_tokens = (tools_chars / 3) as u64;
        let messages_tokens =
            (prefix_message_count * avg_chars_per_message / 4) as u64;

        system_tokens + tools_tokens + messages_tokens
    }
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod cache_break_tests {
    use super::*;

    #[test]
    fn test_cache_break_detection_low_actual() {
        let event = CacheReadEvent {
            timestamp_ms: 1000,
            expected_tokens: 10000,
            actual_tokens: 3000, // 30% — below 50% threshold
            self_invalidated: false,
            invalidation_reason: None,
            is_anomaly: false,
        };
        assert!(event.is_cache_break());
    }

    #[test]
    fn test_no_break_when_self_invalidated() {
        let event = CacheReadEvent {
            timestamp_ms: 1000,
            expected_tokens: 10000,
            actual_tokens: 1000, // 低比例但主动失效了
            self_invalidated: true,
            invalidation_reason: Some("compaction".to_string()),
            is_anomaly: false,
        };
        assert!(!event.is_cache_break());
    }

    #[test]
    fn test_no_break_below_min_tokens() {
        let event = CacheReadEvent {
            timestamp_ms: 1000,
            expected_tokens: 50, // 低于最小检测阈值
            actual_tokens: 10,
            self_invalidated: false,
            invalidation_reason: None,
            is_anomaly: false,
        };
        assert!(!event.is_cache_break());
    }

    #[test]
    fn test_no_break_high_actual() {
        let event = CacheReadEvent {
            timestamp_ms: 1000,
            expected_tokens: 10000,
            actual_tokens: 8000, // 80% — above 50% threshold
            self_invalidated: false,
            invalidation_reason: None,
            is_anomaly: false,
        };
        assert!(!event.is_cache_break());
    }

    #[test]
    fn test_no_break_zero_expected() {
        let event = CacheReadEvent {
            timestamp_ms: 1000,
            expected_tokens: 0,
            actual_tokens: 0,
            self_invalidated: false,
            invalidation_reason: None,
            is_anomaly: false,
        };
        assert!(!event.is_cache_break());
    }

    #[tokio::test]
    async fn test_record_cache_read_anomaly() {
        let cache = PromptCache::new();
        cache.set_break_detection(true).await;
        cache.set_expected_cache_tokens(10000).await;

        // 实际只读取了 3000 tokens → 异常
        let event = cache.record_cache_read(3000).await;
        assert!(event.is_some());
        assert!(event.unwrap().is_anomaly);
    }

    #[tokio::test]
    async fn test_record_cache_read_normal() {
        let cache = PromptCache::new();
        cache.set_break_detection(true).await;
        cache.set_expected_cache_tokens(10000).await;

        // 实际读取了 9000 tokens → 正常
        let event = cache.record_cache_read(9000).await;
        assert!(event.is_none());
    }

    #[tokio::test]
    async fn test_record_cache_read_self_invalidated() {
        let cache = PromptCache::new();
        cache.set_break_detection(true).await;
        cache.set_expected_cache_tokens(10000).await;
        cache.invalidate("compaction triggered").await;

        // 主动失效后低读取 → 不视为异常
        let event = cache.record_cache_read(100).await;
        assert!(event.is_none());
    }

    #[tokio::test]
    async fn test_break_summary_accumulates() {
        let cache = PromptCache::new();
        cache.set_break_detection(true).await;

        // 第一次：异常
        cache.set_expected_cache_tokens(10000).await;
        cache.record_cache_read(1000).await;

        // 第二次：正常
        cache.set_expected_cache_tokens(10000).await;
        cache.record_cache_read(9000).await;

        // 第三次：异常
        cache.set_expected_cache_tokens(10000).await;
        cache.record_cache_read(2000).await;

        let summary = cache.get_break_summary().await;
        assert_eq!(summary.total_reads, 3);
        assert_eq!(summary.anomaly_count, 2);
    }

    #[tokio::test]
    async fn test_estimate_cacheable_tokens() {
        let cache = PromptCache::new();
        let tokens = cache
            .estimate_cacheable_tokens(
                "You are a helpful assistant.", // system prompt
                &["read_file".to_string(), "write_file".to_string()], // tools
                5,       // prefix messages
                500,     // avg chars per message
            )
            .await;

        // 粗略估算应该 > 0
        assert!(tokens > 0);
    }
}
