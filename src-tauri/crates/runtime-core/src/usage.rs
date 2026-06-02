use crate::session::Session;

const DEFAULT_INPUT_COST_PER_MILLION: f64 = 3.0;
const DEFAULT_OUTPUT_COST_PER_MILLION: f64 = 15.0;
const DEFAULT_CACHE_CREATION_COST_PER_MILLION: f64 = 3.75;
const DEFAULT_CACHE_READ_COST_PER_MILLION: f64 = 0.3;

/// Per-million-token pricing used for cost estimation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub cache_creation_cost_per_million: f64,
    pub cache_read_cost_per_million: f64,
}

impl ModelPricing {
    #[must_use]
    pub const fn default_sonnet_tier() -> Self {
        Self {
            input_cost_per_million: DEFAULT_INPUT_COST_PER_MILLION,
            output_cost_per_million: DEFAULT_OUTPUT_COST_PER_MILLION,
            cache_creation_cost_per_million: DEFAULT_CACHE_CREATION_COST_PER_MILLION,
            cache_read_cost_per_million: DEFAULT_CACHE_READ_COST_PER_MILLION,
        }
    }
}

/// Token counters accumulated for a conversation turn or session.
///
/// # 字段语义
/// - `input_tokens`: provider 计费口径的 input（DeepSeek = `prompt_tokens`，
///   OpenAI o-series 同样 = `prompt_tokens`）。
/// - `output_tokens`: provider 计费口径的 output（含 `reasoning_content`，DeepSeek R1 全额计费）。
/// - `cache_creation_input_tokens`: 写入 cache 的 token（Anthropic / OpenAI o-series prompt caching）。
/// - `cache_read_input_tokens`: 命中 cache 的 token（DeepSeek 顶层 `prompt_cache_hit_tokens`，
///   OpenAI 嵌套 `prompt_tokens_details.cached_tokens`）。
/// - `cache_miss_input_tokens`: DeepSeek 顶层 `prompt_cache_miss_tokens` 真值（提供商直接返回，
///   不再依赖 `input - cache_read` 推算）。`None` 表示上游未提供（如 OpenAI/Claude），
///   此时 `cache_hit_rate` 退回到 `input - cache_creation` 推算分母。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    #[serde(default)]
    pub cache_miss_input_tokens: Option<u32>,
}

/// Estimated dollar cost derived from a [`TokenUsage`] sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsageCostEstimate {
    pub input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub cache_creation_cost_usd: f64,
    pub cache_read_cost_usd: f64,
}

impl UsageCostEstimate {
    #[must_use]
    pub fn total_cost_usd(self) -> f64 {
        self.input_cost_usd
            + self.output_cost_usd
            + self.cache_creation_cost_usd
            + self.cache_read_cost_usd
    }
}

/// Returns pricing metadata for a known model alias or family.
#[must_use]
pub fn pricing_for_model(model: &str) -> Option<ModelPricing> {
    let normalized = model.to_ascii_lowercase();

    // ── OpenAI GPT-5.x ──
    if normalized.contains("gpt-5.5") {
        return Some(ModelPricing {
            input_cost_per_million: 5.00,
            output_cost_per_million: 30.00,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.50,
        });
    }
    if normalized.contains("gpt-5.4-mini") || normalized.contains("gpt-5-mini") {
        return Some(ModelPricing {
            input_cost_per_million: 0.75,
            output_cost_per_million: 4.50,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.075,
        });
    }
    if normalized.contains("gpt-5.4")
        || normalized.contains("gpt-5.1")
        || normalized.contains("gpt-5.2")
        || normalized == "gpt-5"
    {
        return Some(ModelPricing {
            input_cost_per_million: 2.50,
            output_cost_per_million: 15.00,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.25,
        });
    }

    // ── OpenAI GPT-4.1 (legacy) ──
    if normalized.contains("gpt-4.1-nano") {
        return Some(ModelPricing {
            input_cost_per_million: 0.10,
            output_cost_per_million: 0.40,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.025,
        });
    }
    if normalized.contains("gpt-4.1-mini") {
        return Some(ModelPricing {
            input_cost_per_million: 0.40,
            output_cost_per_million: 1.60,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.10,
        });
    }
    if normalized.contains("gpt-4.1") {
        return Some(ModelPricing {
            input_cost_per_million: 2.00,
            output_cost_per_million: 8.00,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.50,
        });
    }

    // ── OpenAI o-series reasoning ──
    if normalized.contains("o4-mini") {
        return Some(ModelPricing {
            input_cost_per_million: 1.10,
            output_cost_per_million: 4.40,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.275,
        });
    }
    if normalized.contains("o3-mini") {
        return Some(ModelPricing {
            input_cost_per_million: 1.10,
            output_cost_per_million: 4.40,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.275,
        });
    }
    if normalized == "o3" {
        return Some(ModelPricing {
            input_cost_per_million: 2.00,
            output_cost_per_million: 8.00,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.50,
        });
    }

    // ── Anthropic Claude ──
    if normalized.contains("haiku") {
        return Some(ModelPricing {
            input_cost_per_million: 1.0,
            output_cost_per_million: 5.0,
            cache_creation_cost_per_million: 1.25,
            cache_read_cost_per_million: 0.1,
        });
    }
    if normalized.contains("opus") {
        return Some(ModelPricing {
            input_cost_per_million: 5.0,
            output_cost_per_million: 25.0,
            cache_creation_cost_per_million: 6.25,
            cache_read_cost_per_million: 0.5,
        });
    }
    if normalized.contains("sonnet") {
        return Some(ModelPricing::default_sonnet_tier());
    }

    // ── Qwen (通义千问) ──
    // qwen3.7-max: ¥12/¥36 per 1M tokens ≈ $1.66/$4.98; cache_read = input×20% ≈ $0.332
    if normalized.contains("qwen3.7-max") {
        return Some(ModelPricing {
            input_cost_per_million: 1.66,
            output_cost_per_million: 4.98,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.332,
        });
    }
    // qwen3.6-plus: ¥2/¥6 per 1M tokens ≈ $0.28/$0.83; cache_read = input×20% ≈ $0.056
    if normalized.contains("qwen3.6-plus") {
        return Some(ModelPricing {
            input_cost_per_million: 0.28,
            output_cost_per_million: 0.83,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.056,
        });
    }
    // qwen3.6-flash: ¥0.3/¥0.6 per 1M tokens ≈ $0.04/$0.08; cache_read = input×20% ≈ $0.008
    if normalized.contains("qwen3.6-flash") || normalized.contains("qwen3.5-flash") {
        return Some(ModelPricing {
            input_cost_per_million: 0.04,
            output_cost_per_million: 0.08,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.008,
        });
    }
    // qwen3.5-plus: ¥0.8/¥2 per 1M tokens ≈ $0.11/$0.28; cache_read = input×20% ≈ $0.022
    if normalized.contains("qwen3.5-plus") {
        return Some(ModelPricing {
            input_cost_per_million: 0.11,
            output_cost_per_million: 0.28,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.022,
        });
    }
    // qwen3-max / qwen-plus: ¥2/¥6 per 1M tokens ≈ $0.28/$0.83; cache_read = input×20% ≈ $0.056
    if normalized.contains("qwen3-max")
        || normalized.contains("qwen-plus")
        || normalized.contains("qwen-max")
    {
        return Some(ModelPricing {
            input_cost_per_million: 0.28,
            output_cost_per_million: 0.83,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.056,
        });
    }
    // qwen-turbo / qwen-flash: ¥0.3/¥0.6 per 1M tokens ≈ $0.04/$0.08; cache_read = input×20% ≈ $0.008
    if normalized.contains("qwen-turbo") || normalized.contains("qwen-flash") {
        return Some(ModelPricing {
            input_cost_per_million: 0.04,
            output_cost_per_million: 0.08,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.008,
        });
    }

    // ── Kimi (月之暗面) ──
    // kimi-k2.6: ¥6.5/¥27 per 1M tokens ≈ $0.90/$3.73; cache hit ¥1.10 ≈ $0.15
    if normalized.contains("kimi-k2.6") {
        return Some(ModelPricing {
            input_cost_per_million: 0.90,
            output_cost_per_million: 3.73,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.15,
        });
    }
    // kimi-k2.5: ¥4/¥21 per 1M tokens ≈ $0.55/$2.90; cache hit ¥0.70 ≈ $0.10
    if normalized.contains("kimi-k2.5") {
        return Some(ModelPricing {
            input_cost_per_million: 0.55,
            output_cost_per_million: 2.90,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.10,
        });
    }
    // kimi-k2: ¥4/¥16 per 1M tokens ≈ $0.55/$2.21
    if normalized.contains("kimi-k2")
        && !normalized.contains("k2.5")
        && !normalized.contains("k2.6")
    {
        return Some(ModelPricing {
            input_cost_per_million: 0.55,
            output_cost_per_million: 2.21,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.10,
        });
    }
    // moonshot-v1: ¥12/¥12 per 1M tokens ≈ $1.66/$1.66
    if normalized.contains("moonshot-v1") {
        return Some(ModelPricing {
            input_cost_per_million: 1.66,
            output_cost_per_million: 1.66,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }

    // ── Doubao (豆包) ──
    // doubao-1.5-pro: ¥4/¥16 per 1M tokens ≈ $0.55/$2.21
    if normalized.contains("doubao-1.5-pro") || normalized.contains("doubao-pro") {
        return Some(ModelPricing {
            input_cost_per_million: 0.55,
            output_cost_per_million: 2.21,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }
    // doubao-1.5-lite: ¥0.3/¥0.6 per 1M tokens ≈ $0.04/$0.08
    if normalized.contains("doubao-1.5-lite") || normalized.contains("doubao-lite") {
        return Some(ModelPricing {
            input_cost_per_million: 0.04,
            output_cost_per_million: 0.08,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }

    // ── SiliconFlow (硅基流动) ──
    // Pro/DeepSeek-R1: ¥4/¥16 per 1M tokens ≈ $0.56/$2.22
    if normalized.contains("deepseek-ai/deepseek-r1")
        || normalized.contains("deepseek-ai/deepseek-r1-0120")
    {
        return Some(ModelPricing {
            input_cost_per_million: 0.56,
            output_cost_per_million: 2.22,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }
    // Pro/DeepSeek-V3: ¥2/¥8 per 1M tokens ≈ $0.28/$1.11
    if normalized.contains("deepseek-ai/deepseek-v3") {
        return Some(ModelPricing {
            input_cost_per_million: 0.28,
            output_cost_per_million: 1.11,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }
    // Qwen3-235B-A22B: ¥2.5/¥10 per 1M tokens ≈ $0.35/$1.39
    if normalized.contains("qwen3-235b") {
        return Some(ModelPricing {
            input_cost_per_million: 0.35,
            output_cost_per_million: 1.39,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }
    // Qwen3-32B: ¥1/¥4 per 1M tokens ≈ $0.14/$0.56
    if normalized.contains("qwen3-32b") {
        return Some(ModelPricing {
            input_cost_per_million: 0.14,
            output_cost_per_million: 0.56,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }
    // Qwen3-14B: ¥0.5/¥2 per 1M tokens ≈ $0.07/$0.28
    if normalized.contains("qwen3-14b") {
        return Some(ModelPricing {
            input_cost_per_million: 0.07,
            output_cost_per_million: 0.28,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }
    // Qwen2.5-72B: ¥4.13/¥4.13 per 1M tokens ≈ $0.57/$0.57
    if normalized.contains("qwen2.5-72b") {
        return Some(ModelPricing {
            input_cost_per_million: 0.57,
            output_cost_per_million: 0.57,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }
    // QwQ-32B: ¥1/¥4 per 1M tokens ≈ $0.14/$0.56
    if normalized.contains("qwq-32b") {
        return Some(ModelPricing {
            input_cost_per_million: 0.14,
            output_cost_per_million: 0.56,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0,
        });
    }

    // ── DeepSeek V4 Flash (1M context, free-tier) ──
    if normalized.contains("deepseek") && normalized.contains("v4-flash") {
        return Some(ModelPricing {
            input_cost_per_million: 0.14,
            output_cost_per_million: 0.28,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0028,
        });
    }
    // ── DeepSeek V4 Pro (1M context, 75% off permanent) ──
    if normalized.contains("deepseek") && normalized.contains("v4-pro") {
        return Some(ModelPricing {
            input_cost_per_million: 0.435,
            output_cost_per_million: 0.87,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.003625,
        });
    }
    // DeepSeek legacy aliases: deepseek-chat → V4 Flash, deepseek-reasoner → V4 Pro
    if normalized.contains("deepseek") && normalized.contains("chat") {
        return Some(ModelPricing {
            input_cost_per_million: 0.14,
            output_cost_per_million: 0.28,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0028,
        });
    }
    if normalized.contains("deepseek")
        && (normalized.contains("reasoner") || normalized.contains("r1"))
    {
        return Some(ModelPricing {
            input_cost_per_million: 0.435,
            output_cost_per_million: 0.87,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.003625,
        });
    }
    // DeepSeek V3 legacy (same as V4 Flash pricing)
    if normalized.contains("deepseek") && normalized.contains("v3") {
        return Some(ModelPricing {
            input_cost_per_million: 0.14,
            output_cost_per_million: 0.28,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0028,
        });
    }
    // DeepSeek Coder (legacy, same as V4 Flash pricing)
    if normalized.contains("deepseek") && normalized.contains("coder") {
        return Some(ModelPricing {
            input_cost_per_million: 0.14,
            output_cost_per_million: 0.28,
            cache_creation_cost_per_million: 0.0,
            cache_read_cost_per_million: 0.0028,
        });
    }
    None
}

impl TokenUsage {
    #[must_use]
    pub fn total_tokens(self) -> u32 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }

    /// 计算缓存命中率。
    ///
    /// # 算法
    /// 1. **优先** 使用提供商返回的真值（DeepSeek `prompt_cache_miss_tokens`）作为分母 miss。
    /// 2. **回退** 到 `input - cache_read - cache_creation` 推算分母 miss（OpenAI/Claude 等）。
    /// 3. 命中分母为 `cache_read_input_tokens`。
    /// 4. 当分母为 0（全部走 cache creation）→ 返回 `None`。
    /// 5. 当命中 = 0 但存在 miss token → 返回 `Some(0.0)`。
    #[must_use]
    pub fn cache_hit_rate(self) -> Option<f64> {
        // P0-2: 优先使用 provider 报告的真值 miss 计数
        let cache_miss = match self.cache_miss_input_tokens {
            Some(miss) => miss,
            None => self
                .input_tokens
                .saturating_sub(self.cache_read_input_tokens)
                .saturating_sub(self.cache_creation_input_tokens),
        };
        let denominator = self.cache_read_input_tokens.saturating_add(cache_miss);
        if denominator == 0 {
            return None;
        }
        Some(f64::from(self.cache_read_input_tokens) / f64::from(denominator))
    }

    #[must_use]
    pub fn estimate_cost_usd(self) -> UsageCostEstimate {
        self.estimate_cost_usd_with_pricing(ModelPricing::default_sonnet_tier())
    }

    #[must_use]
    pub fn estimate_cost_usd_with_pricing(self, pricing: ModelPricing) -> UsageCostEstimate {
        UsageCostEstimate {
            input_cost_usd: cost_for_tokens(self.input_tokens, pricing.input_cost_per_million),
            output_cost_usd: cost_for_tokens(self.output_tokens, pricing.output_cost_per_million),
            cache_creation_cost_usd: cost_for_tokens(
                self.cache_creation_input_tokens,
                pricing.cache_creation_cost_per_million,
            ),
            cache_read_cost_usd: cost_for_tokens(
                self.cache_read_input_tokens,
                pricing.cache_read_cost_per_million,
            ),
        }
    }

    #[must_use]
    pub fn summary_lines(self, label: &str) -> Vec<String> {
        self.summary_lines_for_model(label, None)
    }

    #[must_use]
    pub fn summary_lines_for_model(self, label: &str, model: Option<&str>) -> Vec<String> {
        let pricing = model.and_then(pricing_for_model);
        let cost = pricing.map_or_else(
            || self.estimate_cost_usd(),
            |pricing| self.estimate_cost_usd_with_pricing(pricing),
        );
        let model_suffix =
            model.map_or_else(String::new, |model_name| format!(" model={model_name}"));
        // P1-5: 区分三种 fallback 文案
        // - "unknown-model"：调用方传了 model 但 pricing_for_model 返 None（未知型号）
        // - "estimated-default"：调用方未传 model，用 sonnet 兜底
        // - "sonnet-default"：调用方未传 model 且我们明确告知是 sonnet 兜底
        let pricing_suffix = if pricing.is_some() {
            ""
        } else if model.is_some() {
            " pricing=unknown-model"
        } else {
            " pricing=sonnet-default"
        };
        let hit_rate_suffix = self
            .cache_hit_rate()
            .map_or(String::new(), |rate| format!(" hit_rate={:.1}%", rate * 100.0));
        let cache_miss_suffix = self
            .cache_miss_input_tokens
            .map_or(String::new(), |miss| format!(" cache_miss={miss}"));
        vec![
            format!(
                "{label}: total_tokens={} input={} output={} cache_write={} cache_read={}{} estimated_cost={}{}{}{}",
                self.total_tokens(),
                self.input_tokens,
                self.output_tokens,
                self.cache_creation_input_tokens,
                self.cache_read_input_tokens,
                cache_miss_suffix,
                format_usd(cost.total_cost_usd()),
                model_suffix,
                pricing_suffix,
                hit_rate_suffix,
            ),
            format!(
                "  cost breakdown: input={} output={} cache_write={} cache_read={}",
                format_usd(cost.input_cost_usd),
                format_usd(cost.output_cost_usd),
                format_usd(cost.cache_creation_cost_usd),
                format_usd(cost.cache_read_cost_usd),
            ),
        ]
    }
}

fn cost_for_tokens(tokens: u32, usd_per_million_tokens: f64) -> f64 {
    f64::from(tokens) / 1_000_000.0 * usd_per_million_tokens
}

#[must_use]
/// Formats a dollar-denominated value for CLI display.
pub fn format_usd(amount: f64) -> String {
    format!("${amount:.4}")
}

/// Aggregates token usage across a running session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageTracker {
    latest_turn: TokenUsage,
    cumulative: TokenUsage,
    turns: u32,
}

impl UsageTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_session(session: &Session) -> Self {
        let mut tracker = Self::new();
        for message in &session.messages {
            if let Some(usage) = message.usage {
                tracker.record(usage);
            }
        }
        tracker
    }

    pub fn record(&mut self, usage: TokenUsage) {
        self.latest_turn = usage;
        self.cumulative.input_tokens += usage.input_tokens;
        self.cumulative.output_tokens += usage.output_tokens;
        self.cumulative.cache_creation_input_tokens += usage.cache_creation_input_tokens;
        self.cumulative.cache_read_input_tokens += usage.cache_read_input_tokens;
        // P0-2: miss 是真值（Option），需要逐项取 Some(_) 累加并保持 Some
        self.cumulative.cache_miss_input_tokens =
            match (self.cumulative.cache_miss_input_tokens, usage.cache_miss_input_tokens) {
                (Some(acc), Some(delta)) => Some(acc + delta),
                (None, Some(delta)) => Some(delta),
                (Some(acc), None) => Some(acc),
                (None, None) => None,
            };
        self.turns += 1;
    }

    #[must_use]
    pub fn current_turn_usage(&self) -> TokenUsage {
        self.latest_turn
    }

    #[must_use]
    pub fn cumulative_usage(&self) -> TokenUsage {
        self.cumulative
    }

    #[must_use]
    pub fn turns(&self) -> u32 {
        self.turns
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenUsage, UsageTracker, format_usd, pricing_for_model};
    use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};

    #[test]
    fn tracks_true_cumulative_usage() {
        let mut tracker = UsageTracker::new();
        tracker.record(TokenUsage {
            input_tokens: 10,
            output_tokens: 4,
            cache_creation_input_tokens: 2,
            cache_read_input_tokens: 1,
            cache_miss_input_tokens: None,
        });
        tracker.record(TokenUsage {
            input_tokens: 20,
            output_tokens: 6,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 2,
            cache_miss_input_tokens: None,
        });

        assert_eq!(tracker.turns(), 2);
        assert_eq!(tracker.current_turn_usage().input_tokens, 20);
        assert_eq!(tracker.current_turn_usage().output_tokens, 6);
        assert_eq!(tracker.cumulative_usage().output_tokens, 10);
        assert_eq!(tracker.cumulative_usage().input_tokens, 30);
        assert_eq!(tracker.cumulative_usage().total_tokens(), 48);
    }

    #[test]
    fn computes_cost_summary_lines() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_input_tokens: 100_000,
            cache_read_input_tokens: 200_000,
            cache_miss_input_tokens: None,
        };

        let cost = usage.estimate_cost_usd();
        assert_eq!(format_usd(cost.input_cost_usd), "$3.0000");
        assert_eq!(format_usd(cost.output_cost_usd), "$7.5000");
        let lines = usage.summary_lines_for_model("usage", Some("claude-sonnet-4-6"));
        assert!(lines[0].contains("estimated_cost=$10.9350"));
        assert!(lines[0].contains("model=claude-sonnet-4-6"));
        assert!(lines[1].contains("cache_read=$0.0600"));
    }

    #[test]
    fn supports_model_specific_pricing() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        };

        let haiku = pricing_for_model("claude-haiku-4-5").expect("haiku pricing");
        let opus = pricing_for_model("claude-opus-4-8").expect("opus pricing");
        let haiku_cost = usage.estimate_cost_usd_with_pricing(haiku);
        let opus_cost = usage.estimate_cost_usd_with_pricing(opus);
        assert_eq!(format_usd(haiku_cost.total_cost_usd()), "$3.5000");
        assert_eq!(format_usd(opus_cost.total_cost_usd()), "$17.5000");
    }

    #[test]
    fn marks_unknown_model_pricing_as_unknown() {
        // P1-5: 调用方传了 model 但 pricing_for_model 返 None → pricing=unknown-model
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 100,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        };
        let lines = usage.summary_lines_for_model("usage", Some("custom-model"));
        assert!(lines[0].contains("pricing=unknown-model"), "got: {}", lines[0]);
    }

    #[test]
    fn marks_no_model_pricing_as_sonnet_default() {
        // P1-5: 调用方没传 model → pricing=sonnet-default
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 100,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        };
        let lines = usage.summary_lines("usage");
        assert!(lines[0].contains("pricing=sonnet-default"), "got: {}", lines[0]);
    }

    #[test]
    fn computes_cache_hit_rate() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            cache_creation_input_tokens: 100_000,
            cache_read_input_tokens: 200_000,
            cache_miss_input_tokens: None,
        };
        // miss = 1_000_000 - 200_000 - 100_000 = 700_000
        // hit_rate = 200_000 / (200_000 + 700_000) = 0.2222...
        let rate = usage.cache_hit_rate().expect("hit rate");
        assert!((rate - (200_000.0 / 900_000.0)).abs() < 1e-9);
    }

    #[test]
    fn hit_rate_uses_real_miss_when_provided() {
        // P0-2: provider 返回了真值 miss，应该优先使用而非推算
        // input=100, cache_creation=0, cache_read=30
        // 推算 miss = 70; 但 provider 报告 miss=85（不一致场景，仍用 provider 真值）
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 30,
            cache_miss_input_tokens: Some(85),
        };
        // hit_rate = 30 / (30 + 85) = 0.2608...
        let rate = usage.cache_hit_rate().expect("hit rate");
        assert!((rate - (30.0 / 115.0)).abs() < 1e-9, "got: {rate}");
    }

    #[test]
    fn hit_rate_is_none_when_only_cache_writes() {
        // 全部走 cache creation，没有 miss 也没有 hit → 无法定义命中率
        let usage = TokenUsage {
            input_tokens: 500,
            output_tokens: 0,
            cache_creation_input_tokens: 500,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        };
        assert!(usage.cache_hit_rate().is_none());
    }

    #[test]
    fn hit_rate_is_zero_when_no_cache_reads() {
        let usage = TokenUsage {
            input_tokens: 1_000,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: None,
        };
        // miss = 1000, hit = 0 → 0.0
        assert_eq!(usage.cache_hit_rate(), Some(0.0));
    }

    #[test]
    fn deepseek_v3_pricing_is_well_known() {
        for alias in ["deepseek-v4-flash", "deepseek-chat", "DeepSeek-V4-Flash"] {
            let pricing =
                pricing_for_model(alias).unwrap_or_else(|| panic!("v4-flash pricing for {alias}"));
            assert!((pricing.input_cost_per_million - 0.14).abs() < 1e-9);
            assert!((pricing.output_cost_per_million - 0.28).abs() < 1e-9);
            assert!((pricing.cache_read_cost_per_million - 0.0028).abs() < 1e-9);
        }
    }

    #[test]
    fn deepseek_r1_pricing_is_well_known() {
        for alias in [
            "deepseek-v4-pro",
            "deepseek-reasoner",
            "deepseek-r1",
            "DeepSeek-Reasoner",
        ] {
            let pricing =
                pricing_for_model(alias).unwrap_or_else(|| panic!("v4-pro pricing for {alias}"));
            assert!((pricing.input_cost_per_million - 0.435).abs() < 1e-9);
            assert!((pricing.output_cost_per_million - 0.87).abs() < 1e-9);
            assert!((pricing.cache_read_cost_per_million - 0.003625).abs() < 1e-9);
        }
    }

    #[test]
    fn openai_gpt41_pricing_is_well_known() {
        let nano = pricing_for_model("gpt-4.1-nano").expect("nano pricing");
        assert!((nano.input_cost_per_million - 0.10).abs() < 1e-9);
        assert!((nano.output_cost_per_million - 0.40).abs() < 1e-9);

        let mini = pricing_for_model("gpt-4.1-mini").expect("mini pricing");
        assert!((mini.input_cost_per_million - 0.40).abs() < 1e-9);
        assert!((mini.output_cost_per_million - 1.60).abs() < 1e-9);

        let base = pricing_for_model("gpt-4.1").expect("gpt-4.1 pricing");
        assert!((base.input_cost_per_million - 2.00).abs() < 1e-9);
        assert!((base.output_cost_per_million - 8.00).abs() < 1e-9);
    }

    #[test]
    fn openai_gpt5_pricing_is_well_known() {
        let gpt55 = pricing_for_model("gpt-5.5").expect("gpt-5.5 pricing");
        assert!((gpt55.input_cost_per_million - 5.00).abs() < 1e-9);
        assert!((gpt55.output_cost_per_million - 30.00).abs() < 1e-9);
        assert!((gpt55.cache_read_cost_per_million - 0.50).abs() < 1e-9);

        let gpt54 = pricing_for_model("gpt-5.4").expect("gpt-5.4 pricing");
        assert!((gpt54.input_cost_per_million - 2.50).abs() < 1e-9);
        assert!((gpt54.output_cost_per_million - 15.00).abs() < 1e-9);
        assert!((gpt54.cache_read_cost_per_million - 0.25).abs() < 1e-9);

        let mini = pricing_for_model("gpt-5.4-mini").expect("gpt-5.4-mini pricing");
        assert!((mini.input_cost_per_million - 0.75).abs() < 1e-9);
        assert!((mini.output_cost_per_million - 4.50).abs() < 1e-9);
        assert!((mini.cache_read_cost_per_million - 0.075).abs() < 1e-9);
    }

    #[test]
    fn openai_o_series_pricing_is_well_known() {
        let o3 = pricing_for_model("o3").expect("o3 pricing");
        assert!((o3.input_cost_per_million - 2.00).abs() < 1e-9);
        assert!((o3.output_cost_per_million - 8.00).abs() < 1e-9);

        let o4 = pricing_for_model("o4-mini").expect("o4-mini pricing");
        assert!((o4.input_cost_per_million - 1.10).abs() < 1e-9);
        assert!((o4.output_cost_per_million - 4.40).abs() < 1e-9);
    }

    #[test]
    fn summary_lines_includes_hit_rate() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 200_000,
            cache_miss_input_tokens: None,
        };
        // hit_rate = 200_000 / (200_000 + 800_000) = 0.2 → 20.0%
        let lines = usage.summary_lines_for_model("usage", Some("deepseek-chat"));
        assert!(lines[0].contains("hit_rate=20.0%"), "got: {}", lines[0]);
    }

    #[test]
    fn reconstructs_usage_from_session_messages() {
        let mut session = Session::new();
        session.messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text {
                text: "done".to_string(),
            }],
            usage: Some(TokenUsage {
                input_tokens: 5,
                output_tokens: 2,
                cache_creation_input_tokens: 1,
                cache_read_input_tokens: 0,
                cache_miss_input_tokens: None,
            }),
        }];

        let tracker = UsageTracker::from_session(&session);
        assert_eq!(tracker.turns(), 1);
        assert_eq!(tracker.cumulative_usage().total_tokens(), 8);
    }
}
