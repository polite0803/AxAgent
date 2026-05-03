//! ACP 协议类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// ACP 协议版本
pub const ACP_VERSION: &str = "1.0.0";

/// JSON-RPC 2.0 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcpError>,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// ACP 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub session_id: String,
    pub work_dir: String,
    pub status: AcpSessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub permission_mode: String,
    pub active_tasks: usize,
}

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcpSessionStatus {
    Idle,
    Running,
    WaitingForPermission,
    Compacting,
    Closed,
}

/// 权限模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcpPermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

/// Hook 回调注册
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpHookRegistration {
    pub event: String,
    pub callback_url: String,
    pub session_id: String,
}

/// ACP 通知（服务器推送）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpNotification {
    pub event: String,
    pub session_id: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl AcpRequest {
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::Value::String(uuid::Uuid::new_v4().to_string())),
        }
    }
}

impl AcpResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(AcpError {
                code,
                message: message.to_string(),
                data: None,
            }),
            id,
        }
    }

    pub fn is_enabled() -> bool {
        axagent_runtime::feature_flags::global_feature_flags().acp_protocol()
    }
}
