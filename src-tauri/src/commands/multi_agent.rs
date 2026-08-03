// SPDX-License-Identifier: AGPL-3.0-only

//! G5 Multi-Agent 固定角色 pool — Tauri 命令层 + DelegateTaskRunner 实现。
//!
//! 提供：
//! - `delegate_task` Tauri 命令（前端直接调用）
//! - `DelegateTaskRunnerImpl` 实现 harness 契约，供 `DelegateTaskTool` 注入
//! - `init_delegate_task_runner()` wiring 函数，在 init 时注入到 tools crate
//!
//! 调用路径：
//! - 前端 MultiAgentPanel 直接调用 `delegate_task` 命令
//! - Agent LLM 通过 DelegateTaskTool 调用（工具系统触发）
//! - MultiAgentTriggerHook 在 pre_llm_call 中自动委派

use crate::AppState;
#[cfg(not(mobile))]
use crate::commands::provider_ctx::build_vision_context;
use axagent_dao::repo::agent_role;
#[cfg(not(mobile))]
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{DelegateTaskInput, DelegateTaskResult, DelegateTaskRunner};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use tauri::State;

// ── 全局 HookChain（供 conversation loop 挂载使用）──

static GLOBAL_HOOK_CHAIN: OnceLock<Arc<axagent_runtime_core::HookChain>> = OnceLock::new();

/// 获取全局 HookChain 的 Arc 副本（init 时由 `register_global_multi_agent_hook()` 初始化）。
pub fn get_global_hook_chain() -> Option<Arc<axagent_runtime_core::HookChain>> {
    GLOBAL_HOOK_CHAIN.get().cloned()
}

/// 在 init 阶段创建 HookChain、注册 MultiAgentTriggerHook，存入全局 static。
///
/// 调用时机：`init_delegate_task_runner()` 之后，
/// 在 `src/init/state.rs` 的 init 函数中调用。
///
/// 当前在同步 init 上下文中通过 `block_on` 完成异步 registration。
pub fn register_global_multi_agent_hook() {
    // 只初始化一次
    if GLOBAL_HOOK_CHAIN.get().is_some() {
        return;
    }
    let chain = Arc::new(axagent_runtime_core::HookChain::new());
    let hook = axagent_agent::multi_agent_hook::create_multi_agent_trigger_hook();
    // 先 set 再注册 hook
    let _ = GLOBAL_HOOK_CHAIN.set(chain);
    if let Some(chain) = GLOBAL_HOOK_CHAIN.get() {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                chain.register(hook).await;
                tracing::info!(
                    target: "axagent.multi_agent",
                    "已注册 MultiAgentTriggerHook 到全局 HookChain"
                );
            });
        });
    }
}

// 复用 G5 种子化时定义的角色 ID 常量
use crate::commands::multi_agent_setup::seed_multi_agent_roles::{
    ROLE_ANALYST, ROLE_IMPLEMENTER, ROLE_REVIEWER,
};

// ── 实现 DelegateTaskRunner trait ──────────────────────────────────────

/// `DelegateTaskRunner` 的 wiring 层实现。
///
/// 持有 `AppState` 中需要的资源（db / master_key），
/// 由 `init_delegate_task_runner()` 创建并注入到 tools crate。
pub struct DelegateTaskRunnerImpl {
    #[cfg_attr(mobile, allow(dead_code))]
    pub db: sea_orm::DatabaseConnection,
    #[cfg_attr(mobile, allow(dead_code))]
    pub master_key: [u8; 32],
}

#[async_trait::async_trait]
impl DelegateTaskRunner for DelegateTaskRunnerImpl {
    #[cfg(mobile)]
    async fn delegate(&self, _input: DelegateTaskInput) -> Result<DelegateTaskResult, String> {
        Err("Multi-Agent 委派在移动端不可用".to_string())
    }

    #[cfg(not(mobile))]
    async fn delegate(&self, input: DelegateTaskInput) -> Result<DelegateTaskResult, String> {
        validate_role(&input.role_name)?;

        let started = std::time::Instant::now();

        // 1. 从 DB 读取 role 的 system_prompt
        let role = agent_role::get_agent_role(&self.db, &input.role_name)
            .await
            .map_err(|e| format!("DB 查询 agent_role 失败: {}", e))?
            .ok_or_else(|| format!("Role '{}' 未找到", input.role_name))?;

        // 2. 构造 vision context（含 adapter + ctx + api_key）
        let vision = build_vision_context(&self.db, &self.master_key, &input.provider_id).await?;

        // 3. 构造 user message
        let user_content = if input.context.is_null() {
            input.task.clone()
        } else {
            format!(
                "## 任务\n\n{}\n\n## 上下文\n\n```json\n{}\n```",
                input.task,
                serde_json::to_string_pretty(&input.context).unwrap_or_else(|_| "{}".to_string())
            )
        };

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(role.system_prompt.clone()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(user_content),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ];

        let request = Arc::new(ChatRequest {
            model: input.model_id.clone(),
            messages,
            stream: false,
            temperature: Some(input.temperature.unwrap_or(0.2)),
            top_p: None,
            max_tokens: Some(input.max_tokens.unwrap_or(2048)),
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
            response_format: None,
        });

        // 4. 调用 LLM
        let response = vision
            .adapter
            .chat(&vision.ctx, request)
            .await
            .map_err(|e| format!("LLM 调用失败: {}", e))?;

        let duration_ms = started.elapsed().as_millis() as u64;

        Ok(DelegateTaskResult {
            delegation_id: format!("del-{}", axagent_kit::utils::now_ts()),
            role_name: input.role_name,
            content: response.content,
            prompt_tokens: response.usage.input_tokens as u64,
            completion_tokens: response.usage.output_tokens as u64,
            duration_ms,
        })
    }
}

/// 在 init 阶段注入 DelegateTaskRunner 到 tools crate。
pub fn init_delegate_task_runner(db: sea_orm::DatabaseConnection, master_key: [u8; 32]) {
    let runner = Arc::new(DelegateTaskRunnerImpl { db, master_key });
    axagent_tools::tools::multi_agent::set_delegate_task_runner(runner);
}

// ── 原有的 validate_role + Tauri 命令 ──────────────────────────────────

/// 校验 role_name 是否为 G5 固定角色
#[cfg(not(mobile))]
fn validate_role(role_name: &str) -> Result<(), String> {
    match role_name {
        ROLE_ANALYST | ROLE_IMPLEMENTER | ROLE_REVIEWER => Ok(()),
        _ => Err(format!(
            "delegate_task 仅支持 G5 固定角色 (analyst/implementer/reviewer)，收到: {}",
            role_name
        )),
    }
}

/// 委派任务给指定 Multi-Agent 角色（Tauri 命令入口）。
///
/// 委托给 `DelegateTaskRunnerImpl.delegate()` 实现，确保 Tauri 命令与
/// Tool 走同一套业务逻辑。错误统一使用 `ErrorResponse` + 错误码。
#[tauri::command]
pub async fn delegate_task(
    state: State<'_, AppState>,
    input: DelegateTaskInput,
) -> Result<DelegateTaskResult, String> {
    let runner = DelegateTaskRunnerImpl {
        db: state.harness.db().clone(),
        master_key: state.harness.master_key_owned(),
    };
    let result = runner.delegate(input).await.map_err(|e| {
        String::from(
            crate::commands::error::ErrorResponse::new(
                crate::commands::error_code::multi_agent::DELEGATE_FAILED,
            )
            .with_category(crate::commands::error::ErrorCategory::Retryable)
            .with_detail(format!("委托任务执行失败: {}", e)),
        )
    })?;
    Ok(result)
}

/// 列出 G5 Multi-Agent 固定角色（前端 UI 用）
#[tauri::command]
pub async fn list_multi_agent_roles(
    state: State<'_, AppState>,
) -> Result<Vec<MultiAgentRoleInfo>, String> {
    let db = state.harness.db();
    let mut result = Vec::with_capacity(3);
    for &role_id in &[ROLE_ANALYST, ROLE_IMPLEMENTER, ROLE_REVIEWER] {
        if let Ok(Some(role)) = agent_role::get_agent_role(db, role_id).await {
            result.push(MultiAgentRoleInfo {
                id: role.id,
                name: role.name,
                description: role.description.unwrap_or_default(),
                max_concurrent: role.max_concurrent as i32,
                timeout_seconds: role.timeout_seconds as i64,
            });
        }
    }
    Ok(result)
}

/// G5 角色信息（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiAgentRoleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub max_concurrent: i32,
    pub timeout_seconds: i64,
}
