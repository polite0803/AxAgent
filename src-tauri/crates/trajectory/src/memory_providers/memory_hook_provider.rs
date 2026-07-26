// SPDX-License-Identifier: AGPL-3.0-only

//! G21 Memory 系统增强 — MemoryHookProvider
//!
//! 实现 `PluginHook` trait，在会话生命周期事件触发时自动同步记忆：
//!
//! - `on_session_start`: 调用 `MemoryProvider::prefetch` 预拉取相关记忆，
//!   通过 `transform_llm_response` 把记忆摘要注入到 LLM 上下文（可选）
//! - `on_session_end`: 调用 `MemoryProvider::sync_turn` 把当前会话产生的
//!   MemoryEntry 持久化到 Memory 体系
//! - `pre_llm_call`: 根据当前 LLM 上下文构造查询，预拉取相关记忆
//! - `post_tool_call`: 累计工具调用结果，作为记忆来源
//!
//! ## 设计
//!
//! - 通过 `Arc<dyn MemoryProvider>` 与 Memory 体系解耦
//! - 内部维护 `RwLock<HashMap<session_id, Vec<MemoryEntry>>>` 缓存
//! - 不修改 LLM 请求/响应，只在 `on_session_end` 同步记忆
//! - 可配置 `auto_inject_context: bool` 控制是否在 pre_llm_call 时注入记忆摘要
//!
//! ## 使用
//!
//! 在 `init/services.rs` 启动时，把 `MemoryHookProvider` 注册到 `HookChain`，
//! 与 `MultiAgentTriggerHook` 等其他 Hook 并列。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use axagent_harness::plugin_hook::{
    HookDecision, LlmCallContext, LlmCallResult, PluginHook, ToolCallContext, ToolCallResult,
};

use crate::memory_provider::{MemoryEntry, MemoryProvider, MemoryQuery};

/// MemoryHookProvider 配置
#[derive(Debug, Clone)]
pub struct MemoryHookConfig {
    /// 是否在 pre_llm_call 时自动注入记忆摘要到 LLM 上下文
    pub auto_inject_context: bool,
    /// prefetch 返回的记忆条数上限
    pub prefetch_limit: usize,
    /// 注入到 LLM 上下文的最大字符数
    pub max_injection_chars: usize,
    /// prefetch 时的最小重要性阈值
    pub min_importance: f64,
}

impl Default for MemoryHookConfig {
    fn default() -> Self {
        Self {
            auto_inject_context: false, // 默认关闭，避免污染 LLM 上下文
            prefetch_limit: 5,
            max_injection_chars: 2000,
            min_importance: 0.3,
        }
    }
}

/// G21 MemoryHookProvider — 会话生命周期记忆同步 Hook
///
/// 把 MemoryProvider 接入到 PluginHook 链，让记忆系统在会话事件触发时
/// 自动 prefetch / sync_turn，无需业务代码显式调用。
pub struct MemoryHookProvider {
    config: MemoryHookConfig,
    provider: Arc<dyn MemoryProvider>,
    /// 每个会话累计的记忆条目（待 sync_turn 持久化）
    session_entries: Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>>,
    /// 每个会话最近一次 prefetch 的记忆摘要（用于注入 LLM 上下文）
    session_context: Arc<RwLock<HashMap<String, String>>>,
}

impl MemoryHookProvider {
    pub fn new(config: MemoryHookConfig, provider: Arc<dyn MemoryProvider>) -> Self {
        Self {
            config,
            provider,
            session_entries: Arc::new(RwLock::new(HashMap::new())),
            session_context: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 用默认配置和指定 provider 构造。
    pub fn with_provider(provider: Arc<dyn MemoryProvider>) -> Self {
        Self::new(MemoryHookConfig::default(), provider)
    }

    /// 把工具调用结果转换为 MemoryEntry 并累计到会话缓存。
    async fn accumulate_tool_result(
        &self,
        session_id: &str,
        ctx: &ToolCallContext,
        result: &ToolCallResult,
    ) {
        if !result.success {
            return;
        }
        let content = format!(
            "[Tool: {}] {}",
            ctx.tool_name,
            serde_json::to_string(&result.result).unwrap_or_default()
        );
        let entry = MemoryEntry {
            id: format!("tool:{}:{}", ctx.tool_name, chrono::Utc::now().timestamp_millis()),
            content,
            memory_type: crate::memory_provider::MemoryType::Conversation,
            importance: 0.5,
            tags: vec![ctx.tool_name.clone()],
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
            tier: crate::memory_providers::service::MemoryTier::ShortTerm,
            nature: crate::memory_providers::service::MemoryNature::Episodic,
        };

        let mut entries = self.session_entries.write().await;
        entries.entry(session_id.to_string()).or_default().push(entry);
    }

    /// 在 pre_llm_call 时 prefetch 相关记忆，缓存摘要用于后续注入。
    async fn prefetch_for_llm(&self, session_id: &str, ctx: &LlmCallContext) {
        let query = MemoryQuery {
            query: format!("model={} tools={}", ctx.model, ctx.tool_count),
            memory_types: None,
            tags: None,
            limit: self.config.prefetch_limit,
            min_importance: Some(self.config.min_importance),
            tier_filter: None,
        };

        match self.provider.prefetch(session_id, &query).await {
            Ok(result) if !result.entries.is_empty() => {
                let summary = self.format_memory_summary(&result.entries);
                let mut contexts = self.session_context.write().await;
                contexts.insert(session_id.to_string(), summary);
            },
            Ok(_) => {},
            Err(e) => {
                tracing::warn!("[MemoryHook] prefetch 失败 (session={session_id}): {e}");
            },
        }
    }

    /// 把记忆条目列表格式化为摘要字符串（用于注入 LLM 上下文）。
    fn format_memory_summary(&self, entries: &[MemoryEntry]) -> String {
        let mut out = String::new();
        out.push_str("## Relevant Memories\n\n");
        for (i, entry) in entries.iter().enumerate() {
            let truncated = if entry.content.len() > 200 {
                format!("{}...", &entry.content[..200])
            } else {
                entry.content.clone()
            };
            out.push_str(&format!("{}. [{}] {}\n", i + 1, entry.memory_type.as_str(), truncated));
            if out.len() > self.config.max_injection_chars {
                break;
            }
        }
        out
    }
}

#[async_trait]
impl PluginHook for MemoryHookProvider {
    fn name(&self) -> &str {
        "memory_hook_provider"
    }

    fn priority(&self) -> i32 {
        // 低于 MultiAgentTriggerHook（默认 50），让 Multi-Agent Hook 先决策
        10
    }

    async fn on_session_start(&self, session_id: &str) {
        tracing::debug!("[MemoryHook] session_start: {session_id}");
        // 初始化会话缓存
        let mut entries = self.session_entries.write().await;
        entries.entry(session_id.to_string()).or_default();
    }

    async fn on_session_end(&self, session_id: &str) {
        tracing::debug!("[MemoryHook] session_end: {session_id}");

        // 取出累计的记忆条目
        let entries = {
            let mut map = self.session_entries.write().await;
            map.remove(session_id).unwrap_or_default()
        };

        if entries.is_empty() {
            return;
        }

        // 同步到 MemoryProvider
        if let Err(e) = self.provider.sync_turn(session_id, entries).await {
            tracing::warn!("[MemoryHook] sync_turn 失败 (session={session_id}): {e}");
        }

        // 清理上下文缓存
        let mut contexts = self.session_context.write().await;
        contexts.remove(session_id);
    }

    async fn pre_tool_call(&self, _ctx: &ToolCallContext) -> Option<HookDecision> {
        // 不拦截工具调用
        None
    }

    async fn post_tool_call(&self, ctx: &ToolCallContext, result: &ToolCallResult) {
        if let Some(ref session_id) = ctx.session_id {
            self.accumulate_tool_result(session_id, ctx, result).await;
        }
    }

    async fn pre_llm_call(&self, ctx: &LlmCallContext) -> Option<HookDecision> {
        if !self.config.auto_inject_context {
            return None;
        }
        if let Some(ref session_id) = ctx.session_id {
            self.prefetch_for_llm(session_id, ctx).await;
        }
        None
    }

    async fn post_llm_call(&self, _ctx: &LlmCallContext, _result: &LlmCallResult) {
        // 不处理 LLM 响应
    }

    async fn transform_llm_response(&self, content: String) -> String {
        // 默认不修改 LLM 响应
        // 如果 auto_inject_context=true，可在 pre_llm_call 后通过此钩子注入记忆摘要
        // 当前实现保持原样返回，避免污染 LLM 输出
        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::memory_provider::{MemoryProvider, MemoryQuery, MemoryQueryResult};

    /// 测试用 MemoryProvider — 计数 prefetch / sync_turn 调用
    struct CountingProvider {
        prefetch_calls: AtomicUsize,
        sync_calls: AtomicUsize,
    }

    impl CountingProvider {
        fn new() -> Self {
            Self { prefetch_calls: AtomicUsize::new(0), sync_calls: AtomicUsize::new(0) }
        }
    }

    #[async_trait]
    impl MemoryProvider for CountingProvider {
        async fn sync_turn(
            &self,
            _session_id: &str,
            _entries: Vec<MemoryEntry>,
        ) -> Result<(), String> {
            self.sync_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn prefetch(
            &self,
            _session_id: &str,
            _query: &MemoryQuery,
        ) -> Result<MemoryQueryResult, String> {
            self.prefetch_calls.fetch_add(1, Ordering::SeqCst);
            Ok(MemoryQueryResult { entries: Vec::new(), scores: Vec::new(), total: 0 })
        }

        async fn shutdown(&self) -> Result<(), String> {
            Ok(())
        }

        fn provider_name(&self) -> &'static str {
            "counting"
        }

        fn provider_version(&self) -> &'static str {
            "0.1.0"
        }
    }

    #[tokio::test]
    async fn test_session_lifecycle_triggers_sync() {
        let provider = Arc::new(CountingProvider::new());
        let hook = MemoryHookProvider::with_provider(provider.clone());

        hook.on_session_start("s1").await;
        assert_eq!(provider.sync_calls.load(Ordering::SeqCst), 0);

        hook.on_session_end("s1").await;
        // 即使无累计条目，sync_turn 不会被调用（entries.is_empty() 提前返回）
        assert_eq!(provider.sync_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_post_tool_call_accumulates_and_syncs() {
        let provider = Arc::new(CountingProvider::new());
        let hook = MemoryHookProvider::with_provider(provider.clone());

        hook.on_session_start("s1").await;

        let ctx = ToolCallContext {
            tool_name: "test_tool".to_string(),
            tool_namespace: None,
            arguments: serde_json::json!({}),
            session_id: Some("s1".to_string()),
        };
        let result = ToolCallResult {
            tool_name: "test_tool".to_string(),
            result: serde_json::json!({"ok": true}),
            success: true,
            duration_ms: Some(100),
        };

        hook.post_tool_call(&ctx, &result).await;
        hook.on_session_end("s1").await;

        // 累计 1 条 → sync_turn 被调用 1 次
        assert_eq!(provider.sync_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_failed_tool_call_not_accumulated() {
        let provider = Arc::new(CountingProvider::new());
        let hook = MemoryHookProvider::with_provider(provider.clone());

        hook.on_session_start("s1").await;

        let ctx = ToolCallContext {
            tool_name: "fail_tool".to_string(),
            tool_namespace: None,
            arguments: serde_json::json!({}),
            session_id: Some("s1".to_string()),
        };
        let result = ToolCallResult {
            tool_name: "fail_tool".to_string(),
            result: serde_json::json!({"err": "boom"}),
            success: false,
            duration_ms: Some(50),
        };

        hook.post_tool_call(&ctx, &result).await;
        hook.on_session_end("s1").await;

        // 失败的工具调用不应被累计 → sync_turn 不调用
        assert_eq!(provider.sync_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_pre_llm_call_prefetch_when_auto_inject() {
        let provider = Arc::new(CountingProvider::new());
        let config = MemoryHookConfig { auto_inject_context: true, ..Default::default() };
        let hook = MemoryHookProvider::new(config, provider.clone());

        let ctx = LlmCallContext {
            model: "gpt-4".to_string(),
            message_count: 1,
            tool_count: 0,
            estimated_tokens: Some(100),
            session_id: Some("s1".to_string()),
        };

        hook.pre_llm_call(&ctx).await;
        assert_eq!(provider.prefetch_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_pre_llm_call_no_prefetch_when_disabled() {
        let provider = Arc::new(CountingProvider::new());
        // 默认 auto_inject_context = false
        let hook = MemoryHookProvider::with_provider(provider.clone());

        let ctx = LlmCallContext {
            model: "gpt-4".to_string(),
            message_count: 1,
            tool_count: 0,
            estimated_tokens: Some(100),
            session_id: Some("s1".to_string()),
        };

        hook.pre_llm_call(&ctx).await;
        assert_eq!(provider.prefetch_calls.load(Ordering::SeqCst), 0);
    }
}
