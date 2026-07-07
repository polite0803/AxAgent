// SPDX-License-Identifier: AGPL-3.0-only
//! 限流器契约
use async_trait::async_trait;

#[derive(Debug,Clone,PartialEq,Eq)] pub enum RateLimitResult { Allowed, Denied{retry_after_ms:u64} }
#[derive(Debug,Clone)] pub struct RateLimitConfig { pub max_requests: u64, pub window_secs: u64, pub hard_limit: bool }
impl Default for RateLimitConfig { fn default() -> Self { Self{max_requests:60,window_secs:60,hard_limit:true} } }
#[derive(Debug,Clone)] pub struct RateLimitStatus { pub current_count: u64, pub max_requests: u64, pub window_secs: u64, pub remaining: u64, pub reset_after_secs: u64 }

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check(&self, key: &str) -> RateLimitResult;
    async fn record(&self, key: &str);
    async fn reset(&self, key: &str);
    async fn status(&self, key: &str) -> Result<RateLimitStatus, String>;
}
#[derive(Default)] pub struct NoopRateLimiter;
#[async_trait] impl RateLimiter for NoopRateLimiter { async fn check(&self, _: &str) -> RateLimitResult { RateLimitResult::Allowed } async fn record(&self, _: &str) {} async fn reset(&self, _: &str) {} async fn status(&self, _: &str) -> Result<RateLimitStatus, String> { Ok(RateLimitStatus{current_count:0,max_requests:0,window_secs:0,remaining:0,reset_after_secs:0}) } }
