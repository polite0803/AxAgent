// builtin_tools 已迁移至 axagent-tools crate
use axagent_tools::builtin_handlers::SkillMetadata;

#[test]
fn test_builtin_tools_module_has_skill_metadata() {
    let metadata = SkillMetadata {
        name: "test_skill".to_string(),
        description: "A test skill".to_string(),
        version: "1.0.0".to_string(),
    };
    assert_eq!(metadata.name, "test_skill");
    assert_eq!(metadata.description, "A test skill");
    assert_eq!(metadata.version, "1.0.0");
}

#[test]
fn test_builtin_tools_skill_metadata_deserializes() {
    let json = r#"{"name":"test","description":"desc","version":"1.0"}"#;
    let metadata: SkillMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(metadata.name, "test");
}

use axagent_core::types;

#[test]
fn test_chat_message_serialization() {
    let msg = types::ChatMessage {
        role: "user".to_string(),
        content: types::ChatContent::Text("Hello".to_string()),
        tool_calls: None,
        tool_call_id: None,
    };
    let json_str = serde_json::to_string(&msg).unwrap();
    assert!(json_str.contains("Hello"));
    assert!(json_str.contains("user"));
}

#[test]
fn test_chat_request_basic() {
    let request = types::ChatRequest {
        model: "test-model".to_string(),
        messages: vec![],
        stream: false,
        temperature: Some(0.7),
        top_p: None,
        max_tokens: Some(1024),
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };
    assert_eq!(request.model, "test-model");
    assert_eq!(request.temperature, Some(0.7));
}

#[test]
fn test_chat_tool_definition() {
    let tool = types::ChatTool {
        r#type: "function".to_string(),
        function: types::ChatToolFunction {
            name: "read_file".to_string(),
            description: Some("Read a file".to_string()),
            parameters: Some(serde_json::json!({})),
        },
    };
    assert_eq!(tool.function.name, "read_file");
}

#[test]
fn test_token_usage_types() {
    let usage = types::TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };
    assert_eq!(usage.total_tokens, 150);
    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.completion_tokens, 50);
}

#[test]
fn test_provider_type_enum() {
    use axagent_core::types::ProviderType;
    let types = vec![
        ProviderType::OpenAI,
        ProviderType::Anthropic,
        ProviderType::Gemini,
        ProviderType::Ollama,
        ProviderType::Hermes,
        ProviderType::OpenClaw,
        ProviderType::OpenAIResponses,
    ];
    assert_eq!(types.len(), 7);
}
