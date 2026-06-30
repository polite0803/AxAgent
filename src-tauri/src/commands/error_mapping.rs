// SPDX-License-Identifier: AGPL-3.0-only

//! AxAgentError → ErrorResponse 桥接映射
//!
//! 将 crates 层的 AxAgentError 映射到命令层的 ErrorResponse，实现依赖方向正确：
//! commands/ → crates/core （允许）
//! crates/core → commands/ （禁止）
//!
//! 使用方式:
//! ```rust,ignore
//! use crate::commands::error_mapping::map_core_error;
//!
//! let result = some_core_function().map_err(|e| map_core_error(&e, "操作名称"))?;
//! ```

use axagent_core::error::AxAgentError;
use super::error::ErrorResponse;
use super::error_code;

/// 将 AxAgentError 映射为 ErrorResponse
///
/// `operation` 参数提供操作上下文，会包含在 detail 中。
pub fn map_core_error(e: &AxAgentError, operation: &str) -> ErrorResponse {
    let detail = format!("[{}] {}", operation, e);
    let code = error_code_for(e);
    ErrorResponse::new(code).with_detail(detail)
}

/// 将 AxAgentError 映射为 ErrorResponse（不附加操作上下文的简化版）
pub fn map_core_error_simple(e: &AxAgentError) -> ErrorResponse {
    let code = error_code_for(e);
    ErrorResponse::new(code).with_detail(e.to_string())
}

/// 根据 AxAgentError 变体返回对应的错误码
fn error_code_for(e: &AxAgentError) -> &'static str {
    match e {
        AxAgentError::Database(_) => error_code::agent_err::NOT_FOUND, // DB 错误通常表现为查找失败
        AxAgentError::Provider(msg) => {
            if msg.contains("timeout") || msg.contains("Timeout") {
                error_code::provider_err::MODEL_LIST_TIMEOUT
            } else if msg.contains("key") || msg.contains("decrypt") {
                error_code::expert_err::KEY_DECRYPT_FAILED
            } else {
                error_code::expert_err::LLM_CALL_FAILED
            }
        },
        AxAgentError::Gateway(_) => error_code::gateway_err::HTTP_UNAVAILABLE,
        AxAgentError::Crypto(_) => error_code::expert_err::KEY_DECRYPT_FAILED,
        AxAgentError::NotFound(_) => error_code::conv_err::NOT_FOUND,
        AxAgentError::Validation(_) => error_code::tool_err::PARAM_REQUIRED,
        AxAgentError::Io(_) => error_code::storage_err::READ_FILE_FAILED,
        AxAgentError::Config(_) => error_code::skill_err::LOAD_FAILED,
        AxAgentError::Timeout(_) => error_code::mcp_err::TIMEOUT,
        AxAgentError::Workflow { .. } => error_code::agent_err::WORKFLOW_NOT_FOUND,
        AxAgentError::Agent { .. } => error_code::agent_err::NOT_FOUND,
        AxAgentError::Execution { .. } => error_code::tool_err::EXECUTION_ERROR,
        AxAgentError::Internal(_) => "INTERNAL_ERROR",
        AxAgentError::StructuredError { message, .. } => {
            // 尝试从 message 中提取已知错误码
            if message.contains("NOT_FOUND") {
                error_code::conv_err::NOT_FOUND
            } else if message.contains("TIMEOUT") {
                error_code::mcp_err::TIMEOUT
            } else {
                "INTERNAL_ERROR"
            }
        },
        AxAgentError::ModelDownload(_) => error_code::provider_err::FETCH_MODELS_FAILED,
        AxAgentError::ModelIntegrity { .. } => error_code::provider_err::FETCH_MODELS_FAILED,
        AxAgentError::Inference(_) => error_code::expert_err::LLM_CALL_FAILED,
        AxAgentError::Rag(_) => error_code::wiki_err::NO_EMBEDDING_PROVIDER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_not_found_error() {
        let err = AxAgentError::NotFound("User not found".to_string());
        let resp = map_core_error(&err, "test_op");
        assert!(resp.code.contains("NOT_FOUND"));
        assert!(resp.detail.unwrap().contains("test_op"));
    }

    #[test]
    fn test_map_provider_error() {
        let err = AxAgentError::Provider("LLM call timeout".to_string());
        let resp = map_core_error_simple(&err);
        assert!(resp.code.contains("TIMEOUT"));
    }

    #[test]
    fn test_map_database_error() {
        use sea_orm::DbErr;
        let db_err = DbErr::Custom("connection lost".to_string());
        let err = AxAgentError::Database(db_err);
        let resp = map_core_error(&err, "db_query");
        assert!(resp.code.contains("NOT_FOUND"));
    }
}
