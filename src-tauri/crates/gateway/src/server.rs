// SPDX-License-Identifier: AGPL-3.0-only

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::{
    Router,
    body::Body,
    extract::State as AxumState,
    http::{Request, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use sea_orm::DatabaseConnection;
use tokio::task::JoinHandle;

use axagent_harness::core_error::{AxAgentError, Result};

use crate::auth::{ClientIpPolicy, KeyVerifyLimiter};
use crate::qr_bind::QrBindStore;
use crate::realtime_ticket::TicketStore;

/// Shared state for Axum handlers (separate from Tauri AppState).
#[derive(Clone)]
pub struct GatewayAppState {
    pub db: DatabaseConnection,
    pub master_key: [u8; 32],
    pub started_at: i64,
    /// 由 Harness 注入的 Provider 注册表（start_with_registry 使用）
    pub provider_registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    /// 平台层 trait 聚合（provider / settings / gateway_key / request_log / crypto）。
    /// 由 wiring 层构造，把 gateway 与 dao + crypto 解耦。
    pub adapter: Arc<dyn axagent_harness::PlatformAdapter>,
    /// MCP server 元数据查询（消除 gateway→axagent-entities/SeaORM 违规）。
    pub mcp_store: Arc<dyn axagent_harness::mcp_service::McpServerStore>,
    /// MCP 工具发现与调用（消除 gateway→axagent-mcp 违规）。
    pub mcp_client: Arc<dyn axagent_harness::mcp_service::McpClientService>,
    /// Marketplace review service（消除 gateway→kit→dao 违规链）。
    pub marketplace_service: Arc<dyn axagent_harness::marketplace::MarketplaceService>,
    /// In-memory store of single-use tickets for `/v1/realtime` WS auth
    /// (SECURITY P0-2.2). One per gateway instance.
    pub ticket_store: Arc<TicketStore>,
    /// QR 绑定令牌存储（IM 渠道扫码绑定用，参考 nomifun QrTokenStore）。
    pub qr_bind_store: QrBindStore,
    /// SECURITY (Phase 2 Task 2.3): per-IP 限流器，防御 prefix 爆破。
    /// 5 失败 → 60s 冷却（参见 spec 2.3）。
    pub key_verify_limiter: Arc<KeyVerifyLimiter>,
    /// P1-7: 客户端 IP 提取策略（trusted_proxies）。
    /// 默认 trust_all 保留向后兼容；生产环境应通过环境变量 `TRUSTED_PROXIES=...` 显式收紧。
    pub client_ip_policy: Arc<ClientIpPolicy>,
}

/// TLS certificate material.
#[derive(Debug, Clone)]
pub struct GatewayTlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

/// SSL listener configuration: port number plus TLS certificate material.
#[derive(Debug, Clone)]
pub struct GatewaySslConfig {
    pub ssl_port: u16,
    pub tls: GatewayTlsConfig,
}

/// Full configuration passed to [`GatewayServer::start`].
#[derive(Debug, Clone)]
pub struct GatewayStartConfig {
    pub listen_address: String,
    pub http_port: u16,
    /// `None` means HTTP-only mode.
    pub ssl: Option<GatewaySslConfig>,
    /// When `true` and `ssl` is `Some`, the HTTP listener returns 302 redirects
    /// to the HTTPS URL instead of serving the gateway directly.
    pub force_ssl: bool,
}

// ─── SSL redirect handler ─────────────────────────────────────────────────

/// 从环境变量 `TRUSTED_PROXIES` 解析可信代理 IP 列表。
///
/// 格式：逗号分隔的 IP 字面量，例如 `TRUSTED_PROXIES=10.0.0.1,192.168.1.5`。
/// 支持 IPv4 和 IPv6 单地址；CIDR 形式暂不支持（避免引入额外依赖 + CIDR 展开边界 bug）。
///
/// 未设置或解析为空时回退到 `ClientIpPolicy::trust_all()`，并打印一次 warn。
pub(crate) fn client_ip_policy_from_env_or_default() -> ClientIpPolicy {
    let raw = std::env::var("TRUSTED_PROXIES")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let Some(raw) = raw else {
        tracing::warn!(
            "TRUSTED_PROXIES 未设置，client_ip_policy 使用默认值（不信任任何代理）；生产部署建议显式配置 TRUSTED_PROXIES"
        );
        return ClientIpPolicy::default();
    };

    let mut proxies: Vec<std::net::IpAddr> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    for token in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match token.parse::<std::net::IpAddr>() {
            Ok(ip) => proxies.push(ip),
            Err(_) => bad.push(token.to_string()),
        }
    }

    if !bad.is_empty() {
        tracing::warn!("TRUSTED_PROXIES 中以下项解析失败: {:?}（已忽略）", bad);
    }

    if proxies.is_empty() {
        // SECURITY: 默认不信任任何代理，防止 X-Forwarded-For 伪造
        // 生产部署必须配置 TRUSTED_PROXIES 环境变量
        tracing::warn!(
            "TRUSTED_PROXIES 未配置 — 网关不信任任何转发头，\
             远程部署时请配置 TRUSTED_PROXIES 环境变量"
        );
        return ClientIpPolicy::trust_none();
    }

    tracing::info!("TRUSTED_PROXIES 已配置 {} 个可信代理", proxies.len());
    ClientIpPolicy::default().with_trusted(proxies)
}

#[derive(Clone)]
struct RedirectState {
    https_port: u16,
}

async fn ssl_redirect_handler(
    AxumState(state): AxumState<RedirectState>,
    req: Request<Body>,
) -> Response {
    let host_header =
        req.headers().get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("localhost");

    // Strip any existing port from the Host header, handling bracketed IPv6.
    let bare_host = if host_header.starts_with('[') {
        // Bracketed IPv6: "[::1]:port" → "[::1]", or "[::1]" → "[::1]".
        match host_header.find("]:") {
            Some(pos) => &host_header[..pos + 1],
            None => host_header,
        }
    } else {
        match host_header.rfind(':') {
            Some(pos) => &host_header[..pos],
            None => host_header,
        }
    };

    let path_and_query = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

    let location = format!("https://{}:{}{}", bare_host, state.https_port, path_and_query);
    // 307 preserves the request method (POST stays POST), unlike 302.
    (StatusCode::TEMPORARY_REDIRECT, [(header::LOCATION, location)]).into_response()
}

fn create_redirect_router(https_port: u16) -> Router {
    Router::new().fallback(ssl_redirect_handler).with_state(RedirectState { https_port })
}

// ─── GatewayServer ────────────────────────────────────────────────────────

pub struct GatewayServer {
    http_handle: Handle<SocketAddr>,
    http_task: Option<JoinHandle<()>>,
    http_addr: SocketAddr,
    https_handle: Option<Handle<SocketAddr>>,
    https_task: Option<JoinHandle<()>>,
    https_addr: Option<SocketAddr>,
    force_ssl: bool,
    running: Arc<AtomicBool>,
    started_at: i64,
}

impl GatewayServer {
    /// Start the gateway with a pre-built ProviderRegistry (from RuntimeHarness)
    #[allow(clippy::too_many_arguments)]
    pub async fn start_with_registry(
        pool: DatabaseConnection,
        master_key: [u8; 32],
        config: GatewayStartConfig,
        provider_registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
        adapter: Arc<dyn axagent_harness::PlatformAdapter>,
        marketplace_service: Arc<dyn axagent_harness::marketplace::MarketplaceService>,
        mcp_store: Arc<dyn axagent_harness::mcp_service::McpServerStore>,
        mcp_client: Arc<dyn axagent_harness::mcp_service::McpClientService>,
    ) -> Result<Self> {
        let started_at = axagent_harness::util_fns::now_ts();
        let app_state = GatewayAppState {
            db: pool,
            master_key,
            started_at,
            provider_registry,
            adapter,
            marketplace_service,
            mcp_store,
            mcp_client,
            ticket_store: crate::realtime::default_ticket_store(),
            qr_bind_store: crate::qr_bind::QrBindStore::new(),
            // SECURITY (Phase 2 Task 2.3): 5 失败 → 60s 冷却。
            key_verify_limiter: Arc::new(KeyVerifyLimiter::new(5, Duration::from_secs(60))),
            // P1-7: 默认从环境变量 `TRUSTED_PROXIES=ip1,ip2,...` 读取可信代理；
            // 未配置时回退到 `trust_all()` 保留向后兼容，但打 warn 提醒生产环境收紧。
            client_ip_policy: Arc::new(client_ip_policy_from_env_or_default()),
        };
        Self::start_inner(app_state, config).await
    }

    /// Shared HTTP/HTTPS startup logic
    async fn start_inner(app_state: GatewayAppState, config: GatewayStartConfig) -> Result<Self> {
        // ── Bind HTTP listener ──────────────────────────────────────────
        let http_bind: SocketAddr = format!("{}:{}", config.listen_address, config.http_port)
            .parse()
            .map_err(|e| AxAgentError::Gateway(format!("Invalid HTTP bind address: {}", e)))?;
        let http_listener = std::net::TcpListener::bind(http_bind)
            .map_err(|e| AxAgentError::Gateway(format!("Failed to bind HTTP port: {}", e)))?;
        http_listener
            .set_nonblocking(true)
            .map_err(|e| AxAgentError::Gateway(format!("Failed to set nonblocking: {}", e)))?;
        let http_actual_addr = http_listener
            .local_addr()
            .map_err(|e| AxAgentError::Gateway(format!("Failed to get HTTP local addr: {}", e)))?;

        // ── Optionally bind HTTPS listener and load TLS config ──────────
        struct HttpsBinding {
            listener: std::net::TcpListener,
            rustls: RustlsConfig,
            addr: SocketAddr,
        }
        let https_binding: Option<HttpsBinding> = match &config.ssl {
            Some(ssl_cfg) => {
                let https_bind: SocketAddr =
                    format!("{}:{}", config.listen_address, ssl_cfg.ssl_port).parse().map_err(
                        |e| AxAgentError::Gateway(format!("Invalid HTTPS bind address: {}", e)),
                    )?;
                let listener = std::net::TcpListener::bind(https_bind).map_err(|e| {
                    AxAgentError::Gateway(format!("Failed to bind HTTPS port: {}", e))
                })?;
                listener.set_nonblocking(true).map_err(|e| {
                    AxAgentError::Gateway(format!("Failed to set HTTPS nonblocking: {}", e))
                })?;
                let addr = listener.local_addr().map_err(|e| {
                    AxAgentError::Gateway(format!("Failed to get HTTPS local addr: {}", e))
                })?;
                let rustls =
                    RustlsConfig::from_pem_file(&ssl_cfg.tls.cert_path, &ssl_cfg.tls.key_path)
                        .await
                        .map_err(|e| {
                            AxAgentError::Gateway(format!("Failed to load TLS certificate: {}", e))
                        })?;
                Some(HttpsBinding { listener, rustls, addr })
            },
            None => None,
        };

        let https_actual_addr = https_binding.as_ref().map(|b| b.addr);

        // ── Build router(s) ─────────────────────────────────────────────
        // HTTP router: redirect (force_ssl) or full gateway.
        // HTTPS router: always the full gateway when SSL is configured.
        let http_router: Router =
            if let Some(addr) = config.force_ssl.then_some(https_actual_addr).flatten() {
                create_redirect_router(addr.port())
            } else {
                crate::routes::create_router(app_state.clone())
            };
        let https_router: Option<Router> = if https_binding.is_some() {
            Some(crate::routes::create_router(app_state))
        } else {
            None
        };

        // ── Spawn HTTP task ─────────────────────────────────────────────
        // Pre-create the HTTPS Handle (when HTTPS will be active) before
        // spawning the HTTP task so that each task holds a clone of its
        // sibling's handle for mutual-shutdown: if one listener exits
        // unexpectedly, it triggers a graceful shutdown of the other so
        // the gateway never ends up half-dead.
        let running = Arc::new(AtomicBool::new(true));
        let http_handle = Handle::new();
        let https_handle: Option<Handle<SocketAddr>> =
            if https_binding.is_some() && https_router.is_some() {
                Some(Handle::new())
            } else {
                None
            };
        let http_task = {
            let server_handle = http_handle.clone();
            let running_flag = running.clone();
            let router = http_router;
            let addr = http_actual_addr;
            let peer_handle = https_handle.clone();
            tokio::spawn(async move {
                tracing::info!("Gateway HTTP listener on http://{}", addr);
                // SECURITY (Phase 2 Task 2.3): into_make_service_with_connect_info
                // 把 peer SocketAddr 注入 extension，auth_middleware 用
                // 它做 per-IP 限流。无它则所有请求都被归到 "unknown"，
                // 限流器退化为全局 —— 不可接受。
                let result = axum_server::from_tcp(http_listener)
                    .expect("axum-server: from_tcp 失败")
                    .handle(server_handle)
                    .serve(router.into_make_service_with_connect_info::<SocketAddr>())
                    .await;
                if let Err(e) = result {
                    tracing::error!("Gateway HTTP server error: {}", e);
                }
                // Shut down the sibling HTTPS listener if still running.
                if let Some(h) = peer_handle {
                    h.graceful_shutdown(Some(Duration::from_secs(5)));
                }
                running_flag.store(false, Ordering::SeqCst);
                tracing::info!("Gateway HTTP server stopped");
            })
        };

        // ── Spawn HTTPS task (when SSL is configured) ───────────────────
        let https_task = match (https_binding, https_router) {
            (Some(binding), Some(router)) => {
                // SAFETY: https_handle is always Some here because it was set
                // immediately above when the HTTPS binding was created.
                // If this panics, it indicates a logic error in startup ordering.
                let server_handle = https_handle
                    .as_ref()
                    .expect("https_handle must be Some when https_binding is Some")
                    .clone();
                let addr = binding.addr;
                let running_flag = running.clone();
                let peer_handle = http_handle.clone();
                let task = tokio::spawn(async move {
                    tracing::info!("Gateway HTTPS listener on https://{}", addr);
                    let result = axum_server::from_tcp_rustls(binding.listener, binding.rustls)
                        .expect("axum-server: from_tcp_rustls 失败")
                        .handle(server_handle)
                        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
                        .await;
                    if let Err(e) = result {
                        tracing::error!("Gateway HTTPS server error: {}", e);
                    }
                    // Shut down the sibling HTTP listener if still running.
                    peer_handle.graceful_shutdown(Some(Duration::from_secs(5)));
                    running_flag.store(false, Ordering::SeqCst);
                    tracing::info!("Gateway HTTPS server stopped");
                });
                Some(task)
            },
            _ => None,
        };

        Ok(Self {
            http_handle,
            http_task: Some(http_task),
            http_addr: http_actual_addr,
            https_handle,
            https_task,
            https_addr: https_actual_addr,
            force_ssl: config.force_ssl,
            running,
            started_at: axagent_harness::util_fns::now_ts(),
        })
    }

    pub async fn stop(&mut self) -> Result<()> {
        // Signal graceful shutdown on both listeners.
        self.http_handle.graceful_shutdown(Some(Duration::from_secs(5)));
        if let Some(ref h) = self.https_handle {
            h.graceful_shutdown(Some(Duration::from_secs(5)));
        }
        // Await both tasks.
        if let Some(task) = self.http_task.take() {
            let _ = task.await;
        }
        if let Some(task) = self.https_task.take() {
            let _ = task.await;
        }
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Bound address of the HTTP listener.
    pub fn http_addr(&self) -> SocketAddr {
        self.http_addr
    }

    /// Bound address of the HTTPS listener, or `None` if SSL is not active.
    pub fn https_addr(&self) -> Option<SocketAddr> {
        self.https_addr
    }

    pub fn force_ssl(&self) -> bool {
        self.force_ssl
    }

    pub fn started_at(&self) -> i64 {
        self.started_at
    }
}
