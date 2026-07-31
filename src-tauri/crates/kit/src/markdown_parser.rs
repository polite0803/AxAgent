// SPDX-License-Identifier: AGPL-3.0-only

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLink {
    pub target: String,
    pub display_text: Option<String>,
    pub link_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedFrontmatter {
    pub title: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub source: Option<String>,
    pub page_type: Option<String>,
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNote {
    pub frontmatter: ParsedFrontmatter,
    pub content: String,
    pub links: Vec<ParsedLink>,
    pub raw_links: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MarkdownParser {
    link_regex: Regex,
    wiki_link_regex: Regex,
    frontmatter_regex: Regex,
    tag_regex: Regex,
}

impl MarkdownParser {
    pub fn new() -> Self {
        Self {
            link_regex: Regex::new(r"\[([^\]]+)\]\(([^\)]+)\)").expect("static regex"),
            wiki_link_regex: Regex::new(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]").expect("static regex"),
            frontmatter_regex: Regex::new(r"(?s)^---\n(.+?)\n---").expect("static regex"),
            tag_regex: Regex::new(r"(?:^|\s)#([a-zA-Z0-9_-]+)").expect("static regex"),
        }
    }

    pub fn parse(&self, content: &str) -> ParsedNote {
        let frontmatter = self.extract_frontmatter(content);
        let content_without_frontmatter = self.strip_frontmatter(content);
        let links = self.extract_links(&content_without_frontmatter);
        let raw_links = self.extract_raw_wiki_links(&content_without_frontmatter);

        ParsedNote { frontmatter, content: content_without_frontmatter, links, raw_links }
    }

    pub fn extract_frontmatter(&self, content: &str) -> ParsedFrontmatter {
        if let Some(captures) = self.frontmatter_regex.captures(content) {
            let fm_content = captures.get(1).map(|m| m.as_str()).unwrap_or("");

            let mut frontmatter = ParsedFrontmatter {
                title: None,
                author: None,
                tags: Vec::new(),
                created: None,
                source: None,
                page_type: None,
                custom: std::collections::HashMap::new(),
            };

            for line in fm_content.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim();

                    match key {
                        "title" => frontmatter.title = Some(value.to_string()),
                        "author" => frontmatter.author = Some(value.to_string()),
                        "created" => frontmatter.created = Some(value.to_string()),
                        "source" => frontmatter.source = Some(value.to_string()),
                        "page_type" => frontmatter.page_type = Some(value.to_string()),
                        "tags" => {
                            frontmatter.tags = self.parse_tags_list(value);
                        },
                        _ => {
                            if !value.is_empty() {
                                frontmatter.custom.insert(
                                    key.to_string(),
                                    serde_json::Value::String(value.to_string()),
                                );
                            }
                        },
                    }
                }
            }

            frontmatter
        } else {
            ParsedFrontmatter::default()
        }
    }

    pub fn strip_frontmatter(&self, content: &str) -> String {
        self.frontmatter_regex.replace(content, "").to_string()
    }

    pub fn extract_links(&self, content: &str) -> Vec<ParsedLink> {
        let mut links = Vec::new();

        for caps in self.link_regex.captures_iter(content) {
            let display = caps.get(1).map(|m| m.as_str().to_string());
            let url = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

            let link_type = if url.starts_with("http://") || url.starts_with("https://") {
                "url"
            } else if url.starts_with("/") {
                "path"
            } else {
                "file"
            };

            if let Some(target) = display.clone() {
                links.push(ParsedLink {
                    target,
                    display_text: Some(url),
                    link_type: link_type.to_string(),
                });
            }
        }

        for caps in self.wiki_link_regex.captures_iter(content) {
            let target = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let display = caps.get(2).map(|m| m.as_str().to_string());

            links.push(ParsedLink { target, display_text: display, link_type: "wiki".to_string() });
        }

        links
    }

    pub fn extract_raw_wiki_links(&self, content: &str) -> Vec<String> {
        self.wiki_link_regex
            .captures_iter(content)
            .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }

    pub fn extract_tags(&self, content: &str) -> HashSet<String> {
        self.tag_regex
            .captures_iter(content)
            .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }

    fn parse_tags_list(&self, value: &str) -> Vec<String> {
        let value = value.trim();

        if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len() - 1];
            inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\''))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![value.to_string()]
        }
    }

    pub fn extract_title_from_content(&self, content: &str) -> Option<String> {
        let content = self.strip_frontmatter(content);

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix("# ") {
                return Some(stripped.trim().to_string());
            }
        }

        None
    }

    pub fn render_wiki_link(&self, target: &str, display: Option<&str>) -> String {
        match display {
            Some(d) if d != target => format!("[[{}|{}]]", target, d),
            _ => format!("[[{}]]", target),
        }
    }

    pub fn is_valid_wiki_link_target(&self, target: &str) -> bool {
        !target.is_empty()
            && !target.contains('[')
            && !target.contains(']')
            && !target.contains('#')
            && !target.contains('|')
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

// ── 公共工具函数：wikilinks / tags / frontmatter 提取 ──────────
//
// 以下函数原位于 tools/src/tools/obsidian.rs，现统一抽取到 kit crate，
// 供 wiki、search、trajectory 等多个 crate 共用，避免重复实现。

/// 从 markdown body 提取 `[[Note]]` / `[[Note|alias]]` / `[[Note#anchor]]` 链
pub fn extract_wikilinks(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'['
            && bytes[i + 1] == b'['
            && let Some(end) = find_substring(&body[i + 2..], "]]")
        {
            let raw = &body[i + 2..i + 2 + end];
            // 取 | 之前、# 之前的部分作为 note 名
            let name = raw.split('|').next().unwrap_or("").split('#').next().unwrap_or("").trim();
            if !name.is_empty() {
                out.push(name.to_string());
            }
            i += 2 + end + 2;
            continue;
        }
        i += 1;
    }
    out
}

fn find_substring(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}

/// 提取 inline `#tag`（排除 markdown heading）
pub fn extract_inline_tags(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        // 跳过 markdown heading（# 开头后跟空格）
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            continue;
        }
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'#' {
                // 前一字符必须是非字母数字（或行首）
                let prev_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
                if prev_ok && i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
                    let mut j = i + 1;
                    while j < bytes.len()
                        && (bytes[j].is_ascii_alphanumeric()
                            || bytes[j] == b'_'
                            || bytes[j] == b'-'
                            || bytes[j] == b'/')
                    {
                        j += 1;
                    }
                    let tag = &line[i + 1..j];
                    if !tag.is_empty() {
                        out.push(tag.to_string());
                    }
                    i = j;
                    continue;
                }
            }
            i += 1;
        }
    }
    out
}

/// 把 markdown 内容拆为 (frontmatter_json, body)
///
/// frontmatter 必须以 `---\n` 开头并以 `\n---\n` 结束
pub fn split_frontmatter(content: &str) -> (serde_json::Value, String) {
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        return (serde_json::Value::Object(serde_json::Map::new()), content.to_string());
    }
    let rest = &content["---\n".len()..];
    let end = rest.find("\n---\n").or_else(|| rest.find("\r\n---\r\n"));
    let Some(end) = end else {
        return (serde_json::Value::Object(serde_json::Map::new()), content.to_string());
    };
    let yaml_str = &rest[..end];
    let body_start = end + "\n---\n".len();
    let body = if body_start < rest.len() {
        rest[body_start..].to_string()
    } else {
        String::new()
    };
    let fm: serde_json::Value =
        serde_yaml::from_str(yaml_str).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    (fm, body)
}

/// 在 `content` 的 `idx` 位置附近生成 snippet（前后各取一半）
pub fn make_snippet(content: &str, idx: usize, total: usize) -> String {
    let chars: Vec<char> = content.chars().collect();
    let char_idx = content[..idx.min(content.len())].chars().count();
    let half = total / 2;
    let start = char_idx.saturating_sub(half);
    let end = (start + total).min(chars.len());
    let snippet: String = chars[start..end].iter().collect();
    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < chars.len() { "…" } else { "" };
    format!("{}{}{}", prefix, snippet.trim(), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wiki_link() {
        let parser = MarkdownParser::new();
        let content = "This is a link to [[Target Note]] and [[Another Note|Display]].";
        let result = parser.parse(content);

        assert_eq!(result.links.len(), 2);
        assert_eq!(result.links[0].target, "Target Note");
        assert_eq!(result.links[0].display_text, None);
        assert_eq!(result.links[1].target, "Another Note");
        assert_eq!(result.links[1].display_text, Some("Display".to_string()));
    }

    #[test]
    fn test_extract_frontmatter() {
        let parser = MarkdownParser::new();
        let content = r#"---
title: Test Note
author: user
tags: [tag1, tag2]
---

# Main Content
"#;
        let fm = parser.extract_frontmatter(content);

        assert_eq!(fm.title, Some("Test Note".to_string()));
        assert_eq!(fm.author, Some("user".to_string()));
        assert_eq!(fm.tags, vec!["tag1", "tag2"]);
    }
}
