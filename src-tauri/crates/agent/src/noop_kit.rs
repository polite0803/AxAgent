// SPDX-License-Identifier: AGPL-3.0-only

//! 默认空实现（noop）的 kit 桥接 trait，供 convenience 构造器使用。
//! 当 wiring 层未注入真实实现时，agent 以降级模式运行。

use axagent_harness::kit_bridge::{
    KitHtmlCleaner, KitMarkdownParser, KitSkillDirs, KitSlashCommandProcessor,
    KitTokenBudgetDecision, KitTokenBudgetTracker, MdParsedNote, SlashCommandAction,
};

// ── NoopTokenBudgetTracker ────────────────────────────────────

pub struct NoopTokenBudgetTracker;

impl KitTokenBudgetTracker for NoopTokenBudgetTracker {
    fn reset(&mut self) {}
    fn record_tokens(&mut self, _global_turn_tokens: u64) {}
    fn check(&mut self, _budget: Option<u64>, _global_turn_tokens: u64) -> KitTokenBudgetDecision {
        KitTokenBudgetDecision::Continue {
            nudge_message: String::new(),
            continuation_count: 0,
            pct_used: 0,
            turn_tokens: 0,
            budget: 0,
        }
    }
}

// ── NoopHtmlCleaner ───────────────────────────────────────────

pub struct NoopHtmlCleaner;

impl KitHtmlCleaner for NoopHtmlCleaner {
    fn extract_readability(&self, _html: &str) -> (String, String, Vec<String>) {
        Default::default()
    }
    fn detect_language(&self, _text: &str) -> &'static str {
        "unknown"
    }
}

// ── NoopSlashCommandProcessor ─────────────────────────────────

pub struct NoopSlashCommandProcessor;

impl KitSlashCommandProcessor for NoopSlashCommandProcessor {
    fn process(&self, _text: &str) -> Option<SlashCommandAction> {
        None
    }
    fn load_bundle_content(&self, _name: &str, _args: &str) -> Option<String> {
        None
    }
    fn load_skill_content(&self, _name: &str, _args: &str) -> Option<String> {
        None
    }
}

// ── NoopMarkdownParser ────────────────────────────────────────

pub struct NoopMarkdownParser;

impl KitMarkdownParser for NoopMarkdownParser {
    fn parse(&self, content: &str) -> MdParsedNote {
        MdParsedNote {
            frontmatter: Default::default(),
            content: content.to_string(),
            links: Vec::new(),
            raw_links: Vec::new(),
        }
    }
}

// ── NoopSkillDirs ─────────────────────────────────────────────

pub struct NoopSkillDirs;

impl KitSkillDirs for NoopSkillDirs {
    fn skill_dirs(&self) -> Vec<(String, std::path::PathBuf)> {
        Vec::new()
    }
    fn all_skills_dirs(&self) -> Vec<std::path::PathBuf> {
        Vec::new()
    }
}
