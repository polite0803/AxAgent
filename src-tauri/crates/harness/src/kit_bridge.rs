// SPDX-License-Identifier: AGPL-3.0-only

//! Kit-to-Harness bridge: traits + DTOs for kit modules needed by consumer crates.
//! Kit implements these traits so consumers depend only on harness.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── 1. MarkdownParser ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MdParsedLink {
    pub target: String,
    pub display_text: Option<String>,
    pub link_type: String,
}

#[derive(Debug, Clone, Default)]
pub struct MdParsedFrontmatter {
    pub title: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub source: Option<String>,
    pub page_type: Option<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct MdParsedNote {
    pub frontmatter: MdParsedFrontmatter,
    pub content: String,
    pub links: Vec<MdParsedLink>,
    pub raw_links: Vec<String>,
}

pub trait KitMarkdownParser: Send + Sync {
    fn parse(&self, content: &str) -> MdParsedNote;
}

// ── 2. HtmlCleaner ────────────────────────────────────────────

pub trait KitHtmlCleaner: Send + Sync {
    fn extract_readability(&self, html: &str) -> (String, String, Vec<String>);
    fn detect_language(&self, text: &str) -> &'static str;
}

// ── 3. TokenBudgetTracker ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct KitBudgetCompletionEvent {
    pub continuation_count: u32,
    pub pct_used: u32,
    pub turn_tokens: u64,
    pub budget: u64,
    pub diminishing_returns: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KitTokenBudgetDecision {
    Continue {
        nudge_message: String,
        continuation_count: u32,
        pct_used: u32,
        turn_tokens: u64,
        budget: u64,
    },
    /// 建议执行自动 compact（压缩早期步骤释放上下文空间）。
    ///
    /// 触发阈值（在 kit 实现侧配置）：典型为 budget 的 75%，
    /// 留出 compact 本身的开销后，剩余空间仍能容纳若干轮迭代。
    Compact {
        /// 触发 compact 时发给模型的提示（解释即将做什么、为什么）
        nudge_message: String,
        /// 建议保留最近 N 步不压缩（通常 4-6 步保证局部连续性）
        preserve_recent_steps: usize,
        /// 当前已用百分比（0-100）
        pct_used: u32,
        /// 预算上限
        budget: u64,
    },
    Stop {
        completion_event: Option<KitBudgetCompletionEvent>,
    },
}

pub trait KitTokenBudgetTracker: Send + Sync {
    fn reset(&mut self);
    fn record_tokens(&mut self, global_turn_tokens: u64);
    fn check(&mut self, budget: Option<u64>, global_turn_tokens: u64) -> KitTokenBudgetDecision;
}

// ── 4. SkillDirs ──────────────────────────────────────────────

pub trait KitSkillDirs: Send + Sync {
    fn skill_dirs(&self) -> Vec<(String, PathBuf)>;
    fn all_skills_dirs(&self) -> Vec<PathBuf>;
}

// ── 5. SlashCommand ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SlashCommandAction {
    LoadBundle { name: String, args: String },
    LoadSkill { name: String, args: String },
    SwitchPersonality { name: String },
    BuiltIn { command: String, args: String },
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommandPreprocessed {
    pub modified_text: String,
    pub personality_prompt: Option<String>,
    pub is_builtin: bool,
}

pub trait KitSlashCommandProcessor: Send + Sync {
    fn process(&self, text: &str) -> Option<SlashCommandAction>;
    fn load_bundle_content(&self, name: &str, args: &str) -> Option<String>;
    fn load_skill_content(&self, name: &str, args: &str) -> Option<String>;
}
