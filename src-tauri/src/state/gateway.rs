//! Gateway domain state.
//!
//! Owns the embedded gateway server handle (HTTP/HTTPS listeners) and any
//! auxiliary stores that are gateway-scoped. Phase 2 already wired the
//! per-IP `key_verify_limiter` and the single-use `ticket_store` into the
//! `GatewayAppState` exposed to Axum handlers; this struct simply groups
//! the *Tauri* side of the gateway state for the domain split.

use std::sync::Arc;
use tokio::sync::Mutex;

use axagent_gateway::server::GatewayServer;

#[allow(dead_code)]
pub struct GatewayState {
    pub gateway_server: Arc<Mutex<Option<GatewayServer>>>,
}

#[allow(dead_code)]
impl GatewayState {
    pub fn new(gateway_server: Arc<Mutex<Option<GatewayServer>>>) -> Self {
        Self { gateway_server }
    }
}
