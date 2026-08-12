// SPDX-License-Identifier: AGPL-3.0-only

//! 市场平台扫描工具
//!
//! 抽象统一的需求线索采集接口，供闲鱼 / 猪八戒等平台连接器实现。
//! **合法合规优先**：所有连接器均通过平台**官方开放 API** 调用，不涉及爬虫或非法抓取。
//!
//! 三种连接器模式：
//! - `ApiMarketplaceScanner`：官方 API 调用（Token 认证）
//! - `MockMarketplaceScanner`：Mock 数据（测试/演示）
//! - `ManualMarketplaceScanner`：手动补录（最轻路径）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::github_issue_scanner::GitHubIssueScanner;
use super::hacker_news_scanner::HackerNewsScanner;
use super::reddit_scanner::RedditScanner;

// ── DTO 定义 ──────────────────────────────────────────────────

/// 原始线索（平台返回的原始数据，未经归一化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLead {
    pub platform: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub price_text: Option<String>,
    pub contact: Option<String>,
    pub snapshot: serde_json::Value,
}

/// 归一化后的需求线索
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandLead {
    pub id: String,
    pub platform: String,
    pub title: String,
    pub description: String,
    pub budget_min: Option<f64>,
    pub budget_max: Option<f64>,
    pub budget_currency: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub source_url: Option<String>,
    pub raw_snapshot: serde_json::Value,
    pub status: String,
    pub confidence: f64,
}

impl DemandLead {
    pub fn new_from_raw(raw: RawLead) -> Self {
        let id = format!("{}_{}", raw.platform, uuid::Uuid::new_v4().simple());
        Self {
            id,
            platform: raw.platform,
            title: raw.title,
            description: raw.description,
            budget_min: None,
            budget_max: None,
            budget_currency: "CNY".to_string(),
            contact_name: raw.contact,
            contact_email: None,
            contact_phone: None,
            source_url: Some(raw.url),
            raw_snapshot: raw.snapshot,
            status: "new".to_string(),
            confidence: 0.0,
        }
    }
}

// ── 扫描 trait ────────────────────────────────────────────────

/// 市场平台扫描器统一接口
///
/// 各平台实现此 trait，负责**通过官方 API** 定向检索与原始数据采集。
#[async_trait]
pub trait MarketplaceScanner: Send + Sync {
    /// 平台标识（如 "xianyu" / "zhubajie"）
    fn platform(&self) -> &'static str;

    /// 按关键词搜索需求线索（通过官方 API）
    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String>;
}

// ── 聚合扫描器 ────────────────────────────────────────────────

/// 多平台聚合扫描器
pub struct AggregateMarketplaceScanner {
    scanners: Vec<Box<dyn MarketplaceScanner>>,
}

impl AggregateMarketplaceScanner {
    pub fn new() -> Self {
        Self { scanners: Vec::new() }
    }

    pub fn add_scanner(&mut self, scanner: Box<dyn MarketplaceScanner>) {
        self.scanners.push(scanner);
    }

    /// 从平台配置批量注册扫描器
    ///
    /// `platform_type` 对应三种连接器：
    /// - `"api"` → `ApiMarketplaceScanner`（官方 API，需配置 API Token）
    /// - `"mock"` → `MockMarketplaceScanner`（模拟数据，用于测试）
    /// - `"manual"` / 其他 → `ManualMarketplaceScanner`（手动补录）
    pub fn add_platform(
        &mut self,
        platform: &str,
        platform_type: &str,
        base_url: Option<&str>,
        config: &serde_json::Value,
    ) {
        match platform_type {
            "api" => {
                self.add_scanner(Box::new(ApiMarketplaceScanner::new(platform, base_url, config)));
            },
            "mock" => {
                self.add_scanner(Box::new(MockMarketplaceScanner::new(platform)));
            },
            _ => self
                .add_scanner(Box::new(ManualMarketplaceScanner::new(platform, base_url, config))),
        }
    }

    pub async fn search_all(&self, q: &str) -> Result<Vec<DemandLead>, String> {
        let mut leads: Vec<DemandLead> = Vec::new();
        for scanner in &self.scanners {
            match scanner.search(q).await {
                Ok(raw) => {
                    for r in raw {
                        leads.push(DemandLead::new_from_raw(r));
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        platform = scanner.platform(),
                        error = %e,
                        "[AggregateMarketplaceScanner] 扫描器失败，跳过"
                    );
                },
            }
        }
        Ok(leads)
    }
}

impl Default for AggregateMarketplaceScanner {
    fn default() -> Self {
        let mut scanner = Self::new();
        // 注册技术社区扫描器（Phase 1 新增）
        scanner.add_scanner(Box::new(RedditScanner::new()));
        scanner.add_scanner(Box::new(HackerNewsScanner::new()));
        scanner.add_scanner(Box::new(GitHubIssueScanner::new(None)));
        scanner
    }
}

// ── 官方 API 连接器 ──────────────────────────────────────────

/// 官方 API 型平台连接器
///
/// 通过平台**官方开放 API** 进行合法合规的需求线索采集。
/// 支持 Token 认证（Bearer / API Key / 自定义 Header）和完整的请求/响应配置。
///
/// ## 配置字段 (`config`)
///
/// | 字段 | 说明 | 默认值 |
/// |------|------|--------|
/// | `api_token` | API 认证 Token（必需） | - |
/// | `auth_type` | 认证方式: `"bearer"` / `"api_key"` / `"custom_header"` | `"bearer"` |
/// | `auth_header` | 自定义认证头名称（`custom_header` 时使用） | `"Authorization"` |
/// | `http_method` | HTTP 方法: `"get"` / `"post"` | `"get"` |
/// | `search_path` | 搜索 API 路径 | `"/api/v1/search"` |
/// | `query_param` | 搜索关键词参数名（GET）或字段名（POST） | `"q"` |
/// | `keyword_field` | 响应中标题字段名 | `"title"` |
/// | `description_field` | 响应中描述字段名 | `"description"` |
/// | `data_wrapper` | 数据包装字段（如 `data`、`results`） | `"data"` |
/// | `timeout_sec` | 请求超时（秒） | `10` |
pub struct ApiMarketplaceScanner {
    platform: &'static str,
    base_url: String,
    api_token: String,
    auth_type: String,
    auth_header: String,
    http_method: String,
    search_path: String,
    query_param: String,
    keyword_field: String,
    description_field: String,
    data_wrapper: String,
    timeout_sec: u64,
}

impl ApiMarketplaceScanner {
    pub fn new(platform: &str, base_url: Option<&str>, config: &serde_json::Value) -> Self {
        let platform_static: &'static str = Box::leak(platform.to_string().into_boxed_str());
        Self {
            platform: platform_static,
            base_url: base_url
                .map(|u| u.to_string())
                .unwrap_or_else(|| "https://api.example.com".to_string()),
            api_token: config.get("api_token").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            auth_type: config
                .get("auth_type")
                .and_then(|v| v.as_str())
                .unwrap_or("bearer")
                .to_string(),
            auth_header: config
                .get("auth_header")
                .and_then(|v| v.as_str())
                .unwrap_or("Authorization")
                .to_string(),
            http_method: config
                .get("http_method")
                .and_then(|v| v.as_str())
                .unwrap_or("get")
                .to_string(),
            search_path: config
                .get("search_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/api/v1/search")
                .to_string(),
            query_param: config
                .get("query_param")
                .and_then(|v| v.as_str())
                .unwrap_or("q")
                .to_string(),
            keyword_field: config
                .get("keyword_field")
                .and_then(|v| v.as_str())
                .unwrap_or("title")
                .to_string(),
            description_field: config
                .get("description_field")
                .and_then(|v| v.as_str())
                .unwrap_or("description")
                .to_string(),
            data_wrapper: config
                .get("data_wrapper")
                .and_then(|v| v.as_str())
                .unwrap_or("data")
                .to_string(),
            timeout_sec: config.get("timeout_sec").and_then(|v| v.as_u64()).unwrap_or(10),
        }
    }

    /// 构建认证头
    fn build_auth_header(&self) -> (String, String) {
        match self.auth_type.as_str() {
            "bearer" => ("Authorization".to_string(), format!("Bearer {}", self.api_token)),
            "api_key" => ("X-API-Key".to_string(), self.api_token.clone()),
            "custom_header" => (self.auth_header.clone(), self.api_token.clone()),
            _ => ("Authorization".to_string(), format!("Bearer {}", self.api_token)),
        }
    }
}

#[async_trait]
impl MarketplaceScanner for ApiMarketplaceScanner {
    fn platform(&self) -> &'static str {
        self.platform
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if self.api_token.is_empty() {
            return Err(format!("[{}] 未配置 API Token，无法调用官方 API", self.platform));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_sec))
            .user_agent("AxAgent/1.0 (demand-discovery)")
            .build()
            .map_err(|e| format!("HTTP 客户端构建失败: {}", e))?;

        let (auth_key, auth_value) = self.build_auth_header();

        let url = format!("{}{}", self.base_url.trim_end_matches('/'), self.search_path);
        tracing::info!(
            platform = self.platform,
            url = %url,
            auth_type = %self.auth_type,
            "[ApiMarketplaceScanner] 发起官方 API 请求"
        );

        let resp = if self.http_method == "post" {
            let body = serde_json::json!({
                &self.query_param: q,
            });
            client
                .post(&url)
                .header(&auth_key, &auth_value)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("POST 请求失败: {}", e))?
        } else {
            let full_url = format!("{}?{}={}", url, self.query_param, q);
            client
                .get(&full_url)
                .header(&auth_key, &auth_value)
                .send()
                .await
                .map_err(|e| format!("GET 请求失败: {}", e))?
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                platform = self.platform,
                status = %status,
                body = %body,
                "[ApiMarketplaceScanner] API 响应异常"
            );
            return Err(format!(
                "API 返回状态码 {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            ));
        }

        let body = resp.text().await.map_err(|e| format!("响应体读取失败: {}", e))?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body).unwrap_or(serde_json::json!([]));

        // 解析响应数据：支持 { "data": [...] } / { "results": [...] } / 直接数组
        let items = if let Some(arr) = parsed.as_array() {
            arr.clone()
        } else if let Some(obj) = parsed.as_object() {
            if let Some(arr) = obj.get(&self.data_wrapper).and_then(|v| v.as_array()) {
                arr.clone()
            } else if let Some(arr) = obj.get("results").and_then(|v| v.as_array()) {
                arr.clone()
            } else {
                tracing::warn!(
                    platform = self.platform,
                    "[ApiMarketplaceScanner] 未找到数据数组字段"
                );
                return Ok(Vec::new());
            }
        } else {
            return Ok(Vec::new());
        };

        let mut leads: Vec<RawLead> = Vec::new();
        for item in &items {
            let title =
                item.get(&self.keyword_field).and_then(|v| v.as_str()).unwrap_or("").to_string();

            if title.is_empty() {
                continue;
            }

            let description = item
                .get(&self.description_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let item_url = item
                .get("url")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("link").and_then(|v| v.as_str()))
                .or_else(|| item.get("source_url").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();

            let price_text = item
                .get("price")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("budget").and_then(|v| v.as_str()))
                .or_else(|| item.get("price_text").and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            let contact = item
                .get("contact_name")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("contact").and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            leads.push(RawLead {
                platform: self.platform.to_string(),
                title,
                description,
                url: item_url,
                price_text,
                contact,
                snapshot: item.clone(),
            });
        }

        tracing::info!(
            platform = self.platform,
            count = leads.len(),
            "[ApiMarketplaceScanner] API 解析完成"
        );

        Ok(leads)
    }
}

// ── Mock / 测试连接器 ────────────────────────────────────────

/// Mock 平台连接器（用于测试和演示，返回固定的模拟数据）
pub struct MockMarketplaceScanner {
    platform: &'static str,
}

impl MockMarketplaceScanner {
    pub fn new(platform: &str) -> Self {
        let platform_static: &'static str = Box::leak(platform.to_string().into_boxed_str());
        Self { platform: platform_static }
    }
}

#[async_trait]
impl MarketplaceScanner for MockMarketplaceScanner {
    fn platform(&self) -> &'static str {
        self.platform
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        let mock_data = vec![
            RawLead {
                platform: self.platform.to_string(),
                title: format!("官网建设 - 中小型企业展示型网站 (关键词: {})", q),
                description: "需要一个响应式官网，5-8个页面，包含产品展示、关于我们、联系方式。需要支持移动端。".to_string(),
                url: "https://example.com/lead/1".to_string(),
                price_text: Some("8000-15000元".to_string()),
                contact: Some("张经理".to_string()),
                snapshot: serde_json::json!({
                    "source": "mock",
                    "category": "web_development",
                    "posted_at": "2026-08-10"
                }),
            },
            RawLead {
                platform: self.platform.to_string(),
                title: format!("Logo 设计 + VI 视觉系统 (关键词: {})", q),
                description: "新品牌需要 Logo 设计，以及完整的 VI 视觉识别系统，包括名片、信封、PPT 模板等。".to_string(),
                url: "https://example.com/lead/2".to_string(),
                price_text: Some("3000-5000元".to_string()),
                contact: Some("李总".to_string()),
                snapshot: serde_json::json!({
                    "source": "mock",
                    "category": "design",
                    "posted_at": "2026-08-09"
                }),
            },
            RawLead {
                platform: self.platform.to_string(),
                title: format!("微信小程序开发 - 预约系统 (关键词: {})", q),
                description: "开发一个微信小程序，用户可以在线预约服务、查看订单、支付。管理员后台管理预约。".to_string(),
                url: "https://example.com/lead/3".to_string(),
                price_text: Some("20000-30000元".to_string()),
                contact: Some("王主任".to_string()),
                snapshot: serde_json::json!({
                    "source": "mock",
                    "category": "mini_program",
                    "posted_at": "2026-08-11"
                }),
            },
        ];

        Ok(mock_data)
    }
}

// ── 手动补录连接器 ──────────────────────────────────────────

/// 手动补录连接器（最轻路径）
///
/// 当平台暂未接入官方 API 时，使用此连接器返回空结果，
/// 引导用户通过 `opc_create_lead` 手动录入需求线索。
pub struct ManualMarketplaceScanner {
    platform: &'static str,
    base_url: Option<String>,
}

impl ManualMarketplaceScanner {
    pub fn new(platform: &str, base_url: Option<&str>, _config: &serde_json::Value) -> Self {
        let platform_static: &'static str = Box::leak(platform.to_string().into_boxed_str());
        Self { platform: platform_static, base_url: base_url.map(|u| u.to_string()) }
    }
}

#[async_trait]
impl MarketplaceScanner for ManualMarketplaceScanner {
    fn platform(&self) -> &'static str {
        self.platform
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        tracing::info!(
            platform = self.platform,
            base_url = self.base_url.as_deref().unwrap_or("-"),
            query = q,
            "[ManualMarketplaceScanner] 请在 OPC 需求发现面板手动录入需求线索"
        );
        Ok(Vec::new())
    }
}
