// SPDX-License-Identifier: AGPL-3.0-only

//! G5 Multi-Agent Trigger Hook — 基于 PluginHook trait 的自动委派触发器。
//!
//! ## 设计动机
//!
//! DojoAgents 宣传口径中的"Multi-Agent 固定角色 pool"要求系统能在检测到复杂任务时
//! **自动**拆分给 analyst / implementer / reviewer 三个角色。本 Hook 通过 PluginHook
//! 的 `pre_llm_call` 钩子实现"复杂度检测 + 委派建议注入"。
//!
//! ## 触发策略
//!
//! 1. **关键词触发**：检测用户消息中的高复杂度关键词（如"对比分析"、"重构"、"验证"等）
//! 2. **消息长度触发**：单条消息超过 N tokens 时，建议先由 analyst 拆解
//! 3. **工具调用次数触发**：post_tool_call 累计调用次数超过阈值时，建议 reviewer 介入
//!
//! ## 注意
//!
//! Hook 只做"建议注入"，不实际执行 delegate_task。实际委派由：
//! - 前端 MultiAgentPanel 调用 `delegate_task` Tauri 命令
//! - 工作流 AgentNode 通过 ToolResolver 路由到 `delegate_task` 工具
//! - 上层 orchestrator 监听 Hook 注入的元数据后主动调用

use async_trait::async_trait;
use axagent_harness::plugin_hook::{
    HookDecision, LlmCallContext, LlmCallResult, PluginHook, SharedHook, ToolCallContext,
    ToolCallResult,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Multi-Agent 触发阈值配置
#[derive(Debug, Clone)]
pub struct MultiAgentTriggerConfig {
    /// 触发 analyst 的消息长度阈值（估算 tokens）
    pub long_message_threshold: usize,
    /// 触发 reviewer 的工具调用累计次数
    pub tool_call_review_threshold: u64,
    /// 高复杂度关键词列表（命中任一则建议 analyst）
    pub complexity_keywords: Vec<String>,
}

impl Default for MultiAgentTriggerConfig {
    fn default() -> Self {
        Self {
            long_message_threshold: 800,
            tool_call_review_threshold: 5,
            complexity_keywords: vec![
                "对比分析".to_string(),
                "深度研究".to_string(),
                "重构".to_string(),
                "审计".to_string(),
                "验证".to_string(),
                "多角度".to_string(),
                "交叉验证".to_string(),
                "系统性".to_string(),
                "端到端".to_string(),
                "完整闭环".to_string(),
            ],
        }
    }
}

/// G5 Multi-Agent 自动委派触发 Hook
pub struct MultiAgentTriggerHook {
    config: MultiAgentTriggerConfig,
    /// 当前会话累计工具调用次数（原子操作，线程安全）
    tool_call_count: Arc<AtomicU64>,
}

impl MultiAgentTriggerHook {
    pub fn new(config: MultiAgentTriggerConfig) -> Self {
        Self { config, tool_call_count: Arc::new(AtomicU64::new(0)) }
    }

    pub fn with_default() -> Self {
        Self::new(MultiAgentTriggerConfig::default())
    }

    /// 检测 LLM 调用上下文中的复杂度信号，返回是否需要委派给 analyst
    fn detect_complexity(&self, ctx: &LlmCallContext) -> Option<String> {
        // 1. 长消息触发：估算 tokens > 阈值
        if ctx.estimated_tokens.unwrap_or(0) > self.config.long_message_threshold as u64 {
            return Some(format!(
                "检测到长消息（~{} tokens），建议先用 `delegate_task(role_name=\"analyst\")` 拆解任务",
                ctx.estimated_tokens.unwrap_or(0)
            ));
        }

        // 2. 工具数量多 → 建议先 analyst 拆解
        if ctx.tool_count >= 4 {
            return Some(format!(
                "检测到 {} 个工具待调用，建议先用 `delegate_task(role_name=\"analyst\")` 制定假设，再由 `implementer` 执行",
                ctx.tool_count
            ));
        }

        None
    }

    /// 检测工具调用累计次数，返回是否需要 reviewer 介入
    fn detect_review_needed(&self) -> Option<String> {
        let count = self.tool_call_count.load(Ordering::Relaxed);
        if count >= self.config.tool_call_review_threshold {
            return Some(format!(
                "已累计 {} 次工具调用，建议调用 `delegate_task(role_name=\"reviewer\")` 验证执行结果",
                count
            ));
        }
        None
    }

    /// 检测工具参数中的复杂度关键词
    fn detect_keywords_in_arguments(&self, args: &serde_json::Value) -> Option<String> {
        let args_str = args.to_string();
        for keyword in &self.config.complexity_keywords {
            if args_str.contains(keyword) {
                return Some(format!(
                    "检测到复杂度关键词「{}」，建议先用 `delegate_task(role_name=\"analyst\")` 产生假设",
                    keyword
                ));
            }
        }
        None
    }
}

#[async_trait]
impl PluginHook for MultiAgentTriggerHook {
    fn name(&self) -> &str {
        "multi_agent_trigger"
    }

    fn priority(&self) -> i32 {
        // 低于安全/审计 hook（通常 -100~0），高于业务 hook（通常 >10）
        10
    }

    async fn on_session_start(&self, _session_id: &str) {
        // 会话开始时重置工具调用计数
        self.tool_call_count.store(0, Ordering::Relaxed);
    }

    async fn on_session_end(&self, _session_id: &str) {
        // 会话结束时清理（计数器原子操作，无需额外清理）
        let total = self.tool_call_count.load(Ordering::Relaxed);
        tracing::info!(
            target: "axagent.multi_agent",
            session_id = _session_id,
            total_tool_calls = total,
            "MultiAgentTriggerHook: 会话结束"
        );
    }

    async fn pre_tool_call(&self, ctx: &ToolCallContext) -> Option<HookDecision> {
        // 累计工具调用次数
        self.tool_call_count.fetch_add(1, Ordering::Relaxed);

        // 检测参数中的复杂度关键词
        if let Some(hint) = self.detect_keywords_in_arguments(&ctx.arguments) {
            tracing::info!(
                target: "axagent.multi_agent",
                tool = %ctx.tool_name,
                hint = %hint,
                "MultiAgentTriggerHook: 检测到复杂度信号，建议委派"
            );
            // 通过 Modify 注入提示（不阻断工具调用）
            return Some(HookDecision::Modify {
                changes: serde_json::json!({
                    "multi_agent_hint": hint,
                    "suggested_delegation": {
                        "role": "analyst",
                        "reason": "complexity_keyword_detected"
                    }
                }),
            });
        }

        // 检测是否需要 reviewer 介入
        if let Some(hint) = self.detect_review_needed() {
            tracing::info!(
                target: "axagent.multi_agent",
                tool_calls = self.tool_call_count.load(Ordering::Relaxed),
                hint = %hint,
                "MultiAgentTriggerHook: 工具调用次数达阈值，建议 reviewer 介入"
            );
            return Some(HookDecision::Modify {
                changes: serde_json::json!({
                    "multi_agent_hint": hint,
                    "suggested_delegation": {
                        "role": "reviewer",
                        "reason": "tool_call_threshold_reached"
                    }
                }),
            });
        }

        None
    }

    async fn post_tool_call(&self, _ctx: &ToolCallContext, _result: &ToolCallResult) {
        // 当前仅累计次数，实际 review 触发由 pre_tool_call 中的阈值检测完成
    }

    async fn pre_llm_call(&self, ctx: &LlmCallContext) -> Option<HookDecision> {
        if let Some(hint) = self.detect_complexity(ctx) {
            tracing::info!(
                target: "axagent.multi_agent",
                model = %ctx.model,
                hint = %hint,
                "MultiAgentTriggerHook: LLM 调用前检测到复杂度信号"
            );
            return Some(HookDecision::Modify {
                changes: serde_json::json!({
                    "multi_agent_hint": hint,
                    "suggested_delegation": {
                        "role": "analyst",
                        "reason": "long_message_or_many_tools"
                    }
                }),
            });
        }
        None
    }

    async fn post_llm_call(&self, _ctx: &LlmCallContext, _result: &LlmCallResult) {
        // LLM 调用后无需额外处理（reviewer 介入由 pre_tool_call 阈值触发）
    }

    async fn on_error(&self, error: &str, _context: Option<serde_json::Value>) {
        // 错误时建议 implementer 介入修复
        tracing::warn!(
            target: "axagent.multi_agent",
            error = error,
            "MultiAgentTriggerHook: 错误发生，建议委派给 implementer 处理"
        );
    }
}

/// 工厂函数：创建 MultiAgentTriggerHook 的 SharedHook 实例
pub fn create_multi_agent_trigger_hook() -> SharedHook {
    Arc::new(MultiAgentTriggerHook::with_default())
}

/// 工厂函数：使用自定义配置创建 hook
pub fn create_multi_agent_trigger_hook_with_config(config: MultiAgentTriggerConfig) -> SharedHook {
    Arc::new(MultiAgentTriggerHook::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_complexity_long_message() {
        let hook = MultiAgentTriggerHook::with_default();
        let ctx = LlmCallContext {
            model: "gpt-4o".to_string(),
            message_count: 1,
            tool_count: 0,
            estimated_tokens: Some(1000),
            session_id: None,
        };
        let result = hook.detect_complexity(&ctx);
        assert!(result.is_some(), "长消息应触发 analyst 委派");
    }

    #[test]
    fn test_detect_complexity_many_tools() {
        let hook = MultiAgentTriggerHook::with_default();
        let ctx = LlmCallContext {
            model: "gpt-4o".to_string(),
            message_count: 1,
            tool_count: 5,
            estimated_tokens: Some(100),
            session_id: None,
        };
        let result = hook.detect_complexity(&ctx);
        assert!(result.is_some(), "多工具应触发 analyst 委派");
    }

    #[test]
    fn test_detect_keywords() {
        let hook = MultiAgentTriggerHook::with_default();
        let args = serde_json::json!({"task": "请对比分析 A 和 B 的差异"});
        let result = hook.detect_keywords_in_arguments(&args);
        assert!(result.is_some(), "应检测到「对比分析」关键词");
    }

    #[test]
    fn test_detect_review_needed() {
        let hook = MultiAgentTriggerHook::with_default();
        // 模拟 5 次工具调用
        for _ in 0..5 {
            hook.tool_call_count.fetch_add(1, Ordering::Relaxed);
        }
        let result = hook.detect_review_needed();
        assert!(result.is_some(), "5 次工具调用应触发 reviewer 委派");
    }
}
