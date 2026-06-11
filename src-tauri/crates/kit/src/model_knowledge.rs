// SPDX-License-Identifier: AGPL-3.0-only

/// 返回已知模型的上下文窗口大小（最大输入 token 数）。
/// 支持精确匹配和模糊匹配（去除 Ollama 标签后缀、版本号等）。
pub fn get_model_context_window(model_id: &str) -> Option<u32> {
    // 先尝试精确匹配
    if let Some(result) = lookup_exact(model_id) {
        return Some(result);
    }
    // 模糊匹配：去除 :tag 后缀、版本号后缀等
    let normalized = normalize_model_id(model_id);
    lookup_exact(&normalized)
}

fn normalize_model_id(model_id: &str) -> String {
    let id = model_id.to_lowercase();
    // 去除 Ollama 风格标签后缀：:latest, :3b, :7b, :q4_k_m 等
    let id = id.split(':').next().unwrap_or(&id).to_string();
    // 去除日期版本后缀：@2024-08-06, -20241022 等

    id.split('@').next().unwrap_or(&id).to_string()
}

fn lookup_exact(model_id: &str) -> Option<u32> {
    let id = model_id.to_lowercase();
    let id = id.as_str();
    match id {
        // ── OpenAI ──
        "gpt-4o"
        | "gpt-4o-2024-08-06"
        | "gpt-4o-2024-11-20"
        | "gpt-4o-mini"
        | "gpt-4-turbo"
        | "gpt-4-turbo-2024-04-09"
        | "gpt-4"
        | "gpt-4-0613"
        | "gpt-4.5-preview"
        | "gpt-4.5" => Some(128_000),

        // ── OpenAI GPT-5.x ──
        "gpt-5.5" | "gpt-5.4" | "gpt-5.4-mini" | "gpt-5-mini" | "gpt-5-nano" | "gpt-5"
        | "gpt-5.1" | "gpt-5.2" | "gpt-5-chat-latest" => Some(1_048_576),

        // ── OpenAI GPT-4.1 (legacy) ──
        "gpt-4.1" | "gpt-4.1-mini" | "gpt-4.1-nano" => Some(1_047_576),

        "gpt-3.5-turbo" | "gpt-3.5-turbo-0125" | "gpt-3.5-turbo-1106" => Some(16_385),

        "o1" | "o1-mini" | "o3-mini" | "o3" | "o4-mini" => Some(200_000),

        // ── Anthropic ──
        "claude-opus-4-20250514"
        | "claude-opus-4-8"
        | "claude-sonnet-4-20250514"
        | "claude-sonnet-4-6"
        | "claude-3-5-sonnet-20241022"
        | "claude-3-5-haiku-20241022"
        | "claude-haiku-4-5"
        | "claude-3-haiku-20240307"
        | "claude-3-opus-20240229"
        | "claude-3-sonnet-20240229"
        | "claude-4-sonnet" => Some(200_000),

        "claude-3-5-sonnet" | "claude-3-5-haiku" | "claude-3-haiku" | "claude-3-opus"
        | "claude-3-sonnet" | "claude-opus-4" | "claude-sonnet-4" => Some(200_000),

        // ── Google Gemini ──
        "gemini-3.5-flash" | "gemini-2.5-pro" | "gemini-2.5-flash" | "gemini-2.0-flash"
        | "gemini-1.5-pro" | "gemini-1.5-flash" => Some(1_048_576),

        // ── Qwen (通义千问) ──
        "qwen3.7-max"
        | "qwen3.7-max-preview"
        | "qwen3.6-plus"
        | "qwen3.6-flash"
        | "qwen3.6-max-preview"
        | "qwen3.5-plus"
        | "qwen3.5-flash"
        | "qwen3-max"
        | "qwen3-235b-a22b"
        | "qwen3-coder-plus"
        | "qwen3-coder-flash"
        | "qwen-plus"
        | "qwen-max"
        | "qwen-flash"
        | "qwen-turbo" => Some(1_048_576),

        // ── Kimi (月之暗面) ──
        "kimi-k2.6" | "kimi-k2.5" | "kimi-k2" | "moonshot-v1-128k" | "moonshot-v1-32k"
        | "moonshot-v1-8k" => Some(262_144),

        // ── Doubao (豆包) ──
        "doubao-1.5-pro-256k"
        | "doubao-1.5-pro-128k"
        | "doubao-1.5-pro-32k"
        | "doubao-1.5-pro-4k" => Some(262_144),
        "doubao-1.5-lite-128k" | "doubao-1.5-lite-32k" | "doubao-1.5-lite-4k" => Some(131_072),

        // ── SiliconFlow (硅基流动) Pro models ──
        "Pro/deepseek-ai/DeepSeek-R1"
        | "Pro/deepseek-ai/DeepSeek-V3"
        | "deepseek-ai/DeepSeek-R1"
        | "deepseek-ai/DeepSeek-V3"
        | "deepseek-ai/DeepSeek-V2.5" => Some(65_536),
        "Qwen/Qwen3-235B-A22B"
        | "Qwen/Qwen3-32B"
        | "Qwen/Qwen3-14B"
        | "Qwen/Qwen3-8B"
        | "Qwen/Qwen3-30B-A3B"
        | "Qwen/Qwen2.5-72B-Instruct"
        | "Qwen/Qwen2.5-32B-Instruct"
        | "Qwen/Qwen2.5-14B-Instruct"
        | "Qwen/Qwen2.5-7B-Instruct"
        | "Qwen/QwQ-32B"
        | "Qwen/QwQ-32B-Preview" => Some(262_144),

        // ── DeepSeek ──
        "deepseek-v4-flash" | "deepseek-v4-pro" | "deepseek-chat" | "deepseek-reasoner"
        | "deepseek-r1" | "deepseek-v3" | "deepseek-v3.2" => Some(1_048_576),

        // ── GLM ──
        "glm-5" | "glm-4-plus" | "glm-4-flash" | "glm-4.7" | "glm-4" => Some(128_000),

        // ── MiniMax ──
        "minimax-m3" | "minimax-m1" | "minimax-s1" | "minimaxai/minimax-m2.7" => Some(1_000_000),

        // ── NVIDIA / Llama ──
        "meta/llama-3.1-405b-instruct"
        | "meta/llama-3.1-70b-instruct"
        | "nvidia/llama-3.1-nemotron-70b-instruct"
        | "nvidia/llama-3.3-nemotron-super-49b-v1" => Some(128_000),
        "zhipuai/glm-4.7" => Some(128_000),

        // ── Ollama 常见模型（基础名称，模糊匹配会去除 :tag） ──
        "llama3.3" | "llama3.2" | "llama3.1" | "llama3" => Some(128_000),
        "llama2" => Some(4_096),
        "mistral" | "mixtral" | "mistral-nemo" | "mistral-small" | "mistral-large" => Some(32_768),
        "codellama" => Some(16_384),
        "gemma3" | "gemma2" | "gemma" => Some(8_192),
        "phi4" | "phi3" | "phi" => Some(128_000),
        "phi3.5" => Some(128_000),
        "qwen3" | "qwen2.5" | "qwen2" | "qwen" => Some(32_768),
        "command-r" | "command-r-plus" => Some(128_000),
        "deepseek-coder" | "deepseek-coder-v2" => Some(65_536),
        "orca2" | "orca-mini" => Some(4_096),
        "nomic-embed-text" => Some(8_192),
        "mxbai-embed-large" => Some(512),
        "all-minilm" => Some(256),
        "tinyllama" => Some(2_048),
        "stablelm2" | "stablelm-zephyr" => Some(4_096),
        "yi" => Some(4_096),
        "falcon3" | "falcon2" | "falcon" => Some(8_192),
        "starcoder2" | "starcoder" => Some(16_384),
        "sqlcoder" => Some(16_384),
        "wizardlm2" | "wizardcoder" | "wizard-math" | "wizard-vicuna-uncensored" => Some(4_096),
        "openchat" => Some(8_192),
        "zephyr" => Some(32_768),
        "neural-chat" => Some(8_192),
        "dolphin-mixtral" | "dolphin3" | "dolphin-llama3" => Some(32_768),
        "aya" => Some(8_192),
        "reflection" => Some(128_000),
        "tulu3" => Some(32_768),
        "opencoder" => Some(16_384),
        "hermes3" => Some(128_000),
        "llava" | "llava-llama3" | "llava-phi3" | "bakllava" => Some(4_096),
        "minicpm-v" => Some(8_192),
        "moondream" => Some(4_096),
        "solar" => Some(4_096),
        "command-r7b" => Some(128_000),
        "granite3.2" | "granite3.1" | "granite3" | "granite-code" => Some(128_000),
        "nemotron-mini" | "nemotron" => Some(4_096),
        "smollm2" | "smollm" => Some(8_192),
        "openthinker" => Some(32_768),
        "deepscaler" => Some(16_384),

        _ => None,
    }
}
