// SPDX-License-Identifier: AGPL-3.0-only

//! 设备管理与多端同步实现
//!
//! 本 crate 实现 DeviceManager 和 SyncEngine trait，
//! 提供设备配对、变更日志追踪、冲突解决等核心功能。

pub mod conflict_resolver;
pub mod crdt;
pub mod encryption;
pub mod error_codes;
pub mod history_store;
pub mod manager;
pub mod memory_storage;
pub mod permission_checker;
pub mod permission_store;
pub mod persistence;
pub mod policy_store;
pub mod s3_storage;
pub mod scheduler;
pub mod sync_engine;
pub mod transport;
pub mod utils;
pub mod version_vector;
pub mod webdav_storage;

pub use conflict_resolver::ConflictResolver;
pub use crdt::{CrdtEngine, CrdtOperation, OperationType};
pub use encryption::{
    EncryptedSyncData, EncryptionAlgorithm, KeyDerivation, KeyExchangeHelper, SyncEncryptionConfig,
    SyncEncryptor,
};
pub use error_codes::{
    ErrorCategory, SyncError, SyncErrorCode, device_not_found, encryption_failed,
    permission_denied, sync_failed,
};
pub use history_store::HistoryStore;
pub use manager::{DeviceManagerImpl, DeviceStore};
pub use memory_storage::{MemorySyncStorage, create_memory_storage};
pub use permission_checker::{PermissionCheckResult, PermissionChecker};
pub use permission_store::PermissionStore;
pub use persistence::{PersistenceConfig, PersistentStore};
pub use policy_store::PolicyStore;
pub use s3_storage::S3Storage;
pub use scheduler::{SchedulerConfig, SchedulerStatus, SyncPriority, SyncScheduler, SyncTask};
pub use sync_engine::SyncEngineImpl;
pub use transport::{SyncTransport, TransportConfig};
pub use version_vector::VersionVector;
pub use webdav_storage::WebdavStorage;
