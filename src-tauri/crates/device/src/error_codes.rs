// SPDX-License-Identifier: AGPL-3.0-only

//! 设备同步错误码
//!
//! 定义设备同步相关的错误码和错误类型，
//! 用于统一前后端错误处理和国际化。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 同步错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyncErrorCode {
    // ─── 设备管理 ──────────────────────────────────────────────────────
    /// 设备未找到
    DeviceNotFound,
    /// 设备已配对
    DeviceAlreadyPaired,
    /// 设备未配对
    DeviceNotPaired,
    /// 设备已禁用
    DeviceDisabled,
    /// 设备配对码无效
    InvalidPairingCode,
    /// 配对码已过期
    PairingCodeExpired,
    /// 设备信任级别不足
    InsufficientTrustLevel,

    // ─── 权限管理 ──────────────────────────────────────────────────────
    /// 权限不足
    PermissionDenied,
    /// 权限未配置
    PermissionNotConfigured,
    /// 设备未注册
    DeviceNotRegistered,

    // ─── 同步操作 ──────────────────────────────────────────────────────
    /// 同步失败
    SyncFailed,
    /// 同步超时
    SyncTimeout,
    /// 冲突检测失败
    ConflictDetectionFailed,
    /// 冲突解决失败
    ConflictResolutionFailed,
    /// 同步已在进行中
    SyncAlreadyInProgress,
    /// 无待同步变更
    NoChangesToSync,

    // ─── 加密 ──────────────────────────────────────────────────────────
    /// 加密失败
    EncryptionFailed,
    /// 解密失败
    DecryptionFailed,
    /// 密钥派生失败
    KeyDerivationFailed,
    /// 无效的加密数据
    InvalidEncryptedData,

    // ─── 传输 ──────────────────────────────────────────────────────────
    /// 网络连接失败
    ConnectionFailed,
    /// 传输超时
    TransportTimeout,
    /// 数据格式无效
    InvalidDataFormat,

    // ─── CRDT ──────────────────────────────────────────────────────────
    /// CRDT 文档未找到
    CrdtDocumentNotFound,
    /// CRDT 操作转换失败
    CrdtTransformFailed,
    /// CRDT 合并失败
    CrdtMergeFailed,

    // ─── 调度器 ────────────────────────────────────────────────────────
    /// 调度器队列已满
    SchedulerQueueFull,
    /// 调度器已在运行
    SchedulerAlreadyRunning,
    /// 任务未找到
    TaskNotFound,

    // ─── 数据存储 ──────────────────────────────────────────────────────
    /// 存储操作失败
    StorageOperationFailed,
    /// 数据序列化失败
    SerializationFailed,
    /// 数据反序列化失败
    DeserializationFailed,
}

impl SyncErrorCode {
    /// 获取错误码的分类
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::DeviceNotFound
            | Self::DeviceAlreadyPaired
            | Self::DeviceNotPaired
            | Self::DeviceDisabled
            | Self::InvalidPairingCode
            | Self::PairingCodeExpired
            | Self::InsufficientTrustLevel => ErrorCategory::Device,

            Self::PermissionDenied
            | Self::PermissionNotConfigured
            | Self::DeviceNotRegistered => ErrorCategory::Permission,

            Self::SyncFailed
            | Self::SyncTimeout
            | Self::ConflictDetectionFailed
            | Self::ConflictResolutionFailed
            | Self::SyncAlreadyInProgress
            | Self::NoChangesToSync => ErrorCategory::Sync,

            Self::EncryptionFailed
            | Self::DecryptionFailed
            | Self::KeyDerivationFailed
            | Self::InvalidEncryptedData => ErrorCategory::Encryption,

            Self::ConnectionFailed
            | Self::TransportTimeout
            | Self::InvalidDataFormat => ErrorCategory::Transport,

            Self::CrdtDocumentNotFound
            | Self::CrdtTransformFailed
            | Self::CrdtMergeFailed => ErrorCategory::Crdt,

            Self::SchedulerQueueFull
            | Self::SchedulerAlreadyRunning
            | Self::TaskNotFound => ErrorCategory::Scheduler,

            Self::StorageOperationFailed
            | Self::SerializationFailed
            | Self::DeserializationFailed => ErrorCategory::Storage,
        }
    }

    /// 获取默认错误消息
    pub fn default_message(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "设备未找到",
            Self::DeviceAlreadyPaired => "设备已配对",
            Self::DeviceNotPaired => "设备未配对",
            Self::DeviceDisabled => "设备已禁用",
            Self::InvalidPairingCode => "配对码无效",
            Self::PairingCodeExpired => "配对码已过期",
            Self::InsufficientTrustLevel => "信任级别不足",

            Self::PermissionDenied => "权限不足",
            Self::PermissionNotConfigured => "权限未配置",
            Self::DeviceNotRegistered => "设备未注册",

            Self::SyncFailed => "同步失败",
            Self::SyncTimeout => "同步超时",
            Self::ConflictDetectionFailed => "冲突检测失败",
            Self::ConflictResolutionFailed => "冲突解决失败",
            Self::SyncAlreadyInProgress => "同步已在进行中",
            Self::NoChangesToSync => "无待同步变更",

            Self::EncryptionFailed => "加密失败",
            Self::DecryptionFailed => "解密失败",
            Self::KeyDerivationFailed => "密钥派生失败",
            Self::InvalidEncryptedData => "无效的加密数据",

            Self::ConnectionFailed => "网络连接失败",
            Self::TransportTimeout => "传输超时",
            Self::InvalidDataFormat => "数据格式无效",

            Self::CrdtDocumentNotFound => "CRDT 文档未找到",
            Self::CrdtTransformFailed => "CRDT 操作转换失败",
            Self::CrdtMergeFailed => "CRDT 合并失败",

            Self::SchedulerQueueFull => "调度器队列已满",
            Self::SchedulerAlreadyRunning => "调度器已在运行",
            Self::TaskNotFound => "任务未找到",

            Self::StorageOperationFailed => "存储操作失败",
            Self::SerializationFailed => "数据序列化失败",
            Self::DeserializationFailed => "数据反序列化失败",
        }
    }
}

/// 错误分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// 设备管理
    Device,
    /// 权限管理
    Permission,
    /// 同步操作
    Sync,
    /// 加密
    Encryption,
    /// 传输
    Transport,
    /// CRDT
    Crdt,
    /// 调度器
    Scheduler,
    /// 数据存储
    Storage,
}

/// 同步错误
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase")]
pub struct SyncError {
    /// 错误码
    pub code: SyncErrorCode,
    /// 错误分类
    pub category: ErrorCategory,
    /// 错误详情
    pub message: String,
    /// 动态参数（用于国际化）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<std::collections::HashMap<String, String>>,
}

impl SyncError {
    /// 创建新的同步错误
    pub fn new(code: SyncErrorCode) -> Self {
        Self {
            category: code.category(),
            message: code.default_message().to_string(),
            code,
            params: None,
        }
    }

    /// 创建带自定义消息的错误
    pub fn with_message(code: SyncErrorCode, message: impl Into<String>) -> Self {
        Self {
            category: code.category(),
            message: message.into(),
            code,
            params: None,
        }
    }

    /// 创建带参数的错误
    pub fn with_params(
        code: SyncErrorCode,
        params: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            category: code.category(),
            message: code.default_message().to_string(),
            code,
            params: Some(params),
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl From<SyncError> for String {
    fn from(err: SyncError) -> Self {
        serde_json::to_string(&err).unwrap_or(err.message)
    }
}

/// 便捷方法：创建设备未找到错误
pub fn device_not_found(device_id: impl Into<String>) -> SyncError {
    let mut params = std::collections::HashMap::new();
    params.insert("deviceId".to_string(), device_id.into());
    SyncError::with_params(SyncErrorCode::DeviceNotFound, params)
}

/// 便捷方法：创建权限不足错误
pub fn permission_denied(permission: impl Into<String>) -> SyncError {
    let mut params = std::collections::HashMap::new();
    params.insert("permission".to_string(), permission.into());
    SyncError::with_params(SyncErrorCode::PermissionDenied, params)
}

/// 便捷方法：创建同步失败错误
pub fn sync_failed(detail: impl Into<String>) -> SyncError {
    SyncError::with_message(SyncErrorCode::SyncFailed, detail)
}

/// 便捷方法：创建加密错误
pub fn encryption_failed(detail: impl Into<String>) -> SyncError {
    SyncError::with_message(SyncErrorCode::EncryptionFailed, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_category() {
        assert_eq!(
            SyncErrorCode::DeviceNotFound.category(),
            ErrorCategory::Device
        );
        assert_eq!(
            SyncErrorCode::PermissionDenied.category(),
            ErrorCategory::Permission
        );
        assert_eq!(SyncErrorCode::SyncFailed.category(), ErrorCategory::Sync);
    }

    #[test]
    fn test_error_creation() {
        let err = SyncError::new(SyncErrorCode::DeviceNotFound);
        assert_eq!(err.code, SyncErrorCode::DeviceNotFound);
        assert_eq!(err.category, ErrorCategory::Device);
        assert!(!err.message.is_empty());
    }

    #[test]
    fn test_error_with_params() {
        let err = device_not_found("device-123");
        assert_eq!(err.code, SyncErrorCode::DeviceNotFound);
        assert!(err.params.is_some());
    }

    #[test]
    fn test_error_serialization() {
        let err = SyncError::new(SyncErrorCode::SyncFailed);
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("SYNC_FAILED"));
    }
}
