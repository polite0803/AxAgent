// SPDX-License-Identifier: AGPL-3.0-only

use std::net::SocketAddr;
use std::num::NonZero;

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    Quota, RateLimiter as GovernorRateLimiter, clock::DefaultClock, middleware::NoOpMiddleware,
    state::keyed::DefaultKeyedStateStore,
};

/// 每秒 1 请求，允许短时突发至 60（GCRA 算法，比 Token Bucket 更精确）
#[allow(clippy::type_complexity)]
static RATE_LIMITER: std::sync::LazyLock<
    GovernorRateLimiter<
        String,
        DefaultKeyedStateStore<String>,
        DefaultClock,
        NoOpMiddleware<<DefaultClock as governor::clock::Clock>::Instant>,
    >,
> = std::sync::LazyLock::new(|| {
    let quota = Quota::per_second(NonZero::new(1u32).expect("1 > 0"))
        .allow_burst(NonZero::new(60u32).expect("60 > 0"));
    GovernorRateLimiter::keyed(quota)
});

pub async fn rate_limit_middleware(request: Request<Body>, next: Next) -> Response {
    // P1-7: 安全起见，默认用 socket peer IP 作为限流 key。
    // XFF 解析仅在 reverse proxy 部署时启用（需要 explicit configuration）；
    // 当前中间件不知道 trusted_proxies 配置，所以无条件忽略 XFF。
    // 注释里说的 "should be validated" 之前没有实现 — 这里补上。
    let key = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if RATE_LIMITER.check_key(&key).is_err() {
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded. Please try again later.")
            .into_response();
    }

    next.run(request).await
}
