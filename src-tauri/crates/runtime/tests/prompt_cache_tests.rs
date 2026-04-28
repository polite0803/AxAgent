use axagent_runtime::prompt_cache::PromptCache;

#[tokio::test]
async fn test_prompt_cache_new_is_cache_invalid() {
    let cache = PromptCache::new();
    assert!(!cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_record_system_prompt_makes_cache_valid() {
    let cache = PromptCache::new();
    cache.record_system_prompt("You are a helpful assistant.").await;
    assert!(cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_changing_system_prompt_invalidates_cache() {
    let cache = PromptCache::new();
    cache.record_system_prompt("Version 1").await;
    assert!(cache.is_cache_valid().await);

    cache.record_system_prompt("Version 2").await;
    assert!(!cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_same_system_prompt_keeps_cache_valid() {
    let cache = PromptCache::new();
    cache.record_system_prompt("same prompt").await;
    assert!(cache.is_cache_valid().await);

    cache.record_system_prompt("same prompt").await;
    assert!(cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_record_tools() {
    let cache = PromptCache::new();
    let tools = vec!["bash".to_string(), "read".to_string(), "write".to_string()];
    cache.record_tools(&tools).await;

    let state = cache.get_state().await;
    assert!(state.tools_hash.is_some());
    assert!(cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_changing_tools_generates_pending_changes() {
    let cache = PromptCache::new();
    cache.record_tools(&["tool_a".to_string()]).await;
    cache.record_tools(&["tool_b".to_string()]).await;

    assert!(cache.has_pending_changes().await);

    let state = cache.get_state().await;
    let changes = state.pending_changes;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].component, "tools");
}

#[tokio::test]
async fn test_record_memory() {
    let cache = PromptCache::new();
    cache.record_memory("memory content v1").await;

    let state = cache.get_state().await;
    assert!(state.memory_hash.is_some());

    // Same content should not create pending changes
    cache.record_memory("memory content v1").await;
    assert!(!cache.has_pending_changes().await);

    // Different content should create pending changes
    cache.record_memory("memory content v2").await;
    assert!(cache.has_pending_changes().await);
}

#[tokio::test]
async fn test_record_context_files() {
    let cache = PromptCache::new();
    cache.record_context_files("AGENTS.md content").await;

    let state = cache.get_state().await;
    assert!(state.context_files_hash.is_some());
}

#[tokio::test]
async fn test_explicit_invalidation() {
    let cache = PromptCache::new();
    cache.record_system_prompt("test").await;
    assert!(cache.is_cache_valid().await);

    cache.invalidate("manual test invalidation").await;
    assert!(!cache.is_cache_valid().await);

    let state = cache.get_state().await;
    assert_eq!(
        state.last_invalidation_reason,
        Some("manual test invalidation".to_string())
    );
}

#[tokio::test]
async fn test_invalidate_for_new_session() {
    let cache = PromptCache::new();
    cache.record_system_prompt("session 1 prompt").await;
    cache.record_tools(&["s1tool".to_string()]).await;
    cache.record_memory("session 1 memory").await;

    cache.invalidate_for_new_session().await;

    let state = cache.get_state().await;
    assert!(state.system_prompt_hash.is_none());
    assert!(state.tools_hash.is_none());
    assert!(state.memory_hash.is_none());
    assert!(!state.cache_valid);
}

#[tokio::test]
async fn test_apply_pending_changes_clears_them() {
    let cache = PromptCache::new();
    cache.record_system_prompt("v1").await;
    cache.record_system_prompt("v2").await;

    assert!(cache.has_pending_changes().await);

    let changes = cache.apply_pending_changes().await;
    assert!(!changes.is_empty());
    assert!(!cache.has_pending_changes().await);
    assert!(cache.is_cache_valid().await);
}

#[tokio::test]
async fn test_cache_hit_tracking() {
    let cache = PromptCache::new();
    cache.mark_cache_hit(1500).await;
    cache.mark_cache_hit(2300).await;

    assert_eq!(cache.total_tokens_saved().await, 3800);

    let state = cache.get_state().await;
    assert_eq!(state.cache_hits, 2);

    cache.reset_stats().await;
    assert_eq!(cache.total_tokens_saved().await, 0);

    let state = cache.get_state().await;
    assert_eq!(state.cache_hits, 0);
}

#[tokio::test]
async fn test_hash_deterministic() {
    let cache = PromptCache::new();
    cache.record_system_prompt("deterministic test").await;
    let state1 = cache.get_state().await;

    let cache2 = PromptCache::new();
    cache2.record_system_prompt("deterministic test").await;
    let state2 = cache2.get_state().await;

    assert_eq!(state1.system_prompt_hash, state2.system_prompt_hash);
}

#[tokio::test]
async fn test_cache_guard_initial_state() {
    use axagent_runtime::CacheGuard;
    let cache = std::sync::Arc::new(PromptCache::new());
    let guard = CacheGuard::new(cache.clone());

    assert!(!guard.is_force_immediate().await);
    assert!(guard.can_modify_system_prompt().await);
    assert!(guard.can_modify_tools().await);
}

#[tokio::test]
async fn test_cache_guard_blocks_when_cache_valid() {
    use axagent_runtime::CacheGuard;
    let cache = std::sync::Arc::new(PromptCache::new());
    cache.record_system_prompt("test").await;
    let guard = CacheGuard::new(cache);

    assert!(!guard.can_modify_system_prompt().await);
    let result = guard.guard_system_prompt_modification().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cache_guard_allows_with_force_immediate() {
    use axagent_runtime::CacheGuard;
    let cache = std::sync::Arc::new(PromptCache::new());
    cache.record_system_prompt("test").await;
    let guard = CacheGuard::new(cache);

    guard.set_force_immediate(true).await;
    assert!(guard.can_modify_system_prompt().await);
    let result = guard.guard_system_prompt_modification().await;
    assert!(result.is_ok());
}
