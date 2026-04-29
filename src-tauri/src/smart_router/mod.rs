//! Smart Model Router — task-aware model selection for cost-efficient LLM usage.
//!
//! This module implements the "宏观架构" (macro-architecture) layer of the
//! performance optimization plan. It classifies incoming prompts by task
//! complexity and routes them to the most cost-effective model tier.
//!
//! ## Classification Tiers
//!
//! - **trivial**: Format conversion, translation, summary → cheap model (haiku/flash)
//! - **moderate**: Q&A, code explanation, data analysis → mid model (sonnet/4o)
//! - **complex**: Architecture design, multi-step reasoning, debugging → premium (opus/o1)
//!
//! ## Future integration points
//!
//! - User routing preferences from `ModelRoutingConfigPanel.tsx`
//! - Semantic cache hit check (if embedding matches, return cached)
//! - Cost budget enforcement (downgrade tier if budget exceeded)

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ─── Route Decision ───

/// The router's output: which model to use + fallback chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    /// The recommended model tier.
    pub tier: ModelTier,
    /// Minimum token budget for this task type.
    pub min_tokens: u32,
    /// Whether this prompt is a good candidate for semantic caching.
    pub cacheable: bool,
    /// Suggested TTL for cache (seconds), if cacheable.
    pub cache_ttl_secs: Option<u64>,
    /// Brief classification explanation for debugging.
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// Budget models (haiku, flash, gpt-4o-mini, deepseek-chat)
    Budget,
    /// Balanced models (sonnet, gpt-4o, gemini-pro)
    Balanced,
    /// Premium models (opus, o1/o3, gpt-4)
    Premium,
}

impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTier::Budget => "budget",
            ModelTier::Balanced => "balanced",
            ModelTier::Premium => "premium",
        }
    }
}

// ─── Task Classification ───

/// Classify a user prompt and return a routing decision.
///
/// Uses fast, local heuristics (no LLM call required). The classifier
/// examines prompt length, keywords, structural patterns, and code presence.
pub fn classify_and_route(prompt: &str) -> RouteDecision {
    let lower = prompt.to_lowercase();
    let prompt_len = prompt.len();
    let line_count = prompt.lines().count();

    // ── Complex indicators ──
    // Multi-step reasoning, architecture design, system refactoring, debugging
    if is_complex_task(&lower, prompt_len, line_count) {
        return RouteDecision {
            tier: ModelTier::Premium,
            min_tokens: 4096,
            cacheable: false,
            cache_ttl_secs: None,
            reason: "complex task: multi-step reasoning, architecture, or debugging".into(),
        };
    }

    // ── Trivial indicators ──
    // Translation, summarization, format conversion, simple lookups
    if is_trivial_task(&lower, prompt_len) {
        return RouteDecision {
            tier: ModelTier::Budget,
            min_tokens: 512,
            cacheable: true,
            cache_ttl_secs: Some(3600), // 1 hour for simple tasks
            reason: "trivial task: translation, summary, or format conversion".into(),
        };
    }

    // ── Moderate (default) ──
    RouteDecision {
        tier: ModelTier::Balanced,
        min_tokens: 2048,
        cacheable: false,
        cache_ttl_secs: None,
        reason: "moderate task: general Q&A or code explanation".into(),
    }
}

// ─── Classification Helpers ───

fn is_complex_task(lower: &str, prompt_len: usize, line_count: usize) -> bool {
    // Long prompts with many lines suggest complexity
    if prompt_len > 2000 && line_count > 20 {
        return true;
    }

    // Explicit reasoning/design keywords
    let complex_keywords = [
        "architect", "design pattern", "refactor", "refactoring",
        "system design", "multi-step", "step by step reasoning",
        "debug", "troubleshoot", "root cause", "performance optimization",
        "security audit", "code review the entire", "migrate from",
        "implement a", "build a", "create a full", "scalable",
        "production-ready", "enterprise", "microservices",
        "distributed system", "concurrency", "race condition",
        "deadlock", "memory leak", "scale horizontally",
    ];

    for kw in &complex_keywords {
        if lower.contains(kw) {
            return true;
        }
    }

    // Code blocks with multiple languages or complex patterns
    let code_block_count = lower.matches("```").count() / 2;
    if code_block_count >= 3 {
        return true;
    }

    // SQL + explanation pattern
    if lower.contains("sql") && (lower.contains("explain") || lower.contains("optimize")) {
        return true;
    }

    false
}

fn is_trivial_task(lower: &str, prompt_len: usize) -> bool {
    // Very short prompts are usually simple
    if prompt_len < 50 {
        return true;
    }

    // Translation patterns
    let translation_patterns = [
        "translate", "翻译", "traduire", "übersetzen",
        "翻成", "译为", "翻訳",
    ];
    for pat in &translation_patterns {
        if lower.contains(pat) {
            return true;
        }
    }

    // Summarization patterns
    let summary_patterns = [
        "summarize", "summarise", "tldr", "tl;dr",
        "总结", "摘要", "概括", "简述",
        "in a few words", "brief summary", "key points",
    ];
    for pat in &summary_patterns {
        if lower.contains(pat) {
            return true;
        }
    }

    // Format conversion
    let format_patterns = [
        "convert to json", "convert to yaml", "convert to csv",
        "format as", "reformat", "pretty print",
        "json to", "yaml to", "csv to",
    ];
    for pat in &format_patterns {
        if lower.contains(pat) {
            return true;
        }
    }

    // Simple single-line commands
    if prompt_len < 100 && !lower.contains("explain") && !lower.contains("why") {
        let simple_patterns = [
            "what is", "who is", "when did", "where is",
            "how to", "list", "show me", "find",
            "是什么", "什么是", "怎么", "如何",
            "列出", "显示", "查找",
        ];
        for pat in &simple_patterns {
            if lower.starts_with(pat) || lower.contains(&format!(" {} ", pat)) {
                return true;
            }
        }
    }

    false
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trivial_translation() {
        let decision = classify_and_route("Translate 'hello world' to Chinese");
        assert_eq!(decision.tier, ModelTier::Budget);
        assert!(decision.cacheable);
    }

    #[test]
    fn test_trivial_summary() {
        let decision = classify_and_route("Summarize the key points of this article");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_trivial_short() {
        let decision = classify_and_route("What is Rust?");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_moderate_question() {
        let decision = classify_and_route("Explain how async/await works in JavaScript with examples");
        assert_eq!(decision.tier, ModelTier::Balanced);
    }

    #[test]
    fn test_complex_architecture() {
        let decision = classify_and_route(
            "Design a microservices architecture for an e-commerce platform with \
             user authentication, product catalog, payment processing, and order tracking. \
             Include database schema and API design."
        );
        assert_eq!(decision.tier, ModelTier::Premium);
    }

    #[test]
    fn test_complex_refactor() {
        let decision = classify_and_route("Refactor this monolithic codebase into a modular architecture");
        assert_eq!(decision.tier, ModelTier::Premium);
    }

    #[test]
    fn test_complex_long_prompt() {
        let mut long = String::from("I need help with a complex problem:\n");
        for i in 0..30 {
            long.push_str(&format!("Step {}: Do something complex here\n", i));
        }
        let decision = classify_and_route(&long);
        assert_eq!(decision.tier, ModelTier::Premium);
    }

    #[test]
    fn test_format_conversion() {
        let decision = classify_and_route("Convert this JSON to YAML format");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_chinese_translation() {
        let decision = classify_and_route("把这段英文翻译成中文");
        assert_eq!(decision.tier, ModelTier::Budget);
    }

    #[test]
    fn test_chinese_what_is() {
        let decision = classify_and_route("什么是 Rust 的所有权系统？");
        assert_eq!(decision.tier, ModelTier::Budget);
    }
}
