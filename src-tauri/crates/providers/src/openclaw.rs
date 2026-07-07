// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use futures::Stream;
use std::pin::Pin;

use crate::hermes::HermesAdapter;
use crate::{ProviderAdapter, ProviderRequestContext};

// 委托给 HermesAdapter,仅在 base_url 上使用 OPENCLAW 主机。
// 避免与 hermes.rs 中的 ApiMode/resolve/with_mode 系列代码完全重复。
pub struct OpenClawAdapter {
    inner: HermesAdapter,
}

impl OpenClawAdapter {
    pub fn new() -> Self {
        Self { inner: HermesAdapter::new() }
    }

    /// 构造使用 OpenClaw base_url 的 ProviderRequestContext,
    /// 然后委托给 HermesAdapter 执行实际请求。
    fn remap_ctx(&self, ctx: &ProviderRequestContext) -> ProviderRequestContext {
        let mut out = ctx.clone();
        if out.base_url.is_none() {
            out.base_url = Some(axagent_harness::constants::default_url::OPENCLAW_HOST.to_string());
        }
        out
    }
}

impl Default for OpenClawAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderAdapter for OpenClawAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let remapped = self.remap_ctx(ctx);
        self.inner.chat(&remapped, request).await
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        let remapped = self.remap_ctx(ctx);
        self.inner.chat_stream(&remapped, request, cancel_token)
    }

    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        let remapped = self.remap_ctx(ctx);
        self.inner.list_models(&remapped).await
    }

    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        let remapped = self.remap_ctx(ctx);
        self.inner.validate_key(&remapped).await
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        let remapped = self.remap_ctx(ctx);
        self.inner.embed(&remapped, request).await
    }

    // Hermes 专有的 jobs/runs API 在 OpenClaw 上没有对应实现,返回明确错误
    async fn list_jobs(&self, _ctx: &ProviderRequestContext) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn create_job(&self, _ctx: &ProviderRequestContext, _job_data: &str) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn get_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn update_job(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _job_data: &str,
    ) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn delete_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn pause_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn resume_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn trigger_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn list_runs(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn get_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn cancel_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<()> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn get_run_logs(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn trigger_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _params: Option<&str>,
    ) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn retry_run(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _run_id: &str,
    ) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn get_job_schedule(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
    ) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn update_job_schedule(
        &self,
        _ctx: &ProviderRequestContext,
        _job_id: &str,
        _schedule: &str,
    ) -> Result<String> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn enable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }

    async fn disable_job(&self, _ctx: &ProviderRequestContext, _job_id: &str) -> Result<()> {
        Err(AxAgentError::Provider("OpenClaw does not support jobs API".to_string()))
    }
}
