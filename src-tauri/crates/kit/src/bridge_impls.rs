// SPDX-License-Identifier: AGPL-3.0-only

//! Harness bridge trait implementations for kit service structs.

use axagent_harness::kit_bridge::{
    KitHtmlCleaner, KitMarkdownParser, KitSkillDirs, KitSlashCommandProcessor,
    KitTokenBudgetDecision, KitTokenBudgetTracker, MdParsedFrontmatter, MdParsedLink, MdParsedNote,
    SlashCommandAction,
};

// ── MarkdownParser ────────────────────────────────────────────

impl KitMarkdownParser for crate::markdown_parser::MarkdownParser {
    fn parse(&self, content: &str) -> MdParsedNote {
        let p = self.parse(content);
        MdParsedNote {
            frontmatter: MdParsedFrontmatter {
                title: p.frontmatter.title,
                author: p.frontmatter.author,
                tags: p.frontmatter.tags,
                created: p.frontmatter.created,
                source: p.frontmatter.source,
                page_type: p.frontmatter.page_type,
                custom: p.frontmatter.custom,
            },
            content: p.content,
            links: p
                .links
                .into_iter()
                .map(|l| MdParsedLink {
                    target: l.target,
                    display_text: l.display_text,
                    link_type: l.link_type,
                })
                .collect(),
            raw_links: p.raw_links,
        }
    }
}

// ── HtmlCleaner ───────────────────────────────────────────────

impl KitHtmlCleaner for crate::html_cleaner::HtmlCleaner {
    fn extract_readability(&self, html: &str) -> (String, String, Vec<String>) {
        self.extract_readability(html)
    }

    fn detect_language(&self, text: &str) -> &'static str {
        Self::detect_language(text)
    }
}

// ── TokenBudgetTracker ────────────────────────────────────────

impl KitTokenBudgetTracker for crate::token_budget::TokenBudgetTracker {
    fn reset(&mut self) {
        self.reset();
    }
    fn record_tokens(&mut self, global_turn_tokens: u64) {
        self.record_tokens(global_turn_tokens);
    }
    fn check(&mut self, budget: Option<u64>, global_turn_tokens: u64) -> KitTokenBudgetDecision {
        match self.check(budget, global_turn_tokens) {
            crate::token_budget::TokenBudgetDecision::Continue {
                nudge_message,
                continuation_count,
                pct_used,
                turn_tokens,
                budget,
            } => KitTokenBudgetDecision::Continue {
                nudge_message,
                continuation_count,
                pct_used,
                turn_tokens,
                budget,
            },
            crate::token_budget::TokenBudgetDecision::Compact {
                nudge_message,
                preserve_recent_steps,
                pct_used,
                budget,
            } => KitTokenBudgetDecision::Compact {
                nudge_message,
                preserve_recent_steps,
                pct_used,
                budget,
            },
            crate::token_budget::TokenBudgetDecision::Stop { completion_event } => {
                KitTokenBudgetDecision::Stop {
                    completion_event: completion_event.map(|e| {
                        axagent_harness::kit_bridge::KitBudgetCompletionEvent {
                            continuation_count: e.continuation_count,
                            pct_used: e.pct_used,
                            turn_tokens: e.turn_tokens,
                            budget: e.budget,
                            diminishing_returns: e.diminishing_returns,
                            duration_ms: e.duration_ms,
                        }
                    }),
                }
            },
        }
    }
}

// ── SkillDirs ─────────────────────────────────────────────────

pub struct KitSkillDirsImpl;

impl KitSkillDirs for KitSkillDirsImpl {
    fn skill_dirs(&self) -> Vec<(String, std::path::PathBuf)> {
        crate::skill_dirs::skill_dirs()
    }
    fn all_skills_dirs(&self) -> Vec<std::path::PathBuf> {
        crate::skill_dirs::all_skills_dirs()
    }
}

// ── SlashCommand ──────────────────────────────────────────────

/// Wraps kit's slash_command module as a trait implementation.
pub struct KitSlashCommandProcessorImpl;

impl KitSlashCommandProcessor for KitSlashCommandProcessorImpl {
    fn process(&self, text: &str) -> Option<SlashCommandAction> {
        crate::slash_command::process_slash_command(text).map(|a| match a {
            crate::slash_command::SlashCommandAction::LoadBundle { name, args } => {
                SlashCommandAction::LoadBundle { name, args }
            },
            crate::slash_command::SlashCommandAction::LoadSkill { name, args } => {
                SlashCommandAction::LoadSkill { name, args }
            },
            crate::slash_command::SlashCommandAction::SwitchPersonality { name } => {
                SlashCommandAction::SwitchPersonality { name }
            },
            crate::slash_command::SlashCommandAction::BuiltIn { command, args } => {
                SlashCommandAction::BuiltIn { command, args }
            },
            crate::slash_command::SlashCommandAction::Unknown => SlashCommandAction::Unknown,
        })
    }

    fn load_bundle_content(&self, name: &str, args: &str) -> Option<String> {
        crate::slash_command::load_bundle_content(name, args)
    }

    fn load_skill_content(&self, name: &str, args: &str) -> Option<String> {
        crate::slash_command::load_skill_content(name, args)
    }
}
