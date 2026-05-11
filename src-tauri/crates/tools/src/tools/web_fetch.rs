use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

const MAX_CONTENT_LENGTH: usize = 200_000;
const BINARY_CONTENT_TYPES: &[&str] = &[
    "application/pdf",
    "application/zip",
    "application/x-rar",
    "application/x-tar",
    "application/x-gzip",
    "application/x-bzip2",
    "application/x-7z-compressed",
    "application/octet-stream",
    "image/",
    "video/",
    "audio/",
    "font/",
];
const MAX_REDIRECTS: u32 = 5;
const MAX_RETRIES: u32 = 2;
const RATE_LIMIT_INTERVAL_MS: u64 = 500;

static RATE_LIMITER: parking_lot::Mutex<u64> = parking_lot::Mutex::new(0);

fn build_http_client() -> Arc<reqwest::Client> {
    Arc::new(
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= MAX_REDIRECTS as usize {
                    attempt.stop()
                } else {
                    let url = attempt.url();
                    let _host = url.host_str().unwrap_or("");
                    if axagent_core::search::is_safe_url(url.as_str()) {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }
            }))
            .cookie_store(true)
            .build()
            .expect("Failed to build HTTP client"),
    )
}

fn get_shared_client() -> Arc<reqwest::Client> {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<Arc<reqwest::Client>> = OnceLock::new();
    CLIENT.get_or_init(build_http_client).clone()
}

fn check_rate_limit() {
    let mut last = RATE_LIMITER.lock();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let elapsed = now.saturating_sub(*last);
    if elapsed < RATE_LIMIT_INTERVAL_MS {
        std::thread::sleep(std::time::Duration::from_millis(RATE_LIMIT_INTERVAL_MS - elapsed));
    }
    *last = now;
}

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }
    fn description(&self) -> &str {
        "Fetch content from a URL and convert it to text. Use this to retrieve web pages, documents, or API data. \
         Supports HTML pages (extracts main content), JSON APIs (pretty-printed), and plain text. \
         The 'prompt' parameter specifies what information to extract from the page — the tool will \
         focus extraction on relevant sections when possible."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要抓取的 URL"
                },
                "prompt": {
                    "type": "string",
                    "description": "从页面中提取什么的指令，例如'提取价格信息'、'提取技术规格'"
                }
            },
            "required": ["url"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("WebFetch", "缺少 url 参数"))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::invalid_input("url 必须以 http:// 或 https:// 开头"));
        }

        if !axagent_core::search::is_safe_url(url) {
            return Err(ToolError::permission_denied("WebFetch", "禁止访问内网地址"));
        }

        if !ctx.allow_network {
            return Err(ToolError::permission_denied("WebFetch", "当前上下文不允许网络请求"));
        }

        Ok(())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap();
        let prompt = input["prompt"].as_str().unwrap_or("").to_string();

        check_rate_limit();

        let client = get_shared_client();

        let response = fetch_with_retry(&client, url).await?;

        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        for binary_ct in BINARY_CONTENT_TYPES {
            if content_type.contains(binary_ct) {
                return Ok(ToolResult::success(format!(
                    "## URL: {}\n状态: {}\nContent-Type: {}\n\n[二进制内容，无法提取文本。请使用专门的下载工具处理此类型文件。]",
                    url, status, content_type
                )));
            }
        }

        let raw_bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::execution_failed(format!("读取响应失败: {}", e)))?;

        let body = decode_body(&raw_bytes, &content_type);

        let (title, extracted) =
            if content_type.contains("text/html") || content_type.contains("application/xhtml") {
                let (t, text) = html_to_text_with_title(&body, &prompt);
                (t, text)
            } else if content_type.contains("application/json") {
                let title = "JSON Response".to_string();
                let text = if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    serde_json::to_string_pretty(&json).unwrap_or(body)
                } else {
                    body
                };
                (title, text)
            } else {
                let title = "Plain Text".to_string();
                (title, body)
            };

        let header = format!(
            "## URL: {}\n标题: {}\n状态: {}\nContent-Type: {}\n",
            url, title, status, content_type
        );

        let prompt_hint = if !prompt.is_empty() {
            format!("\n[提取目标: {}]\n", prompt)
        } else {
            String::new()
        };

        let header_len = header.len() + prompt_hint.len();
        let available = MAX_CONTENT_LENGTH.saturating_sub(header_len);

        let truncated = extracted.len() > available;
        let content = if truncated {
            format!(
                "{}{}{}\n\n[内容已截断，已显示约 {}/{} 字符]",
                header,
                prompt_hint,
                &extracted[..available],
                available,
                extracted.len()
            )
        } else {
            format!("{}{}{}", header, prompt_hint, extracted)
        };

        Ok(ToolResult::success(content))
    }
}

async fn fetch_with_retry(
    client: &reqwest::Client,
    url: &str,
) -> Result<reqwest::Response, ToolError> {
    let mut last_err = String::new();
    for attempt in 0..=MAX_RETRIES {
        let result = client.get(url).send().await;
        match result {
            Ok(resp) => {
                if resp.status().is_success() || resp.status().is_redirection() {
                    return Ok(resp);
                }
                if resp.status().is_server_error() && attempt < MAX_RETRIES {
                    let delay = std::time::Duration::from_millis(500 * (attempt as u64 + 1));
                    tokio::time::sleep(delay).await;
                    last_err = format!("HTTP {}", resp.status());
                    continue;
                }
                return Ok(resp);
            },
            Err(e) => {
                if attempt < MAX_RETRIES && (e.is_timeout() || e.is_connect()) {
                    let delay = std::time::Duration::from_millis(500 * (attempt as u64 + 1));
                    tokio::time::sleep(delay).await;
                    last_err = e.to_string();
                    continue;
                }
                return Err(ToolError::execution_failed(format!(
                    "HTTP 请求失败（重试{}次后）: {}",
                    attempt, e
                )));
            },
        }
    }
    Err(ToolError::execution_failed(format!(
        "HTTP 请求失败（重试{}次后）: {}",
        MAX_RETRIES, last_err
    )))
}

fn decode_body(raw: &[u8], content_type: &str) -> String {
    if let Some(charset) = extract_charset(content_type) {
        if charset.eq_ignore_ascii_case("utf-8") || charset.eq_ignore_ascii_case("utf8") {
            return String::from_utf8_lossy(raw).into_owned();
        }
        if let Some(enc) = encoding_rs::Encoding::for_label(charset.as_bytes()) {
            let (decoded, _, _) = enc.decode(raw);
            return decoded.into_owned();
        }
    }

    if let Ok(s) = std::str::from_utf8(raw) {
        return s.to_string();
    }

    if let Some(meta_charset) = detect_html_charset(raw) {
        if let Some(enc) = encoding_rs::Encoding::for_label(meta_charset.as_bytes()) {
            let (decoded, _, _) = enc.decode(raw);
            return decoded.into_owned();
        }
    }

    let (decoded, _) = encoding_rs::UTF_8.decode_with_bom_removal(raw);
    decoded.into_owned()
}

fn extract_charset(content_type: &str) -> Option<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if part.starts_with("charset=") {
            let charset = part.strip_prefix("charset=").unwrap().trim();
            let charset = charset.trim_matches('"').trim_matches('\'');
            if !charset.is_empty() {
                return Some(charset.to_string());
            }
        }
    }
    None
}

fn detect_html_charset(html: &[u8]) -> Option<String> {
    let head_end = html
        .windows(6)
        .position(|w| w == b"</head>" || w == b"</HEAD>")?;
    let head = &html[..head_end];

    let head_str = String::from_utf8_lossy(head);

    if let Some(pos) = head_str.find("charset=") {
        let rest = &head_str[pos + 8..];
        let charset: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !charset.is_empty() {
            return Some(charset);
        }
    }

    if let Some(pos) = head_str.find("charset ") {
        let rest = &head_str[pos + 8..];
        let charset: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !charset.is_empty() {
            return Some(charset);
        }
    }

    None
}

fn html_to_text_with_title(html: &str, prompt: &str) -> (String, String) {
    let mut doc = scraper::Html::parse_document(html);

    let title = doc
        .select(&scraper::Selector::parse("title").unwrap())
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or_default()
        .trim()
        .to_string();

    let noise_sel = scraper::Selector::parse(
        "script, style, nav, footer, header, aside, iframe, noscript, svg, form, \
         button, input, select, textarea, [role='navigation'], [role='banner'], \
         [role='contentinfo'], [role='complementary'], .sidebar, .nav, .menu, \
         .footer, .header, .ad, .ads, .advertisement, .cookie, .popup, .modal, \
         .overlay, #sidebar, #nav, #footer, #header, #menu, .social, .share, \
         .related, .comments",
    )
    .unwrap();

    let noise_ids: Vec<ego_tree::NodeId> = doc.select(&noise_sel).map(|el| el.id()).collect();
    for nid in noise_ids {
        if let Some(mut node) = doc.tree.get_mut(nid) {
            node.detach();
        }
    }

    let content_sel = scraper::Selector::parse(
        "main, article, [role='main'], [role='article'], .content, .post, \
         .article, .entry, #content, #main, .main-content, .post-content, \
         .article-content, .entry-content",
    )
    .unwrap();

    let root = doc
        .select(&content_sel)
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

    let heading_sel = scraper::Selector::parse("h1, h2, h3, h4, h5, h6").unwrap();
    let block_sel =
        scraper::Selector::parse("p, li, td, th, blockquote, pre, dd, dt, div").unwrap();

    let headings: Vec<(String, ego_tree::NodeId)> = root
        .select(&heading_sel)
        .map(|el| {
            let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            (text, el.id())
        })
        .collect();

    let blocks: Vec<(String, ego_tree::NodeId)> = root
        .select(&block_sel)
        .map(|el| {
            let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            (text, el.id())
        })
        .collect();

    if headings.is_empty() && blocks.is_empty() {
        let all_text: String = root.text().collect::<Vec<_>>().join(" ");
        let cleaned = all_text.split_whitespace().collect::<Vec<_>>().join(" ");
        return (title, cleaned);
    }

    let mut sections: Vec<Section> = Vec::new();
    let mut current_heading = String::new();
    let mut current_text = String::new();
    let mut in_relevant_section = prompt_terms.is_empty();

    let mut heading_idx = 0;
    let mut block_idx = 0;

    loop {
        let next_heading = headings.get(heading_idx);
        let next_block = blocks.get(block_idx);

        match (next_heading, next_block) {
            (None, None) => break,
            (Some(_), None) => {
                let (text, _) = headings[heading_idx].clone();
                if !current_text.is_empty() || !current_heading.is_empty() {
                    sections.push(Section {
                        heading: current_heading.clone(),
                        text: current_text.trim().to_string(),
                        relevant: in_relevant_section,
                    });
                    current_text.clear();
                }
                current_heading = text;
                if !prompt_terms.is_empty() {
                    let heading_lower = current_heading.to_lowercase();
                    in_relevant_section = prompt_terms
                        .iter()
                        .any(|term| heading_lower.contains(term.as_str()));
                } else {
                    in_relevant_section = true;
                }
                heading_idx += 1;
            },
            (None, Some(_)) => {
                let (text, _) = blocks[block_idx].clone();
                if !text.is_empty() {
                    if !current_text.is_empty() {
                        current_text.push('\n');
                    }
                    current_text.push_str(&text);
                }
                block_idx += 1;
            },
            (Some((_, h_id)), Some((_, b_id))) => {
                if h_id < b_id {
                    let (text, _) = headings[heading_idx].clone();
                    if !current_text.is_empty() || !current_heading.is_empty() {
                        sections.push(Section {
                            heading: current_heading.clone(),
                            text: current_text.trim().to_string(),
                            relevant: in_relevant_section,
                        });
                        current_text.clear();
                    }
                    current_heading = text;
                    if !prompt_terms.is_empty() {
                        let heading_lower = current_heading.to_lowercase();
                        in_relevant_section = prompt_terms
                            .iter()
                            .any(|term| heading_lower.contains(term.as_str()));
                    } else {
                        in_relevant_section = true;
                    }
                    heading_idx += 1;
                } else {
                    let (text, _) = blocks[block_idx].clone();
                    if !text.is_empty() {
                        if !current_text.is_empty() {
                            current_text.push('\n');
                        }
                        current_text.push_str(&text);
                    }
                    block_idx += 1;
                }
            },
        }
    }

    if !current_text.is_empty() || !current_heading.is_empty() {
        sections.push(Section {
            heading: current_heading,
            text: current_text.trim().to_string(),
            relevant: in_relevant_section,
        });
    }

    let has_relevant = sections.iter().any(|s| s.relevant);
    let filtered: Vec<&Section> = if has_relevant && !prompt_terms.is_empty() {
        sections
            .iter()
            .filter(|s| s.relevant)
            .chain(sections.iter().filter(|s| !s.relevant).take(3))
            .collect()
    } else {
        sections.iter().collect()
    };

    let mut result = String::new();
    for section in filtered {
        if !section.heading.is_empty() {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str("### ");
            result.push_str(&section.heading);
            result.push('\n');
        }
        if !section.text.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&section.text);
        }
    }

    (title, result)
}

struct Section {
    heading: String,
    text: String,
    relevant: bool,
}
