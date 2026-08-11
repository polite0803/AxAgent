// SPDX-License-Identifier: AGPL-3.0-only

//! 网关运维增强数据模型 (P2-16)
//!
//! 配对授权、生命周期账本、优雅关停相关 DTO

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 配对授权
// ---------------------------------------------------------------------------

/// 配对授权请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingAuthorization {
    /// 授权 ID
    pub id: String,
    /// 客户端 ID
    pub client_id: String,
    /// 客户端名称
    pub client_name: String,
    /// 客户端公钥
    pub client_public_key: String,
    /// 权限范围
    pub scopes: Vec<String>,
    /// 过期时间
    pub expires_at: String,
    /// 状态
    pub status: AuthorizationStatus,
    /// 创建时间
    pub created_at: String,
}

/// 授权状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationStatus {
    Pending,
    Active,
    Expired,
    Revoked,
}

// ---------------------------------------------------------------------------
// 生命周期账本
// ---------------------------------------------------------------------------

/// 生命周期账本
///
/// 记录网关的生命周期事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleLedger {
    /// 账本 ID
    pub id: String,
    /// 事件列表
    pub events: Vec<LifecycleEvent>,
    /// 网关状态
    pub gateway_status: GatewayOpsStatus,
    /// 启动时间
    pub started_at: Option<String>,
    /// 最后更新时间
    pub last_updated: String,
}

/// 生命周期事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub id: String,
    pub event_type: LifecycleEventType,
    pub timestamp: String,
    pub details: HashMap<String, String>,
}

/// 生命周期事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEventType {
    Started,
    ConfigReloaded,
    ClientConnected,
    ClientDisconnected,
    RequestProcessed,
    ErrorOccurred,
    GracefulShutdownStarted,
    GracefulShutdownCompleted,
    ForcedShutdown,
}

/// 网关状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayOpsStatus {
    Starting,
    Running,
    Draining,
    ShuttingDown,
    Stopped,
    Error,
}

// ---------------------------------------------------------------------------
// 优雅关停
// ---------------------------------------------------------------------------

/// 优雅关停配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GracefulShutdownConfig {
    /// 优雅关停超时时间（秒）
    pub timeout_seconds: u64,
    /// 等待进行中请求完成的时间（秒）
    pub wait_for_requests_seconds: u64,
    /// 客户端通知超时（秒）
    pub client_notification_timeout_seconds: u64,
    /// 是否强制关闭超时的请求
    pub force_kill_timeout_requests: bool,
    /// 关闭顺序
    pub shutdown_order: Vec<ShutdownPhase>,
}

impl Default for GracefulShutdownConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            wait_for_requests_seconds: 10,
            client_notification_timeout_seconds: 5,
            force_kill_timeout_requests: false,
            shutdown_order: vec![
                ShutdownPhase::StopNewRequests,
                ShutdownPhase::DrainExistingRequests,
                ShutdownPhase::NotifyClients,
                ShutdownPhase::CloseConnections,
                ShutdownPhase::ReleaseResources,
            ],
        }
    }
}

/// 关停阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShutdownPhase {
    StopNewRequests,
    DrainExistingRequests,
    NotifyClients,
    CloseConnections,
    ReleaseResources,
}

/// 关停进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownProgress {
    pub phase: ShutdownPhase,
    pub status: ShutdownPhaseStatus,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub pending_requests: u64,
}

/// 阶段状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShutdownPhaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}
