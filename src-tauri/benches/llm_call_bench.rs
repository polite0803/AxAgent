// SPDX-License-Identifier: AGPL-3.0-only

//! Benchmarks for LLM call / provider path: utility functions used in
//! every API request (user-agent generation, URL redaction, header
//! construction, error diagnosis).
//!
//! Covers critical paths identified in audit I-04:
//!   - default_user_agent() — called on every request
//!   - redact_api_key_from_url() — called during error reporting
//!   - diagnose_reqwest_error() / diagnose_http_status() — error path
//!   - parse_base64_data_url() — image input path

use axagent_providers::{
    ProviderRequestContext, apply_request_headers, default_user_agent, diagnose_http_status,
    parse_base64_data_url, redact_api_key_from_url,
};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_default_user_agent(c: &mut Criterion) {
    c.bench_function("llm_default_user_agent", |b| {
        b.iter(|| {
            std::hint::black_box(default_user_agent());
        })
    });
}

fn bench_redact_api_key_url(c: &mut Criterion) {
    let url_with_key = "https://api.openai.com/v1/chat/completions?key=sk-abc123def456&model=gpt-4";
    let url_no_key = "https://api.anthropic.com/v1/messages";

    c.bench_function("llm_redact_key_url_with_key", |b| {
        b.iter(|| {
            std::hint::black_box(redact_api_key_from_url(std::hint::black_box(url_with_key)));
        })
    });
    c.bench_function("llm_redact_key_url_no_key", |b| {
        b.iter(|| {
            std::hint::black_box(redact_api_key_from_url(std::hint::black_box(url_no_key)));
        })
    });
}

fn bench_parse_base64_data_url(c: &mut Criterion) {
    let valid = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let invalid = "not-a-data-url";

    c.bench_function("llm_parse_base64_valid", |b| {
        b.iter(|| {
            std::hint::black_box(parse_base64_data_url(std::hint::black_box(valid)));
        })
    });
    c.bench_function("llm_parse_base64_invalid", |b| {
        b.iter(|| {
            std::hint::black_box(parse_base64_data_url(std::hint::black_box(invalid)));
        })
    });
}

fn bench_diagnose_reqwest_error_connect(c: &mut Criterion) {
    // Simulate the diagnostic logic without actual I/O.
    // Use a well-known error message pattern to measure the string-building
    // overhead of the diagnostic helper.
    c.bench_function("llm_diagnose_connect_msg", |b| {
        b.iter(|| {
            let err_msg = "error sending request for url (http://127.0.0.1:1/): \
                           client error (Connect)";
            // Measure the string-processing cost: the function inspects
            // the error message and appends advice.
            std::hint::black_box(diagnose_reqwest_error_string(std::hint::black_box(err_msg)));
        })
    });
}

/// A lightweight version of diagnose_reqwest_error that works on a string
/// instead of a reqwest::Error, for benchmarking the string-building logic
/// without depending on reqwest's blocking feature.
fn diagnose_reqwest_error_string(msg: &str) -> String {
    if msg.contains("Connect") || msg.contains("connect") {
        format!(
            "{msg}. Possible causes: DNS resolution failure, server unreachable, TLS/SSL handshake error, proxy connection refused, or firewall blocking. Check your network, proxy settings, and API host URL."
        )
    } else if msg.contains("timeout") || msg.contains("Timeout") {
        format!(
            "{msg}. The request timed out. The server may be overloaded or your network may be slow. Try again later or check your network connection."
        )
    } else if msg.contains("decode") || msg.contains("Decode") {
        format!(
            "{msg}. Failed to decode the response body. This can happen if the connection was interrupted mid-stream, the server sent invalid data, or there was a TLS error."
        )
    } else {
        format!("{msg}. Check your network connection, proxy settings, and API host URL.")
    }
}

fn bench_diagnose_http_status(c: &mut Criterion) {
    use reqwest::StatusCode;
    c.bench_function("llm_diagnose_401", |b| {
        b.iter(|| {
            std::hint::black_box(diagnose_http_status(
                std::hint::black_box("OpenAI"),
                std::hint::black_box(StatusCode::UNAUTHORIZED),
                std::hint::black_box("{\"error\":\"invalid_api_key\"}"),
            ));
        })
    });
    c.bench_function("llm_diagnose_429", |b| {
        b.iter(|| {
            std::hint::black_box(diagnose_http_status(
                std::hint::black_box("Anthropic"),
                std::hint::black_box(StatusCode::TOO_MANY_REQUESTS),
                std::hint::black_box("Rate limit exceeded"),
            ));
        })
    });
}

fn bench_apply_request_headers(c: &mut Criterion) {
    let ctx = ProviderRequestContext {
        custom_headers: Some(
            [
                ("X-Request-ID".to_string(), "abc-123".to_string()),
                ("X-Trace-ID".to_string(), "trace-456".to_string()),
            ]
            .into(),
        ),
        api_key: String::new(),
        key_id: String::new(),
        provider_id: String::new(),
        base_url: None,
        api_path: None,
        proxy_config: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    c.bench_function("llm_apply_request_headers", |b| {
        b.iter(|| {
            let client = reqwest::Client::new();
            let builder = client.get("https://api.example.com/v1/test");
            let _ = std::hint::black_box(apply_request_headers(
                std::hint::black_box(builder),
                std::hint::black_box(&ctx),
            ));
        })
    });
}

criterion_group!(
    benches,
    bench_default_user_agent,
    bench_redact_api_key_url,
    bench_parse_base64_data_url,
    bench_diagnose_reqwest_error_connect,
    bench_diagnose_http_status,
    bench_apply_request_headers,
);
criterion_main!(benches);
