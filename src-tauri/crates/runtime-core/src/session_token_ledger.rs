// SPDX-License-Identifier: AGPL-3.0-only
//! G11 SessionTokenLedger + ContextCompactedEvent
//!
//! 会话级 token 用量账本，跟踪整个会话的 token 消耗，并在上下文压缩时
//! 发出 `ContextCompactedEvent` 通知前端。
//!
//! ## SessionTokenLedger
//!
//! 记录会话中每次 LLM 调用的 token 用量，支持：
//! - 累计 input/output/cache token
//! - 按 provider/model 分组统计
//! - 成本估算（基于 harness 定价表）
//! - 压缩节省量统计（compaction_saved_tokens）
//!
//! ## ContextCompactedEvent
//!
//! 当上下文窗口压缩触发时发出，包含：
//! - 压缩前后的 token 数
//! - 压缩策略（emergency / auto / micro）
//! - 节省的 token 数
//! - 触发的消息范围

use axagent_harness::conversation_model::TokenUsage;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 上下文压缩事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCompactedEvent {
    /// 会话 ID
    pub session_id: String,
    /// 压缩前 token 数
    pub tokens_before: u64,
    /// 压缩后 token 数
    pub tokens_after: u64,
    /// 节省的 token 数
    pub tokens_saved: u64,
    /// 压缩策略
    pub strategy: CompactionStrategy,
    /// 压缩的消息范围（起始-结束索引）
    pub message_range: Option<(usize, usize)>,
    /// 时间戳（Unix 毫秒）
    pub ts: i64,
}

/// 压缩策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// 紧急压缩（接近上下文窗口上限）
    Emergency,
    /// 自动压缩（达到阈值）
    Auto,
    /// 微压缩（仅压缩少量消息）
    Micro,
    /// 会话记忆压缩
    Memory,
    /// 用户手动触发
    Manual,
}

impl CompactionStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Emergency => "emergency",
            Self::Auto => "auto",
            Self::Micro => "micro",
            Self::Memory => "memory",
            Self::Manual => "manual",
        }
    }
}

/// 单次 LLM 调用的 token 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRecord {
    /// provider ID
    pub provider_id: String,
    /// model ID
    pub model_id: String,
    /// token 用量
    pub usage: TokenUsage,
    /// 估算成本（美元）
    pub cost_usd: f64,
    /// 时间戳（Unix 毫秒）
    pub ts: i64,
}

/// 按 provider+model 分组的统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModelStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cost_usd: f64,
    pub call_count: u64,
}

/// 会话级 token 账本
pub struct SessionTokenLedger {
    /// 会话 ID
    session_id: String,
    /// 所有调用记录（内部加锁）
    records: Mutex<Vec<TokenRecord>>,
    /// 按 provider+model 分组的统计缓存
    stats_cache: Mutex<HashMap<String, ProviderModelStats>>,
    /// 累计节省的 token 数（通过压缩）
    compaction_saved_tokens: Mutex<u64>,
    /// 压缩事件历史
    compaction_events: Mutex<Vec<ContextCompactedEvent>>,
}

impl SessionTokenLedger {
    /// 创建新的账本
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            records: Mutex::new(Vec::new()),
            stats_cache: Mutex::new(HashMap::new()),
            compaction_saved_tokens: Mutex::new(0),
            compaction_events: Mutex::new(Vec::new()),
        }
    }

    /// 会话 ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 记录一次 LLM 调用的 token 用量
    pub fn record(
        &self,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        usage: TokenUsage,
        cost_usd: f64,
    ) {
        let provider_id = provider_id.into();
        let model_id = model_id.into();
        let ts = now_ms();

        let record = TokenRecord {
            provider_id: provider_id.clone(),
            model_id: model_id.clone(),
            usage,
            cost_usd,
            ts,
        };

        self.records.lock().push(record);

        // 更新统计缓存
        let key = format!("{provider_id}/{model_id}");
        let mut stats = self.stats_cache.lock();
        let entry = stats.entry(key).or_default();
        entry.total_input_tokens += usage.input_tokens as u64;
        entry.total_output_tokens += usage.output_tokens as u64;
        entry.total_cache_read_tokens += usage.cache_read_input_tokens as u64;
        entry.total_cache_creation_tokens += usage.cache_creation_input_tokens as u64;
        entry.total_cost_usd += cost_usd;
        entry.call_count += 1;
    }

    /// 记录上下文压缩事件
    pub fn record_compaction(&self, mut event: ContextCompactedEvent) {
        // 确保 session_id 一致
        event.session_id = self.session_id.clone();
        event.ts = now_ms();

        let saved = event.tokens_saved;
        *self.compaction_saved_tokens.lock() += saved;
        self.compaction_events.lock().push(event);
    }

    /// 获取累计 token 用量
    pub fn total_usage(&self) -> TokenUsage {
        let mut total = TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        };
        for record in self.records.lock().iter() {
            total.input_tokens += record.usage.input_tokens;
            total.output_tokens += record.usage.output_tokens;
            total.cache_creation_input_tokens += record.usage.cache_creation_input_tokens;
            total.cache_read_input_tokens += record.usage.cache_read_input_tokens;
        }
        total
    }

    /// 获取累计成本（美元）
    pub fn total_cost_usd(&self) -> f64 {
        self.records.lock().iter().map(|r| r.cost_usd).sum()
    }

    /// 获取压缩节省的 token 数
    pub fn compaction_saved_tokens(&self) -> u64 {
        *self.compaction_saved_tokens.lock()
    }

    /// 获取所有调用记录
    pub fn records(&self) -> Vec<TokenRecord> {
        self.records.lock().clone()
    }

    /// 获取按 provider+model 分组的统计
    pub fn provider_model_stats(&self) -> HashMap<String, ProviderModelStats> {
        self.stats_cache.lock().clone()
    }

    /// 获取所有压缩事件
    pub fn compaction_events(&self) -> Vec<ContextCompactedEvent> {
        self.compaction_events.lock().clone()
    }

    /// 获取调用次数
    pub fn call_count(&self) -> usize {
        self.records.lock().len()
    }

    /// 清空账本
    pub fn clear(&self) {
        self.records.lock().clear();
        self.stats_cache.lock().clear();
        *self.compaction_saved_tokens.lock() = 0;
        self.compaction_events.lock().clear();
    }
}

/// 当前 Unix 毫秒时间戳
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 会话级账本注册表（管理多个会话的账本）
pub struct SessionLedgerRegistry {
    ledgers: Mutex<HashMap<String, std::sync::Arc<SessionTokenLedger>>>,
}

impl Default for SessionLedgerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionLedgerRegistry {
    pub fn new() -> Self {
        Self { ledgers: Mutex::new(HashMap::new()) }
    }

    /// 获取或创建会话账本
    pub fn get_or_create(&self, session_id: &str) -> std::sync::Arc<SessionTokenLedger> {
        let mut ledgers = self.ledgers.lock();
        ledgers
            .entry(session_id.to_string())
            .or_insert_with(|| std::sync::Arc::new(SessionTokenLedger::new(session_id)))
            .clone()
    }

    /// 获取会话账本（不创建）
    pub fn get(&self, session_id: &str) -> Option<std::sync::Arc<SessionTokenLedger>> {
        self.ledgers.lock().get(session_id).cloned()
    }

    /// 移除会话账本
    pub fn remove(&self, session_id: &str) -> bool {
        self.ledgers.lock().remove(session_id).is_some()
    }

    /// 列出所有会话 ID
    pub fn list_sessions(&self) -> Vec<String> {
        self.ledgers.lock().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_usage(input: u32, output: u32) -> TokenUsage {
        TokenUsage {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        }
    }

    #[test]
    fn test_ledger_record_and_total() {
        let ledger = SessionTokenLedger::new("session_1");
        ledger.record("openai", "gpt-4", make_usage(100, 50), 0.01);
        ledger.record("openai", "gpt-4", make_usage(200, 80), 0.02);
        ledger.record("anthropic", "claude-3", make_usage(150, 60), 0.015);

        let total = ledger.total_usage();
        assert_eq!(total.input_tokens, 450);
        assert_eq!(total.output_tokens, 190);
        assert_eq!(ledger.call_count(), 3);
        assert!((ledger.total_cost_usd() - 0.045).abs() < 0.0001);
    }

    #[test]
    fn test_provider_model_stats() {
        let ledger = SessionTokenLedger::new("session_1");
        ledger.record("openai", "gpt-4", make_usage(100, 50), 0.01);
        ledger.record("openai", "gpt-4", make_usage(200, 80), 0.02);
        ledger.record("anthropic", "claude-3", make_usage(150, 60), 0.015);

        let stats = ledger.provider_model_stats();
        assert_eq!(stats.len(), 2);

        let openai_stats = &stats["openai/gpt-4"];
        assert_eq!(openai_stats.total_input_tokens, 300);
        assert_eq!(openai_stats.total_output_tokens, 130);
        assert_eq!(openai_stats.call_count, 2);
    }

    #[test]
    fn test_compaction_event() {
        let ledger = SessionTokenLedger::new("session_1");

        let event = ContextCompactedEvent {
            session_id: "session_1".to_string(),
            tokens_before: 10000,
            tokens_after: 5000,
            tokens_saved: 5000,
            strategy: CompactionStrategy::Auto,
            message_range: Some((0, 10)),
            ts: 0,
        };

        ledger.record_compaction(event);
        assert_eq!(ledger.compaction_saved_tokens(), 5000);
        assert_eq!(ledger.compaction_events().len(), 1);

        // 再记录一次
        let event2 = ContextCompactedEvent {
            session_id: "session_1".to_string(),
            tokens_before: 8000,
            tokens_after: 4000,
            tokens_saved: 4000,
            strategy: CompactionStrategy::Emergency,
            message_range: None,
            ts: 0,
        };
        ledger.record_compaction(event2);
        assert_eq!(ledger.compaction_saved_tokens(), 9000);
        assert_eq!(ledger.compaction_events().len(), 2);
    }

    #[test]
    fn test_ledger_clear() {
        let ledger = SessionTokenLedger::new("session_1");
        ledger.record("openai", "gpt-4", make_usage(100, 50), 0.01);
        ledger.record_compaction(ContextCompactedEvent {
            session_id: "session_1".to_string(),
            tokens_before: 1000,
            tokens_after: 500,
            tokens_saved: 500,
            strategy: CompactionStrategy::Auto,
            message_range: None,
            ts: 0,
        });

        assert_eq!(ledger.call_count(), 1);
        assert_eq!(ledger.compaction_saved_tokens(), 500);

        ledger.clear();
        assert_eq!(ledger.call_count(), 0);
        assert_eq!(ledger.compaction_saved_tokens(), 0);
        assert!(ledger.compaction_events().is_empty());
    }

    #[test]
    fn test_ledger_registry() {
        let registry = SessionLedgerRegistry::new();

        let l1 = registry.get_or_create("session_1");
        let l2 = registry.get_or_create("session_1");
        assert!(std::sync::Arc::ptr_eq(&l1, &l2));

        let l3 = registry.get_or_create("session_2");
        assert!(!std::sync::Arc::ptr_eq(&l1, &l3));

        assert_eq!(registry.list_sessions().len(), 2);
        assert!(registry.get("session_1").is_some());
        assert!(registry.remove("session_1"));
        assert!(registry.get("session_1").is_none());
        assert!(!registry.remove("session_1"));
    }

    #[test]
    fn test_compaction_strategy_serialization() {
        assert_eq!(CompactionStrategy::Emergency.as_str(), "emergency");
        assert_eq!(CompactionStrategy::Auto.as_str(), "auto");
        assert_eq!(CompactionStrategy::Micro.as_str(), "micro");
        assert_eq!(CompactionStrategy::Memory.as_str(), "memory");
        assert_eq!(CompactionStrategy::Manual.as_str(), "manual");
    }
}
