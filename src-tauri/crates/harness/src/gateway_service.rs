// SPDX-License-Identifier: AGPL-3.0-only
//! Gateway 服务契约
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayStatus {
    Stopped,
    Starting,
    Running {
        http_addr: String,
        ssl_port: Option<u16>,
    },
    Failed(String),
}
#[derive(Debug, Clone)]
pub struct GatewayInfo {
    pub status: GatewayStatus,
    pub started_at: i64,
    pub total_requests: u64,
    pub active_connections: u32,
}

#[async_trait]
pub trait GatewayService: Send + Sync {
    async fn start(&self) -> Result<(), String>;
    async fn stop(&self) -> Result<(), String>;
    async fn status(&self) -> Result<GatewayStatus, String>;
    async fn restart(&self) -> Result<(), String> {
        self.stop().await?;
        self.start().await
    }
}
#[derive(Default)]
pub struct NoopGatewayService;
#[async_trait]
impl GatewayService for NoopGatewayService {
    async fn start(&self) -> Result<(), String> {
        Err("not configured".into())
    }
    async fn stop(&self) -> Result<(), String> {
        Ok(())
    }
    async fn status(&self) -> Result<GatewayStatus, String> {
        Ok(GatewayStatus::Stopped)
    }
}
