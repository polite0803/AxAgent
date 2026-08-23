// SPDX-License-Identifier: AGPL-3.0-only

//! 错误分类与故障转移命令 (P0-5)
//!
//! 借鉴 Hermes Agent 的 error_classifier.py：
//! - 结构化三级分类（HTTP 状态码 → 错误码 → 消息正则）
//! - FailoverReason 枚举供前端精确提示

use crate::AppState;
use agent_macro::agent_command;
use axagent_agent::recovery_strategies::ErrorClassifier;
use axagent_harness::error_classifier::ClassifiedError;
use serde::{Deserialize, Serialize};
use tauri::State;

/// 分类 HTTP 错误请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyHttpErrorRequest {
    pub status: u16,
    pub message: String,
    pub provider_error_code: Option<String>,
}

/// 分类非 HTTP 错误请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyNonHttpErrorRequest {
    pub message: String,
}

/// 分类 HTTP 错误
#[agent_command(domain = system, safety = Safe, call_mode = StateOnly, description = "分类 HTTP 错误")]
#[tauri::command]
pub async fn classify_http_error(
    _state: State<'_, AppState>,
    req: ClassifyHttpErrorRequest,
) -> Result<ClassifiedError, String> {
    let classifier = ErrorClassifier::new();
    Ok(classifier.classify_http_error(req.status, &req.message))
}

/// 分类非 HTTP 错误
#[agent_command(domain = system, safety = Safe, call_mode = StateOnly, description = "分类非 HTTP 错误")]
#[tauri::command]
pub async fn classify_non_http_error(
    _state: State<'_, AppState>,
    req: ClassifyNonHttpErrorRequest,
) -> Result<ClassifiedError, String> {
    let classifier = ErrorClassifier::new();
    Ok(classifier.classify_with_reason(&req.message))
}

/// 获取故障转移原因列表
#[agent_command(domain = system, safety = Safe, call_mode = StateOnly, description = "获取故障转移原因")]
#[tauri::command]
pub async fn get_failover_reasons() -> Result<Vec<serde_json::Value>, String> {
    use axagent_harness::error_classifier::FailoverReason;

    let reasons = vec![
        FailoverReason::NetworkTimeout,
        FailoverReason::RateLimit,
        FailoverReason::ProviderOutage,
        FailoverReason::AuthFailed,
        FailoverReason::QuotaExceeded,
        FailoverReason::ContextLength,
        FailoverReason::ModelNotFound,
        FailoverReason::InvalidParameters,
        FailoverReason::UpstreamProviderError,
        FailoverReason::ContentBlocked,
        FailoverReason::CostLimit,
        FailoverReason::UnknownError,
    ];

    let result: Vec<serde_json::Value> = reasons
        .iter()
        .map(|reason| {
            // description 由前端按 code 走 i18n 翻译（11 语言），此处不硬编码
            serde_json::json!({
                "code": reason.as_str(),
                "retryable": reason.is_retryable(),
                "requiresUserAction": reason.requires_user_action(),
                "shouldFailFast": reason.should_fail_fast(),
            })
        })
        .collect();

    Ok(result)
}
