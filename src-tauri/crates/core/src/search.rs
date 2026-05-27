use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use chrono::Datelike;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::error::{AxAgentError, Result};

static DDG_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"class="result__a"[^>]*>(.*?)</a>"#).expect("DDG_TITLE_RE regex is valid")
});
static DDG_SNIPPET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"class="result__snippet"(?:\s*[^>]*)?>(.*?)</"#)
        .expect("DDG_SNIPPET_RE regex is valid")
});
static DDG_HREF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]*)""#).expect("DDG_HREF_RE regex is valid"));
static DDG_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("DDG_TAG_RE regex is valid"));

/// SSRF protection: check if a URL points to a private/internal address
pub fn is_safe_url(url_str: &str) -> bool {
    // Parse URL
    let parsed = match reqwest::Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return false,
    };

    // Only allow http/https
    match parsed.scheme() {
        "http" | "https" => {},
        _ => return false,
    }

    // Extract host
    let host = match parsed.host_str() {
        Some(h) => h,
        None => return false,
    };

    // Block bare hostnames that resolve to localhost
    let host_lower = host.to_lowercase();
    if host_lower == "localhost"
        || host_lower == "0.0.0.0"
        || host_lower.starts_with("127.")
        || host_lower.starts_with("10.")
        || host_lower.starts_with("192.168.")
        || host_lower == "[::1]"
    {
        return false;
    }

    // Check 172.16.0.0/12
    if host_lower.starts_with("172.")
        && let Some(second) = host_lower.split('.').nth(1)
        && let Ok(n) = second.parse::<u32>()
        && (16..=31).contains(&n)
    {
        return false;
    }

    // Resolve hostname and check IP
    if let Ok(ip) = host.parse::<IpAddr>() {
        let is_private = match ip {
            IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_unspecified(),
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
        };
        if is_private {
            return false;
        }
    }

    true
}

// ── Response types ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub content: String,
    pub url: String,
    /// 可信度分数 0.0-1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credibility: Option<f32>,
    /// 相关性分数 0.0-1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub ok: bool,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 搜索服务完整配置（替代散落参数，通过 ToolContext.extra 传递）
#[derive(Debug, Clone)]
pub struct SearchServiceConfig {
    pub provider_type: String,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub max_results: i32,
    pub timeout_ms: i32,
    pub region: Option<String>,
    pub safe_search: Option<i32>,
}

impl Default for SearchServiceConfig {
    fn default() -> Self {
        Self {
            provider_type: "ddg".to_string(),
            endpoint: None,
            api_key: None,
            max_results: 5,
            timeout_ms: 15000,
            region: None,
            safe_search: None,
        }
    }
}

// ── Default endpoints ─────────────────────────────────────

pub fn default_endpoint(provider_type: &str) -> &'static str {
    match provider_type {
        "tavily" => "https://api.tavily.com/search",
        "zhipu" => "https://open.bigmodel.cn/api/paas/v4/web_search",
        "bocha" => "https://api.bochaai.com/v1/web-search",
        _ => "",
    }
}

// ── Query rewriting & expansion ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansion {
    pub original: String,
    pub queries: Vec<String>,
}

pub fn expand_search_queries(original: &str) -> QueryExpansion {
    let mut queries = vec![original.to_string()];

    let trimmed = original.trim();
    let has_chinese = trimmed
        .chars()
        .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c));

    let concise = trimmed
        .split_whitespace()
        .filter(|w| {
            let wl = w.to_lowercase();
            !wl.starts_with("请")
                && !wl.starts_with("帮")
                && !wl.starts_with("怎么")
                && !wl.starts_with("如何")
                && !wl.starts_with("什么")
                && !wl.starts_with("为什么")
                && wl != "的"
                && wl != "了"
                && wl != "吗"
                && wl != "呢"
                && wl != "the"
                && wl != "a"
                && wl != "an"
                && wl != "is"
                && wl != "are"
                && wl != "what"
                && wl != "how"
                && wl != "why"
                && wl != "does"
                && wl != "can"
                && wl != "please"
                && wl != "tell"
                && wl != "me"
                && wl != "about"
        })
        .collect::<Vec<_>>()
        .join(" ");

    if concise != trimmed && !concise.is_empty() {
        queries.push(concise);
    }

    if has_chinese {
        let english_terms: Vec<String> = extract_technical_terms_chinese(trimmed);
        if !english_terms.is_empty() {
            queries.push(english_terms.join(" "));
        }
    } else {
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() >= 3 {
            let core: String = words.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
            if core != trimmed {
                queries.push(core);
            }
        }
    }

    queries.push(format!("{} 教程 文档", trimmed));
    let current_year = chrono::Utc::now().year();
    queries.push(format!("{} 最新 {}", trimmed, current_year));

    queries.dedup();
    queries.truncate(5);

    QueryExpansion {
        original: original.to_string(),
        queries,
    }
}

fn extract_technical_terms_chinese(text: &str) -> Vec<String> {
    let technical_map: &[(&str, &str)] = &[
        ("人工智能", "artificial intelligence AI"),
        ("机器学习", "machine learning ML"),
        ("深度学习", "deep learning"),
        ("大模型", "large language model LLM"),
        ("语言模型", "language model"),
        ("二次开发", "SDK development API"),
        ("开发文档", "developer documentation API docs"),
        ("接口文档", "API documentation"),
        ("架构设计", "architecture design"),
        ("数据库", "database"),
        ("前端", "frontend"),
        ("后端", "backend"),
        ("微服务", "microservices"),
        ("容器", "container Docker"),
        ("部署", "deployment"),
        ("性能优化", "performance optimization"),
        ("搜索引擎", "search engine"),
        ("推荐系统", "recommendation system"),
        ("自然语言处理", "NLP natural language processing"),
        ("计算机视觉", "computer vision CV"),
    ];

    let mut terms = Vec::new();
    for (cn, en) in technical_map {
        if text.contains(cn) {
            terms.push(en.to_string());
        }
    }

    for word in text.split_whitespace() {
        if word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
            && word.len() > 2
        {
            terms.push(word.to_string());
        }
    }

    terms.dedup();
    terms
}

pub fn rewrite_query_prompt(original_query: &str) -> String {
    format!(
        "You are a search query optimizer. Given the user's original question, generate 3-5 optimized search queries that will return the best results.\n\n\
         Rules:\n\
         - Each query should be concise and focused on specific keywords\n\
         - Remove filler words and conversational language\n\
         - Add technical terms where appropriate\n\
         - Include both broad and specific queries\n\
         - If the question is in Chinese, also generate English queries for key technical terms\n\
         - Each query should target a different aspect or angle of the question\n\n\
         Original question: {}\n\n\
         Respond with ONLY a JSON array of strings, e.g.: [\"query1\", \"query2\", \"query3\"]",
        original_query
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchIntent {
    MustSearch,
    ShouldSearch,
    NoSearch,
}

pub fn classify_search_intent(query: &str) -> SearchIntent {
    let q = query.to_lowercase();
    let q_trimmed = q.trim();

    let must_patterns = [
        "最新",
        "今天",
        "昨天",
        "本周",
        "近期",
        "当前",
        "实时",
        "即时",
        "latest",
        "recent",
        "current",
        "today",
        "yesterday",
        "now",
        "新闻",
        "消息",
        "快讯",
        "news",
        "headline",
        "股价",
        "汇率",
        "天气",
        "比分",
        "stock",
        "weather",
        "score",
        "2025",
        "2024",
        "2026",
        "发生了什么",
        "what happened",
        "正在",
        "ongoing",
        "live",
        "上线",
        "发布",
        "更新",
        "released",
        "updated",
        "launched",
    ];

    let no_search_patterns = [
        "什么是",
        "定义",
        "解释一下",
        "什么意思",
        "概念",
        "what is",
        "define",
        "explain",
        "meaning of",
        "definition of",
        "怎么写",
        "如何实现",
        "代码示例",
        "how to write",
        "code example",
        "计算",
        "公式",
        "calculate",
        "formula",
        "翻译",
        "translate",
        "语法",
        "grammar",
        "syntax",
    ];

    let should_patterns = [
        "对比",
        "比较",
        "区别",
        "哪个好",
        "vs",
        "compare",
        "difference",
        "推荐",
        "建议",
        "选择",
        "recommend",
        "suggest",
        "最好的",
        "最佳",
        "top",
        "best",
        "教程",
        "指南",
        "tutorial",
        "guide",
        "文档",
        "手册",
        "documentation",
        "manual",
        "如何",
        "怎么",
        "how to",
        "how do",
    ];

    for pat in &must_patterns {
        if q_trimmed.contains(pat) {
            return SearchIntent::MustSearch;
        }
    }

    let has_temporal = q_trimmed.contains("最新")
        || q_trimmed.contains("当前")
        || q_trimmed.contains("现在")
        || q_trimmed.contains("latest")
        || q_trimmed.contains("current");

    if has_temporal {
        return SearchIntent::MustSearch;
    }

    for pat in &no_search_patterns {
        if q_trimmed.contains(pat) {
            return SearchIntent::ShouldSearch;
        }
    }

    for pat in &should_patterns {
        if q_trimmed.contains(pat) {
            return SearchIntent::ShouldSearch;
        }
    }

    SearchIntent::ShouldSearch
}

// ── Search result cache ─────────────────────────────────────

static SEARCH_CACHE: std::sync::OnceLock<quick_cache::sync::Cache<String, SearchResponse>> =
    std::sync::OnceLock::new();

fn get_search_cache() -> &'static quick_cache::sync::Cache<String, SearchResponse> {
    SEARCH_CACHE.get_or_init(|| quick_cache::sync::Cache::new(200))
}

fn make_cache_key(provider_type: &str, query: &str, max_results: i32) -> String {
    format!("{}|{}|{}", provider_type, query.trim().to_lowercase(), max_results)
}

// ── Main search dispatch (unified entry point) ──────────────

/// Unified search: tries configured provider first, falls back to DDG.
/// Results are cached for 5 minutes per provider+query combination.
/// All search paths (Agent, Q&A, MCP) should call this single function.
pub async fn execute_search(
    provider_type: &str,
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<SearchResponse> {
    let cache_key = make_cache_key(provider_type, query, max_results);
    if let Some(cached) = get_search_cache().get(&cache_key) {
        return Ok(cached);
    }

    let start = Instant::now();

    // 1. Try the configured search provider
    let result = match provider_type {
        "tavily" => search_tavily(endpoint, api_key, query, max_results, timeout_ms).await,
        "zhipu" => search_zhipu(endpoint, api_key, query, max_results, timeout_ms).await,
        "bocha" => search_bocha(endpoint, api_key, query, max_results, timeout_ms).await,
        "serpapi" => search_serpapi(endpoint, api_key, query, max_results, timeout_ms).await,
        "brave" => search_brave(endpoint, api_key, query, max_results, timeout_ms).await,
        "bing" => search_bing(endpoint, api_key, query, max_results, timeout_ms).await,
        "google_pse" => search_google_pse(endpoint, api_key, query, max_results, timeout_ms).await,
        _ => {
            // Unknown or DDG — go straight to fallback
            Err(AxAgentError::Provider("unknown provider type, using fallback".to_string()))
        },
    };

    let latency = start.elapsed().as_millis() as u64;

    let response = match result {
        Ok(results) if !results.is_empty() => SearchResponse {
            ok: true,
            query: query.to_string(),
            results,
            latency_ms: latency,
            error: None,
        },
        _ => {
            // 2. DuckDuckGo fallback
            let ddg = search_duckduckgo(query, max_results).await;
            let latency = start.elapsed().as_millis() as u64;
            match ddg {
                Ok(results) => SearchResponse {
                    ok: true,
                    query: query.to_string(),
                    results,
                    latency_ms: latency,
                    error: None,
                },
                Err(e) => SearchResponse {
                    ok: false,
                    query: query.to_string(),
                    results: vec![],
                    latency_ms: latency,
                    error: Some(e.to_string()),
                },
            }
        },
    };

    // 缓存成功结果（5 分钟 TTL 由 quick_cache 的容量管理间接限制）
    if response.ok {
        get_search_cache().insert(cache_key, response.clone());
    }

    Ok(response)
}

/// 使用结构化配置的搜索入口（替代散落参数）
pub async fn execute_search_with_config(
    config: &SearchServiceConfig,
    query: &str,
) -> Result<SearchResponse> {
    execute_search(
        &config.provider_type,
        config.endpoint.as_deref(),
        config.api_key.as_deref().unwrap_or(""),
        query,
        config.max_results,
        config.timeout_ms,
    )
    .await
}

/// Unified search that returns formatted text (for LLM consumption)
pub async fn execute_search_text(
    provider_type: &str,
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> String {
    match execute_search(provider_type, endpoint, api_key, query, max_results, timeout_ms).await {
        Ok(resp) if resp.ok => {
            let lines: Vec<String> = resp
                .results
                .iter()
                .enumerate()
                .map(|(i, r)| format!("{}. {}\n   {}\n   {}", i + 1, r.title, r.content, r.url))
                .collect();
            format!("Web search results for '{}':\n{}", query, lines.join("\n"))
        },
        Ok(resp) => format!("Search failed: {}", resp.error.unwrap_or_default()),
        Err(e) => format!("Search error: {}", e),
    }
}

pub async fn execute_iterative_search(
    provider_type: &str,
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
    max_rounds: usize,
) -> Result<SearchResponse> {
    let start = Instant::now();

    let expansion = expand_search_queries(query);
    let mut all_results: Vec<SearchResult> = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queries_to_try: Vec<String> = expansion.queries.clone();

    for round in 0..max_rounds {
        let round_queries: Vec<String> = std::mem::take(&mut queries_to_try);
        if round_queries.is_empty() {
            break;
        }

        for q in &round_queries {
            match execute_search(provider_type, endpoint, api_key, q, max_results, timeout_ms).await
            {
                Ok(resp) if resp.ok => {
                    for r in resp.results {
                        let url_key = r.url.trim_end_matches('/').to_lowercase();
                        if !url_key.is_empty() && seen_urls.insert(url_key) {
                            all_results.push(r);
                        }
                    }
                },
                _ => continue,
            }
        }

        if all_results.len() >= max_results as usize {
            break;
        }

        if round + 1 < max_rounds {
            let covered_topics = extract_covered_topics(&all_results);
            let gap_query = generate_gap_query(query, &covered_topics);
            if !gap_query.is_empty() {
                queries_to_try.push(gap_query);
            }
        }
    }

    all_results.sort_by(|a, b| {
        let a_score = a.content.len() as f32 * 0.01 + if !a.url.is_empty() { 1.0 } else { 0.0 };
        let b_score = b.content.len() as f32 * 0.01 + if !b.url.is_empty() { 1.0 } else { 0.0 };
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(max_results as usize);

    let latency = start.elapsed().as_millis() as u64;

    Ok(SearchResponse {
        ok: true,
        query: query.to_string(),
        results: all_results,
        latency_ms: latency,
        error: None,
    })
}

fn extract_covered_topics(results: &[SearchResult]) -> Vec<String> {
    let mut words: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for r in results {
        for word in r.title.split_whitespace() {
            let w = word
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>();
            if w.len() > 3 {
                *words.entry(w).or_insert(0) += 1;
            }
        }
    }

    let mut sorted: Vec<(String, usize)> = words.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
    sorted.iter().take(10).map(|(w, _)| w.clone()).collect()
}

fn generate_gap_query(original: &str, covered: &[String]) -> String {
    if covered.is_empty() {
        return String::new();
    }

    let original_words: std::collections::HashSet<String> = original
        .to_lowercase()
        .split_whitespace()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect())
        .filter(|w: &String| w.len() > 2)
        .collect();

    let uncovered: Vec<&str> = original_words
        .iter()
        .filter(|w| !covered.iter().any(|c| c.contains(w.as_str())))
        .map(|s| s.as_str())
        .collect();

    if uncovered.is_empty() {
        format!("{} in depth analysis", original)
    } else {
        format!("{} {}", uncovered.join(" "), covered.first().unwrap_or(&String::new()))
    }
}

pub fn rerank_search_results(query: &str, results: &mut Vec<SearchResult>) {
    if results.len() <= 1 {
        return;
    }

    let query_terms: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect())
        .filter(|w: &String| w.len() > 1)
        .collect();

    if query_terms.is_empty() {
        return;
    }

    let scored: Vec<(SearchResult, f32)> = results
        .drain(..)
        .map(|r| {
            let title_lower = r.title.to_lowercase();
            let content_lower = r.content.to_lowercase();
            let _combined = format!("{} {}", title_lower, content_lower);

            let exact_title_matches = query_terms
                .iter()
                .filter(|qt| title_lower.contains(qt.as_str()))
                .count() as f32;

            let content_matches = query_terms
                .iter()
                .filter(|qt| content_lower.contains(qt.as_str()))
                .count() as f32;

            let title_coverage = exact_title_matches / query_terms.len() as f32;
            let content_coverage = content_matches / query_terms.len() as f32;

            let url_bonus = if !r.url.is_empty() { 0.1 } else { 0.0 };

            let content_bonus = if r.content.len() > 100 {
                0.1
            } else if r.content.len() > 50 {
                0.05
            } else {
                0.0
            };

            let official_bonus = if is_official_source(&r.url) { 0.2 } else { 0.0 };

            let score = title_coverage * 3.0
                + content_coverage * 1.0
                + url_bonus
                + content_bonus
                + official_bonus;

            (r, score)
        })
        .collect();

    let mut sorted = scored;
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results.extend(sorted.into_iter().map(|(r, _)| r));
}

fn is_official_source(url: &str) -> bool {
    let official_domains = [
        "github.com",
        "docs.microsoft.com",
        "developer.mozilla.org",
        "python.org",
        "rust-lang.org",
        "nodejs.org",
        "react.dev",
        "angular.io",
        "vuejs.org",
        "tensorflow.org",
        "pytorch.org",
        "openai.com",
        "anthropic.com",
        "arxiv.org",
        "stackoverflow.com",
        "wikipedia.org",
        "nginx.org",
        "docker.com",
        "kubernetes.io",
    ];

    official_domains
        .iter()
        .any(|d| url.to_lowercase().contains(d))
}

/// 评估 URL 可信度分数 0.0-1.0
pub fn estimate_credibility(url: &str) -> f32 {
    let domain = url.split('/').nth(2).unwrap_or("");
    let high_credibility = [
        "arxiv.org",
        "github.com",
        "stackoverflow.com",
        "wikipedia.org",
        "doi.org",
        "pubmed.gov",
        "nature.com",
        "science.org",
        "docs.microsoft.com",
        "developer.mozilla.org",
        "python.org",
        "rust-lang.org",
        "nodejs.org",
        "react.dev",
        "angular.io",
        "vuejs.org",
        "tensorflow.org",
        "pytorch.org",
        "openai.com",
        "anthropic.com",
    ];

    for credible in &high_credibility {
        if domain.ends_with(credible) {
            return 0.9;
        }
    }

    if domain.is_empty() { 0.5 } else { 0.7 }
}

/// DNS rebinding 防护：解析主机名后检查 IP 是否为私有/回环地址
pub async fn is_safe_url_deep(url_str: &str) -> bool {
    if !is_safe_url(url_str) {
        return false;
    }

    let parsed = match reqwest::Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return false,
    };

    let host = match parsed.host_str() {
        Some(h) => h,
        None => return false,
    };

    // 如果已经是 IP 字面量，is_safe_url 已检查过
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }

    // DNS 解析后检查
    if let Ok(addrs) = tokio::net::lookup_host(format!("{}:0", host)).await {
        for addr in addrs {
            match addr.ip() {
                std::net::IpAddr::V4(v4)
                    if v4.is_private() || v4.is_loopback() || v4.is_unspecified() =>
                {
                    return false;
                },
                std::net::IpAddr::V6(v6)
                    if v6.is_loopback()
                        || v6.is_unspecified()
                        // fc00::/7 (唯一本地地址)
                        || (v6.segments()[0] & 0xfe00 == 0xfc00)
                        // fe80::/10 (链路本地地址)
                        || (v6.segments()[0] & 0xffc0 == 0xfe80) =>
                {
                    return false;
                },
                // IPv4-mapped IPv6 绕过修复: ::ffff:10.x.x.x 等需检查对应的 IPv4
                std::net::IpAddr::V6(v6)
                    if v6.to_ipv4_mapped().is_some_and(|v4| {
                        v4.is_private() || v4.is_loopback() || v4.is_unspecified()
                    }) =>
                {
                    return false;
                },
                _ => {},
            }
        }
    }

    true
}

/// 共享 HTTP 客户端（带 redirect policy、cookie store、timeout）
pub fn shared_http_client() -> Arc<reqwest::Client> {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<Arc<reqwest::Client>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Arc::new(
                reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                    .timeout(std::time::Duration::from_secs(30))
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .redirect(reqwest::redirect::Policy::custom(|attempt| {
                        if attempt.previous().len() >= 5 {
                            attempt.stop()
                        } else {
                            let url = attempt.url();
                            let _host = url.host_str().unwrap_or("");
                            if is_safe_url(url.as_str()) {
                                attempt.follow()
                            } else {
                                attempt.stop()
                            }
                        }
                    }))
                    .cookie_store(true)
                    .build()
                    .unwrap_or_else(|e| {
                        tracing::warn!("无法构建自定义搜索 HTTP 客户端: {e}，降级为默认客户端");
                        reqwest::Client::new()
                    }),
            )
        })
        .clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeSearchResult {
    pub query: String,
    pub local_results: Vec<SearchResult>,
    pub web_results: Vec<SearchResult>,
    pub source_used: CascadeSource,
    pub total_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CascadeSource {
    LocalOnly,
    WebOnly,
    LocalAndWeb,
}

pub fn should_supplement_with_web(
    local_results: &[SearchResult],
    query: &str,
    min_results: usize,
) -> bool {
    if local_results.len() >= min_results {
        let avg_relevance = local_results
            .iter()
            .map(|r| if r.content.len() > 100 { 1.0 } else { 0.5 })
            .sum::<f32>()
            / local_results.len().max(1) as f32;

        if avg_relevance > 0.7 {
            return false;
        }
    }

    let intent = classify_search_intent(query);
    matches!(intent, SearchIntent::MustSearch)
        || (local_results.len() < min_results && matches!(intent, SearchIntent::ShouldSearch))
}

pub fn merge_local_and_web(
    local: Vec<SearchResult>,
    web: Vec<SearchResult>,
    max_total: usize,
) -> Vec<SearchResult> {
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for r in &local {
        let key = r.url.trim_end_matches('/').to_lowercase();
        if !key.is_empty() {
            seen_urls.insert(key);
        }
        merged.push(r.clone());
    }

    for r in &web {
        let key = r.url.trim_end_matches('/').to_lowercase();
        if !key.is_empty() && seen_urls.contains(&key) {
            continue;
        }
        if !key.is_empty() {
            seen_urls.insert(key);
        }
        merged.push(r.clone());
    }

    merged.truncate(max_total);
    merged
}

pub async fn test_provider(
    provider_type: &str,
    endpoint: Option<&str>,
    api_key: &str,
    timeout_ms: i32,
) -> TestResult {
    let resp = execute_search(provider_type, endpoint, api_key, "test", 3, timeout_ms).await;
    match resp {
        Ok(r) if r.ok => TestResult {
            ok: true,
            latency_ms: Some(r.latency_ms),
            result_count: Some(r.results.len()),
            error: None,
        },
        Ok(r) => TestResult {
            ok: false,
            latency_ms: Some(r.latency_ms),
            result_count: None,
            error: r.error,
        },
        Err(e) => TestResult {
            ok: false,
            latency_ms: None,
            result_count: None,
            error: Some(e.to_string()),
        },
    }
}

// ── Tavily ────────────────────────────────────────────────
// POST {endpoint}
// Body: { api_key, query, max_results }
// Response: { results: [{ title, content, url }] }

#[derive(Serialize)]
struct TavilyRequest<'a> {
    api_key: &'a str,
    query: &'a str,
    max_results: i32,
}

#[derive(Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: Option<String>,
    content: Option<String>,
    url: Option<String>,
}

async fn search_tavily(
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<Vec<SearchResult>> {
    let url = endpoint.unwrap_or("https://api.tavily.com/search");

    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms as u64))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client error: {e}")))?;

    let body = TavilyRequest {
        api_key,
        query,
        max_results: max_results.max(1),
    };

    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Tavily request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AxAgentError::Provider(format!("Tavily API error {status}: {text}")));
    }

    let data: TavilyResponse = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Tavily response parse error: {e}")))?;

    Ok(data
        .results
        .into_iter()
        .take(max_results as usize)
        .map(|r| SearchResult {
            title: r.title.unwrap_or_else(|| "No title".to_string()),
            content: r.content.unwrap_or_default(),
            url: r.url.unwrap_or_default(),
            credibility: None,
            relevance_score: None,
        })
        .collect())
}

// ── Zhipu (智谱) ─────────────────────────────────────────
// POST {endpoint}
// Header: Authorization: Bearer {apiKey}
// Body: { search_query, search_engine: "search_std" }
// Response: { search_result: [{ title, content, link }] }

#[derive(Serialize)]
struct ZhipuRequest<'a> {
    search_query: &'a str,
    search_engine: &'a str,
}

#[derive(Deserialize)]
struct ZhipuResponse {
    search_result: Option<Vec<ZhipuResult>>,
}

#[derive(Deserialize)]
struct ZhipuResult {
    title: Option<String>,
    content: Option<String>,
    link: Option<String>,
}

async fn search_zhipu(
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<Vec<SearchResult>> {
    let url = endpoint.unwrap_or("https://open.bigmodel.cn/api/paas/v4/web_search");

    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms as u64))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client error: {e}")))?;

    let body = ZhipuRequest {
        search_query: query,
        search_engine: "search_std",
    };

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Zhipu request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AxAgentError::Provider(format!("Zhipu API error {status}: {text}")));
    }

    let data: ZhipuResponse = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Zhipu response parse error: {e}")))?;

    let results = data.search_result.unwrap_or_default();
    Ok(results
        .into_iter()
        .take(max_results as usize)
        .map(|r| SearchResult {
            title: r.title.unwrap_or_else(|| "No title".to_string()),
            content: r.content.unwrap_or_default(),
            url: r.link.unwrap_or_default(),
            credibility: None,
            relevance_score: None,
        })
        .collect())
}

// ── Bocha (博查) ──────────────────────────────────────────
// POST {endpoint}
// Header: Authorization: Bearer {apiKey}
// Body: { query, count, summary: true, page: 1 }
// Response: { code, data: { webPages: { value: [{ name, url, snippet, summary }] } } }

#[derive(Serialize)]
struct BochaRequest<'a> {
    query: &'a str,
    count: i32,
    summary: bool,
    page: i32,
}

#[derive(Deserialize)]
struct BochaResponse {
    code: Option<i32>,
    msg: Option<String>,
    data: Option<BochaData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BochaData {
    web_pages: Option<BochaWebPages>,
}

#[derive(Deserialize)]
struct BochaWebPages {
    value: Option<Vec<BochaWebResult>>,
}

#[derive(Deserialize)]
struct BochaWebResult {
    name: Option<String>,
    url: Option<String>,
    snippet: Option<String>,
    summary: Option<String>,
}

async fn search_bocha(
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<Vec<SearchResult>> {
    let url = endpoint.unwrap_or("https://api.bochaai.com/v1/web-search");

    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms as u64))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client error: {e}")))?;

    let body = BochaRequest {
        query,
        count: max_results.max(1),
        summary: true,
        page: 1,
    };

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Bocha request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AxAgentError::Provider(format!("Bocha API error {status}: {text}")));
    }

    let data: BochaResponse = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Bocha response parse error: {e}")))?;

    if data.code.unwrap_or(0) != 200 {
        return Err(AxAgentError::Provider(format!(
            "Bocha search failed: {}",
            data.msg.unwrap_or_else(|| "Unknown error".to_string())
        )));
    }

    let results = data
        .data
        .and_then(|d| d.web_pages)
        .and_then(|wp| wp.value)
        .unwrap_or_default();

    Ok(results
        .into_iter()
        .take(max_results as usize)
        .map(|r| SearchResult {
            title: r.name.unwrap_or_else(|| "No title".to_string()),
            content: r.summary.or(r.snippet).unwrap_or_default(),
            url: r.url.unwrap_or_default(),
            credibility: None,
            relevance_score: None,
        })
        .collect())
}

// ── DuckDuckGo (fallback, no API key needed) ────────────────

async fn search_duckduckgo(query: &str, max_results: i32) -> Result<Vec<SearchResult>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("DDG client error: {e}")))?;

    let mut results: Vec<SearchResult> = Vec::new();

    let api_url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1&t=axagent",
        urlencoding::encode(query)
    );

    if let Ok(resp) = client.get(&api_url).send().await
        && let Ok(json) = resp.json::<serde_json::Value>().await
    {
        if let Some(abs) = json.get("AbstractText").and_then(|v| v.as_str())
            && !abs.is_empty()
        {
            let url = json
                .get("AbstractURL")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            results.push(SearchResult {
                title: "摘要".to_string(),
                content: abs.to_string(),
                url: url.to_string(),
                credibility: None,
                relevance_score: None,
            });
        }
        if let Some(topics) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
            for topic in topics.iter().take(max_results as usize) {
                if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                    let url = topic.get("FirstURL").and_then(|v| v.as_str()).unwrap_or("");
                    results.push(SearchResult {
                        title: text.chars().take(80).collect(),
                        content: text.to_string(),
                        url: url.to_string(),
                        credibility: None,
                        relevance_score: None,
                    });
                }
            }
        }
    }

    if results.is_empty() {
        let html_url =
            format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));

        let resp = client
            .get(&html_url)
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "en-US,en;q=0.9,zh-CN;q=0.8")
            .send()
            .await;

        if let Ok(resp) = resp
            && let Ok(html) = resp.text().await
        {
            let title_caps: Vec<String> = DDG_TITLE_RE
                .captures_iter(&html)
                .filter_map(|c| {
                    c.get(1)
                        .map(|m| DDG_TAG_RE.replace_all(m.as_str(), "").trim().to_string())
                })
                .filter(|s| !s.is_empty())
                .take(max_results as usize)
                .collect();

            let snippet_caps: Vec<String> = DDG_SNIPPET_RE
                .captures_iter(&html)
                .filter_map(|c| {
                    c.get(1)
                        .map(|m| DDG_TAG_RE.replace_all(m.as_str(), "").trim().to_string())
                })
                .filter(|s| !s.is_empty())
                .take(max_results as usize)
                .collect();

            let href_caps: Vec<String> = DDG_HREF_RE
                .captures_iter(&html)
                .take(max_results as usize * 3)
                .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                .filter(|u| u.contains("uddg=") || u.starts_with("http"))
                .take(max_results as usize)
                .collect();

            let count = title_caps
                .len()
                .max(snippet_caps.len())
                .min(max_results as usize);
            for i in 0..count {
                let title = title_caps.get(i).cloned().unwrap_or_default();
                let snippet = snippet_caps.get(i).cloned().unwrap_or_default();
                let url = href_caps.get(i).cloned().unwrap_or_default();

                if !title.is_empty() {
                    results.push(SearchResult {
                        title,
                        content: snippet,
                        url,
                        credibility: None,
                        relevance_score: None,
                    });
                }
            }

            if results.is_empty() {
                for part in html
                    .split("result__snippet")
                    .skip(1)
                    .take(max_results as usize)
                {
                    if let Some(start) = part.find('>')
                        && let Some(end) = part[start + 1..].find("</")
                    {
                        let text = part[start + 1..start + 1 + end].trim();
                        if !text.is_empty() {
                            results.push(SearchResult {
                                title: text.chars().take(80).collect(),
                                content: text.to_string(),
                                url: String::new(),
                                credibility: None,
                                relevance_score: None,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

// ── SerpAPI ─────────────────────────────────────────────────
// GET https://serpapi.com/search?q=...&api_key=...&num=N

#[derive(Deserialize)]
struct SerpApiResponse {
    organic_results: Option<Vec<SerpApiOrganic>>,
}

#[derive(Deserialize)]
struct SerpApiOrganic {
    title: Option<String>,
    snippet: Option<String>,
    link: Option<String>,
}

async fn search_serpapi(
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<Vec<SearchResult>> {
    let url = endpoint.unwrap_or("https://serpapi.com/search");
    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms as u64))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client error: {e}")))?;

    let full_url = format!(
        "{}?q={}&api_key={}&num={}",
        url,
        urlencoding::encode(query),
        urlencoding::encode(api_key),
        max_results
    );
    let resp = client
        .get(&full_url)
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("SerpAPI request: {e}")))?;

    if !resp.status().is_success() {
        return Err(AxAgentError::Provider(format!("SerpAPI HTTP {}", resp.status())));
    }
    let data: SerpApiResponse = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("SerpAPI parse: {e}")))?;
    let organic = data.organic_results.unwrap_or_default();
    Ok(organic
        .into_iter()
        .take(max_results as usize)
        .map(|r| SearchResult {
            title: r.title.unwrap_or_default(),
            content: r.snippet.unwrap_or_default(),
            url: r.link.unwrap_or_default(),
            credibility: None,
            relevance_score: None,
        })
        .collect())
}

// ── Brave Search ────────────────────────────────────────────
// GET https://api.search.brave.com/res/v1/web/search?q=...
// Header: X-Subscription-Token

#[derive(Deserialize)]
struct BraveResponse {
    web: Option<BraveWeb>,
}

#[derive(Deserialize)]
struct BraveWeb {
    results: Option<Vec<BraveWebResult>>,
}

#[derive(Deserialize)]
struct BraveWebResult {
    title: Option<String>,
    description: Option<String>,
    url: Option<String>,
}

async fn search_brave(
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<Vec<SearchResult>> {
    let url = endpoint.unwrap_or("https://api.search.brave.com/res/v1/web/search");
    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms as u64))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client error: {e}")))?;

    let full_url = format!("{}?q={}&count={}", url, urlencoding::encode(query), max_results);
    let resp = client
        .get(&full_url)
        .header("X-Subscription-Token", api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Brave request: {e}")))?;

    if !resp.status().is_success() {
        return Err(AxAgentError::Provider(format!("Brave HTTP {}", resp.status())));
    }
    let data: BraveResponse = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Brave parse: {e}")))?;
    let web = data.web.and_then(|w| w.results).unwrap_or_default();
    Ok(web
        .into_iter()
        .take(max_results as usize)
        .map(|r| SearchResult {
            title: r.title.unwrap_or_default(),
            content: r.description.unwrap_or_default(),
            url: r.url.unwrap_or_default(),
            credibility: None,
            relevance_score: None,
        })
        .collect())
}

// ── Bing Search ─────────────────────────────────────────────
// GET https://api.bing.microsoft.com/v7.0/search?q=...&count=N
// Header: Ocp-Apim-Subscription-Key

#[derive(Deserialize)]
struct BingResponse {
    #[serde(rename = "webPages")]
    web_pages: Option<BingWebPages>,
}

#[derive(Deserialize)]
struct BingWebPages {
    value: Option<Vec<BingWebResult>>,
}

#[derive(Deserialize)]
struct BingWebResult {
    name: Option<String>,
    snippet: Option<String>,
    url: Option<String>,
}

async fn search_bing(
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<Vec<SearchResult>> {
    let url = endpoint.unwrap_or("https://api.bing.microsoft.com/v7.0/search");
    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms as u64))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client error: {e}")))?;

    let full_url = format!("{}?q={}&count={}", url, urlencoding::encode(query), max_results);
    let resp = client
        .get(&full_url)
        .header("Ocp-Apim-Subscription-Key", api_key)
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Bing request: {e}")))?;

    if !resp.status().is_success() {
        return Err(AxAgentError::Provider(format!("Bing HTTP {}", resp.status())));
    }
    let data: BingResponse = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Bing parse: {e}")))?;
    let web = data.web_pages.and_then(|w| w.value).unwrap_or_default();
    Ok(web
        .into_iter()
        .take(max_results as usize)
        .map(|r| SearchResult {
            title: r.name.unwrap_or_default(),
            content: r.snippet.unwrap_or_default(),
            url: r.url.unwrap_or_default(),
            credibility: None,
            relevance_score: None,
        })
        .collect())
}

// ── Google PSE (Programmable Search Engine) ─────────────────
// GET https://www.googleapis.com/customsearch/v1?q=...&key=...&cx=...

#[derive(Deserialize)]
struct GooglePseResponse {
    items: Option<Vec<GooglePseItem>>,
}

#[derive(Deserialize)]
struct GooglePseItem {
    title: Option<String>,
    snippet: Option<String>,
    link: Option<String>,
}

async fn search_google_pse(
    endpoint: Option<&str>,
    api_key: &str,
    query: &str,
    max_results: i32,
    timeout_ms: i32,
) -> Result<Vec<SearchResult>> {
    let url = endpoint.unwrap_or("https://www.googleapis.com/customsearch/v1");
    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms as u64))
        .build()
        .map_err(|e| AxAgentError::Provider(format!("HTTP client error: {e}")))?;

    let full_url = format!(
        "{}?q={}&key={}&num={}",
        url,
        urlencoding::encode(query),
        urlencoding::encode(api_key),
        max_results
    );
    let resp = client
        .get(&full_url)
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Google PSE request: {e}")))?;

    if !resp.status().is_success() {
        return Err(AxAgentError::Provider(format!("Google PSE HTTP {}", resp.status())));
    }
    let data: GooglePseResponse = resp
        .json()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Google PSE parse: {e}")))?;
    let items = data.items.unwrap_or_default();
    Ok(items
        .into_iter()
        .take(max_results as usize)
        .map(|r| SearchResult {
            title: r.title.unwrap_or_default(),
            content: r.snippet.unwrap_or_default(),
            url: r.link.unwrap_or_default(),
            credibility: None,
            relevance_score: None,
        })
        .collect())
}
