//! 审计日志契约 —— 记录节点执行的完整审计信息。
//!
//! 提供 `AuditEntry` 数据结构和 `AuditRecorder` trait，
//! 下游 crate 可注入具体实现（如写入文件、数据库或发送到外部审计服务）。

use serde::{Deserialize, Serialize};

/// 单条审计日志条目
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: u64,
    pub execution_type: String,
    pub session_id: Option<String>,
    pub tool_name: Option<String>,
    pub node_id: Option<String>,
    pub workflow_id: Option<String>,
    pub input_hash: String,
    pub output_hash: String,
    pub duration_ms: u64,
    pub status: String,
    pub error: Option<String>,
}

/// 审计记录器 trait —— 由外部注入具体实现。
pub trait AuditRecorder: Send + Sync {
    fn record(&self, entry: AuditEntry);
    fn get_history(&self) -> Vec<AuditEntry>;
}
