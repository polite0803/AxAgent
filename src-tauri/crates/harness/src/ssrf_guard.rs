// SPDX-License-Identifier: AGPL-3.0-only
//! SSRF 防护契约
use async_trait::async_trait;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlSafety {
    Safe,
    Loopback,
    PrivateNetwork,
    SchemeNotAllowed,
    BlockedIp,
    Blocked(String),
}
#[derive(Debug, Clone)]
pub struct SsrFConfig {
    pub allowed_schemes: Vec<String>,
    pub block_loopback: bool,
    pub block_private: bool,
    pub blocked_ip_prefixes: Vec<String>,
}
impl Default for SsrFConfig {
    fn default() -> Self {
        Self {
            allowed_schemes: vec!["http".into(), "https".into()],
            block_loopback: true,
            block_private: true,
            blocked_ip_prefixes: Vec::new(),
        }
    }
}

#[async_trait]
pub trait SsrFGuard: Send + Sync {
    async fn check_url(&self, url: &str) -> UrlSafety;
    fn config(&self) -> &SsrFConfig;
    async fn safe_client(&self) -> Result<reqwest::Client, String>;
}
