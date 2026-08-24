// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::disallowed_types)]

use std::net::SocketAddr;
use std::num::NonZero;
// SAFETY: 此处 parking_lot::RwLock 不跨 await 使用，rate_limit_middleware 中 guard 已在块内释放。
// SAFETY: 后台重建线程使用同步 RwLock，临界区内无 await。
#[allow(clippy::disallowed_types)]
use parking_lot::RwLock;

use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    Quota, RateLimiter as GovernorRateLimiter, clock::DefaultClock, middleware::NoOpMiddleware,
    state::keyed::DefaultKeyedStateStore,
};

type KeyedLimiter = GovernorRateLimiter<
    String,
    DefaultKeyedStateStore<String>,
    DefaultClock,
    NoOpMiddleware<<DefaultClock as governor::clock::Clock>::Instant>,
>;

fn create_limiter() -> KeyedLimiter {
    let quota = Quota::per_second(NonZero::new(1u32).expect("1 > 0"))
        .allow_burst(NonZero::new(60u32).expect("60 > 0"));
    GovernorRateLimiter::keyed(quota)
}

/// 每秒 1 请求，允许短时突发至 60（GCRA 算法，比 Token Bucket 更精确）。
///
/// SECURITY: `DefaultKeyedStateStore` 内部使用 `HashMap` 且永不过期清理条目。
/// 攻击者可通过来自不同 IP 的持续请求导致内存无限增长（OOM DoS）。
/// 修复：包装在 `RwLock` 中，后台线程每 60 分钟重建一次 limiter 以清除累积状态。
/// 运行时开销：O(1) —— 仅丢弃旧 limiter，新 limiter 从空状态开始。
#[allow(clippy::type_complexity)]
#[allow(clippy::disallowed_types)]
static RATE_LIMITER: std::sync::LazyLock<RwLock<KeyedLimiter>> = std::sync::LazyLock::new(|| {
    let limiter = RwLock::new(create_limiter());
    // Background cleanup: recreate limiter every hour to prevent unbounded
    // memory growth from the DefaultKeyedStateStore (which never expires entries).
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
            let mut guard = RATE_LIMITER.write();
            *guard = create_limiter();
            tracing::info!(
                target: "axagent.gateway.rate_limit",
                "Recreated rate limiter to clear stale entries"
            );
        }
    });
    limiter
});

pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    // P1-7: 安全起见，默认用 socket peer IP 作为限流 key。
    // XFF 解析仅在 reverse proxy 部署时启用（需要 explicit configuration）；
    // 当前中间件不知道 trusted_proxies 配置，所以无条件忽略 XFF。
    let key = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // RwLockReadGuard 是 !Send，不能跨 await 持有，放入块中提前释放。
    let is_limited = {
        let limiter = RATE_LIMITER.read();
        limiter.check_key(&key).is_err()
    };
    if is_limited {
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded. Please try again later.")
            .into_response();
    }

    next.run(request).await
}
