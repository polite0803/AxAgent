//! Infrastructure domain state.
//!
//! Owns low-level, cross-cutting services: the runtime harness container,
//! the vector store, the indexing concurrency limiter, the file
//! authorizer, and the application data directory. None of these are
//! domain-specific.

use std::path::PathBuf;
use std::sync::Arc;

use axagent_runtime::harness::RuntimeHarness;
use axagent_search::vector_store::VectorStore;
use axagent_storage::file_authorizer::FileAuthorizer;

pub struct InfraState {
    /// Harness 容器（统一管理核心基础设施注入）
    pub harness: RuntimeHarness,
    pub vector_store: Arc<VectorStore>,
    pub indexing_semaphore: Arc<tokio::sync::Semaphore>,
    pub file_authorizer: Arc<FileAuthorizer>,
    pub app_data_dir: PathBuf,
}

impl InfraState {
    pub fn new(
        harness: RuntimeHarness,
        vector_store: Arc<VectorStore>,
        indexing_semaphore: Arc<tokio::sync::Semaphore>,
        file_authorizer: Arc<FileAuthorizer>,
        app_data_dir: PathBuf,
    ) -> Self {
        Self { harness, vector_store, indexing_semaphore, file_authorizer, app_data_dir }
    }
}
