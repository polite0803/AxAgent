// 设计行业适配器
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue, MetricType};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::rules::ValidationError;
use super::super::workflow::{
    DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowInputField, WorkflowStepDef,
};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct DesignIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl DesignIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("design", "设计") }
    }
}

impl Default for DesignIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for DesignIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "name".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "设计项目名称不能为空".to_string(),
            },
            ValidationDef {
                field: "design_type".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "设计类型不能为空".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        // 用户输入变量通过 input_mapping 注入 AgentNode context
        let user_inputs = HashMap::from([
            ("project_brief".to_string(), "project_brief".to_string()),
            ("brand_style".to_string(), "brand_style".to_string()),
            ("design_target".to_string(), "design_target".to_string()),
        ]);
        vec![
            WorkflowStepDef {
                name: "产品 UI 设计".to_string(),
                description: "根据设计需求产出产品界面设计方案".to_string(),
                prompt: Some(
                    "你是一名 UI 设计师。请根据设计需求产出产品界面设计方案。\
                     输出 JSON {layout, color_scheme, typography, component_list}"
                        .to_string(),
                ),
                tools: vec![
                    "OpcCreateContentAsset".to_string(),
                    "FileWrite".to_string(),
                    "WebSearch".to_string(),
                ],
                agent_profile_id: None,
                error_handling: "stop".to_string(),
                order: 1,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "品牌视觉设计".to_string(),
                description: "设计品牌视觉体系与 VI 规范".to_string(),
                prompt: Some(
                    "你是一名品牌设计师。请设计品牌视觉体系与 VI 规范。\
                     输出 JSON {brand_identity, logo_concept, brand_guidelines, touchpoints}"
                        .to_string(),
                ),
                tools: vec![
                    "OpcCreateContentAsset".to_string(),
                    "OpcCreateLandingPage".to_string(),
                ],
                agent_profile_id: None,
                error_handling: "continue".to_string(),
                order: 2,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "设计系统构建".to_string(),
                description: "构建可复用的设计系统与组件库".to_string(),
                prompt: Some(
                    "你是一名设计系统专家。请构建可复用的设计系统与组件库。\
                     输出 JSON {design_tokens, component_library, usage_rules, documentation}"
                        .to_string(),
                ),
                tools: vec!["OpcCreateContentAsset".to_string(), "FileWrite".to_string()],
                agent_profile_id: None,
                error_handling: "continue".to_string(),
                order: 3,
                inputs: user_inputs,
            },
        ]
    }

    fn input_fields(&self) -> Vec<WorkflowInputField> {
        vec![
            WorkflowInputField {
                key: "project_brief".to_string(),
                label: "设计需求".to_string(),
                field_type: "textarea".to_string(),
                required: true,
                placeholder: Some(
                    "如：面向企业用户的 SaaS 管理后台，需要信息清晰、操作高效".to_string(),
                ),
                default: None,
            },
            WorkflowInputField {
                key: "brand_style".to_string(),
                label: "品牌风格".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：科技感、简约、活力".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "design_target".to_string(),
                label: "设计目标".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：提升转化率、增强品牌辨识度".to_string()),
                default: None,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "design_completion_rate".to_string(),
                name: "设计完成率".to_string(),
            },
            KpiCalculationDef {
                key: "design_review_cycles".to_string(),
                name: "设计评审周期".to_string(),
            },
            KpiCalculationDef {
                key: "user_satisfaction_score".to_string(),
                name: "用户满意度评分".to_string(),
            },
            KpiCalculationDef {
                key: "design_system_adoption".to_string(),
                name: "设计系统采纳率".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "design_review_reminder",
                "设计评审提醒",
                vec![
                    AutomationCondition::StatusIs { status: "ready_for_review".to_string() },
                    AutomationCondition::EntityTypeIs { entity_type: "design_project".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "design_team".to_string(),
                    message: "设计项目已准备好评审".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "a11y_check_reminder",
                "无障碍检查提醒",
                vec![
                    AutomationCondition::FieldBelow {
                        field: "accessibility_score".to_string(),
                        threshold: 0.8,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "design_project".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "design_team".to_string(),
                    message: "无障碍评分低于 80%，需要改进".to_string(),
                }],
            ),
        ]
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "active_projects".to_string(),
                title: "进行中设计项目".to_string(),
                kpi_key: "active_design_projects".to_string(),
            },
            DashboardCardDef {
                id: "completion_rate".to_string(),
                title: "设计完成率".to_string(),
                kpi_key: "design_completion_rate".to_string(),
            },
            DashboardCardDef {
                id: "review_cycles".to_string(),
                title: "平均评审周期".to_string(),
                kpi_key: "design_review_cycles".to_string(),
            },
        ]
    }

    fn requires_approval(&self) -> bool {
        true
    }

    async fn validate(
        &self,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match entity_type {
            "design_project" => {
                if entity_data.get("name").is_none_or(|p| p.as_str().is_none_or(|s| s.is_empty())) {
                    errors.push(ValidationError::new("name", "设计项目名称不能为空"));
                }
                if let Some(design_type) = entity_data.get("design_type") {
                    let valid_types = ["ui_ux", "brand", "illustration", "motion", "product"];
                    if design_type.as_str().is_none_or(|t| !valid_types.contains(&t)) {
                        errors.push(ValidationError::new("design_type", "无效的设计类型"));
                    }
                }
            },
            "design_review" => {
                if entity_data
                    .get("reviewer")
                    .is_none_or(|r| r.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("reviewer", "评审人不能为空"));
                }
                if let Some(score) = entity_data.get("score") {
                    if score.as_f64().is_none_or(|s| !(0.0..=10.0).contains(&s)) {
                        errors.push(ValidationError::new("score", "评分必须在 0-10 之间"));
                    }
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn entity_types(&self) -> Vec<String> {
        vec!["design_project".to_string(), "design_review".to_string()]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("user_research", "用户研究", "通过用户访谈和可用性测试收集需求")
                .with_order(1),
            WorkflowStep::new("concept_design", "概念设计", "基于研究结果创建设计概念和线框图")
                .with_order(2),
            WorkflowStep::new("visual_design", "视觉设计", "完成高保真视觉设计和交互原型")
                .with_order(3),
            WorkflowStep::new("design_review", "设计评审", "组织设计评审会议，收集反馈并迭代")
                .with_order(4),
            WorkflowStep::new("delivery", "交付交付", "输出设计规范、资源包和开发文档")
                .with_order(5),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.define_automation_rules()
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        self.define_dashboard_cards()
            .into_iter()
            .map(|c| DashboardCard::new(&c.id, &c.title, &c.kpi_key, "—"))
            .collect()
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("design_completion_rate", "设计完成率", "%", MetricType::Percentage),
            KpiDefinition::new("design_review_cycles", "平均评审周期", "天", MetricType::Count),
            KpiDefinition::new("user_satisfaction_score", "用户满意度", "分", MetricType::Gauge),
            KpiDefinition::new(
                "design_system_adoption",
                "设计系统采纳率",
                "%",
                MetricType::Percentage,
            ),
        ]
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let _ = time_range;
        let now = chrono::Utc::now().timestamp();
        Ok(vec![
            KpiValue {
                key: "design_completion_rate".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "design_review_cycles".to_string(),
                value: 0.0,
                target: None,
                unit: Some("天".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "user_satisfaction_score".to_string(),
                value: 0.0,
                target: None,
                unit: Some("分".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "design_system_adoption".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
        ])
    }
}
