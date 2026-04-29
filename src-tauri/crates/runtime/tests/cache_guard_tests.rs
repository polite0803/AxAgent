use axagent_runtime::{CacheGuard, PromptCache};
use std::sync::Arc;

#[tokio::test]
async fn test_cache_guard_default_allows_modifications() {
    let cache = Arc::new(PromptCache::new());
    let guard = CacheGuard::new(cache);

    assert!(guard.can_modify_system_prompt().await);
    assert!(guard.can_modify_tools().await);
    assert!(guard.can_reload_memory().await);
}

#[tokio::test]
async fn test_cache_guard_blocks_when_valid() {
    let cache = Arc::new(PromptCache::new());
    cache.record_system_prompt("test prompt").await;
    let guard = CacheGuard::new(cache);

    assert!(!guard.can_modify_system_prompt().await);
    assert!(!guard.can_modify_tools().await);
    assert!(!guard.can_reload_memory().await);
}

#[tokio::test]
async fn test_cache_guard_allows_with_force_immediate() {
    let cache = Arc::new(PromptCache::new());
    cache.record_system_prompt("test prompt").await;
    let guard = CacheGuard::new(cache);

    guard.set_force_immediate(true).await;
    assert!(guard.is_force_immediate().await);
    assert!(guard.can_modify_system_prompt().await);
    assert!(guard.can_modify_tools().await);
    assert!(guard.can_reload_memory().await);
}

#[tokio::test]
async fn test_cache_guard_system_prompt_guard_error() {
    let cache = Arc::new(PromptCache::new());
    cache.record_system_prompt("active prompt").await;
    let guard = CacheGuard::new(cache);

    let result = guard.guard_system_prompt_modification().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cache_guard_system_prompt_guard_ok_with_now() {
    let cache = Arc::new(PromptCache::new());
    cache.record_system_prompt("active prompt").await;
    let guard = CacheGuard::new(cache);

    guard.set_force_immediate(true).await;
    let result = guard.guard_system_prompt_modification().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cache_guard_tool_guard_error() {
    let cache = Arc::new(PromptCache::new());
    cache.record_system_prompt("active session").await;
    cache.record_tools(&["tool_a".to_string()]).await;
    let guard = CacheGuard::new(cache);

    let result = guard.guard_tool_modification().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cache_guard_memory_guard_error() {
    let cache = Arc::new(PromptCache::new());
    cache.record_system_prompt("test").await;
    let guard = CacheGuard::new(cache);

    let result = guard.guard_memory_reload().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cache_guard_any_change_guard() {
    let cache = Arc::new(PromptCache::new());
    cache.record_system_prompt("setup").await;
    let guard = CacheGuard::new(cache);

    let result = guard.guard_any_cache_sensitive_change("skills").await;
    assert!(result.is_err());

    guard.set_force_immediate(true).await;
    let result = guard.guard_any_cache_sensitive_change("skills").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cache_guard_force_immediate_toggle() {
    let cache = Arc::new(PromptCache::new());
    let guard = CacheGuard::new(cache);

    assert!(!guard.is_force_immediate().await);
    guard.set_force_immediate(true).await;
    assert!(guard.is_force_immediate().await);
    guard.set_force_immediate(false).await;
    assert!(!guard.is_force_immediate().await);
}

#[tokio::test]
async fn test_cache_guard_with_empty_cache() {
    let cache = Arc::new(PromptCache::new());
    let guard = CacheGuard::new(cache);

    // When cache is empty (no system prompt recorded), all guards should pass
    assert!(guard.guard_system_prompt_modification().await.is_ok());
    assert!(guard.guard_tool_modification().await.is_ok());
    assert!(guard.guard_memory_reload().await.is_ok());
}
