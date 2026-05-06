/// 返回已知模型的上下文窗口大小（最大输入 token 数）。
/// 无匹配时返回 None。
pub fn get_model_context_window(model_id: &str) -> Option<u32> {
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

        "o1" | "o1-mini" | "o3-mini" | "o3" | "o4-mini" => Some(200_000),

        "gpt-4.1" | "gpt-4.1-mini" | "gpt-4.1-nano" => Some(1_047_576),

        "gpt-3.5-turbo" | "gpt-3.5-turbo-0125" | "gpt-3.5-turbo-1106" => Some(16_385),

        // ── Anthropic ──
        "claude-opus-4-20250514"
        | "claude-sonnet-4-20250514"
        | "claude-3-5-sonnet-20241022"
        | "claude-3-5-haiku-20241022"
        | "claude-3-haiku-20240307"
        | "claude-3-opus-20240229"
        | "claude-3-sonnet-20240229"
        | "claude-4-sonnet" => Some(200_000),

        // ── Google Gemini ──
        "gemini-2.5-pro" | "gemini-2.5-flash" | "gemini-2.0-flash" | "gemini-1.5-pro"
        | "gemini-1.5-flash" => Some(1_048_576),

        // ── DeepSeek ──
        "deepseek-chat" | "deepseek-reasoner" => Some(65_536),
        "deepseek-r1" | "deepseek-v3" => Some(65_536),

        // ── xAI ──
        "grok-3" | "grok-3-mini" | "grok-2" => Some(131_072),

        // ── GLM ──
        "glm-4-plus" | "glm-4-flash" | "glm-4.7" | "glm-4" => Some(128_000),

        // ── MiniMax ──
        "minimax-m1" | "minimax-s1" | "minimaxai/minimax-m2.7" => Some(1_000_000),

        // ── NVIDIA / Llama ──
        "meta/llama-3.1-405b-instruct"
        | "meta/llama-3.1-70b-instruct"
        | "nvidia/llama-3.1-nemotron-70b-instruct"
        | "nvidia/llama-3.3-nemotron-super-49b-v1" => Some(128_000),
        "zhipuai/glm-4.7" => Some(128_000),

        // ── Ollama 常见模型 ──
        "llama3" | "llama3.1" | "llama3.2" | "llama3.3" | "llama3.3:latest" => Some(128_000),
        "mistral" | "mixtral" | "mixtral:8x7b" | "mistral-nemo" => Some(32_768),
        "codellama" | "codellama:7b" | "codellama:13b" | "codellama:34b" => Some(16_384),
        "gemma" | "gemma2" | "gemma:7b" | "gemma:2b" => Some(8_192),
        "phi" | "phi3" | "phi3:mini" | "phi3:small" | "phi3:medium" | "phi4" => Some(128_000),
        "qwen" | "qwen2" | "qwen2.5" | "qwen3" => Some(32_768),
        "command-r" | "command-r-plus" => Some(128_000),

        _ => None,
    }
}
