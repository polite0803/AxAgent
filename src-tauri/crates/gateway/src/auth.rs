use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sea_orm::DatabaseConnection;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axagent_harness::platform_adapter::PlatformAdapter;
use axagent_harness::types::GatewayKey;
use parking_lot::Mutex;

/// Authenticated key injected into request extensions after auth middleware.
#[derive(Clone, Debug)]
pub struct AuthenticatedKey(pub GatewayKey);

/// SECURITY (Phase 2 Task 2.3): per-IP 失败计数 + 冷却。
///
/// 攻击模型：API key 的 `key_prefix` 只 8 字符，攻击者持有**一个**有效
/// key 后可以在线枚举 prefix 空间（~48 bits），目标是获取 key 身份
/// 维度的情报。本 limiter 在 verify_key() 失败时按 IP 计数：达到阈值
/// 后该 IP 在冷却期内直接返回 429，强制攻击者减速到 `cooldown` 一次。
///
/// 设计选择：
/// - `parking_lot::Mutex` 而非 `tokio::sync::Mutex` —— 锁住的操作是
///   O(1) HashMap lookup，sync 锁比 async 锁在该场景下延迟更低。
/// - 失败计数用 (count, first_attempt_at) 而不是滑动窗口 —— 简单的
///   计数足以防御爆破；sliding window 实现复杂度没价值。
/// - 成功验证后**清空**该 IP 的 entry —— 正常用户不该被冷启动流量
///   影响。
/// - 内存以 (count, first_ts) tuple + IP 字符串计，上限约 100KB
///   （10k IPs × 32 bytes），对正常 API gateway 体量可控。
pub struct KeyVerifyLimiter {
    failures: Mutex<HashMap<String, (u32, Instant)>>,
    max_failures: u32,
    cooldown: Duration,
}

impl KeyVerifyLimiter {
    pub fn new(max_failures: u32, cooldown: Duration) -> Self {
        Self {
            failures: Mutex::new(HashMap::new()),
            max_failures,
            cooldown,
        }
    }

    /// 检查 IP 是否仍在冷却期内（被 ban）。返回 `true` 表示允许请求。
    pub fn check(&self, ip: &str) -> bool {
        let map = self.failures.lock();
        match map.get(ip) {
            None => true,
            Some((count, first_at)) => {
                if *count < self.max_failures {
                    true
                } else {
                    // 已超阈值，检查是否过冷却期
                    first_at.elapsed() >= self.cooldown
                }
            }
        }
    }

    /// 记录一次失败。冷却期内被 ban 时，刷新 first_ts（防重置攻击）。
    pub fn record_failure(&self, ip: &str) {
        let mut map = self.failures.lock();
        let now = Instant::now();
        let entry = map.entry(ip.to_string()).or_insert((0, now));
        if entry.0 >= self.max_failures && entry.1.elapsed() < self.cooldown {
            // 仍处于 ban 中：把 first_at 重置回 now，让 cooldown
            // 再持续一次完整窗口。这样攻击者连续打不会被绕过。
            entry.1 = now;
        }
        entry.0 = entry.0.saturating_add(1);
    }

    /// 记录一次成功 —— 清空该 IP 的失败计数。
    pub fn record_success(&self, ip: &str) {
        let mut map = self.failures.lock();
        map.remove(ip);
    }
}

/// 鉴权中间件需要的运行时状态（adapter）。由 routes.rs 用 `from_fn_with_state` 注入。
#[derive(Clone)]
pub struct AuthState {
    /// 数据库连接，update_last_used 后台任务用
    pub db: DatabaseConnection,
    pub adapter: Arc<dyn PlatformAdapter>,
    /// SECURITY (Phase 2 Task 2.3): per-IP 限流器。共享给所有 request。
    pub key_verify_limiter: Arc<KeyVerifyLimiter>,
}

/// 提取 client IP：X-Forwarded-For (首个) → peer addr → "unknown"。
///
/// 部署场景：在 nginx / cloud LB 后面时，X-Forwarded-For 包含真实
/// 客户端 IP（首段）；裸 socket 时 fallback 到 ConnectInfo。三个
/// fallback 保证任何 axum 部署都能拿到一个有意义的 key。
pub fn extract_client_ip<B>(request: &Request<B>, fallback: Option<SocketAddr>) -> String {
    if let Some(xff) = request.headers().get("x-forwarded-for") {
        if let Ok(s) = xff.to_str() {
            if let Some(first) = s.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    fallback
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Auth middleware: extracts Bearer token, verifies against gateway_keys, updates last_used_at.
pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // 从 extension 拿 ConnectInfo 而非作为 extractor —— 后者要求 caller
    // 配 `into_make_service_with_connect_info`，会让所有 unit test 都
    // 需要加 ConnectInfo 才能跑通。extension lookup 兼容 prod（已配）
    // 和 test（未配，后备到 XFF/"unknown"）两种场景。
    let peer_addr = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0);
    let client_ip = extract_client_ip(&request, peer_addr);

    // SECURITY (Phase 2 Task 2.3): 限流检查。check() 走 sync 锁极快，
    // 不应在 request path 上构成瓶颈。
    if !state.key_verify_limiter.check(&client_ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": {
                    "message": "Too many failed authentication attempts. Please retry later.",
                    "type": "rate_limit_error",
                    "code": "key_verify_rate_limited"
                }
            })),
        )
            .into_response();
    }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            state.key_verify_limiter.record_failure(&client_ip);
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": {
                        "message": "Missing or invalid Authorization header. Expected: Bearer <api-key>",
                        "type": "invalid_request_error",
                        "code": "invalid_api_key"
                    }
                })),
            )
                .into_response();
        },
    };

    match state.adapter.gateway_keys().verify_key(token).await {
        Ok(Some(key)) => {
            // 成功：清空该 IP 失败计数。
            state.key_verify_limiter.record_success(&client_ip);

            // Update last_used_at in background (non-blocking)
            let adapter_bg = state.adapter.clone();
            let key_id = key.id.clone();
            tokio::spawn(async move {
                if let Err(e) = adapter_bg.gateway_keys().update_last_used(&key_id).await {
                    tracing::warn!(%e, "Failed to update gateway key last_used");
                }
            });

            request.extensions_mut().insert(AuthenticatedKey(key));
            next.run(request).await
        },
        _ => {
            // 失败：计数 +1。
            state.key_verify_limiter.record_failure(&client_ip);
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": {
                        "message": "Invalid or disabled API key",
                        "type": "invalid_request_error",
                        "code": "invalid_api_key"
                    }
                })),
            )
                .into_response()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_allows_until_threshold() {
        let limiter = KeyVerifyLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.check("1.2.3.4"));
        limiter.record_failure("1.2.3.4");
        assert!(limiter.check("1.2.3.4"));
        limiter.record_failure("1.2.3.4");
        assert!(limiter.check("1.2.3.4"));
        limiter.record_failure("1.2.3.4");
        // 已达阈值
        assert!(!limiter.check("1.2.3.4"));
    }

    #[test]
    fn limiter_ips_are_isolated() {
        let limiter = KeyVerifyLimiter::new(2, Duration::from_secs(60));
        for _ in 0..2 {
            limiter.record_failure("1.2.3.4");
        }
        assert!(!limiter.check("1.2.3.4"));
        // 其他 IP 不受影响
        assert!(limiter.check("5.6.7.8"));
    }

    #[test]
    fn limiter_recovers_after_cooldown() {
        let limiter = KeyVerifyLimiter::new(2, Duration::from_millis(50));
        limiter.record_failure("1.2.3.4");
        limiter.record_failure("1.2.3.4");
        assert!(!limiter.check("1.2.3.4"));
        std::thread::sleep(Duration::from_millis(80));
        assert!(
            limiter.check("1.2.3.4"),
            "should allow after cooldown elapses"
        );
    }

    #[test]
    fn limiter_success_clears_failures() {
        let limiter = KeyVerifyLimiter::new(2, Duration::from_secs(60));
        limiter.record_failure("1.2.3.4");
        limiter.record_failure("1.2.3.4");
        assert!(!limiter.check("1.2.3.4"));
        // 成功一次清除
        limiter.record_success("1.2.3.4");
        assert!(limiter.check("1.2.3.4"));
    }

    #[test]
    fn limiter_during_ban_resets_window() {
        // 攻击者连续打：每次 record_failure 都会把 first_at 重置回 now，
        // 冷却期永远不过去。
        let limiter = KeyVerifyLimiter::new(2, Duration::from_millis(50));
        limiter.record_failure("1.2.3.4");
        limiter.record_failure("1.2.3.4");
        assert!(!limiter.check("1.2.3.4"));
        // 攻击者继续打
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(20));
            limiter.record_failure("1.2.3.4");
        }
        // 仍被 ban —— 因为每次 record_failure 都会重置 first_at
        assert!(!limiter.check("1.2.3.4"));
    }

    #[test]
    fn extract_client_ip_prefers_xff() {
        let mut req = Request::new(Body::empty());
        req.headers_mut().insert(
            "x-forwarded-for",
            "203.0.113.1, 10.0.0.1".parse().unwrap(),
        );
        let ip = extract_client_ip(&req, None);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn extract_client_ip_falls_back_to_x_real_ip() {
        let mut req = Request::new(Body::empty());
        req.headers_mut()
            .insert("x-real-ip", "203.0.113.2".parse().unwrap());
        let ip = extract_client_ip(&req, None);
        assert_eq!(ip, "203.0.113.2");
    }

    #[test]
    fn extract_client_ip_falls_back_to_peer() {
        let req = Request::new(Body::empty());
        let addr: SocketAddr = "10.0.0.5:54321".parse().unwrap();
        let ip = extract_client_ip(&req, Some(addr));
        assert_eq!(ip, "10.0.0.5");
    }

    #[test]
    fn extract_client_ip_unknown_when_no_signal() {
        let req = Request::new(Body::empty());
        let ip = extract_client_ip(&req, None);
        assert_eq!(ip, "unknown");
    }
}
