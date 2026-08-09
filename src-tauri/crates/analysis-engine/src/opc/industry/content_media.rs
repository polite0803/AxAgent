// 内容与媒体行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct ContentMediaIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl ContentMediaIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("content_media", "内容与媒体") }
    }
}

impl Default for ContentMediaIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for ContentMediaIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "title".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "文章标题不能为空".to_string(),
            },
            ValidationDef {
                field: "duration_seconds".to_string(),
                r#type: "positive".to_string(),
                error_message: "视频时长必须大于零".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            // 爆款内容生成：选题策划 → 内容创作 → 优化打磨
            WorkflowStepDef {
                name: "选题策划".to_string(),
                description: "分析当前热点和用户需求，策划具有爆款潜力的内容主题".to_string(),
                prompt: Some(
                    "你是一名资深内容策划专家。请分析当前热点和用户需求，策划具有爆款潜力的内容主题。输出 JSON {topic, angle, target_audience, hook_points}".to_string(),
                ),
                tools: vec!["OpcListBlogPosts".to_string(), "WebSearch".to_string()],
                agent_profile_id: Some("opc-cmo-cmo-content-strategist".to_string()),
                error_handling: "stop".to_string(),
                order: 1,
            },
            WorkflowStepDef {
                name: "内容创作".to_string(),
                description: "根据选题创作高质量文章或内容".to_string(),
                prompt: Some(
                    "你是一名资深内容创作专家。请根据选题创作高质量文章。使用 OpcCreateBlogPost 发布博客。输出 JSON {post_id, title, summary, tags}".to_string(),
                ),
                tools: vec!["OpcCreateBlogPost".to_string(), "FileWrite".to_string(), "WebSearch".to_string()],
                agent_profile_id: Some("opc-cmo-cmo-content-creator".to_string()),
                error_handling: "stop".to_string(),
                order: 2,
            },
            WorkflowStepDef {
                name: "优化打磨".to_string(),
                description: "对内容进行 SEO 优化和传播力增强".to_string(),
                prompt: Some(
                    "你是一名 SEO 优化专家。请对内容进行 SEO 优化和传播力增强。输出 JSON {optimized_title, meta_description, seo_score}".to_string(),
                ),
                tools: vec!["WebSearch".to_string(), "FileRead".to_string()],
                agent_profile_id: Some("opc-cmo-cmo-seo-expert".to_string()),
                error_handling: "continue".to_string(),
                order: 3,
            },
            WorkflowStepDef {
                name: "多平台发布".to_string(),
                description: "将内容发布到多个社交媒体平台".to_string(),
                prompt: Some(
                    "你是一名社交媒体运营专家。请将内容发布到多个社交媒体平台。输出 JSON {platforms, post_urls, scheduling}".to_string(),
                ),
                tools: vec!["OpcCreatePublishSchedule".to_string(), "OpcListPublishSchedules".to_string()],
                agent_profile_id: Some("opc-cmo-cmo-social-manager".to_string()),
                error_handling: "continue".to_string(),
                order: 4,
            },
            WorkflowStepDef {
                name: "IP 打造".to_string(),
                description: "构建个人品牌和 IP 影响力".to_string(),
                prompt: Some(
                    "你是一名品牌策划专家。请构建个人品牌和 IP 影响力。输出 JSON {brand_voice, content_pillars, growth_strategy}".to_string(),
                ),
                tools: vec!["OpcCreateContentAsset".to_string(), "WebSearch".to_string(), "OpcListCustomers".to_string()],
                agent_profile_id: Some("opc-cmo-cmo-brand-strategist".to_string()),
                error_handling: "continue".to_string(),
                order: 5,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "content_published".to_string(),
                name: "内容发布量".to_string(),
            },
            KpiCalculationDef {
                key: "subscriber_growth".to_string(),
                name: "订阅增长".to_string(),
            },
            KpiCalculationDef {
                key: "content_assets_count".to_string(),
                name: "内容资产数".to_string(),
            },
            KpiCalculationDef {
                key: "blog_posts_count".to_string(),
                name: "博客文章数".to_string(),
            },
            KpiCalculationDef {
                key: "landing_pages_count".to_string(),
                name: "落地页数".to_string(),
            },
            KpiCalculationDef {
                key: "publish_schedules_pending".to_string(),
                name: "待发布计划".to_string(),
            },
            KpiCalculationDef {
                key: "publish_schedules_published".to_string(),
                name: "已发布计划".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![DashboardCardDef {
            id: "pub".to_string(),
            title: "发布量".to_string(),
            kpi_key: "content_published".to_string(),
        }]
    }

    fn requires_approval(&self) -> bool {
        false
    }

    async fn validate(
        &self,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match entity_type {
            "article" => {
                if entity_data.get("title").is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("title", "文章标题不能为空"));
                }
                if let Some(word_count) = entity_data.get("word_count") {
                    if word_count.as_i64().is_none_or(|w| w < 100) {
                        errors.push(ValidationError::new("word_count", "文章至少需要 100 字"));
                    }
                }
            },
            "video_content" => {
                if let Some(duration) = entity_data.get("duration_seconds") {
                    if duration.as_i64().is_none_or(|d| d <= 0) {
                        errors.push(ValidationError::new("duration_seconds", "视频时长必须大于零"));
                    }
                }
            },
            "social_post" => {
                if let Some(platform) = entity_data.get("platform") {
                    let valid_platforms =
                        ["weibo", "wechat", "douyin", "xiaohongshu", "twitter", "linkedin"];
                    if platform.as_str().is_none_or(|p| !valid_platforms.contains(&p)) {
                        errors.push(ValidationError::new("platform", "不支持的社交平台"));
                    }
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let Some(data) = self.data_service() else {
            return Ok(Vec::new());
        };
        let (from, to) = (time_range.start, time_range.end);
        let now = chrono::Utc::now().timestamp();

        let content_count = data.count_blog_posts(from, to).await? as f64;
        let page_views = data.sum_blog_post_views(from, to).await?;
        let content_assets_count = data.count_content_assets(from, to).await? as f64;
        let blog_posts_count = data.count_blog_posts(from, to).await? as f64;
        let landing_pages_count = data.count_landing_pages(from, to).await? as f64;
        let schedules_pending = data.count_publish_schedules_pending().await? as f64;
        let schedules_published = data.count_publish_schedules_published(from, to).await? as f64;

        Ok(vec![
            KpiValue {
                key: "content_published".to_string(),
                value: content_count,
                target: Some(20.0),
                unit: Some("篇".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "page_views".to_string(),
                value: page_views,
                target: Some(50000.0),
                unit: Some("次".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "content_assets_count".to_string(),
                value: content_assets_count,
                target: Some(50.0),
                unit: Some("个".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "blog_posts_count".to_string(),
                value: blog_posts_count,
                target: Some(20.0),
                unit: Some("篇".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "landing_pages_count".to_string(),
                value: landing_pages_count,
                target: Some(10.0),
                unit: Some("个".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "publish_schedules_pending".to_string(),
                value: schedules_pending,
                target: Some(10.0),
                unit: Some("个".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "publish_schedules_published".to_string(),
                value: schedules_published,
                target: Some(30.0),
                unit: Some("个".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "content_published".to_string(),
                name: "内容发布量".to_string(),
                description: "已发布博客文章数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(20.0),
                unit: Some("篇".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "page_views".to_string(),
                name: "阅读量".to_string(),
                description: "所有博客文章的累计阅读量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(50000.0),
                unit: Some("次".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "content_assets_count".to_string(),
                name: "内容资产数".to_string(),
                description: "内容资产总数".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(50.0),
                unit: Some("个".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "blog_posts_count".to_string(),
                name: "博客文章数".to_string(),
                description: "博客文章发布数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(20.0),
                unit: Some("篇".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "landing_pages_count".to_string(),
                name: "落地页数".to_string(),
                description: "落地页总数".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(10.0),
                unit: Some("个".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "publish_schedules_pending".to_string(),
                name: "待发布计划".to_string(),
                description: "待发布的内容计划数".to_string(),
                metric_type: super::super::analytics::MetricType::Gauge,
                target: Some(10.0),
                unit: Some("个".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "publish_schedules_published".to_string(),
                name: "已发布计划".to_string(),
                description: "已完成发布的计划数".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(30.0),
                unit: Some("个".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "article".to_string(),
            "video_content".to_string(),
            "social_post".to_string(),
            "newsletter".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("create", "创作", "产出内容草稿").with_order(1),
            WorkflowStep::new("review", "审核", "内容质量检查").with_order(2),
            WorkflowStep::new("publish", "发布", "多平台发布").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "publish_notify",
                "发布通知",
                vec![AutomationCondition::EntityTypeIs { entity_type: "article".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "#content".to_string(),
                    message: "新内容已发布".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "content_long_article",
                "长文标记",
                vec![AutomationCondition::FieldExceeds {
                    field: "word_count".to_string(),
                    threshold: 3000.0,
                }],
                vec![AutomationAction::UpdateField {
                    field: "is_featured".to_string(),
                    value: serde_json::json!(true),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![DashboardCard::new("pub", "发布量", "content_published", "篇")]
    }
}
