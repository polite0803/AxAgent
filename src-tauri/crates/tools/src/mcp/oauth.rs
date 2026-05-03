//! MCP OAuth 认证

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

pub struct OAuthClient;

impl OAuthClient {
    pub fn new() -> Self {
        Self
    }
    pub fn authorization_url(&self, _config: &OAuthConfig) -> String {
        String::new()
    }
    pub fn exchange_code(&self, _code: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

impl Default for OAuthClient {
    fn default() -> Self {
        Self::new()
    }
}
