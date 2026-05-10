use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashSet;

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

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input["query"].as_str().unwrap();

        let expansion = axagent_core::search::expand_search_queries(query);
        let queries: Vec<&str> = expansion
            .queries
            .iter()
            .take(MAX_EXPANDED_QUERIES)
            .map(|s| s.as_str())
            .collect();

        let mut all_results: Vec<axagent_core::search::SearchResult> = Vec::new();
        let mut seen_urls: HashSet<String> = HashSet::new();

        for q in &queries {
            if let Ok(resp) = axagent_core::search::execute_search("ddg", None, "", q, 5, 15000).await {
                if resp.ok {
                    for r in resp.results {
                        let url_key = r.url.trim_end_matches('/').to_lowercase();
                        if seen_urls.insert(url_key) {
                            all_results.push(r);
                        }
                    }
                }
            }
        }

        all_results.sort_by(|a, b| {
            let a_has = !a.content.is_empty() && !a.url.is_empty();
            let b_has = !b.content.is_empty() && !b.url.is_empty();
            b_has.cmp(&a_has)
        });
        all_results.truncate(10);

        if all_results.is_empty() {
            return Ok(ToolResult::success(format!(
                "No search results found for '{}'",
                query
            )));
        }

        let brief: Vec<String> = all_results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}\n   {}\n   {}", i + 1, r.title, r.content, r.url))
            .collect();
        let mut enriched = format!(
            "Web search results for '{}' (expanded queries: {}):\n{}\n\n---\n\n## Full Page Content\n\n",
            query,
            queries.join(", "),
            brief.join("\n")
        );

        let top_urls: Vec<&str> = all_results
            .iter()
            .take(MAX_FETCH_URLS)
            .map(|r| r.url.as_str())
            .filter(|u| !u.is_empty() && u.starts_with("http"))
            .collect();

        if !top_urls.is_empty() {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default();

            let mut fetched = 0usize;
            for url in &top_urls {
                if fetched >= MAX_FETCH_URLS {
                    break;
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
                            let text = extract_page_text(&body);
                            if !text.is_empty() {
                                let per_url_limit = MAX_CONTENT_LENGTH / MAX_FETCH_URLS;
                                let truncated = if text.len() > per_url_limit {
                                    format!("{}...\n[Content truncated]", &text[..per_url_limit])
                                } else {
                                    text
                                };
                                enriched.push_str(&format!(
                                    "### Source: {}\n\n{}\n\n",
                                    url, truncated
                                ));
                                fetched += 1;
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }

        if enriched.len() > MAX_CONTENT_LENGTH {
            enriched.truncate(MAX_CONTENT_LENGTH);
            enriched.push_str("\n\n[Total content truncated]");
        }

        Ok(ToolResult::success(enriched))
    }
}

fn extract_page_text(html: &str) -> String {
    let mut doc = scraper::Html::parse_document(html);

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
        if let Some(node) = doc.tree.get_mut(nid) {
            node.detach();
        }
    }

    let content_sel = scraper::Selector::parse(
        "main, article, [role='main'], [role='article'], .content, .post, \
         .article, .entry, #content, #main, .main-content, .post-content, \
         .article-content, .entry-content",
    )
    .unwrap();

    let root = doc.select(&content_sel).next().unwrap_or_else(|| doc.root_element());

    root.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}
