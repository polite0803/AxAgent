// SPDX-License-Identifier: AGPL-3.0-only

//! POST /v1/audio/transcriptions — 语音识别（STT）端点。
//! POST /v1/audio/speech — 语音合成（TTS）端点。
//!
//! 两个端点均通过 `ProviderAdapter` 的 `transcribe` / `speech` 方法调用
//! 实际 provider（如 OpenAI Whisper / TTS）。
//!
//! 注意：axum 的 `multipart` feature 未在 Cargo.toml 中启用（且本任务
//! 约束不修改 Cargo.toml），因此 `/v1/audio/transcriptions` 不走
//! multipart/form-data，而是接收原始音频字节（`application/octet-stream`
//! 或任意 `audio/*` Content-Type），model / language / prompt 通过 query
//! 参数传递。该简化实现仍然实际调用 provider 的 `transcribe` 方法。

use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;

use axagent_harness::ProviderAdapter;
use axagent_harness::ProviderRequestContext;
use axagent_harness::speech::{AudioChunkStream, SpeakRequest, SpeechInput};
use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::{error_response, provider_type_to_str, record_log};
use crate::server::GatewayAppState;

/// OpenAI 标准：音频转录文件大小上限 25MB
const MAX_AUDIO_FILE_SIZE: usize = 25 * 1024 * 1024;

/// `/v1/audio/transcriptions` 的 query 参数。
#[derive(Debug, Deserialize)]
pub struct TranscriptionQuery {
    /// 必填：模型名（如 whisper-1）
    pub model: String,
    /// 可选：语言代码（如 zh、en）—— 当前 provider trait 未接收，预留 API 兼容
    #[serde(default)]
    #[allow(dead_code)]
    pub language: Option<String>,
    /// 可选：引导提示词 —— 当前 provider trait 未接收，预留 API 兼容
    #[serde(default)]
    #[allow(dead_code)]
    pub prompt: Option<String>,
    /// 可选：采样率，默认 16000
    #[serde(default)]
    pub sample_rate: Option<u32>,
    /// 可选：声道数，默认 1
    #[serde(default)]
    pub channels: Option<u8>,
    /// 可选：编码方式，"pcm16" 或 "opus"，默认 pcm16
    #[serde(default)]
    pub encoding: Option<String>,
}

/// `/v1/audio/speech` 的 JSON 请求体。
#[derive(Debug, Deserialize)]
pub struct SpeechRequest {
    /// 模型名（如 tts-1、gpt-4o-mini-tts）
    pub model: String,
    /// 待合成的文本
    pub input: String,
    /// 音色（如 alloy、nova）
    #[serde(default)]
    pub voice: Option<String>,
    /// 响应格式：mp3 / opus / pcm，默认 mp3
    #[serde(default)]
    pub response_format: Option<String>,
    /// 语速（0.25 ~ 4.0）—— 当前 provider trait 未接收，预留 API 兼容
    #[serde(default)]
    #[allow(dead_code)]
    pub speed: Option<f32>,
}

/// 解析编码字符串为 `AudioEncoding`，默认 Pcm16。
fn parse_encoding(s: &str) -> AudioEncoding {
    match s.to_lowercase().as_str() {
        "opus" => AudioEncoding::Opus,
        _ => AudioEncoding::Pcm16,
    }
}

/// 把 `response_format` 映射为 `AudioFormat`。
/// mp3 / aac → Pcm16（provider 内部处理编码转换）
/// opus → Opus
/// pcm → Pcm16
fn response_format_to_audio_format(fmt: Option<&str>) -> AudioFormat {
    let encoding = match fmt.map(|s| s.to_lowercase()).as_deref() {
        Some("opus") => AudioEncoding::Opus,
        _ => AudioEncoding::Pcm16,
    };
    AudioFormat { sample_rate: 24000, channels: 1, encoding }
}

/// 根据 response_format 推断 HTTP Content-Type。
fn content_type_for_format(fmt: Option<&str>) -> &'static str {
    match fmt.map(|s| s.to_lowercase()).as_deref() {
        Some("opus") => "audio/ogg",
        Some("pcm") => "audio/pcm",
        Some("wav") => "audio/wav",
        _ => "audio/mpeg",
    }
}

/// 查找第一个支持语音能力的 OpenAI 兼容 provider，并构建请求上下文。
///
/// 返回 `(provider, model_id, ctx, adapter)`。`require_stt` / `require_tts`
/// 用于过滤 provider 是否具备所需语音能力。
async fn resolve_speech_provider(
    state: &GatewayAppState,
    model: &str,
    require_stt: bool,
    require_tts: bool,
) -> Result<
    (ProviderConfig, String, ProviderRequestContext, std::sync::Arc<dyn ProviderAdapter>),
    Response,
> {
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
            tracing::error!(error = ?_e, "Failed to list providers for audio endpoint");
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred while listing providers",
            ));
        },
    };

    // 选第一个 enabled 的 provider（音频端点不做 model 级路由，简化处理）
    let provider = match providers.iter().find(|p| p.enabled) {
        Some(p) => p.clone(),
        None => {
            return Err(error_response(
                StatusCode::BAD_GATEWAY,
                "No enabled OpenAI-compatible provider configured for audio endpoints",
            ));
        },
    };

    let provider_key = match state.adapter.providers().get_active_key(&provider.id).await {
        Ok(k) => k,
        Err(_) => {
            return Err(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("No active API key for provider '{}'", provider.name),
            ));
        },
    };

    let api_key = match state.adapter.crypto().decrypt_key(&provider_key.key_encrypted) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to decrypt provider key: {}", e);
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal key error"));
        },
    };

    let provider_type_str = provider_type_to_str(&provider.provider_type);
    let adapter = match state.provider_registry.get(provider_type_str) {
        Some(a) => a,
        None => match state.provider_registry.get("openai") {
            Some(a) => a,
            None => {
                return Err(error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("No adapter for provider type '{}'", provider_type_str),
                ));
            },
        },
    };

    // 检查 provider 是否具备所需语音能力
    let caps = adapter.supports_speech();
    if require_stt && !caps.stt {
        return Err(error_response(
            StatusCode::NOT_IMPLEMENTED,
            "Speech-to-text (transcription) is not supported by the configured provider",
        ));
    }
    if require_tts && !caps.tts {
        return Err(error_response(
            StatusCode::NOT_IMPLEMENTED,
            "Text-to-speech is not supported by the configured provider",
        ));
    }

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

    Ok((provider, model.to_string(), ctx, adapter))
}

/// POST /v1/audio/transcriptions — 语音识别端点。
///
/// 接收原始音频字节（非 multipart，因 axum multipart feature 未启用），
/// model / language / prompt 等参数通过 query string 传递。
/// 文件大小限制 25MB（OpenAI 标准）。
pub async fn create_transcription(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Query(query): Query<TranscriptionQuery>,
    body: Bytes,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    // 文件大小检查
    if body.len() > MAX_AUDIO_FILE_SIZE {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            &format!(
                "Audio file too large: {} bytes (max {} bytes / 25MB)",
                body.len(),
                MAX_AUDIO_FILE_SIZE
            ),
        );
    }

    if body.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Audio file body is empty");
    }

    let (provider, model_id, ctx, adapter) =
        match resolve_speech_provider(&state, &query.model, true, false).await {
            Ok(t) => t,
            Err(resp) => return resp,
        };

    let format = AudioFormat {
        sample_rate: query.sample_rate.unwrap_or(16000),
        channels: query.channels.unwrap_or(1),
        encoding: parse_encoding(query.encoding.as_deref().unwrap_or("pcm16")),
    };

    let input = SpeechInput { data: body.to_vec(), format };

    match adapter.transcribe(&ctx, input).await {
        Ok(text) => {
            let elapsed = start_time.elapsed().as_millis() as i64;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/v1/audio/transcriptions",
                Some(&model_id),
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );
            axum::Json(json!({ "text": text })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i64;
            let err_msg = e.to_string();
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/v1/audio/transcriptions",
                Some(&model_id),
                &provider.id,
                502,
                elapsed,
                0,
                0,
                Some(&err_msg)
            );
            tracing::error!(error = %err_msg, "Transcription failed");
            error_response(StatusCode::BAD_GATEWAY, "Transcription request failed")
        },
    }
}

/// POST /v1/audio/speech — 语音合成端点。
///
/// 接收 JSON 请求体，调用 provider 的 `speech` 方法获取流式音频，
/// 以二进制流返回（默认 Content-Type: audio/mpeg）。
pub async fn create_speech(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(request): Json<SpeechRequest>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    if request.input.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Input text is empty");
    }

    let (provider, model_id, ctx, adapter) =
        match resolve_speech_provider(&state, &request.model, false, true).await {
            Ok(t) => t,
            Err(resp) => return resp,
        };

    let audio_format = response_format_to_audio_format(request.response_format.as_deref());
    let speak_req = SpeakRequest {
        text: request.input,
        voice: request.voice,
        format: audio_format,
        model: Some(model_id.clone()),
    };

    let stream: AudioChunkStream = match adapter.speech(&ctx, speak_req).await {
        Ok(s) => s,
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i64;
            let err_msg = e.to_string();
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/v1/audio/speech",
                Some(&model_id),
                &provider.id,
                502,
                elapsed,
                0,
                0,
                Some(&err_msg)
            );
            tracing::error!(error = %err_msg, "Speech synthesis failed");
            return error_response(StatusCode::BAD_GATEWAY, "Speech synthesis request failed");
        },
    };

    let content_type = content_type_for_format(request.response_format.as_deref());

    // 把 AudioChunkStream 转为 axum Body 流
    // 首个 chunk 如果是错误，需要返回错误响应而非空音频流。
    let body_stream = stream.map(|result| match result {
        Ok(bytes) => Ok::<Bytes, std::io::Error>(bytes.into()),
        Err(e) => {
            tracing::error!(error = %e, "Audio stream chunk error");
            Err(std::io::Error::other("audio stream error"))
        },
    });

    let elapsed = start_time.elapsed().as_millis() as i64;
    record_log!(
        &state.adapter,
        gateway_key,
        "POST",
        "/v1/audio/speech",
        Some(&model_id),
        &provider.id,
        200,
        elapsed,
        0,
        0,
        None
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .body(Body::from_stream(body_stream))
        .unwrap_or_else(|_| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build audio response")
        })
}
