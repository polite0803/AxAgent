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

    // ─── 冲突处理 ──────────────────────────────────────────────────────
    /// 冲突未找到
    ConflictNotFound,

    // ─── 加密验证 ──────────────────────────────────────────────────────
    /// 需要密码
    PasswordRequired,
    /// 密码为空
    PasswordEmpty,
    /// 需要盐值
    SaltRequired,

    // ─── 策略管理 ──────────────────────────────────────────────────────
    /// 策略操作失败
    PolicyOperationFailed,

    // ─── 权限管理（扩展） ──────────────────────────────────────────────
    /// 权限未找到
    PermissionsNotFound,
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
            | Self::DeviceNotRegistered
            | Self::PermissionsNotFound => ErrorCategory::Permission,

            Self::SyncFailed
            | Self::SyncTimeout
            | Self::ConflictDetectionFailed
            | Self::ConflictResolutionFailed
            | Self::ConflictNotFound
            | Self::SyncAlreadyInProgress
            | Self::NoChangesToSync => ErrorCategory::Sync,

            Self::EncryptionFailed
            | Self::DecryptionFailed
            | Self::KeyDerivationFailed
            | Self::InvalidEncryptedData
            | Self::PasswordRequired
            | Self::PasswordEmpty
            | Self::SaltRequired => ErrorCategory::Encryption,

            Self::ConnectionFailed | Self::TransportTimeout | Self::InvalidDataFormat => {
                ErrorCategory::Transport
            },

            Self::CrdtDocumentNotFound | Self::CrdtTransformFailed | Self::CrdtMergeFailed => {
                ErrorCategory::Crdt
            },

            Self::SchedulerQueueFull | Self::SchedulerAlreadyRunning | Self::TaskNotFound => {
                ErrorCategory::Scheduler
            },

            Self::StorageOperationFailed
            | Self::SerializationFailed
            | Self::DeserializationFailed => ErrorCategory::Storage,

            Self::PolicyOperationFailed => ErrorCategory::Sync,
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
            Self::PermissionsNotFound => "权限未找到",

            Self::SyncFailed => "同步失败",
            Self::SyncTimeout => "同步超时",
            Self::ConflictDetectionFailed => "冲突检测失败",
            Self::ConflictResolutionFailed => "冲突解决失败",
            Self::ConflictNotFound => "冲突未找到",
            Self::SyncAlreadyInProgress => "同步已在进行中",
            Self::NoChangesToSync => "无待同步变更",

            Self::EncryptionFailed => "加密失败",
            Self::DecryptionFailed => "解密失败",
            Self::KeyDerivationFailed => "密钥派生失败",
            Self::InvalidEncryptedData => "无效的加密数据",
            Self::PasswordRequired => "需要提供密码",
            Self::PasswordEmpty => "密码不能为空",
            Self::SaltRequired => "需要提供盐值",

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

            Self::PolicyOperationFailed => "策略操作失败",
        }
    }

    /// 转换为错误码字符串（供前端国际化使用）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "DEVICE_SYNC_DEVICE_NOT_FOUND",
            Self::DeviceAlreadyPaired => "DEVICE_SYNC_DEVICE_ALREADY_PAIRED",
            Self::DeviceNotPaired => "DEVICE_SYNC_DEVICE_NOT_PAIRED",
            Self::DeviceDisabled => "DEVICE_SYNC_DEVICE_DISABLED",
            Self::InvalidPairingCode => "DEVICE_SYNC_INVALID_PAIRING_CODE",
            Self::PairingCodeExpired => "DEVICE_SYNC_PAIRING_CODE_EXPIRED",
            Self::InsufficientTrustLevel => "DEVICE_SYNC_INSUFFICIENT_TRUST_LEVEL",

            Self::PermissionDenied => "DEVICE_SYNC_PERMISSION_DENIED",
            Self::PermissionNotConfigured => "DEVICE_SYNC_PERMISSION_NOT_CONFIGURED",
            Self::DeviceNotRegistered => "DEVICE_SYNC_DEVICE_NOT_REGISTERED",
            Self::PermissionsNotFound => "DEVICE_SYNC_PERMISSIONS_NOT_FOUND",

            Self::SyncFailed => "DEVICE_SYNC_FAILED",
            Self::SyncTimeout => "DEVICE_SYNC_TIMEOUT",
            Self::ConflictDetectionFailed => "DEVICE_SYNC_CONFLICT_DETECTION_FAILED",
            Self::ConflictResolutionFailed => "DEVICE_SYNC_CONFLICT_RESOLUTION_FAILED",
            Self::ConflictNotFound => "DEVICE_SYNC_CONFLICT_NOT_FOUND",
            Self::SyncAlreadyInProgress => "DEVICE_SYNC_ALREADY_IN_PROGRESS",
            Self::NoChangesToSync => "DEVICE_SYNC_NO_CHANGES_TO_SYNC",

            Self::EncryptionFailed => "DEVICE_SYNC_ENCRYPTION_FAILED",
            Self::DecryptionFailed => "DEVICE_SYNC_DECRYPTION_FAILED",
            Self::KeyDerivationFailed => "DEVICE_SYNC_KEY_DERIVATION_FAILED",
            Self::InvalidEncryptedData => "DEVICE_SYNC_INVALID_ENCRYPTED_DATA",
            Self::PasswordRequired => "DEVICE_SYNC_PASSWORD_REQUIRED",
            Self::PasswordEmpty => "DEVICE_SYNC_PASSWORD_EMPTY",
            Self::SaltRequired => "DEVICE_SYNC_SALT_REQUIRED",

            Self::ConnectionFailed => "DEVICE_SYNC_CONNECTION_FAILED",
            Self::TransportTimeout => "DEVICE_SYNC_TRANSPORT_TIMEOUT",
            Self::InvalidDataFormat => "DEVICE_SYNC_INVALID_DATA_FORMAT",

            Self::CrdtDocumentNotFound => "DEVICE_SYNC_CRDT_DOCUMENT_NOT_FOUND",
            Self::CrdtTransformFailed => "DEVICE_SYNC_CRDT_TRANSFORM_FAILED",
            Self::CrdtMergeFailed => "DEVICE_SYNC_CRDT_MERGE_FAILED",

            Self::SchedulerQueueFull => "DEVICE_SYNC_SCHEDULER_QUEUE_FULL",
            Self::SchedulerAlreadyRunning => "DEVICE_SYNC_SCHEDULER_ALREADY_RUNNING",
            Self::TaskNotFound => "DEVICE_SYNC_TASK_NOT_FOUND",

            Self::StorageOperationFailed => "DEVICE_SYNC_STORAGE_OPERATION_FAILED",
            Self::SerializationFailed => "DEVICE_SYNC_SERIALIZATION_FAILED",
            Self::DeserializationFailed => "DEVICE_SYNC_DESERIALIZATION_FAILED",

            Self::PolicyOperationFailed => "DEVICE_SYNC_POLICY_OPERATION_FAILED",
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
        Self { category: code.category(), message: message.into(), code, params: None }
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
        assert_eq!(SyncErrorCode::DeviceNotFound.category(), ErrorCategory::Device);
        assert_eq!(SyncErrorCode::PermissionDenied.category(), ErrorCategory::Permission);
        assert_eq!(SyncErrorCode::SyncFailed.category(), ErrorCategory::Sync);
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(SyncErrorCode::DeviceNotFound.as_str(), "DEVICE_SYNC_DEVICE_NOT_FOUND");
        assert_eq!(SyncErrorCode::PermissionDenied.as_str(), "DEVICE_SYNC_PERMISSION_DENIED");
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
        let json = serde_json::to_string(&err).expect("测试：JSON序列化应成功");
        assert!(json.contains("SYNC_FAILED"));
    }
}
