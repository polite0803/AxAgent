//! 统一 HTML 清理模块
//!
//! 消除项目中 3 处重复的 HTML 清理代码（tools/web_search.rs、tools/web_fetch.rs、
//! agent/web_search.rs），提供可配置的 HTML 内容提取能力。

use scraper::{Html, Selector};

const DEFAULT_MAX_LENGTH: usize = 80_000;

/// 预编译所有 CSS 选择器的 HTML 清理器
pub struct HtmlCleaner {
    noise_selector: Selector,
    content_selector: Selector,
    heading_selector: Selector,
    block_selector: Selector,
    link_selector: Selector,
    title_selector: Selector,
}

#[derive(Debug, Clone)]
pub struct CleanedHtml {
    pub title: String,
    pub body_text: String,
    pub links: Vec<String>,
    pub headings: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CleanOptions {
    pub max_length: usize,
    pub extract_links: bool,
    pub extract_headings: bool,
    pub noise_words_to_dedup: bool,
    pub max_links: usize,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            max_length: DEFAULT_MAX_LENGTH,
            extract_links: false,
            extract_headings: false,
            noise_words_to_dedup: false,
            max_links: 20,
        }
    }
}

impl HtmlCleaner {
    pub fn new() -> Self {
        let noise_selector = Selector::parse(
            "script, style, nav, footer, header, aside, iframe, noscript, svg, form, \
             button, input, select, textarea, [role='navigation'], [role='banner'], \
             [role='contentinfo'], [role='complementary'], .sidebar, .nav, .menu, \
             .footer, .header, .ad, .ads, .advertisement, .cookie, .popup, .modal, \
             .overlay, #sidebar, #nav, #footer, #header, #menu, .social, .share, \
             .related, .comments",
        )
        .expect("noise selector parse failed");

        let content_selector = Selector::parse(
            "main, article, [role='main'], [role='article'], .content, .post, \
             .article, .entry, #content, #main, .main-content, .post-content, \
             .article-content, .entry-content",
        )
        .expect("content selector parse failed");

        let heading_selector =
            Selector::parse("h1, h2, h3, h4, h5, h6").expect("heading selector parse failed");

        let block_selector = Selector::parse("p, li, td, th, blockquote, pre, dd, dt, div")
            .expect("block selector parse failed");

        let link_selector = Selector::parse("a[href]").expect("link selector parse failed");

        let title_selector = Selector::parse("title").expect("title selector parse failed");

        Self {
            noise_selector,
            content_selector,
            heading_selector,
            block_selector,
            link_selector,
            title_selector,
        }
    }

    /// 核心方法：清理 HTML 并返回结构化结果
    pub fn clean(&self, html: &str, options: &CleanOptions) -> CleanedHtml {
        let doc = Html::parse_document(html);

        let title = get_title(&doc, &self.title_selector);

        let links: Vec<String> = if options.extract_links {
            doc.select(&self.link_selector)
                .filter_map(|el| el.value().attr("href").map(|h| h.to_string()))
                .filter(|l| l.starts_with("http://") || l.starts_with("https://"))
                .take(options.max_links)
                .collect()
        } else {
            Vec::new()
        };

        let noise_text: String = doc
            .select(&self.noise_selector)
            .flat_map(|el| el.text())
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        let root = doc
            .select(&self.content_selector)
            .next()
            .unwrap_or_else(|| doc.root_element());

        let full_text: String = root.text().collect::<Vec<_>>().join(" ");

        let body_text = if options.noise_words_to_dedup
            && !noise_text.is_empty()
            && full_text.len() > noise_text.len() * 2
        {
            let noise_words: std::collections::HashSet<String> = noise_text
                .split_whitespace()
                .take(200)
                .map(|s| s.to_string())
                .collect();

            full_text
                .split_whitespace()
                .filter(|w| w.len() > 3 || !noise_words.contains(&w.to_lowercase()))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            full_text
        };

        let body_text = clean_whitespace(&body_text);
        let body_text = truncate_if_needed(&body_text, options.max_length);

        let headings: Vec<(String, String)> = if options.extract_headings {
            root.select(&self.heading_selector)
                .map(|el| {
                    let tag = el.value().name().to_string();
                    let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    (tag, text)
                })
                .collect()
        } else {
            Vec::new()
        };

        CleanedHtml {
            title,
            body_text,
            links,
            headings,
        }
    }

    /// 仅提取纯文本（替代 web_search.rs 的 extract_page_text）
    pub fn extract_text(&self, html_str: &str, max_length: usize) -> String {
        let mut doc = Html::parse_document(html_str);

        // 收集并移除噪声节点
        let noise_ids: Vec<_> = doc.select(&self.noise_selector).map(|el| el.id()).collect();
        for nid in noise_ids {
            if let Some(mut node) = doc.tree.get_mut(nid) {
                node.detach();
            }
        }

        let text = doc
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        truncate_if_needed(&text, max_length)
    }

    /// 带 prompt 的标题+正文提取（替代 web_fetch.rs 的 html_to_text_with_title）
    pub fn extract_with_title(
        &self,
        html_str: &str,
        prompt: &str,
        max_length: usize,
    ) -> (String, String) {
        let mut doc = Html::parse_document(html_str);

        let title = get_title(&doc, &self.title_selector);

        // 移除噪声节点
        let noise_ids: Vec<_> = doc.select(&self.noise_selector).map(|el| el.id()).collect();
        for nid in noise_ids {
            if let Some(mut node) = doc.tree.get_mut(nid) {
                node.detach();
            }
        }

        let root = doc
            .select(&self.content_selector)
            .next()
            .unwrap_or_else(|| doc.root_element());

        let prompt_terms: Vec<String> = if !prompt.is_empty() {
            prompt
                .to_lowercase()
                .split_whitespace()
                .filter(|w| w.len() > 1)
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let headings: Vec<String> = root
            .select(&self.heading_selector)
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .collect();

        let blocks: Vec<String> = root
            .select(&self.block_selector)
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .collect();

        if headings.is_empty() && blocks.is_empty() {
            let all_text: String = root.text().collect::<Vec<_>>().join(" ");
            let cleaned = clean_whitespace(&all_text);
            return (title, cleaned);
        }

        // heading 和 block 交替（简化，不依赖 NodeId 排序）
        let mut result = String::new();
        let has_prompt = !prompt_terms.is_empty();

        let max_lines = headings.len().max(blocks.len());
        for i in 0..max_lines {
            let heading_text = headings.get(i).cloned().unwrap_or_default();
            let block_text = blocks.get(i).cloned().unwrap_or_default();

            if !heading_text.is_empty() {
                let relevant = if has_prompt {
                    let hlower = heading_text.to_lowercase();
                    prompt_terms.iter().any(|t| hlower.contains(t.as_str()))
                } else {
                    true
                };

                if !result.is_empty() {
                    result.push_str("\n\n");
                }
                result.push_str("### ");
                result.push_str(&heading_text);
                result.push('\n');

                if !block_text.is_empty() && relevant {
                    result.push('\n');
                    result.push_str(&block_text);
                }
            } else if !block_text.is_empty() && !has_prompt {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&block_text);
            }
        }

        (title, truncate_if_needed(&result, max_length))
    }

    /// 提取标题+正文+链接（替代 agent/web_search.rs 的 extract_readability）
    pub fn extract_readability(&self, html: &str) -> (String, String, Vec<String>) {
        let options = CleanOptions {
            extract_links: true,
            noise_words_to_dedup: true,
            ..Default::default()
        };
        let cleaned = self.clean(html, &options);
        (cleaned.title, cleaned.body_text, cleaned.links)
    }

    /// 检测文本主要语言
    pub fn detect_language(text: &str) -> &'static str {
        let sample = &text[..text.len().min(500)];
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

impl Default for HtmlCleaner {
    fn default() -> Self {
        Self::new()
    }
}

// ── 内部辅助 ──

fn get_title(doc: &Html, title_sel: &Selector) -> String {
    doc.select(title_sel)
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn clean_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_if_needed(text: &str, max_length: usize) -> String {
    if text.len() > max_length {
        format!("{}...\n[Content truncated at {} chars]", &text[..max_length], max_length)
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_basic() {
        let cleaner = HtmlCleaner::new();
        let html = r#"<html><head><title>Test</title></head><body><main><p>Hello world content</p></main></body></html>"#;
        let text = cleaner.extract_text(html, 5000);
        assert!(text.contains("Hello world content"));
    }

    #[test]
    fn test_extract_text_removes_noise() {
        let cleaner = HtmlCleaner::new();
        let html = r#"<html><body><script>var x=1;</script><main><p>Real content</p></main><nav>Menu</nav></body></html>"#;
        let text = cleaner.extract_text(html, 5000);
        assert!(text.contains("Real content"));
        assert!(!text.contains("var x=1"));
    }

    #[test]
    fn test_extract_with_title() {
        let cleaner = HtmlCleaner::new();
        let html = r#"<html><head><title>My Page</title></head><body><main><h1>Section A</h1><p>Content A</p><h2>Section B</h2><p>Content B</p></main></body></html>"#;
        let (title, text) = cleaner.extract_with_title(html, "", 5000);
        assert_eq!(title, "My Page");
        assert!(text.contains("Section A"));
        assert!(text.contains("Content B"));
    }

    #[test]
    fn test_extract_with_title_prompt_relevance() {
        let cleaner = HtmlCleaner::new();
        let html = r#"<html><body><main><h2>Pricing</h2><p>$99 per month</p><h2>About Us</h2><p>We are a company.</p></main></body></html>"#;
        let (_title, text) = cleaner.extract_with_title(html, "pricing cost", 5000);
        assert!(text.contains("Pricing"));
        assert!(text.contains("$99"));
    }

    #[test]
    fn test_extract_readability() {
        let cleaner = HtmlCleaner::new();
        let html = r#"<html><head><title>Links</title></head><body><article><p>Main text.</p><a href="https://example.com">Example</a><a href="https://other.com">Other</a></article></body></html>"#;
        let (title, body, links) = cleaner.extract_readability(html);
        assert_eq!(title, "Links");
        assert!(body.contains("Main text."));
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com".to_string()));
    }

    #[test]
    fn test_detect_language_chinese() {
        let text = "这是一段中文测试文本用于语言检测";
        assert_eq!(HtmlCleaner::detect_language(text), "zh");
    }

    #[test]
    fn test_detect_language_english() {
        let text = "This is an English test for language detection";
        assert_eq!(HtmlCleaner::detect_language(text), "en");
    }

    #[test]
    fn test_clean_empty_html() {
        let cleaner = HtmlCleaner::new();
        let text = cleaner.extract_text("", 5000);
        assert!(text.is_empty());
    }

    #[test]
    fn test_truncation() {
        let cleaner = HtmlCleaner::new();
        let html = "<html><body><p>short</p></body></html>";
        let text = cleaner.extract_text(html, 3);
        assert!(text.contains("truncated"));
    }
}
