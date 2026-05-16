use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::Mutex;

const RATE_LIMIT_CAPACITY: u64 = 60;
const RATE_LIMIT_REFILL_PER_SEC: u64 = 1;

struct TokenBucket {
    tokens: u64,
    max_tokens: u64,
    refill_rate: u64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs();
        if elapsed > 0 {
            let added = elapsed * self.refill_rate;
            self.tokens = (self.tokens + added).min(self.max_tokens);
            self.last_refill = Instant::now();
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn allow(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(RATE_LIMIT_CAPACITY, RATE_LIMIT_REFILL_PER_SEC));
        bucket.try_consume()
    }
}

static RATE_LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();

fn global_limiter() -> &'static RateLimiter {
    RATE_LIMITER.get_or_init(RateLimiter::new)
}

pub async fn rate_limit_middleware(request: Request<Body>, next: Next) -> Response {
    let key = request
        .headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let limiter = global_limiter();

    if !limiter.allow(&key).await {
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded. Please try again later.")
            .into_response();
    }

    next.run(request).await
}
