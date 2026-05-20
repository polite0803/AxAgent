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
    let key = if let Some(ci) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        ci.0.ip().to_string()
    } else {
        // NOTE: In production behind a reverse proxy, x-forwarded-for and x-real-ip
        // headers should be validated against trusted proxy IPs to prevent IP spoofing.
        request
            .headers()
            .get("x-forwarded-for")
            .or_else(|| request.headers().get("x-real-ip"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string()
    };

    if RATE_LIMITER.check_key(&key).is_err() {
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded. Please try again later.")
            .into_response();
    }

    next.run(request).await
}
