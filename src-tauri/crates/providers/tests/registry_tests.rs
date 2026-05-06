use axagent_providers::registry::ProviderRegistry;

#[test]
fn test_registry_new_is_empty() {
    let registry = ProviderRegistry::new();
    assert!(registry.get("openai").is_none());
    assert!(registry.get("anthropic").is_none());
}

#[test]
fn test_registry_create_default_has_all_providers() {
    let registry = ProviderRegistry::create_default();
    assert!(registry.get("openai").is_some());
    assert!(registry.get("anthropic").is_some());
    assert!(registry.get("gemini").is_some());
    assert!(registry.get("openclaw").is_some());
    assert!(registry.get("hermes").is_some());
    assert!(registry.get("ollama").is_some());
    assert!(registry.get("openai_responses").is_some());
}

#[test]
fn test_registry_get_unknown_returns_none() {
    let registry = ProviderRegistry::create_default();
    assert!(registry.get("nonexistent-provider").is_none());
}

#[test]
fn test_registry_register_and_get() {
    let mut registry = ProviderRegistry::new();
    assert!(registry.get("openai").is_none());

    registry.register("openai", Box::new(axagent_providers::openai::OpenAIAdapter::new()));
    assert!(registry.get("openai").is_some());
}

#[test]
fn test_registry_register_overwrites_existing() {
    let mut registry = ProviderRegistry::new();
    registry.register("test", Box::new(axagent_providers::openai::OpenAIAdapter::new()));
    assert!(registry.get("test").is_some());

    // Register another adapter under the same key — should be fine
    registry.register("test", Box::new(axagent_providers::anthropic::AnthropicAdapter::new()));
    assert!(registry.get("test").is_some());
}

#[test]
fn test_registry_default_and_new_produce_different_results() {
    let new_registry = ProviderRegistry::new();
    let default_registry = ProviderRegistry::create_default();

    assert!(new_registry.get("openai").is_none());
    assert!(default_registry.get("openai").is_some());
}

#[test]
fn test_registry_get_returns_valid_adapter() {
    let registry = ProviderRegistry::create_default();
    let adapter = registry.get("anthropic");
    assert!(adapter.is_some());
    // The adapter trait object should be usable — at minimum it's not null
    let _ = adapter.unwrap();
}
