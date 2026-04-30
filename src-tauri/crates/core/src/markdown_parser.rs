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
            link_regex: Regex::new(r"\[([^\]]+)\]\(([^\)]+)\)").unwrap(),
            wiki_link_regex: Regex::new(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]").unwrap(),
            frontmatter_regex: Regex::new(r"(?s)^---\n(.+?)\n---").unwrap(),
            tag_regex: Regex::new(r"(?:^|\s)#([a-zA-Z0-9_-]+)").unwrap(),
        }
    }

    pub fn parse(&self, content: &str) -> ParsedNote {
        let frontmatter = self.extract_frontmatter(content);
        let content_without_frontmatter = self.strip_frontmatter(content);
        let links = self.extract_links(&content_without_frontmatter);
        let raw_links = self.extract_raw_wiki_links(&content_without_frontmatter);

        ParsedNote {
            frontmatter,
            content: content_without_frontmatter,
            links,
            raw_links,
        }
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
                        }
                        _ => {
                            if !value.is_empty() {
                                frontmatter.custom.insert(
                                    key.to_string(),
                                    serde_json::Value::String(value.to_string()),
                                );
                            }
                        }
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

            links.push(ParsedLink {
                target,
                display_text: display,
                link_type: "wiki".to_string(),
            });
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