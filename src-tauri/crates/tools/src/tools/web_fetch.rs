use crate::{ProgressEntry, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_core::html_cleaner::HtmlCleaner;
use axagent_core::search::{is_safe_url_deep, shared_http_client};
use serde_json::Value;
use std::time::Instant;

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
const MAX_RETRIES: u32 = 2;
const RATE_LIMIT_INTERVAL_MS: u64 = 500;
const DEFAULT_JS_RENDER_WAIT_MS: u64 = 2000;

static RATE_LIMITER: parking_lot::Mutex<u64> = parking_lot::Mutex::new(0);

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
         focus extraction on relevant sections when possible. \
         Set 'render_js' to true for JavaScript-rendered pages (SPA)."
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
                },
                "render_js": {
                    "type": "boolean",
                    "description": "是否启用 JS 渲染获取 SPA 页面内容（默认 false）"
                },
                "render_wait_ms": {
                    "type": "integer",
                    "description": "JS 渲染后等待时间（毫秒，默认 2000）"
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

        if !is_safe_url_deep(url).await {
            return Err(ToolError::permission_denied("WebFetch", "禁止访问内网或私有地址"));
        }

        if !ctx.allow_network {
            return Err(ToolError::permission_denied("WebFetch", "当前上下文不允许网络请求"));
        }

        Ok(())
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap();
        let prompt = input["prompt"].as_str().unwrap_or("").to_string();
        let render_js = input
            .get("render_js")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let render_wait_ms = input
            .get("render_wait_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_JS_RENDER_WAIT_MS);

        let start = Instant::now();
        let mut progress = Vec::new();

        check_rate_limit();

        // JS 渲染分支
        if render_js {
            return self
                .fetch_with_js_rendering(url, &prompt, render_wait_ms, start)
                .await;
        }

        let client = shared_http_client();

        let response = fetch_with_retry(&client, url).await?;

        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        progress.push(ProgressEntry {
            phase: "fetching".into(),
            message: format!("获取响应: HTTP {} ({})", status.as_u16(), content_type),
            percent: Some(30),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        for binary_ct in BINARY_CONTENT_TYPES {
            if content_type.contains(binary_ct) {
                return Ok(ToolResult {
                    content: format!(
                        "## URL: {}\n状态: {}\nContent-Type: {}\n\n[二进制内容，无法提取文本。请使用专门的下载工具处理此类型文件。]",
                        url, status, content_type
                    ),
                    truncated: false,
                    is_error: false,
                    metadata: None,
                    duration_ms: Some(start.elapsed().as_millis() as u64),
                    progress,
                });
            }
        }

        let raw_bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::execution_failed(format!("读取响应失败: {}", e)))?;

        let body = decode_body(&raw_bytes, &content_type);

        let (title, extracted) =
            if content_type.contains("text/html") || content_type.contains("application/xhtml") {
                let cleaner = HtmlCleaner::new();
                progress.push(ProgressEntry {
                    phase: "cleaning".into(),
                    message: "提取页面内容中...".into(),
                    percent: Some(60),
                    timestamp_ms: start.elapsed().as_millis() as u64,
                });
                cleaner.extract_with_title(&body, &prompt, MAX_CONTENT_LENGTH)
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

        progress.push(ProgressEntry {
            phase: "done".into(),
            message: format!("完成，耗时 {}ms", start.elapsed().as_millis()),
            percent: Some(100),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        Ok(ToolResult {
            content,
            truncated,
            is_error: false,
            metadata: None,
            duration_ms: Some(start.elapsed().as_millis() as u64),
            progress,
        })
    }
}

impl WebFetchTool {
    /// JS 渲染路径：启动 headless browser 导航 → 等待 → 提取内容
    async fn fetch_with_js_rendering(
        &self,
        url: &str,
        prompt: &str,
        wait_ms: u64,
        start: Instant,
    ) -> Result<ToolResult, ToolError> {
        let mut progress = Vec::new();
        progress.push(ProgressEntry {
            phase: "rendering".into(),
            message: "启动浏览器引擎...".into(),
            percent: Some(10),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        let pool = axagent_core::browser_automation::shared_browser_pool();
        let mut guard = pool.lock().await;
        if guard.is_none() {
            *guard = Some(
                axagent_core::browser_automation::PlaywrightClient::launch()
                    .await
                    .map_err(|e| ToolError::execution_failed(format!("浏览器启动失败: {}", e)))?,
            );
        }
        let client = guard
            .as_mut()
            .ok_or_else(|| ToolError::execution_failed("浏览器未启动"))?;

        progress.push(ProgressEntry {
            phase: "rendering".into(),
            message: format!("导航到 {}", url),
            percent: Some(30),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        client
            .navigate(url)
            .await
            .map_err(|e| ToolError::execution_failed(format!("页面导航失败: {}", e)))?;

        let actual_wait = wait_ms.min(10_000);
        tokio::time::sleep(std::time::Duration::from_millis(actual_wait)).await;

        progress.push(ProgressEntry {
            phase: "rendering".into(),
            message: "提取渲染后页面内容...".into(),
            percent: Some(70),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        let html = client
            .get_content()
            .await
            .map_err(|e| ToolError::execution_failed(format!("获取页面内容失败: {}", e)))?;

        drop(guard);

        progress.push(ProgressEntry {
            phase: "cleaning".into(),
            message: "清理页面内容...".into(),
            percent: Some(85),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        let cleaner = HtmlCleaner::new();
        let (title, body_text) = cleaner.extract_with_title(&html, prompt, MAX_CONTENT_LENGTH);

        let header = format!(
            "## URL: {}\n标题: {}\n渲染方式: JavaScript ({}ms 等待)\n",
            url, title, actual_wait
        );

        let header_len = header.len();
        let available = MAX_CONTENT_LENGTH.saturating_sub(header_len);

        let truncated = body_text.len() > available;
        let content = if truncated {
            format!(
                "{}\n[提取目标: {}]\n{}\n\n[内容已截断，已显示约 {}/{} 字符]",
                header,
                prompt,
                &body_text[..available],
                available,
                body_text.len()
            )
        } else {
            format!("{}\n[提取目标: {}]\n{}", header, prompt, body_text)
        };

        progress.push(ProgressEntry {
            phase: "done".into(),
            message: format!("JS 渲染抓取完成，耗时 {}ms", start.elapsed().as_millis()),
            percent: Some(100),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        Ok(ToolResult {
            content,
            truncated,
            is_error: false,
            metadata: None,
            duration_ms: Some(start.elapsed().as_millis() as u64),
            progress,
        })
    }
}

// ── HTTP fetch + 编解码（保留不变）──────────────────────────

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
