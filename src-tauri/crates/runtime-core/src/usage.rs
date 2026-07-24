// SPDX-License-Identifier: AGPL-3.0-only

//! 用量与成本聚合。
//!
//! `ModelPricing` / `pricing_for_model` / `UsageCostEstimate` /
//! `cost_for_tokens` / `format_usd` 的权威定义已下沉到
//! `axagent_harness::usage_pricing`（foundation 层），以便 gateway 等
//! consumer crate 在不依赖 runtime-core 的情况下也能换算成本。
//! 本 crate 通过 `pub use` 透传，保持现有引用路径不变。
//!
//! `TokenCost` trait（对 `TokenUsage` 的扩展）与 `UsageTracker`（运行时
//! 累积器）仍保留在本 crate，因为它们属于 consumer 层的运行时行为。

use crate::session::Session;
pub use axagent_harness::TokenUsage;
// 从 harness foundation 层透传定价相关类型（权威定义下沉后保持旧路径可用）
pub use axagent_harness::usage_pricing::{
    ModelPricing, UsageCostEstimate, cost_for_tokens, format_usd, pricing_for_model,
};

// ── TokenCost trait (extension pattern: define trait in runtime-core, impl for harness type) ──

/// Cost estimation and cache-rate analysis for [`TokenUsage`].
///
/// This trait lives in consumer territory (`axagent-runtime-core`) so that
/// pricing tables and formatting logic do not pollute the harness foundation.
pub trait TokenCost {
    /// 计算缓存命中率。
    ///
    /// # 算法
    /// 1. **优先** 使用提供商返回的真值（DeepSeek `prompt_cache_miss_tokens`）作为分母 miss。
    /// 2. **回退** 到 `input - cache_read - cache_creation` 推算分母 miss（OpenAI/Claude 等）。
    /// 3. 命中分母为 `cache_read_input_tokens`。
    /// 4. 当分母为 0（全部走 cache creation）→ 返回 `None`。
    /// 5. 当命中 = 0 但存在 miss token → 返回 `Some(0.0)`。
    #[must_use]
    fn cache_hit_rate(self) -> Option<f64>;

    #[must_use]
    fn estimate_cost_usd(self) -> UsageCostEstimate;

    #[must_use]
    fn estimate_cost_usd_with_pricing(self, pricing: ModelPricing) -> UsageCostEstimate;

    #[must_use]
    fn summary_lines(self, label: &str) -> Vec<String>;

    #[must_use]
    fn summary_lines_for_model(self, label: &str, model: Option<&str>) -> Vec<String>;
}

impl TokenCost for TokenUsage {
    fn cache_hit_rate(self) -> Option<f64> {
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

    fn estimate_cost_usd(self) -> UsageCostEstimate {
        self.estimate_cost_usd_with_pricing(ModelPricing::default_sonnet_tier())
    }

    fn estimate_cost_usd_with_pricing(self, pricing: ModelPricing) -> UsageCostEstimate {
        pricing.cost_for(self)
    }

    fn summary_lines(self, label: &str) -> Vec<String> {
        self.summary_lines_for_model(label, None)
    }

    fn summary_lines_for_model(self, label: &str, model: Option<&str>) -> Vec<String> {
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
            .map_or_else(String::new, |rate| format!(" hit_rate={:.1}%", rate * 100.0));
        let cache_miss_suffix = self
            .cache_miss_input_tokens
            .map_or_else(String::new, |miss| format!(" cache_miss={miss}"));
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
    use super::{TokenCost, TokenUsage, UsageTracker, format_usd, pricing_for_model};
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
        for alias in ["deepseek-v4-pro", "deepseek-reasoner", "deepseek-r1", "DeepSeek-Reasoner"] {
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
            blocks: vec![ContentBlock::Text { text: "done".to_string() }],
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
