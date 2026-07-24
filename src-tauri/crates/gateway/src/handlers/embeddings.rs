// SPDX-License-Identifier: AGPL-3.0-only

//! POST /v1/embeddings — OpenAI 兼容的向量嵌入端点。
//!
//! 复用 `ProviderAdapter::embed` 方法，通过现有 provider 解析逻辑定位到
//! 支持 embed 的 provider（OpenAI / OpenClaw / Hermes / Ollama 等），
//! 返回 OpenAI 标准的 embeddings 响应体。

use std::collections::HashSet;
use std::time::Instant;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use axagent_harness::ProviderRequestContext;
use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::{error_response, provider_type_to_str, record_log};
use crate::handlers::models::{
    build_provider_public_id_map, parse_model_field, resolve_provider_for_model,
};
use crate::server::GatewayAppState;

/// OpenAI 兼容的 embeddings 请求体。
///
/// `input` 字段既支持单个字符串也支持字符串数组，这里用 `Value` 接收后
/// 统一规整为 `Vec<String>`。
#[derive(Debug, Deserialize)]
pub struct EmbeddingsRequest {
    pub model: String,
    pub input: Value,
    /// 编码格式："float"（默认）或 "base64"
    #[serde(default)]
    pub encoding_format: Option<String>,
    /// 可选维度（部分模型支持）
    #[serde(default)]
    pub dimensions: Option<usize>,
}

/// 单条嵌入数据项
#[derive(Debug, Serialize)]
struct EmbeddingItem {
    object: &'static str,
    embedding: Value,
    index: usize,
}

/// embeddings 用量统计
#[derive(Debug, Serialize)]
struct EmbeddingsUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

/// 把 `input` 字段规整为 `Vec<String>`。
///
/// 支持：
/// - 字符串 → 单元素数组
/// - 字符串数组 → 原样返回
/// - 其他类型 → 报错
fn normalize_input(input: &Value) -> Result<Vec<String>, String> {
    match input {
        Value::String(s) => Ok(vec![s.clone()]),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    Value::String(s) => out.push(s.clone()),
                    _ => return Err("input array must contain only strings".to_string()),
                }
            }
            Ok(out)
        },
        _ => Err("input must be a string or an array of strings".to_string()),
    }
}

/// POST /v1/embeddings — 调用 provider 的 embed 方法，返回 OpenAI 兼容格式。
pub async fn create_embedding(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(request): Json<EmbeddingsRequest>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    // 规整 input 字段
    let inputs = match normalize_input(&request.input) {
        Ok(v) => v,
        Err(msg) => {
            return error_response(StatusCode::BAD_REQUEST, &msg);
        },
    };

    // 拉取 provider 列表，过滤出支持 chat/embed 的 provider 类型
    let providers: Vec<ProviderConfig> = match state.adapter.providers().list_providers().await {
        Ok(p) => p
            .into_iter()
            .filter(|p| {
                matches!(
                    p.provider_type,
                    ProviderType::OpenAI
                        | ProviderType::OpenClaw
                        | ProviderType::Hermes
                        | ProviderType::Ollama
                )
            })
            .collect(),
        Err(_e) => {
            tracing::error!(error = ?_e, "Failed to list providers for embeddings");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred while listing providers",
            );
        },
    };

    let public_id_map = build_provider_public_id_map(&providers);
    let known_public_ids: HashSet<String> = public_id_map.values().cloned().collect();
    let parsed = parse_model_field(&request.model, &known_public_ids);

    let (provider, model_id) = match resolve_provider_for_model(&providers, &public_id_map, &parsed)
    {
        Ok(pair) => pair,
        Err(resp) => return resp,
    };

    // 获取并解密 API Key
    let provider_key = match state.adapter.providers().get_active_key(&provider.id).await {
        Ok(k) => k,
        Err(_) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("No active API key for provider '{}'", provider.name),
            );
        },
    };

    let api_key = match state.adapter.crypto().decrypt_key(&provider_key.key_encrypted) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to decrypt provider key: {}", e);
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal key error");
        },
    };

    let provider_type_str = provider_type_to_str(&provider.provider_type);

    let global_settings = state.adapter.settings().get_settings().await.unwrap_or_default();
    let resolved_proxy = axagent_harness::types::provider_model::resolve_provider_proxy(
        &provider.proxy_config,
        &global_settings,
    );

    let ctx = ProviderRequestContext {
        api_key,
        key_id: provider_key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: provider.custom_headers.as_ref().and_then(|s| {
            serde_json::from_str(s)
                .map_err(|e| tracing::warn!(error = %e, "Failed to parse custom headers"))
                .ok()
        }),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter = match state.provider_registry.get(provider_type_str) {
        Some(a) => a,
        None => match state.provider_registry.get("openai") {
            Some(a) => a,
            None => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("No adapter for provider type '{}'", provider_type_str),
                );
            },
        },
    };

    let embed_request =
        EmbedRequest { model: model_id.clone(), input: inputs, dimensions: request.dimensions };

    let embed_response = match adapter.embed(&ctx, embed_request).await {
        Ok(r) => r,
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i64;
            let err_msg = e.to_string();
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/v1/embeddings",
                Some(&model_id),
                &provider.id,
                502,
                elapsed,
                0,
                0,
                Some(&err_msg)
            );
            tracing::error!(
                error = %err_msg,
                provider = %provider.id,
                model = %model_id,
                "Embedding request failed"
            );
            return error_response(StatusCode::BAD_GATEWAY, "Embedding request failed");
        },
    };

    // 构造 OpenAI 兼容的响应体
    let encoding_format = request.encoding_format.as_deref().unwrap_or("float");
    let total_tokens: u32 =
        embed_response.embeddings.iter().map(|v| v.len() as u32).sum::<u32>() / 4; // 粗略估算：4 chars ≈ 1 token

    let data: Vec<EmbeddingItem> = embed_response
        .embeddings
        .into_iter()
        .enumerate()
        .map(|(index, embedding)| {
            let embedding_value = if encoding_format == "base64" {
                let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
                Value::String(base64::engine::general_purpose::STANDARD.encode(&bytes))
            } else {
                json!(embedding)
            };
            EmbeddingItem { object: "embedding", embedding: embedding_value, index }
        })
        .collect();

    let usage = EmbeddingsUsage { prompt_tokens: total_tokens, total_tokens };

    let elapsed = start_time.elapsed().as_millis() as i64;
    record_log!(
        &state.adapter,
        gateway_key,
        "POST",
        "/v1/embeddings",
        Some(&model_id),
        &provider.id,
        200,
        elapsed,
        total_tokens as i64,
        0,
        None
    );

    Json(json!({
        "object": "list",
        "data": data,
        "model": request.model,
        "usage": usage,
    }))
    .into_response()
}
