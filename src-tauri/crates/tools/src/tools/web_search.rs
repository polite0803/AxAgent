use crate::context_keys;
use crate::{ProgressEntry, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_core::html_cleaner::HtmlCleaner;
use axagent_core::search::{
    SearchServiceConfig, estimate_credibility, execute_search_with_config, rerank_search_results,
};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Instant;

const MAX_FETCH_URLS: usize = 3;
const MAX_CONTENT_LENGTH: usize = 60_000;
const MAX_EXPANDED_QUERIES: usize = 3;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn description(&self) -> &str {
        "Search the internet for information. Use this tool ONLY when:\n\
         - The user asks about current/recent events, news, or real-time data (stock prices, weather, sports)\n\
         - The question requires information beyond your training data cutoff\n\
         - The user explicitly asks to search or look something up online\n\
         - You need to verify or update factual claims that may have changed\n\n\
         Do NOT use this tool when:\n\
         - The answer is within your general knowledge (math, coding, general concepts)\n\
         - The user asks for creative writing, opinions, or advice\n\
         - The question is about well-established facts that don't change\n\n\
         Returns web results with titles, snippets, URLs, and extracted page content."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "搜索查询词" }
            },
            "required": ["query"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError> {
        input["query"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("WebSearch", "缺少 query 参数"))?;
        if !ctx.allow_network {
            return Err(ToolError::permission_denied("WebSearch", "当前上下文不允许网络请求"));
        }
        Ok(())
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input["query"].as_str().unwrap();
        let start = Instant::now();

        // 从 ToolContext.extra 读取搜索配置，fallback 到 DDG
        let config = SearchServiceConfig {
            provider_type: ctx
                .extra
                .get(context_keys::SEARCH_PROVIDER_TYPE)
                .cloned()
                .unwrap_or_else(|| "ddg".to_string()),
            endpoint: ctx.extra.get(context_keys::SEARCH_ENDPOINT).cloned(),
            api_key: ctx.extra.get(context_keys::SEARCH_API_KEY).cloned(),
            max_results: ctx
                .extra
                .get(context_keys::SEARCH_MAX_RESULTS)
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            timeout_ms: ctx
                .extra
                .get(context_keys::SEARCH_TIMEOUT_MS)
                .and_then(|s| s.parse().ok())
                .unwrap_or(15000),
            region: ctx.extra.get(context_keys::SEARCH_REGION).cloned(),
            safe_search: ctx
                .extra
                .get(context_keys::SEARCH_SAFE_SEARCH)
                .and_then(|s| s.parse().ok()),
        };

        let mut progress = Vec::new();

        let expansion = axagent_core::search::expand_search_queries(query);
        let queries: Vec<&str> = expansion
            .queries
            .iter()
            .take(MAX_EXPANDED_QUERIES)
            .map(|s| s.as_str())
            .collect();

        let mut all_results: Vec<axagent_core::search::SearchResult> = Vec::new();
        let mut seen_urls: HashSet<String> = HashSet::new();

        let total_queries = queries.len();
        for (idx, q) in queries.iter().enumerate() {
            progress.push(ProgressEntry {
                phase: "searching".into(),
                message: format!("正在搜索 ({}/{})：{}", idx + 1, total_queries, q),
                percent: Some(((idx + 1) as u8 * 100 / total_queries as u8).min(100)),
                timestamp_ms: start.elapsed().as_millis() as u64,
            });

            let resp = execute_search_with_config(&config, q).await;
            match resp {
                Ok(resp) if resp.ok => {
                    for mut r in resp.results {
                        let url_key = r.url.trim_end_matches('/').to_lowercase();
                        if seen_urls.insert(url_key) {
                            r.credibility = Some(estimate_credibility(&r.url));
                            all_results.push(r);
                        }
                    }
                },
                _ => continue,
            }
        }

        all_results.sort_by(|a, b| {
            let a_has = !a.content.is_empty() && !a.url.is_empty();
            let b_has = !b.content.is_empty() && !b.url.is_empty();
            b_has.cmp(&a_has)
        });
        all_results.truncate(10);

        rerank_search_results(query, &mut all_results);

        // 填充 relevance_score
        let query_terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect())
            .filter(|w: &String| !w.is_empty())
            .collect();
        for r in &mut all_results {
            if r.relevance_score.is_none() && !query_terms.is_empty() {
                let title_lower = r.title.to_lowercase();
                let content_lower = r.content.to_lowercase();
                let mut s: f32 = 0.0;
                for qt in &query_terms {
                    if title_lower.contains(qt) {
                        s += 0.3;
                    }
                    if content_lower.contains(qt) {
                        s += 0.1;
                    }
                }
                r.relevance_score = Some(s.min(1.0));
            }
        }

        if all_results.is_empty() {
            return Ok(ToolResult::success(format!("No search results found for '{}'", query)));
        }

        let brief: Vec<String> = all_results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let cred = r
                    .credibility
                    .map(|c| format!(" [可信度:{:.0}]", c * 10.0))
                    .unwrap_or_default();
                format!("{}. {}{}\n   {}\n   {}", i + 1, r.title, cred, r.content, r.url)
            })
            .collect();
        let mut enriched = format!(
            "Web search results for '{}' (expanded queries: {}, provider: {}):\n{}\n\n---\n\n## Full Page Content\n\n",
            query,
            queries.join(", "),
            config.provider_type,
            brief.join("\n")
        );

        let top_urls: Vec<&str> = all_results
            .iter()
            .take(MAX_FETCH_URLS)
            .map(|r| r.url.as_str())
            .filter(|u| !u.is_empty() && u.starts_with("http"))
            .collect();

        if !top_urls.is_empty() {
            let client = axagent_core::search::shared_http_client();
            let cleaner = HtmlCleaner::new();
            let mut fetched = 0usize;

            for (idx, url) in top_urls.iter().enumerate() {
                if fetched >= MAX_FETCH_URLS {
                    break;
                }
                progress.push(ProgressEntry {
                    phase: "fetching".into(),
                    message: format!("正在抓取页面内容 ({}/{})：{}", idx + 1, top_urls.len(), url),
                    percent: None,
                    timestamp_ms: start.elapsed().as_millis() as u64,
                });

                // SSRF 防护：对搜索结果 URL 做 DNS 解析后验证
                if !axagent_core::search::is_safe_url_deep(url).await {
                    continue;
                }
                match client.get(*url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let ct = resp
                            .headers()
                            .get("content-type")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_string();

                        let body = resp.text().await.unwrap_or_default();

                        if ct.contains("text/html") || ct.contains("application/xhtml") {
                            let per_url_limit = MAX_CONTENT_LENGTH / MAX_FETCH_URLS;
                            let text = cleaner.extract_text(&body, per_url_limit);
                            if !text.is_empty() {
                                enriched.push_str(&format!("### Source: {}\n\n{}\n\n", url, text));
                                fetched += 1;
                            }
                        }
                    },
                    _ => continue,
                }
            }
        }

        if enriched.len() > MAX_CONTENT_LENGTH {
            enriched.truncate(MAX_CONTENT_LENGTH);
            enriched.push_str("\n\n[Total content truncated]");
        }

        progress.push(ProgressEntry {
            phase: "done".into(),
            message: format!(
                "搜索完成：{} 条结果，耗时 {}ms",
                all_results.len(),
                start.elapsed().as_millis()
            ),
            percent: Some(100),
            timestamp_ms: start.elapsed().as_millis() as u64,
        });

        let metadata = serde_json::json!({
            "results": all_results.iter().map(|r| serde_json::json!({
                "title": r.title,
                "url": r.url,
                "credibility": r.credibility,
                "relevance_score": r.relevance_score,
            })).collect::<Vec<_>>(),
            "provider": config.provider_type,
            "query": query,
        });

        Ok(ToolResult {
            content: enriched,
            truncated: false,
            is_error: false,
            metadata: Some(metadata),
            duration_ms: Some(start.elapsed().as_millis() as u64),
            progress,
        })
    }
}
