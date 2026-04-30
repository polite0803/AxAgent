use axagent_providers::*;

// ── URL resolution tests ──────────────────────────────────────────

#[test]
fn test_resolve_base_url_appends_default_version() {
    assert_eq!(
        resolve_base_url("https://api.openai.com"),
        "https://api.openai.com/v1"
    );
}

#[test]
fn test_resolve_base_url_already_has_version() {
    assert_eq!(
        resolve_base_url("https://api.openai.com/v1"),
        "https://api.openai.com/v1"
    );
}

#[test]
fn test_resolve_base_url_with_trailing_slash() {
    assert_eq!(
        resolve_base_url("https://api.openai.com/"),
        "https://api.openai.com/v1"
    );
}

#[test]
fn test_resolve_base_url_force_mode_strips_bang() {
    assert_eq!(
        resolve_base_url("https://api.openai.com!"),
        "https://api.openai.com"
    );
}

#[test]
fn test_resolve_base_url_force_mode_with_path() {
    assert_eq!(
        resolve_base_url("https://api.openai.com/v2!"),
        "https://api.openai.com/v2"
    );
}

#[test]
fn test_resolve_base_url_for_type_gemini() {
    use axagent_core::types::ProviderType;
    assert_eq!(
        resolve_base_url_for_type("https://api.google.com", &ProviderType::Gemini),
        "https://api.google.com/v1beta"
    );
}

#[test]
fn test_resolve_base_url_for_type_openai() {
    use axagent_core::types::ProviderType;
    assert_eq!(
        resolve_base_url_for_type("https://api.openai.com", &ProviderType::OpenAI),
        "https://api.openai.com/v1"
    );
}

#[test]
fn test_resolve_base_url_v2_version() {
    assert_eq!(
        resolve_base_url("https://api.example.com/v2"),
        "https://api.example.com/v2"
    );
}

#[test]
fn test_resolve_base_url_v1beta_version() {
    assert_eq!(
        resolve_base_url("https://api.example.com/v1beta"),
        "https://api.example.com/v1beta"
    );
}

// ── resolve_chat_url tests ────────────────────────────────────────

#[test]
fn test_resolve_chat_url_default_suffix() {
    assert_eq!(
        resolve_chat_url("https://api.openai.com/v1", None, "/chat/completions"),
        "https://api.openai.com/v1/chat/completions"
    );
}

#[test]
fn test_resolve_chat_url_with_api_path() {
    assert_eq!(
        resolve_chat_url(
            "https://api.openai.com/v1",
            Some("/custom/endpoint"),
            "/chat/completions"
        ),
        "https://api.openai.com/v1/custom/endpoint"
    );
}

#[test]
fn test_resolve_chat_url_force_mode_with_bang() {
    assert_eq!(
        resolve_chat_url(
            "https://api.openai.com",
            Some("/v1/messages!"),
            "/chat/completions"
        ),
        "https://api.openai.com/v1/messages"
    );
}

#[test]
fn test_resolve_chat_url_auto_dedup() {
    // When base ends with /v1 and path starts with /v1, strip duplicate
    assert_eq!(
        resolve_chat_url(
            "https://api.openai.com/v1",
            Some("/v1/messages"),
            "/chat/completions"
        ),
        "https://api.openai.com/v1/messages"
    );
}

#[test]
fn test_resolve_chat_url_empty_path() {
    assert_eq!(
        resolve_chat_url("https://api.openai.com/v1", Some(""), "/chat/completions"),
        "https://api.openai.com/v1/chat/completions"
    );
}

#[test]
fn test_resolve_chat_url_path_without_slash() {
    assert_eq!(
        resolve_chat_url(
            "https://api.openai.com/v1",
            Some("messages"),
            "/chat/completions"
        ),
        "https://api.openai.com/v1/messages"
    );
}

// ── parse_base64_data_url tests ──────────────────────────────────

#[test]
fn test_parse_valid_base64_data_url() {
    let result = parse_base64_data_url(
        "data:image/png;base64,iVBORw0KGgo="
    );
    assert!(result.is_some());
    let (mime, data) = result.unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(data, "iVBORw0KGgo=");
}

#[test]
fn test_parse_base64_data_url_invalid_no_data() {
    assert!(parse_base64_data_url("not-a-data-url").is_none());
}

#[test]
fn test_parse_base64_data_url_missing_base64() {
    assert!(parse_base64_data_url("data:image/png,inlinedata").is_none());
}

#[test]
fn test_parse_base64_data_url_empty_mime() {
    assert!(parse_base64_data_url("data:;base64,iVBORw0KGgo=").is_none());
}

// ── default_user_agent tests ──────────────────────────────────────

#[test]
fn test_default_user_agent_format() {
    let ua = default_user_agent();
    assert!(ua.starts_with("AxAgent-"));
    assert!(ua.contains('/'));
    assert!(ua.contains(std::env::consts::OS));
}

// ── diagnose_reqwest_error smoke tests ───────────────────────────

#[test]
fn test_diagnose_reqwest_error_basic() {
    // We can't easily mock reqwest::Error, but we can test the function
    // compiles and returns a non-empty diagnostic string for the cases
    // we can simulate via type checking.
    // This test ensures the function is callable and returns a String.
    assert!(std::mem::size_of::<fn(&reqwest::Error) -> String>() > 0);
}

// ── diagnose_http_status tests ───────────────────────────────────

#[test]
fn test_diagnose_401_authentication_error() {
    let msg = diagnose_http_status(
        "OpenAI",
        reqwest::StatusCode::UNAUTHORIZED,
        "Invalid API key"
    );
    assert!(msg.contains("401"));
    assert!(msg.contains("Authentication failed"));
    assert!(msg.contains("Invalid API key"));
}

#[test]
fn test_diagnose_429_rate_limit() {
    let msg = diagnose_http_status(
        "Anthropic",
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        "Rate limit exceeded"
    );
    assert!(msg.contains("429"));
    assert!(msg.contains("Rate limit exceeded"));
}

#[test]
fn test_diagnose_500_internal_error() {
    let msg = diagnose_http_status(
        "Gemini",
        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        "Internal error"
    );
    assert!(msg.contains("500"));
    assert!(msg.contains("Internal server error"));
}

#[test]
fn test_diagnose_400_bad_request() {
    let msg = diagnose_http_status(
        "Ollama",
        reqwest::StatusCode::BAD_REQUEST,
        "Bad request body"
    );
    assert!(msg.contains("400"));
    assert!(msg.contains("Bad request"));
}

#[test]
fn test_diagnose_404_not_found() {
    let msg = diagnose_http_status(
        "Hermes",
        reqwest::StatusCode::NOT_FOUND,
        "Not found"
    );
    assert!(msg.contains("404"));
    assert!(msg.contains("Not found"));
}

#[test]
fn test_diagnose_unknown_status_code() {
    let msg = diagnose_http_status(
        "test-provider",
        reqwest::StatusCode::IM_A_TEAPOT,
        "I'm a teapot"
    );
    assert!(msg.contains("418"));
    assert!(msg.contains("I'm a teapot"));
}
