// AI 研究与咨询行业适配器
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::project::ProjectStatus;
use super::super::rules::ValidationError;
use super::super::workflow::{
    DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowInputField, WorkflowStepDef,
};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct AiResearchIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl AiResearchIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("ai_research", "AI 研究与咨询") }
    }
}

impl Default for AiResearchIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for AiResearchIndustryAdapter {
    impl_industry_base!();

    fn input_fields(&self) -> Vec<WorkflowInputField> {
        vec![
            WorkflowInputField {
                key: "research_topic".to_string(),
                label: "研究主题".to_string(),
                field_type: "string".to_string(),
                required: true,
                placeholder: Some("如：大模型推理优化、多模态融合".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "scope".to_string(),
                label: "研究范围".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：仅限开源方案、包含商业产品".to_string()),
                default: None,
            },
        ]
    }

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "topic".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "研究主题不能为空".to_string(),
            },
            ValidationDef {
                field: "duration_minutes".to_string(),
                r#type: "range".to_string(),
                error_message: "咨询时长必须在 1-480 分钟之间".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        let user_inputs = HashMap::from([
            ("research_topic".to_string(), "research_topic".to_string()),
            ("scope".to_string(), "scope".to_string()),
        ]);
        vec![
            WorkflowStepDef {
                name: "需求分析".to_string(),
                description: "定义研究主题、范围与可交付物".to_string(),
                prompt: Some(
                    "你是一名 AI 研究负责人。请将用户的研究需求拆解为明确的范围、方法、交付物与评估标准。输出 JSON {topic, scope, deliverables, success_criteria}".to_string(),
                ),
                tools: vec![
                    "OpcListProjects".to_string(),
                    "OpcCreateProject".to_string(),
                    "OpcSearchWiki".to_string(),
                ],
                agent_profile_id: Some("opc-ai_researcher-ai-research-director".to_string()),
                error_handling: "stop".to_string(),
                order: 1,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "文献调研".to_string(),
                description: "扫描论文与技术资料，提取关键进展".to_string(),
                prompt: Some(
                    "你是一名 AI 文献分析师。请基于网络搜索与内部知识库，调研目标方向的最新论文与技术资料，提取关键突破并评估可信度。输出 JSON {key_findings, source_references, confidence}".to_string(),
                ),
                tools: vec![
                    "WebSearch".to_string(),
                    "FileRead".to_string(),
                    "OpcSearchWiki".to_string(),
                ],
                agent_profile_id: Some("opc-ai_researcher-ai-literature-analyst".to_string()),
                error_handling: "continue".to_string(),
                order: 2,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "模型评测".to_string(),
                description: "对比主流模型能力与适用场景".to_string(),
                prompt: Some(
                    "你是一名 AI 模型评测专家。请对比主流大模型在该场景下的能力边界、性能与成本，给出选型建议。输出 JSON {model_scores, tradeoffs, recommendation}".to_string(),
                ),
                tools: vec![
                    "Bash".to_string(),
                    "FileRead".to_string(),
                    "FileWrite".to_string(),
                ],
                agent_profile_id: Some("opc-ai_researcher-ai-benchmark-analyst".to_string()),
                error_handling: "continue".to_string(),
                order: 3,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "报告输出".to_string(),
                description: "整合研究结论，输出报告并记录 KPI".to_string(),
                prompt: Some(
                    "你是一名 AI 报告分析师。请整合前序研究成果，撰写结构化研究报告，输出结论与后续建议。输出 JSON {summary, conclusion, next_steps}".to_string(),
                ),
                tools: vec![
                    "FileWrite".to_string(),
                    "OpcListKpis".to_string(),
                    "OpcRecordKpi".to_string(),
                    "OpcSendNotification".to_string(),
                ],
                agent_profile_id: Some("opc-ai_researcher-ai-report-analyst".to_string()),
                error_handling: "continue".to_string(),
                order: 4,
                inputs: user_inputs,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "task_completion_rate".to_string(),
                name: "任务完成率".to_string(),
            },
            KpiCalculationDef {
                key: "research_projects_completed".to_string(),
                name: "完成研究项目".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "completion".to_string(),
                title: "任务完成率".to_string(),
                kpi_key: "task_completion_rate".to_string(),
            },
            DashboardCardDef {
                id: "projects".to_string(),
                title: "完成项目".to_string(),
                kpi_key: "research_projects_completed".to_string(),
            },
        ]
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
            "research_project" => {
                if entity_data.get("topic").is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("topic", "研究主题不能为空"));
                }
                if let Some(model) = entity_data.get("model_preference") {
                    let valid_models = ["gpt-4", "claude-3", "gemini-pro", "mixtral"];
                    if model.as_str().is_none_or(|m| !valid_models.contains(&m)) {
                        errors.push(ValidationError::new("model_preference", "不支持的模型偏好"));
                    }
                }
            },
            "consulting_session" => {
                if let Some(duration) = entity_data.get("duration_minutes") {
                    if duration.as_i64().is_none_or(|d| d <= 0 || d > 480) {
                        errors.push(ValidationError::new(
                            "duration_minutes",
                            "咨询时长必须在 1-480 分钟之间",
                        ));
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

        let completed = data.count_projects(&[ProjectStatus::Completed], from, to).await? as f64;
        // 任务完成率 = 已完成 / (已完成 + 进行中 + 规划中 + 暂停)，Cancelled 不计入分母
        let active = data.count_projects(&[ProjectStatus::Active], from, to).await? as f64;
        let planning = data.count_projects(&[ProjectStatus::Planning], from, to).await? as f64;
        let paused = data.count_projects(&[ProjectStatus::Paused], from, to).await? as f64;
        let total = completed + active + planning + paused;
        let completion_rate = if total > 0.0 {
            ((completed / total) * 1000.0).round() / 10.0
        } else {
            0.0
        };

        Ok(vec![
            KpiValue {
                key: "task_completion_rate".to_string(),
                value: completion_rate,
                target: Some(80.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "research_projects_completed".to_string(),
                value: completed,
                target: Some(10.0),
                unit: Some("个".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "task_completion_rate".to_string(),
                name: "任务完成率".to_string(),
                description: "已完成研究项目占全部项目（不含取消）的百分比".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(80.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "research_projects_completed".to_string(),
                name: "完成研究项目".to_string(),
                description: "已完成的研究项目数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(10.0),
                unit: Some("个".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "research_project".to_string(),
            "consulting_session".to_string(),
            "experiment".to_string(),
            "paper".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("propose", "立项", "定义研究主题与可交付物").with_order(1),
            WorkflowStep::new("research", "研究", "执行模型实验与资料收集").with_order(2),
            WorkflowStep::new("deliver", "交付", "输出报告并收集反馈").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "research_sync",
                "研究进展同步",
                vec![AutomationCondition::EntityTypeIs {
                    entity_type: "research_project".to_string(),
                }],
                vec![AutomationAction::SendNotification {
                    target: "#research".to_string(),
                    message: "研究项目进展已更新，请同步关注".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "feedback_collect",
                "客户反馈收集",
                vec![
                    AutomationCondition::EntityTypeIs {
                        entity_type: "research_project".to_string(),
                    },
                    AutomationCondition::StatusIs { status: "completed".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "客户邮箱".to_string(),
                    message: "项目已完成，请填写反馈".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![DashboardCard::new("projects", "完成项目", "research_projects_completed", "个")]
    }
}
