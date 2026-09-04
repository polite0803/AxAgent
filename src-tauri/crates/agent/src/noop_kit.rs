// SPDX-License-Identifier: AGPL-3.0-only

//! 默认空实现（noop）的 kit 桥接 trait，供 convenience 构造器使用。
//! 当 wiring 层未注入真实实现时，agent 以降级模式运行。

use axagent_harness::kit_bridge::{
    KitHtmlCleaner, KitMarkdownParser, KitSkillDirs, KitTokenBudgetDecision, KitTokenBudgetTracker,
    MdParsedFrontmatter, MdParsedNote,
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
    fn extract_readability(&self, html: &str) -> (String, String, Vec<String>) {
        // 降级实现：用轻量字符串解析提供与真实 kit 解析器一致的基础结果
        // （title / 去标签正文 / 绝对链接），使 web_search 等消费者在缺少
        // 真实 kit 解析器时行为合理。不引入 HTML 解析依赖。
        let title = html
            .lines()
            .find_map(|line| {
                if let Some(pos) = line.find("<title>") {
                    let rest = &line[pos + 7..];
                    if let Some(end) = rest.find("</title>") {
                        return Some(rest[..end].trim().to_string());
                    }
                }
                None
            })
            .unwrap_or_default();

        let mut links = Vec::new();
        let mut cursor = 0;
        while let Some(a_pos) = html[cursor..].find("<a ") {
            let a_start = cursor + a_pos;
            let tag_len = html[a_start..].find('>').unwrap_or(html.len() - a_start);
            let tag = &html[a_start..a_start + tag_len];
            if let Some(href_pos) = tag.find("href=") {
                let after = &tag[href_pos + 5..];
                if let Some(q) = after.chars().next() {
                    let q = q.to_string();
                    if (q == "\"" || q == "'")
                        && let Some(end) = after[1..].find(q.as_str())
                    {
                        let href = &after[1..1 + end];
                        if (href.starts_with("http://") || href.starts_with("https://"))
                            && !links.iter().any(|l| l == href)
                        {
                            links.push(href.to_string());
                        }
                    }
                }
            }
            cursor = a_start + tag_len + 1;
        }

        let body_text = strip_html_tags(html);
        (title, body_text, links)
    }
    fn detect_language(&self, text: &str) -> &'static str {
        // 镜像 kit::HtmlCleaner::detect_language 的基础启发式，
        // 使降级实现与真实实现在语言检测上行为一致（CJK 占比 >30% 判为中文）。
        let end = text.char_indices().nth(500).map(|(i, _)| i).unwrap_or(text.len());
        let sample = &text[..end];
        let cjk_count = sample
            .chars()
            .filter(|c| {
                ('\u{4E00}'..='\u{9FFF}').contains(c)
                    || ('\u{3040}'..='\u{309F}').contains(c)
                    || ('\u{AC00}'..='\u{D7AF}').contains(c)
            })
            .count();
        let total = sample.chars().count().max(1);
        if cjk_count as f32 / total as f32 > 0.3 {
            "zh"
        } else {
            "en"
        }
    }
}

// ── NoopMarkdownParser ────────────────────────────────────────

pub struct NoopMarkdownParser;

impl KitMarkdownParser for NoopMarkdownParser {
    fn parse(&self, content: &str) -> MdParsedNote {
        // 降级模式仍解析 YAML frontmatter 的基础字段（title/author/tags），
        // 使 lint_checker 等消费者在缺少真实 kit 解析器时，行为与生产解析器一致。
        let mut frontmatter = MdParsedFrontmatter::default();

        if let Some(body) = extract_frontmatter_block(content) {
            for line in body.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("title:") {
                    frontmatter.title = Some(rest.trim().to_string());
                } else if let Some(rest) = line.strip_prefix("author:") {
                    frontmatter.author = Some(rest.trim().to_string());
                } else if let Some(rest) = line.strip_prefix("tags:") {
                    frontmatter.tags = parse_yaml_tags(rest.trim());
                }
            }
        }

        MdParsedNote {
            frontmatter,
            content: content.to_string(),
            links: Vec::new(),
            raw_links: Vec::new(),
        }
    }
}

/// 提取 `---` 包裹的 frontmatter 主体（不含定界符）。
fn extract_frontmatter_block(content: &str) -> Option<String> {
    let stripped = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))?;
    let end = stripped.find("\n---").or_else(|| stripped.find("\r\n---"))?;
    Some(stripped[..end].to_string())
}

/// 解析 YAML `tags` 值：支持内联数组 `[a, b]` 与单个值。
fn parse_yaml_tags(value: &str) -> Vec<String> {
    let value = value.trim();
    if value.is_empty() {
        return Vec::new();
    }
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![value.to_string()]
    }
}

/// 去除 HTML 标签，保留可见文本（降级解析用，不处理实体与嵌套转义）。
fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {},
        }
    }
    out
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
