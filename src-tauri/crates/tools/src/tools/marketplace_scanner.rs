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

use crate::tools::arxiv_scanner::ArxivScanner;
use crate::tools::csdn_scanner::CsdnScanner;
use crate::tools::dribbble_scanner::DribbbleScanner;
use crate::tools::github_discussions_scanner::GitHubDiscussionsScanner;
use crate::tools::github_issue_scanner::GitHubIssueScanner;
use crate::tools::hacker_news_scanner::HackerNewsScanner;
use crate::tools::huggingface_scanner::HuggingFaceScanner;
use crate::tools::linkedin_scanner::LinkedInScanner;
use crate::tools::package_ecosystem_scanner::PackageEcosystemScanner;
use crate::tools::product_hunt_scanner::ProductHuntScanner;
use crate::tools::reddit_scanner::RedditScanner;
use crate::tools::stackoverflow_scanner::StackOverflowScanner;
use crate::tools::twitter_scanner::TwitterScanner;
use crate::tools::upwork_scanner::UpworkScanner;
use crate::tools::xianyu_scanner::XianyuScanner;
use crate::tools::zhihu_scanner::ZhihuScanner;
use crate::tools::zhubajie_scanner::ZhubajieScanner;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 内联评估类型（原 axagent-analysis-engine::opc::evaluator 精简版） ──

/// 需求类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum DemandType {
    #[default]
    Unknown,
    ToolSoftware,
    ContentCreation,
    Design,
    Development,
    Operations,
    Marketing,
    Education,
    EnterpriseService,
    Outsourcing,
    Consulting,
}

/// 价格区间
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PriceRange {
    min: f64,
    max: f64,
    currency: String,
    confidence: f64,
}

/// 需求价值评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemandEvaluation {
    demand_id: String,
    pain_score: f64,
    existing_solutions: u32,
    market_gap_score: f64,
    commercial_value_score: f64,
    opportunity_level: String,
    confidence: f64,
    demand_type: DemandType,
    extracted_price_range: Option<PriceRange>,
    market_fit_score: f64,
}

impl DemandEvaluation {
    pub fn opportunity_level(&self) -> &str {
        match self.commercial_value_score {
            v if v >= 80.0 => "very_high",
            v if v >= 60.0 => "high",
            v if v >= 40.0 => "medium",
            _ => "low",
        }
    }
}

/// 简化版需求评估：基于关键词密度返回启发式评分
fn evaluate_demand_value(
    demand_id: &str,
    title: &str,
    description: &str,
    _known_competitors: Option<u32>,
) -> DemandEvaluation {
    let text = format!("{} {}", title, description).to_lowercase();
    let pain_keywords = [
        "urgent",
        "critical",
        "frustrated",
        "painful",
        "deadline",
        "urgent",
        "急需",
        "痛点",
        "麻烦",
        "困难",
    ];
    let pain_hits = pain_keywords.iter().filter(|k| text.contains(*k)).count() as f64;
    let pain_score = (pain_hits * 20.0).clamp(10.0, 90.0);
    let market_gap_score = 50.0; // 无真实评估引擎时取中位
    let commercial_value_score = (pain_score * 0.5 + market_gap_score * 0.5).round();

    DemandEvaluation {
        demand_id: demand_id.to_string(),
        pain_score,
        existing_solutions: 0,
        market_gap_score,
        commercial_value_score,
        opportunity_level: String::new(),
        confidence: 0.3,
        demand_type: DemandType::Unknown,
        extracted_price_range: None,
        market_fit_score: 50.0,
    }
}

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
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
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
            contact_email: raw.contact_email,
            contact_phone: raw.contact_phone,
            source_url: Some(raw.url),
            raw_snapshot: raw.snapshot,
            status: "new".to_string(),
            confidence: 0.0,
        }
    }

    /// 将线索转换为评估请求
    pub fn to_evaluation_input(&self) -> (String, String, String) {
        (self.id.clone(), self.title.clone(), self.description.clone())
    }
}

/// 带评估结果的需求线索
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedDemandLead {
    pub lead: DemandLead,
    pub evaluation: DemandEvaluation,
}

impl EvaluatedDemandLead {
    pub fn new(lead: DemandLead, evaluation: DemandEvaluation) -> Self {
        Self { lead, evaluation }
    }

    pub fn value_score(&self) -> f64 {
        self.evaluation.commercial_value_score
    }

    pub fn opportunity_level(&self) -> String {
        self.evaluation.opportunity_level.clone()
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
    /// 记录被禁用的平台名称
    disabled_platforms: std::collections::HashSet<String>,
}

impl AggregateMarketplaceScanner {
    pub fn new() -> Self {
        Self { scanners: Vec::new(), disabled_platforms: std::collections::HashSet::new() }
    }

    pub fn add_scanner(&mut self, scanner: Box<dyn MarketplaceScanner>) {
        self.scanners.push(scanner);
    }

    /// 禁用指定平台的扫描器
    pub fn disable_scanner(&mut self, platform: &str) {
        self.disabled_platforms.insert(platform.to_string());
        tracing::info!(platform = platform, "[AggregateMarketplaceScanner] 已禁用扫描器");
    }

    /// 启用指定平台的扫描器
    pub fn enable_scanner(&mut self, platform: &str) {
        self.disabled_platforms.remove(platform);
        tracing::info!(platform = platform, "[AggregateMarketplaceScanner] 已启用扫描器");
    }

    /// 检查扫描器是否启用
    pub fn is_scanner_enabled(&self, platform: &str) -> bool {
        !self.disabled_platforms.contains(platform)
    }

    /// 列出所有已注册的平台及其启用状态
    pub fn list_scanners(&self) -> Vec<(String, bool)> {
        self.scanners
            .iter()
            .map(|s| {
                let p = s.platform().to_string();
                let enabled = !self.disabled_platforms.contains(&p);
                (p, enabled)
            })
            .collect()
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
            let platform = scanner.platform();
            if !self.is_scanner_enabled(platform) {
                tracing::debug!(
                    platform = platform,
                    "[AggregateMarketplaceScanner] 扫描器已禁用，跳过"
                );
                continue;
            }

            match scanner.search(q).await {
                Ok(raw) => {
                    for r in raw {
                        leads.push(DemandLead::new_from_raw(r));
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        platform = platform,
                        error = %e,
                        "[AggregateMarketplaceScanner] 扫描器失败，跳过"
                    );
                },
            }
        }
        Ok(leads)
    }

    /// 搜索需求线索并执行价值评估
    ///
    /// 完整流水线：扫描 → 评估 → 筛选高价值 → 排序
    pub async fn search_and_evaluate(&self, q: &str) -> Result<Vec<EvaluatedDemandLead>, String> {
        let leads = self.search_all(q).await?;

        let mut evaluated: Vec<EvaluatedDemandLead> = leads
            .into_iter()
            .map(|lead| {
                let (id, title, desc) = lead.to_evaluation_input();
                let evaluation = evaluate_demand_value(&id, &title, &desc, None);
                EvaluatedDemandLead::new(lead, evaluation)
            })
            .collect();

        // 按价值分排序
        evaluated.sort_by(|a, b| {
            b.value_score().partial_cmp(&a.value_score()).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(evaluated)
    }

    /// 搜索并筛选高价值需求
    ///
    /// # 参数
    /// - `q`: 搜索关键词
    /// - `min_score`: 最低价值分阈值（默认 50.0）
    ///
    /// # 返回
    /// 高价值需求列表（已按价值分排序）
    pub async fn search_high_value(
        &self,
        q: &str,
        min_score: f64,
    ) -> Result<Vec<EvaluatedDemandLead>, String> {
        let evaluated = self.search_and_evaluate(q).await?;
        let filtered: Vec<EvaluatedDemandLead> =
            evaluated.into_iter().filter(|e| e.value_score() >= min_score).collect();
        Ok(filtered)
    }

    /// 对已有线索进行批量评估
    pub fn evaluate_leads(&self, leads: Vec<DemandLead>) -> Vec<EvaluatedDemandLead> {
        leads
            .into_iter()
            .map(|lead| {
                let (id, title, desc) = lead.to_evaluation_input();
                let evaluation = evaluate_demand_value(&id, &title, &desc, None);
                EvaluatedDemandLead::new(lead, evaluation)
            })
            .collect()
    }
}

impl Default for AggregateMarketplaceScanner {
    fn default() -> Self {
        let mut scanner = Self::new();
        // 注册技术社区扫描器
        scanner.add_scanner(Box::new(RedditScanner::new()));
        scanner.add_scanner(Box::new(HackerNewsScanner::new()));
        scanner.add_scanner(Box::new(GitHubIssueScanner::new()));
        scanner.add_scanner(Box::new(GitHubDiscussionsScanner::new()));
        scanner.add_scanner(Box::new(StackOverflowScanner::new()));
        // 注册产品生态扫描器
        scanner.add_scanner(Box::new(ProductHuntScanner::new()));
        scanner.add_scanner(Box::new(HuggingFaceScanner::new()));
        scanner.add_scanner(Box::new(PackageEcosystemScanner::new()));
        // 注册研究动态扫描器
        scanner.add_scanner(Box::new(ArxivScanner::new()));
        // 注册社交媒体扫描器
        scanner.add_scanner(Box::new(TwitterScanner::new()));
        // 注册中国市场扫描器
        scanner.add_scanner(Box::new(ZhubajieScanner::new()));
        scanner.add_scanner(Box::new(XianyuScanner::new()));
        // 注册 B2B/企业需求扫描器
        scanner.add_scanner(Box::new(LinkedInScanner::new()));
        // 注册中国开发者社区扫描器
        scanner.add_scanner(Box::new(ZhihuScanner::new()));
        scanner.add_scanner(Box::new(CsdnScanner::csdn()));
        scanner.add_scanner(Box::new(CsdnScanner::juejin()));
        // 注册设计需求扫描器
        scanner.add_scanner(Box::new(DribbbleScanner::new()));
        // 注册国际外包市场扫描器
        scanner.add_scanner(Box::new(UpworkScanner::new()));
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
                .or_else(|| {
                    // 尝试从 name / author / owner 字段提取
                    item.get("name")
                        .and_then(|v| v.as_str())
                        .or_else(|| item.get("author").and_then(|v| v.as_str()))
                        .or_else(|| item.get("owner").and_then(|v| v.as_str()))
                })
                .map(|s| s.to_string());

            // 提取邮箱：尝试多个常见字段名
            let contact_email = item
                .get("contact_email")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("email").and_then(|v| v.as_str()))
                .or_else(|| item.get("e-mail").and_then(|v| v.as_str()))
                .or_else(|| item.get("mail").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .or_else(|| extract_email_from_text(&description));

            // 提取电话：尝试多个常见字段名
            let contact_phone = item
                .get("contact_phone")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("phone").and_then(|v| v.as_str()))
                .or_else(|| item.get("mobile").and_then(|v| v.as_str()))
                .or_else(|| item.get("tel").and_then(|v| v.as_str()))
                .or_else(|| item.get("wechat").and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            leads.push(RawLead {
                platform: self.platform.to_string(),
                title,
                description,
                url: item_url,
                price_text,
                contact,
                contact_email,
                contact_phone,
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
                description: "需要一个响应式官网，5-8个页面，包含产品展示、关于我们、联系方式。需要支持移动端。联系邮箱：zhang@example.com".to_string(),
                url: "https://example.com/lead/1".to_string(),
                price_text: Some("8000-15000元".to_string()),
                contact: Some("张经理".to_string()),
                contact_email: Some("zhang@example.com".to_string()),
                contact_phone: Some("13800138000".to_string()),
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
                contact_email: Some("li@design-studio.com".to_string()),
                contact_phone: None,
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
                contact_email: None,
                contact_phone: Some("微信: wangzhuren_biz".to_string()),
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

// ── 工具函数 ──────────────────────────────────────────────────

/// 从文本中提取邮箱地址
///
/// 使用简单的正则匹配模式，尝试从描述文本中提取邮箱。
/// 这是一个辅助手段，主要依赖 API 返回的结构化字段。
fn extract_email_from_text(text: &str) -> Option<String> {
    // 匹配常见邮箱格式：xxx@yyy.zzz
    let email_patterns = [
        // 带 @ 的标准邮箱
        r"[\w.+-]+@[\w-]+\.[\w.-]+",
    ];

    for pattern in email_patterns {
        if let Some(captures) = regex_find(text, pattern) {
            return Some(captures);
        }
    }
    None
}

/// 简单的正则查找（不依赖 regex crate）
///
/// 注意：这里使用简化的字符串匹配，仅作为兜底方案。
/// 如果需要更完善的正则支持，建议引入 regex crate。
fn regex_find(text: &str, pattern: &str) -> Option<String> {
    // 简单实现：检查是否包含 @ 符号的文本
    if pattern.contains('@') {
        // 查找包含 @ 的子串
        let bytes = text.as_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if byte == b'@' {
                // 向前找用户名部分
                let mut start = i;
                while start > 0
                    && (bytes[start - 1].is_ascii_alphanumeric()
                        || bytes[start - 1] == b'.'
                        || bytes[start - 1] == b'+'
                        || bytes[start - 1] == b'-'
                        || bytes[start - 1] == b'_')
                {
                    start -= 1;
                }

                // 向后找域名部分
                let mut end = i + 1;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric()
                        || bytes[end] == b'.'
                        || bytes[end] == b'-'
                        || bytes[end] == b'_')
                {
                    end += 1;
                }

                if end > start && end - start > 5 {
                    // 确保有域名后缀
                    let email = &text[start..end];
                    if email.contains('.') {
                        return Some(email.to_string());
                    }
                }
            }
        }
    }
    None
}

/// 判断错误是否为网络环境问题（网络集成测试专用）
///
/// 覆盖两类环境性失败，用于网络集成测试「离线/CI 网络不可达或服务端限流时跳过」：
/// - 网络层错误（连接失败、DNS 解析失败、超时等，来自 reqwest 等）
/// - HTTP 限流/服务端错误（429 限流、403 拒绝、5xx 临时故障）
///
/// 注意：不匹配 4xx 中的其他错误（如 400），避免掩盖请求构造类的真实逻辑缺陷。
#[cfg(test)]
pub(crate) fn is_network_env_error(err: &str) -> bool {
    let err_lower = err.to_lowercase();

    // 网络层错误
    if err_lower.contains("connection")
        || err_lower.contains("dns")
        || err_lower.contains("timed out")
        || err_lower.contains("error sending request")
        || err_lower.contains("timeout")
    {
        return true;
    }

    // HTTP 限流 / 服务端临时错误（各平台错误格式略有差异，如「状态码 429」/「状态码: 429」/「status 429」）
    [
        "状态码 429",
        "状态码: 429",
        "status 429",
        "status: 429",
        "状态码 403",
        "状态码: 403",
        "status 403",
        "status: 403",
        "状态码 5",
        "状态码: 5",
        "status 5",
        "status: 5",
    ]
    .iter()
    .any(|pattern| err_lower.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_aggregate_scanner() {
        let scanner = AggregateMarketplaceScanner::new();
        assert!(scanner.disabled_platforms.is_empty());
    }

    #[test]
    fn test_is_network_env_error() {
        // 网络层错误（reqwest 等）
        assert!(is_network_env_error("ArXiv API 请求失败: error sending request for url"));
        assert!(is_network_env_error("连接失败: connection reset by peer"));
        assert!(is_network_env_error("DNS 解析失败"));
        assert!(is_network_env_error("请求超时: operation timed out"));

        // HTTP 限流 / 服务端临时错误（CI 数据中心 IP 常见）
        assert!(is_network_env_error("ArXiv API 返回状态码 429"));
        assert!(is_network_env_error("HN Algolia API 返回状态码: 429"));
        assert!(is_network_env_error("API 返回状态码 503: Service Unavailable"));
        assert!(is_network_env_error("ArXiv API 返回状态码 403"));

        // 真实逻辑错误不应被跳过
        assert!(!is_network_env_error("ArXiv API 返回状态码 400"));
        assert!(!is_network_env_error("未配置 API Token，无法调用官方 API"));
        assert!(!is_network_env_error("响应解析失败: unexpected token"));
    }

    #[test]
    fn test_disable_and_enable_scanner() {
        let mut scanner = AggregateMarketplaceScanner::new();
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("test_platform")));

        // 默认启用
        assert!(scanner.is_scanner_enabled("test_platform"));

        // 禁用
        scanner.disable_scanner("test_platform");
        assert!(!scanner.is_scanner_enabled("test_platform"));

        // 重新启用
        scanner.enable_scanner("test_platform");
        assert!(scanner.is_scanner_enabled("test_platform"));
    }

    #[test]
    fn test_list_scanners_status() {
        let mut scanner = AggregateMarketplaceScanner::new();
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("platform_a")));
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("platform_b")));

        // 禁用 platform_a
        scanner.disable_scanner("platform_a");

        let status = scanner.list_scanners();
        assert_eq!(status.len(), 2);

        let platform_a_status = status.iter().find(|(p, _)| p == "platform_a");
        assert!(platform_a_status.is_some());
        assert!(!platform_a_status.unwrap().1); // disabled

        let platform_b_status = status.iter().find(|(p, _)| p == "platform_b");
        assert!(platform_b_status.is_some());
        assert!(platform_b_status.unwrap().1); // enabled
    }

    #[tokio::test]
    async fn test_search_all_skips_disabled_scanners() {
        let mut scanner = AggregateMarketplaceScanner::new();
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("enabled_platform")));
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("disabled_platform")));

        // 禁用一个扫描器
        scanner.disable_scanner("disabled_platform");

        // 搜索
        let results = scanner.search_all("test").await.unwrap();

        // 应该只包含 enabled_platform 的结果
        assert!(results.iter().all(|r| r.platform != "disabled_platform"));
        assert!(results.iter().any(|r| r.platform == "enabled_platform"));
    }

    #[tokio::test]
    async fn test_search_all_without_disabled_scanners() {
        let mut scanner = AggregateMarketplaceScanner::new();
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("platform_a")));

        // 不禁用任何扫描器
        let results = scanner.search_all("test").await.unwrap();

        // 应该包含 platform_a 的结果
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.platform == "platform_a"));
    }

    #[test]
    fn test_disable_nonexistent_platform() {
        let mut scanner = AggregateMarketplaceScanner::new();
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("test_platform")));

        // 禁用不存在的平台不应报错
        scanner.disable_scanner("nonexistent_platform");

        // 原平台仍应启用
        assert!(scanner.is_scanner_enabled("test_platform"));
    }

    #[tokio::test]
    async fn test_search_and_evaluate() {
        let mut scanner = AggregateMarketplaceScanner::new();
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("test")));

        let results = scanner.search_and_evaluate("test").await.unwrap();

        assert!(!results.is_empty(), "应返回评估后的线索");
        for evaluated in &results {
            assert!(evaluated.value_score() >= 0.0 && evaluated.value_score() <= 100.0);
            assert!(!evaluated.opportunity_level().is_empty());
        }

        // 验证已按价值分排序
        for i in 0..results.len().saturating_sub(1) {
            assert!(results[i].value_score() >= results[i + 1].value_score());
        }
    }

    #[tokio::test]
    async fn test_search_high_value() {
        let mut scanner = AggregateMarketplaceScanner::new();
        scanner.add_scanner(Box::new(MockMarketplaceScanner::new("test")));

        let results = scanner.search_high_value("test", 0.0).await.unwrap();
        assert!(!results.is_empty(), "阈值为0时应返回所有结果");

        let results = scanner.search_high_value("test", 100.0).await.unwrap();
        assert!(results.is_empty(), "阈值为100时应无结果");
    }

    #[test]
    fn test_evaluate_leads() {
        let scanner = AggregateMarketplaceScanner::new();

        let leads = vec![DemandLead {
            id: "test-1".to_string(),
            platform: "test".to_string(),
            title: "高价值需求".to_string(),
            description: "这是一个非常紧急且昂贵的痛点问题".to_string(),
            budget_min: None,
            budget_max: None,
            budget_currency: "CNY".to_string(),
            contact_name: None,
            contact_email: None,
            contact_phone: None,
            source_url: None,
            raw_snapshot: serde_json::Value::Null,
            status: "new".to_string(),
            confidence: 0.0,
        }];

        let evaluated = scanner.evaluate_leads(leads);
        assert_eq!(evaluated.len(), 1);
        assert!(evaluated[0].value_score() >= 0.0);
    }

    #[test]
    fn test_evaluated_demand_lead() {
        let lead = DemandLead {
            id: "test".to_string(),
            platform: "test".to_string(),
            title: "Test".to_string(),
            description: "Description".to_string(),
            budget_min: None,
            budget_max: None,
            budget_currency: "CNY".to_string(),
            contact_name: None,
            contact_email: None,
            contact_phone: None,
            source_url: None,
            raw_snapshot: serde_json::Value::Null,
            status: "new".to_string(),
            confidence: 0.0,
        };

        let evaluation = evaluate_demand_value("test", "Test", "Description", None);

        let evaluated = EvaluatedDemandLead::new(lead, evaluation);
        assert!(evaluated.value_score() >= 0.0);
        assert!(!evaluated.opportunity_level().is_empty());
    }

    #[test]
    fn test_extract_email_from_text() {
        // 测试标准邮箱格式
        let text = "请联系我：test@example.com 获取详细信息";
        let email = extract_email_from_text(text);
        assert_eq!(email, Some("test@example.com".to_string()));

        // 测试多个邮箱时只提取第一个
        let text = "联系 a@b.com 或 c@d.org";
        let email = extract_email_from_text(text);
        assert_eq!(email, Some("a@b.com".to_string()));

        // 测试无邮箱的情况
        let text = "这是一段没有邮箱的文本";
        let email = extract_email_from_text(text);
        assert!(email.is_none());

        // 测试带特殊字符的邮箱
        let text = "我的邮箱是 user.name+tag@domain.co.uk";
        let email = extract_email_from_text(text);
        assert!(email.is_some());
    }

    #[test]
    fn test_contact_info_in_demand_lead() {
        // 测试 RawLead 到 DemandLead 的联系方式转换
        let raw = RawLead {
            platform: "test".to_string(),
            title: "Test".to_string(),
            description: "Description".to_string(),
            url: "https://example.com".to_string(),
            price_text: None,
            contact: Some("张三".to_string()),
            contact_email: Some("zhangsan@example.com".to_string()),
            contact_phone: Some("13800000000".to_string()),
            snapshot: serde_json::Value::Null,
        };

        let lead = DemandLead::new_from_raw(raw);
        assert_eq!(lead.contact_name, Some("张三".to_string()));
        assert_eq!(lead.contact_email, Some("zhangsan@example.com".to_string()));
        assert_eq!(lead.contact_phone, Some("13800000000".to_string()));
    }
}
