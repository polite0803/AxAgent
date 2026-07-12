// SPDX-License-Identifier: AGPL-3.0-only

//! 工具体系需要的运行时服务契约 —— 让 tools crate 不直接依赖 runtime-core。
//!
//! 包含：McpTransport 枚举、CronJobStore trait、HookEventFirer trait。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// MCP 传输层协议枚举 —— 从 `axagent-runtime-core::config` 上移。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio,
    Sse,
    Http,
    Ws,
    Sdk,
    ManagedProxy,
}

/// CronJob 数据 DTO —— tools 通过此 DTO 与调度器交互。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobData {
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub description: String,
    pub is_active: bool,
    pub run_count: u32,
}

/// CronJob 存储 —— 由 runtime-core 实现，tools 通过 OnceLock 注入使用。
#[async_trait]
pub trait CronJobStore: Send + Sync {
    async fn add(&self, job: CronJobData) -> String;
    async fn remove(&self, id: &str) -> bool;
    async fn get(&self, id: &str) -> Option<CronJobData>;
    async fn list(&self) -> Vec<CronJobData>;
    async fn count(&self) -> usize;
}

/// 空实现（默认降级）。
pub struct NoopCronJobStore;

#[async_trait]
impl CronJobStore for NoopCronJobStore {
    async fn add(&self, job: CronJobData) -> String {
        job.name
    }
    async fn remove(&self, _id: &str) -> bool {
        false
    }
    async fn get(&self, _id: &str) -> Option<CronJobData> {
        None
    }
    async fn list(&self) -> Vec<CronJobData> {
        Vec::new()
    }
    async fn count(&self) -> usize {
        0
    }
}

/// Hook 事件触发 —— 替换 tools 中的 `HookRunner::new(RuntimeHookConfig::default())` 模式。
///
/// 默认实现为空操作，因为 tools 中所有 HookRunner 调用都使用空配置。
pub trait HookEventFirer: Send + Sync {
    fn fire_hook(&self, event: &str, data: &str);
}

/// 空实现（默认降级）。
pub struct NoopHookEventFirer;

impl HookEventFirer for NoopHookEventFirer {
    fn fire_hook(&self, _event: &str, _data: &str) {}
}
