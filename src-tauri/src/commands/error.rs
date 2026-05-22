//! 统一的错误响应结构
//!
//! 后端返回错误码，前端负责翻译
//!
//! 使用方式:
//! ```rust
//! use crate::commands::error::ErrorResponse;
//!
//! // 简单错误
//! return Err(ErrorResponse::new(error_code::conversation::NOT_FOUND));
//!
//! // 带详情错误
//! return Err(ErrorResponse::new(error_code::tool::NOT_FOUND)
//!     .with_detail(format!("Tool '{}' not found", tool_name)));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::string::ToString;

/// 统一错误响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 错误码，用于前端 i18n 翻译查询
    pub code: String,

    /// 技术详情，用于调试和日志记录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// 翻译参数，用于替换错误消息中的占位符
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, String>>,
}

impl ErrorResponse {
    /// 创建新的错误响应
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: None,
            params: None,
        }
    }

    pub fn err(code: impl Into<String>) -> String {
        Self::new(code).to_string()
    }
    pub fn err_with_detail(code: impl Into<String>, detail: impl Into<String>) -> String {
        Self::new(code).with_detail(detail).to_string()
    }
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// 添加翻译参数
    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = Some(params);
        self
    }

    /// 添加单个翻译参数
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        let value = value.into();

        match self.params {
            Some(ref mut params) => {
                params.insert(key, value);
            },
            None => {
                let mut params = HashMap::new();
                params.insert(key, value);
                self.params = Some(params);
            },
        }
        self
    }
}

/// 从 String 转换为 ErrorResponse
impl From<String> for ErrorResponse {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// 将 ErrorResponse 转换为 String，使 `?` 运算符和 `.into()` 可以直接使用
impl From<ErrorResponse> for String {
    fn from(e: ErrorResponse) -> Self {
        e.to_string()
    }
}

/// 从 &str 转换为 ErrorResponse
impl From<&str> for ErrorResponse {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// 从 (String, String) 元组转换为 ErrorResponse
/// 元组格式: (code, detail)
impl From<(String, String)> for ErrorResponse {
    fn from((code, detail): (String, String)) -> Self {
        Self::new(code).with_detail(detail)
    }
}

/// 从 (&str, &str) 元组转换为 ErrorResponse
impl From<(&str, &str)> for ErrorResponse {
    fn from((code, detail): (&str, &str)) -> Self {
        Self::new(code).with_detail(detail)
    }
}

/// 将 ErrorResponse 转换为 JSON 字符串
impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                r#"{{"code":"{}","detail":{}}}"#,
                self.code,
                self.detail
                    .as_ref()
                    .map(|d| format!(r#""{}""#, d))
                    .unwrap_or_else(|| "null".to_string())
            )
        });
        write!(f, "{}", json)
    }
}
