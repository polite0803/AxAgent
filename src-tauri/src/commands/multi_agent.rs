// SPDX-License-Identifier: AGPL-3.0-only

//! G5 Multi-Agent 固定角色 pool — Tauri 命令层。
//!
//! 提供 `delegate_task` 命令：将子任务委派给指定 role（analyst/implementer/reviewer），
//! 由后端按 role 的 system_prompt 调用 LLM 完成子任务，返回结构化结果。
//!
//! 设计动机：DojoAgents 宣传口径中的"Multi-Agent 固定角色 pool"——任意 Agent 在执行
//! 过程中可通过 delegate_task 把子任务分给专家角色，实现角色间协作。
//!
//! 调用路径：
//! - 前端 MultiAgentPanel 直接调用 `delegate_task` 命令
//! - 工作流 AgentNode 通过 ToolResolver 路由到 `delegate_task` 工具
//! - MultiAgentTriggerHook 在 pre_llm_call 中检测复杂任务时自动委派

use crate::AppState;
use crate::commands::screen_vision::{build_vision_context, resolve_provider_adapter};
use axagent_dao::repo::agent_role;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

// 复用 G5 种子化时定义的角色 ID 常量
use crate::commands::stock_analysis_setup::seed_multi_agent_roles::{
    ROLE_ANALYST, ROLE_IMPLEMENTER, ROLE_REVIEWER,
};

/// delegate_task 输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateTaskInput {
    /// 目标角色 ID：analyst / implementer / reviewer
    pub role_name: String,
    /// 子任务描述（中文，作为 user message）
    pub task: String,
    /// 上下文变量（可选，会以 JSON 形式注入到 user message 前）
    #[serde(default)]
    pub context: serde_json::Value,
    /// LLM 供应商 ID
    pub provider_id: String,
    /// 模型 ID
    pub model_id: String,
    /// 温度（可选，默认 0.2 以保证稳定性）
    #[serde(default)]
    pub temperature: Option<f64>,
    /// 最大输出 tokens（可选，默认 2048）
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// delegate_task 输出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateTaskResult {
    /// 委派 ID（用于追踪）
    pub delegation_id: String,
    /// 角色 ID
    pub role_name: String,
    /// LLM 生成的文本输出
    pub content: String,
    /// token 使用情况（prompt + completion）
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// 调用耗时（毫秒）
    pub duration_ms: u64,
}

/// 校验 role_name 是否为 G5 固定角色
fn validate_role(role_name: &str) -> Result<(), String> {
    match role_name {
        ROLE_ANALYST | ROLE_IMPLEMENTER | ROLE_REVIEWER => Ok(()),
        _ => Err(format!(
            "delegate_task 仅支持 G5 固定角色 (analyst/implementer/reviewer)，收到: {}",
            role_name
        )),
    }
}

/// 委派任务给指定 Multi-Agent 角色。
///
/// 内部流程：
/// 1. 校验 role_name ∈ {analyst, implementer, reviewer}
/// 2. 从 DB agent_roles 表读取该 role 的 system_prompt
/// 3. 构造 ChatRequest（system: role.system_prompt, user: task + context）
/// 4. 调用 LLM provider 完成子任务
/// 5. 返回结构化结果（含 token 用量、耗时）
#[tauri::command]
pub async fn delegate_task(
    state: State<'_, AppState>,
    input: DelegateTaskInput,
) -> Result<DelegateTaskResult, String> {
    validate_role(&input.role_name)?;

    let started = std::time::Instant::now();

    // 1. 从 DB 读取 role 的 system_prompt
    let role = agent_role::get_agent_role(state.harness.db(), &input.role_name)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| format!("Role '{}' not found in agent_roles table", input.role_name))?;

    // 2. 构造 vision context（含 adapter + ctx + api_key）
    let vision =
        build_vision_context(state.harness.db(), state.harness.master_key(), &input.provider_id)
            .await?;

    // 3. 构造 user message（task + context 拼接）
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

/// 让 resolve_provider_adapter 在本模块可见（screen_vision 已 pub(crate)）
#[allow(dead_code)]
fn _ensure_resolve_visible() {
    let _ = resolve_provider_adapter;
}
