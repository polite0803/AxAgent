//! axagent-storage — 文件存储层
//!
//! 路径管理、文件授权、存储清单、存储迁移、同步冲突检测、
//! 云存储（WebDAV/S3）、工作空间 URI。

pub mod cloud_storage;
pub mod cloud_workspace;
pub mod file_authorizer;
pub mod file_store;
pub mod path_vars;
pub mod storage_inventory;
pub mod storage_migration;
pub mod storage_paths;
pub mod sync_conflict;
pub mod webdav;
pub mod workspace_uri;
