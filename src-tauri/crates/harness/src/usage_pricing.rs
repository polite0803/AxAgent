// SPDX-License-Identifier: AGPL-3.0-only

//! 模型定价表与成本估算（foundation 层权威定义）。
//!
//! 本模块原位于 `axagent-runtime-core::usage`，因 gateway（consumer）
//! 需要在 `record_usage` 时换算成本，而 consumer 之间不能互相依赖，
//! 故将 `ModelPricing` / `pricing_for_model` / `UsageCostEstimate`
//! 下沉到 harness foundation 层。`runtime-core` 通过 `pub use` 引用。
//!
//! 设计原则：本模块只包含纯数据结构 + 无副作用的查询/计算函数，
//! 不引入任何运行时行为或外部依赖，符合 harness "零业务逻辑" 约束。

use crate::conversation_model::TokenUsage;

const DEFAULT_INPUT_COST_PER_MILLION: f64 = 3.0;
const DEFAULT_OUTPUT_COST_PER_MILLION: f64 = 15.0;
const DEFAULT_CACHE_CREATION_COST_PER_MILLION: f64 = 3.75;
const DEFAULT_CACHE_READ_COST_PER_MILLION: f64 = 0.3;

/// 每百万 token 的单价（美元），用于成本估算。
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

    /// 根据一份 [`TokenUsage`] 样本计算美元成本明细。
    ///
    /// 公式：`cost = tokens / 1_000_000 * usd_per_million`。
    /// 该方法不读取任何外部状态，可安全在 gateway 热路径调用。
    #[must_use]
    pub fn cost_for(self, usage: TokenUsage) -> UsageCostEstimate {
        UsageCostEstimate {
            input_cost_usd: cost_for_tokens(usage.input_tokens, self.input_cost_per_million),
            output_cost_usd: cost_for_tokens(usage.output_tokens, self.output_cost_per_million),
            cache_creation_cost_usd: cost_for_tokens(
                usage.cache_creation_input_tokens,
                self.cache_creation_cost_per_million,
            ),
            cache_read_cost_usd: cost_for_tokens(
                usage.cache_read_input_tokens,
                self.cache_read_cost_per_million,
            ),
        }
    }
}

/// 由 [`TokenUsage`] 样本推导出的美元成本明细。
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

/// 按已知模型别名或家族返回定价元数据。
///
/// 匹配规则：大小写不敏感的子串匹配，按 OpenAI / Anthropic / Qwen /
/// Kimi / Doubao / SiliconFlow / DeepSeek 的顺序尝试命中。
/// 未知模型返回 `None`，调用方应回退到 [`ModelPricing::default_sonnet_tier`]。
#[must_use]
#[allow(clippy::too_many_lines)]
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

/// 按 token 数量 × 每百万单价计算美元成本。
///
/// `tokens / 1_000_000 * usd_per_million_tokens`。
/// 设为 `pub` 是因为 `runtime-core::TokenCost` trait 实现需要复用该公式。
#[must_use]
pub fn cost_for_tokens(tokens: u32, usd_per_million_tokens: f64) -> f64 {
    f64::from(tokens) / 1_000_000.0 * usd_per_million_tokens
}

/// 格式化美元金额用于日志/展示（保留 4 位有效数字）。
#[must_use]
pub fn format_usd(amount: f64) -> String {
    format!("${amount:.4}")
}

#[cfg(test)]
mod tests {
    use super::{ModelPricing, TokenUsage, cost_for_tokens, format_usd, pricing_for_model};

    #[test]
    fn default_sonnet_tier_constants() {
        let p = ModelPricing::default_sonnet_tier();
        assert!((p.input_cost_per_million - 3.0).abs() < 1e-9);
        assert!((p.output_cost_per_million - 15.0).abs() < 1e-9);
        assert!((p.cache_creation_cost_per_million - 3.75).abs() < 1e-9);
        assert!((p.cache_read_cost_per_million - 0.3).abs() < 1e-9);
    }

    #[test]
    fn cost_for_matches_formula() {
        // 1M tokens × $3/M = $3
        assert!((cost_for_tokens(1_000_000, 3.0) - 3.0).abs() < 1e-9);
        // 0 tokens → $0
        assert!((cost_for_tokens(0, 3.0)).abs() < 1e-9);
    }

    #[test]
    fn cost_for_aggregates_all_categories() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_input_tokens: 100_000,
            cache_read_input_tokens: 200_000,
            cache_miss_input_tokens: None,
        };
        let cost = ModelPricing::default_sonnet_tier().cost_for(usage);
        // input 1M * 3.0 = 3.0
        assert!((cost.input_cost_usd - 3.0).abs() < 1e-9);
        // output 500K * 15.0 = 7.5
        assert!((cost.output_cost_usd - 7.5).abs() < 1e-9);
        // cache_creation 100K * 3.75 = 0.375
        assert!((cost.cache_creation_cost_usd - 0.375).abs() < 1e-9);
        // cache_read 200K * 0.3 = 0.06
        assert!((cost.cache_read_cost_usd - 0.06).abs() < 1e-9);
        // total
        assert!((cost.total_cost_usd() - 10.935).abs() < 1e-9);
    }

    #[test]
    fn pricing_for_known_and_unknown_models() {
        assert!(pricing_for_model("claude-sonnet-4-6").is_some());
        assert!(pricing_for_model("gpt-5.5").is_some());
        assert!(pricing_for_model("deepseek-chat").is_some());
        assert!(pricing_for_model("totally-unknown-xyz").is_none());
    }

    #[test]
    fn format_usd_keeps_four_significant_digits() {
        assert_eq!(format_usd(0.0), "$0.0000");
        assert_eq!(format_usd(3.0), "$3.0000");
    }
}
