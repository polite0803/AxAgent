// SPDX-License-Identifier: AGPL-3.0-only
//! 工具访问控制契约
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessDecision {
    Allow,
    Deny { reason: String },
    RequireConfirmation { prompt: String },
}
#[derive(Debug, Clone)]
pub struct ToolAccessRequest {
    pub tool_name: String,
    pub user_input: String,
    pub session_id: String,
    pub workspace_path: Option<String>,
}

#[async_trait]
pub trait ToolAccessControl: Send + Sync {
    async fn check_access(&self, req: &ToolAccessRequest) -> AccessDecision;
    async fn record_result(&self, req: &ToolAccessRequest, success: bool, error: Option<&str>);
}
#[derive(Default)]
pub struct NoopToolAccessControl;
#[async_trait]
impl ToolAccessControl for NoopToolAccessControl {
    async fn check_access(&self, _: &ToolAccessRequest) -> AccessDecision {
        AccessDecision::Allow
    }
    async fn record_result(&self, _: &ToolAccessRequest, _: bool, _: Option<&str>) {}
}
