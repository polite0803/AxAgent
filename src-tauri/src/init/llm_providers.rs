// SPDX-License-Identifier: AGPL-3.0-only

//! Wiring 层：LLM 推理 / 评判 / 分解 provider 工厂
//!
//! 将 DB 中的 provider 配置装配为具体的 trait 实现并注入到消费者：
//!
//! - [`build_reasoning_provider_from_db`] → 构造 [`LlmDrivenReasoningProvider`]
//!   实现 [`LlmReasoningProvider`]，注入到 `ReActEngine`。
//! - [`build_llm_judge_from_db`] → 构造 [`ProviderLlmBridge`]（已实现 [`LlmJudge`]），
//!   注入到 `RLEngine`。
//! - [`build_llm_decomposer_from_db`] → 构造 [`LlmBasedDecomposer`]
//!   实现 [`MissionDecomposer`]，注入到 `OrchestratorExecutor`。
//!
//! 三个工厂都走 [`build_llm_components_from_db`] 获取 `(adapter, ctx, model)` 三元组，
//! 避免重复 DB 查询逻辑（AGENTS.md 第 12 条：禁止重复定义）。
//!
//! **降级策略**：任一工厂返回 `None` 时，调用方应回退到规则化占位实现
//! （`DefaultReasoningProvider` / `DefaultLlmJudge` / `RuleBasedDecomposer`），
//! 这些占位实现返回 `Err(NotConfigured)` 或走规则兜底。

use std::sync::Arc;

use axagent_agent::ProviderLlmBridge;
use axagent_agent::react_engine::{LlmDrivenReasoningProvider, LlmReasoningProvider};
use axagent_harness::llm_execution::{
    LlmCallConfig as HarnessLlmCallConfig, LlmCallResult as HarnessLlmCallResult,
    LlmExecutionService,
};
use axagent_harness::retry_policy::RetryPolicy;
use axagent_harness::types::ChatRequest;
use axagent_harness::{LlmCallConfig, ProviderAdapter, ProviderRequestContext, execute_llm};
use axagent_orchestrator::decomposer::{LlmBasedDecomposer, MissionDecomposer};
use axagent_runtime::llm_bridge::build_llm_components_from_db;

// ── BridgeLlmExecutionService ─────────────────────────────────────────

/// 桥接 `execute_llm()` 到 `LlmExecutionService` trait 的适配器。
///
/// `LlmBasedDecomposer` 需要 `Arc<dyn LlmExecutionService>` 注入，而项目统一的
/// LLM 调用入口是 `axagent_harness::execute_llm()`。本结构体负责把
/// `LlmExecutionService::execute()` 的 JSON 消息反序列化回 `ChatRequest`，
/// 再走中心化 `execute_llm()` 路径（含重试 / PromptGuard / 审计）。
pub struct BridgeLlmExecutionService {
    retry_policy: RetryPolicy,
}

impl BridgeLlmExecutionService {
    pub fn new() -> Self {
        Self { retry_policy: RetryPolicy::default_llm() }
    }
}

impl Default for BridgeLlmExecutionService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmExecutionService for BridgeLlmExecutionService {
    async fn execute(
        &self,
        adapter: &(dyn ProviderAdapter + '_),
        ctx: &ProviderRequestContext,
        messages: serde_json::Value,
        _config: &HarnessLlmCallConfig,
    ) -> Result<HarnessLlmCallResult, String> {
        // 反序列化 JSON → ChatRequest（LlmBasedDecomposer/LlmDrivenReasoningProvider
        // 在调用方把 ChatRequest 序列化为 JSON 传入）
        let request: ChatRequest = serde_json::from_value(messages)
            .map_err(|e| format!("BridgeLlmExecutionService: 反序列化 ChatRequest 失败: {e}"))?;

        // 走中心化 execute_llm() 路径（含重试 / PromptGuard / 审计）
        let llm_config =
            LlmCallConfig { retry_policy: Some(self.retry_policy.clone()), ..Default::default() };
        let result = execute_llm(adapter, ctx, request, &llm_config).await?;
        Ok(HarnessLlmCallResult { content: result.response.content })
    }
}

// ── 工厂函数 ──────────────────────────────────────────────────────────

/// 从 DB 构建 LLM 推理 provider（`LlmDrivenReasoningProvider`）。
///
/// 返回 `None` 表示未配置可用 provider，调用方应回退到 `DefaultReasoningProvider`
/// （其 trait 方法返回 `Err(NotConfigured)`）。
///
/// **Prompt 设计**（由 `LlmDrivenReasoningProvider` 内部实现）：
/// - `analyze`：结构化分析输入的复杂度 / 主题 / 是否含代码或提问 / 主要目标
/// - `think`：在 ReAct 循环中生成下一步思考（含上下文窗口摘要）
/// - `plan`：输出 JSON Action（tool_call / llm_call / user_confirm / plan）
/// - `reflect`：回顾进度，给出策略调整建议
/// - `synthesize`：综合推理链与观察，生成最终响应
pub async fn build_reasoning_provider_from_db(
    master_key: &[u8; 32],
) -> Option<Arc<dyn LlmReasoningProvider>> {
    let (adapter, ctx, model) = build_llm_components_from_db(master_key).await?;
    let provider = LlmDrivenReasoningProvider::new(adapter, ctx, model);
    tracing::info!("[wiring] LlmReasoningProvider 注入完成 (LlmDrivenReasoningProvider)");
    Some(Arc::new(provider))
}

/// 从 DB 构建 LLM 评判器（`ProviderLlmBridge`，已实现 `LlmJudge`）。
///
/// 返回 `None` 表示未配置可用 provider，调用方应回退到 `DefaultLlmJudge`
/// （其 trait 方法返回 `Err(NotConfigured)`）。
pub async fn build_llm_judge_from_db(
    master_key: &[u8; 32],
) -> Option<Box<dyn axagent_harness::trajectory_types::LlmJudge>> {
    let (adapter, ctx, model) = build_llm_components_from_db(master_key).await?;
    let bridge = ProviderLlmBridge::new(adapter, ctx, model);
    tracing::info!("[wiring] LlmJudge 注入完成 (ProviderLlmBridge)");
    Some(Box::new(bridge))
}

/// 从 DB 构建 LLM 任务分解器（`LlmBasedDecomposer`）。
///
/// 返回 `None` 表示未配置可用 provider，调用方应回退到 `RuleBasedDecomposer`
/// （规则化兜底，不返回 Err）。
///
/// **Prompt 设计**（由 `LlmBasedDecomposer` 内部实现）：
/// - 系统提示要求 LLM 作为「task decomposition engine」
/// - 用户提示包含 mission / strategy / 可选专家清单
/// - 输出约束为 JSON `{ "sub_tasks": [...] }`，每项含 id / name / description / role / dependencies
/// - 失败时自动回退到 `RuleBasedDecomposer`
pub async fn build_llm_decomposer_from_db(
    master_key: &[u8; 32],
) -> Option<Box<dyn MissionDecomposer>> {
    let (adapter, ctx, _model) = build_llm_components_from_db(master_key).await?;
    let llm_service: Arc<dyn LlmExecutionService> = Arc::new(BridgeLlmExecutionService::new());
    let decomposer = LlmBasedDecomposer::new(adapter, ctx, llm_service);
    tracing::info!("[wiring] MissionDecomposer 注入完成 (LlmBasedDecomposer)");
    Some(Box::new(decomposer))
}
