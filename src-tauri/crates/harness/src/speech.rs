// SPDX-License-Identifier: AGPL-3.0-only

//! 语音能力契约（STT / TTS）
//!
//! 把语音识别与语音合成抽象为 `ProviderAdapter` 上的两个能力方法，
//! 后端 realtime 编排据此从「系统已有的模型服务商」中按能力选择后端，
//! 不绑定任何具体厂商。harness 仅定义 trait + 纯 DTO，零业务逻辑。

use crate::core_error::Result;
use crate::types::AudioFormat;
use futures::Stream;
use std::pin::Pin;

/// 语音能力声明：provider 是否支持 STT（语音识别）/ TTS（语音合成）。
///
/// 默认都不支持；支持语音的 provider（如 OpenAI）覆写 `ProviderAdapter::supports_speech`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpeechCapabilities {
    /// 支持把音频转写成文本（Speech-to-Text）
    pub stt: bool,
    /// 支持把文本合成为音频（Text-to-Speech）
    pub tts: bool,
}

impl SpeechCapabilities {
    /// 既不支持 STT 也不支持 TTS（默认能力）
    pub fn none() -> Self {
        Self { stt: false, tts: false }
    }

    /// 同时支持 STT 与 TTS
    pub fn all() -> Self {
        Self { stt: true, tts: true }
    }
}

/// STT 输入：一段原始音频字节 + 其格式描述。
///
/// 音频可由前端以 PCM16 裸流上传，后端负责封装（如包成 WAV）后再发给服务商。
#[derive(Debug, Clone)]
pub struct SpeechInput {
    /// 原始音频字节（编码见 `format`）
    pub data: Vec<u8>,
    /// 音频格式（采样率 / 声道 / 编码）
    pub format: AudioFormat,
}

/// TTS 请求：要合成的文本、音色、目标格式。
#[derive(Debug, Clone)]
pub struct SpeakRequest {
    /// 待合成的文本
    pub text: String,
    /// 音色（服务商特定，如 OpenAI 的 "alloy" / "nova"）
    pub voice: Option<String>,
    /// 目标音频格式（后端据此选择服务商参数，如 PCM16 → OpenAI `response_format=pcm`）
    pub format: AudioFormat,
    /// 可选 TTS 模型（如 OpenAI 的 "tts-1" / "gpt-4o-mini-tts"）；为空时由 provider 用默认模型
    pub model: Option<String>,
}

/// 流式音频块流（TTS 输出）。
///
/// 每个 `Ok(Vec<u8>)` 是一段原始音频字节（编码由 `SpeakRequest.format.encoding` 决定，
/// 通常为 PCM16 小端），调用方按格式解码后边收边播。
pub type AudioChunkStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>>> + Send>>;

/// 默认 TTS 实现：返回一个立即以「不支持」错误结束的流。
///
/// provider 未实现语音合成时，`ProviderAdapter::speech` 走此默认分支。
pub fn unsupported_speech_stream() -> AudioChunkStream {
    use futures::stream;
    Box::pin(stream::once(async {
        Err(crate::core_error::AxAgentError::Provider(
            "speech (TTS) is not supported by this provider".to_string(),
        ))
    }))
}
