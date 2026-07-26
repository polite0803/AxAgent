// SPDX-License-Identifier: AGPL-3.0-only

//! Plugin hook trait — dependency inversion boundary for plugin system.
//!
//! `axagent-runtime-core` implements this trait; `axagent-plugins` consumes it
//! without depending on `runtime-core`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallContext {
    pub tool_name: String,
    pub tool_namespace: Option<String>,
    pub arguments: serde_json::Value,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub success: bool,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallContext {
    pub model: String,
    pub message_count: usize,
    pub tool_count: usize,
    pub estimated_tokens: Option<u64>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallResult {
    pub content: String,
    pub tool_calls: Option<Vec<String>>,
    pub usage_prompt_tokens: Option<u32>,
    pub usage_completion_tokens: Option<u32>,
    pub duration_ms: Option<u64>,
}

// ── G20: API 请求钩子上下文与结果 ──────────────────────────────────────
//
// `pre_llm_call` / `post_llm_call` 仅覆盖 LLM 推理调用；
// `pre_api_request` / `post_api_request` 覆盖所有出站 HTTP API 请求
// （LLM 推理 / 数据源 / 第三方集成 / Webhook 等），
// 用于统一拦截、计量、限流、审计、Header 注入与重写。

/// API 请求上下文 — 发起 HTTP 调用前的元信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallContext {
    /// 请求目标 URL（已合并 base_url + path）
    pub url: String,
    /// HTTP 方法（GET/POST/PUT/DELETE/...）
    pub method: String,
    /// 调用类别：`llm` / `data_source` / `webhook` / `third_party` / `internal`
    pub category: String,
    /// 目标服务标识（如 provider_id、vendor_name、webhook_id）
    pub service_id: Option<String>,
    /// 请求头（敏感字段如 Authorization 由 post_api_request 脱敏后回传）
    pub headers: serde_json::Value,
    /// 请求体（JSON Value；非 JSON 请求体此处可能为 null）
    pub body: serde_json::Value,
    /// 超时（毫秒）
    pub timeout_ms: Option<u64>,
    /// 关联会话 ID（若有）
    pub session_id: Option<String>,
    /// 重试次数（首次为 0）
    pub retry_count: u32,
}

impl ApiCallContext {
    pub fn new(
        url: impl Into<String>,
        method: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            method: method.into(),
            category: category.into(),
            service_id: None,
            headers: serde_json::json!({}),
            body: serde_json::json!(null),
            timeout_ms: None,
            session_id: None,
            retry_count: 0,
        }
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn with_service(mut self, service_id: &str) -> Self {
        self.service_id = Some(service_id.to_string());
        self
    }
}

/// API 请求结果 — HTTP 调用完成后的元信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallResult {
    /// 最终响应 URL（经历重定向后可能与请求 URL 不同）
    pub url: String,
    /// HTTP 状态码
    pub status: u16,
    /// 是否被判定为成功（2xx）
    pub success: bool,
    /// 响应头（脱敏后的子集，仅保留必要字段）
    pub headers: serde_json::Value,
    /// 响应体（JSON Value；非 JSON 响应体此处为字符串）
    pub body: serde_json::Value,
    /// 实际耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如有）
    pub error: Option<String>,
    /// 重试次数（实际发生）
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub hook_name: String,
    pub session_id: Option<String>,
    pub metadata: serde_json::Value,
}

impl HookContext {
    pub fn new(hook_name: &str) -> Self {
        Self { hook_name: hook_name.to_string(), session_id: None, metadata: serde_json::json!({}) }
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookDecision {
    Allow,
    Veto { reason: String },
    Modify { changes: serde_json::Value },
}

#[async_trait]
pub trait PluginHook: Send + Sync {
    fn name(&self) -> &str;

    fn priority(&self) -> i32 {
        0
    }

    async fn on_session_start(&self, _session_id: &str) {}
    async fn on_session_end(&self, _session_id: &str) {}

    async fn pre_tool_call(&self, _ctx: &ToolCallContext) -> Option<HookDecision> {
        None
    }

    async fn post_tool_call(&self, _ctx: &ToolCallContext, _result: &ToolCallResult) {}

    async fn transform_tool_result(
        &self,
        _tool_name: &str,
        result: serde_json::Value,
    ) -> Option<serde_json::Value> {
        Some(result)
    }

    async fn pre_llm_call(&self, _ctx: &LlmCallContext) -> Option<HookDecision> {
        None
    }

    async fn post_llm_call(&self, _ctx: &LlmCallContext, _result: &LlmCallResult) {}

    // ── G20: 通用 HTTP API 请求钩子（覆盖 LLM / 数据源 / Webhook 等所有出站请求）──
    //
    // 与 `pre_llm_call` / `post_llm_call` 的区别：
    // - LLM 钩子只覆盖大模型推理调用，语义聚焦于 prompt/tool_calls
    // - API 钩子覆盖所有 HTTP 出站请求，语义聚焦于 url/method/headers/body/status
    //
    // 用例：限流 / SSRF 防护 / Authorization 注入 / 审计日志 / 重试策略 / 脱敏

    /// HTTP 请求发起前触发。
    ///
    /// 返回 `Some(HookDecision::Veto { .. })` 阻止请求；
    /// 返回 `Some(HookDecision::Modify { changes })` 修改请求（changes 字段：
    /// `headers`/`body`/`url`/`timeout_ms`，由调用方解释并应用）；
    /// 返回 `None` 放行原请求。
    async fn pre_api_request(&self, _ctx: &ApiCallContext) -> Option<HookDecision> {
        None
    }

    /// HTTP 请求完成后触发（无论成功或失败）。
    ///
    /// `result.success == false` 时 `result.error` 包含错误信息；
    /// `result.success == true` 时 `result.body` 包含解析后的响应体。
    async fn post_api_request(&self, _ctx: &ApiCallContext, _result: &ApiCallResult) {}

    async fn transform_llm_response(&self, content: String) -> String {
        content
    }

    async fn transform_terminal_output(&self, output: String) -> String {
        output
    }

    async fn on_error(&self, _error: &str, _context: Option<serde_json::Value>) {}
}

pub type SharedHook = Arc<dyn PluginHook>;

// ── G20: HTTP Hook 执行器 ──────────────────────────────────────────────
//
// `HttpHookExecutor` 封装 pre_api_request/post_api_request 的调用流程，
// 让 reqwest 调用点能用最少代码接入 G20 hook。
//
// ## 用法
//
// ```ignore
// use axagent_harness::plugin_hook::{ApiCallContext, HttpHookExecutor};
//
// let executor = HttpHookExecutor::new(hooks);  // hooks: Vec<SharedHook>
// let ctx = ApiCallContext::new(url, "POST", "llm")
//     .with_service("openai")
//     .with_session(&session_id);
//
// // 1. 调用 pre_api_request，检查是否被 Veto / Modify
// let decision = executor.pre_request(&ctx).await;
// match decision {
//     None | Some(HookDecision::Allow) => { /* 放行 */ },
//     Some(HookDecision::Veto { reason }) => { return Err(reason); },
//     Some(HookDecision::Modify { changes }) => { /* 应用 changes 到请求 */ },
// }
//
// // 2. 执行 reqwest 请求...
//
// // 3. 调用 post_api_request
// executor.post_request(&ctx, &result).await;
// ```

/// G20 HTTP Hook 执行器：在 reqwest 调用点附近包裹 pre/post hook。
///
/// 设计原则：
/// - **零依赖**：不依赖 reqwest，仅依赖 `ApiCallContext`/`ApiCallResult` DTO
/// - **顺序保证**：pre 按 priority 升序执行，post 按 priority 降序执行（类似洋葱模型）
/// - **短路**：pre 返回 `Veto` 时立即停止后续 hook 并返回
pub struct HttpHookExecutor {
    hooks: Vec<SharedHook>,
}

/// pre_api_request 的执行结果
pub enum PreApiHookOutcome {
    /// 所有 hook 都放行
    Allow,
    /// 至少一个 hook 否决了请求
    Veto { reason: String, hook_name: String },
    /// 至少一个 hook 修改了请求（后执行的 hook 看到前一个的修改）
    Modify { changes: serde_json::Value, hook_name: String },
}

impl HttpHookExecutor {
    pub fn new(hooks: Vec<SharedHook>) -> Self {
        // 按 priority 升序排列（数值小的先执行）
        let mut sorted = hooks;
        sorted.sort_by_key(|h| h.priority());
        Self { hooks: sorted }
    }

    /// 空执行器（无 hook）—— 用于未启用 plugin 的场景
    pub fn noop() -> Self {
        Self { hooks: Vec::new() }
    }

    /// 是否完全没有 hook
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// 执行所有 hook 的 pre_api_request，返回最终决策
    pub async fn pre_request(&self, ctx: &ApiCallContext) -> PreApiHookOutcome {
        for hook in &self.hooks {
            match hook.pre_api_request(ctx).await {
                Some(HookDecision::Veto { reason }) => {
                    return PreApiHookOutcome::Veto { reason, hook_name: hook.name().to_string() };
                },
                Some(HookDecision::Modify { changes }) => {
                    return PreApiHookOutcome::Modify {
                        changes,
                        hook_name: hook.name().to_string(),
                    };
                },
                Some(HookDecision::Allow) | None => {},
            }
        }
        PreApiHookOutcome::Allow
    }

    /// 执行所有 hook 的 post_api_request（按 priority 降序，洋葱模型）
    pub async fn post_request(&self, ctx: &ApiCallContext, result: &ApiCallResult) {
        // post 按 priority 降序执行（与 pre 相反，形成洋葱模型）
        for hook in self.hooks.iter().rev() {
            hook.post_api_request(ctx, result).await;
        }
    }
}
