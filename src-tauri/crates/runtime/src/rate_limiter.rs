use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio::time::interval;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_second: f64,
    pub requests_per_minute: u64,
    pub requests_per_hour: u64,
    pub burst_size: usize,
    pub enable_adaptive: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10.0,
            requests_per_minute: 100,
            requests_per_hour: 1000,
            burst_size: 20,
            enable_adaptive: false,
        }
    }
}

#[derive(Debug)]
struct RateLimitBucket {
    tokens: f64,
    last_update: Instant,
    config: RateLimitConfig,
}

impl RateLimitBucket {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            tokens: config.burst_size as f64,
            last_update: Instant::now(),
            config,
        }
    }

    fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        let refill_rate = self.config.requests_per_second;
        self.tokens = (self.tokens + elapsed * refill_rate).min(self.config.burst_size as f64);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn get_wait_time(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            let wait_time = (1.0 - self.tokens) / self.config.requests_per_second;
            Duration::from_secs_f64(wait_time)
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub wait_time_ms: u64,
    pub remaining: usize,
    pub reset_in_ms: u64,
}

pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, RateLimitBucket>>>,
    minute_counters: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    hour_counters: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    default_config: RateLimitConfig,
    cleanup_interval: Duration,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            minute_counters: Arc::new(RwLock::new(HashMap::new())),
            hour_counters: Arc::new(RwLock::new(HashMap::new())),
            default_config: RateLimitConfig::default(),
            cleanup_interval: Duration::from_secs(60),
        }
    }

    pub fn with_config(mut self, config: RateLimitConfig) -> Self {
        self.default_config = config;
        self
    }

    pub async fn check_rate_limit(&self, key: &str) -> RateLimitResult {
        let config = self.get_config_for_key(key).await;

        let mut buckets = self.buckets.write().await;
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| RateLimitBucket::new(config.clone()));

        let allowed = bucket.try_acquire();
        let wait_time_ms = if allowed { 0 } else { bucket.get_wait_time().as_millis() as u64 };
        let remaining = bucket.tokens as usize;

        drop(buckets);

        if allowed {
            self.record_minute_request(key).await;
            self.record_hour_request(key).await;
        }

        let minute_count = self.count_recent_requests(key, Duration::from_secs(60), &self.minute_counters).await;
        let hour_count = self.count_recent_requests(key, Duration::from_secs(3600), &self.hour_counters).await;

        let reset_in_ms = if minute_count >= config.requests_per_minute {
            Duration::from_secs(60).as_millis() as u64
        } else if hour_count >= config.requests_per_hour {
            Duration::from_secs(3600).as_millis() as u64
        } else {
            0
        };

        RateLimitResult {
            allowed,
            wait_time_ms,
            remaining,
            reset_in_ms,
        }
    }

    async fn get_config_for_key(&self, _key: &str) -> RateLimitConfig {
        self.default_config.clone()
    }

    async fn record_minute_request(&self, key: &str) {
        let mut counters = self.minute_counters.write().await;
        let entries = counters.entry(key.to_string()).or_insert_with(Vec::new);
        entries.push(Instant::now());

        entries.retain(|t| t.elapsed() < Duration::from_secs(60));
    }

    async fn record_hour_request(&self, key: &str) {
        let mut counters = self.hour_counters.write().await;
        let entries = counters.entry(key.to_string()).or_insert_with(Vec::new);
        entries.push(Instant::now());

        entries.retain(|t| t.elapsed() < Duration::from_secs(3600));
    }

    async fn count_recent_requests(&self, key: &str, duration: Duration, counters: &Arc<RwLock<HashMap<String, Vec<Instant>>>>) -> u64 {
        let counters = counters.read().await;
        if let Some(entries) = counters.get(key) {
            let cutoff = Instant::now() - duration;
            entries.iter().filter(|t| **t > cutoff).count() as u64
        } else {
            0
        }
    }

    pub async fn start_cleanup_task(&self) {
        let minute_counters = self.minute_counters.clone();
        let hour_counters = self.hour_counters.clone();
        let cleanup_interval = self.cleanup_interval;

        tokio::spawn(async move {
            let mut ticker = interval(cleanup_interval);
            loop {
                ticker.tick().await;

                let mut minute = minute_counters.write().await;
                for entries in minute.values_mut() {
                    entries.retain(|t| t.elapsed() < Duration::from_secs(60));
                }

                let mut hour = hour_counters.write().await;
                for entries in hour.values_mut() {
                    entries.retain(|t| t.elapsed() < Duration::from_secs(3600));
                }
            }
        });
    }

    pub async fn get_stats(&self, key: &str) -> RateLimiterStats {
        let buckets = self.buckets.read().await;
        let bucket = buckets.get(key);

        let (tokens, remaining) = if let Some(b) = bucket {
            (b.tokens, b.tokens as usize)
        } else {
            (self.default_config.burst_size as f64, self.default_config.burst_size)
        };

        let minute_count = self.count_recent_requests(key, Duration::from_secs(60), &self.minute_counters).await;
        let hour_count = self.count_recent_requests(key, Duration::from_secs(3600), &self.hour_counters).await;

        RateLimiterStats {
            key: key.to_string(),
            tokens_available: tokens,
            requests_this_minute: minute_count,
            requests_this_hour: hour_count,
            limit_per_minute: self.default_config.requests_per_minute,
            limit_per_hour: self.default_config.requests_per_hour,
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub key: String,
    pub tokens_available: f64,
    pub requests_this_minute: u64,
    pub requests_this_hour: u64,
    pub limit_per_minute: u64,
    pub limit_per_hour: u64,
}

pub struct AdaptiveRateLimiter {
    base_limiter: RateLimiter,
    current_rate: f64,
    min_rate: f64,
    max_rate: f64,
    success_count: u64,
    error_count: u64,
}

impl AdaptiveRateLimiter {
    pub fn new() -> Self {
        Self {
            base_limiter: RateLimiter::new(),
            current_rate: 10.0,
            min_rate: 1.0,
            max_rate: 100.0,
            success_count: 0,
            error_count: 0,
        }
    }

    pub async fn check(&mut self, key: &str) -> RateLimitResult {
        let config = RateLimitConfig {
            requests_per_second: self.current_rate,
            ..Default::default()
        };

        let limiter = RateLimiter::with_config(RateLimiter::new(), config);
        limiter.check_rate_limit(key).await
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        if self.success_count >= 100 && self.error_count < 5 {
            self.current_rate = (self.current_rate * 1.1).min(self.max_rate);
            self.success_count = 0;
        }
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
        self.success_count = 0;
        if self.error_count >= 10 {
            self.current_rate = (self.current_rate * 0.9).max(self.min_rate);
            self.error_count = 0;
        }
    }
}

impl Default for AdaptiveRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new();
        let result = limiter.check_rate_limit("test_key").await;
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_burst() {
        let limiter = RateLimiter::new();
        for _ in 0..20 {
            let result = limiter.check_rate_limit("test_key").await;
            assert!(result.allowed);
        }
    }
}