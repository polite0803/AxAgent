// 游戏开发行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue, MetricType};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct GameDevIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl GameDevIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("game_dev", "游戏开发") }
    }
}

impl Default for GameDevIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for GameDevIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "game_title".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "游戏名称不能为空".to_string(),
            },
            ValidationDef {
                field: "game_engine".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "游戏引擎不能为空".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "概念设计".to_string(),
                description: "确定游戏核心玩法和美术风格".to_string(),
                order: 1,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "原型开发".to_string(),
                description: "开发游戏核心玩法原型".to_string(),
                order: 2,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "内容生产".to_string(),
                description: "关卡、角色、道具等游戏内容开发".to_string(),
                order: 3,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "测试优化".to_string(),
                description: "功能测试、性能优化和Bug修复".to_string(),
                order: 4,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "上线运营".to_string(),
                description: "正式上线和后续运营支持".to_string(),
                order: 5,
                ..Default::default()
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "daily_active_users".to_string(),
                name: "日活跃用户数".to_string(),
            },
            KpiCalculationDef {
                key: "average_session_time".to_string(),
                name: "平均游戏时长".to_string(),
            },
            KpiCalculationDef {
                key: "monetization_rate".to_string(),
                name: "付费转化率".to_string(),
            },
            KpiCalculationDef {
                key: "player_retention".to_string(),
                name: "玩家留存率".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "crash_rate_alert",
                "崩溃率告警",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "crash_rate".to_string(),
                        threshold: 0.02,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "game_build".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "dev_team".to_string(),
                    message: "游戏版本崩溃率超过 2%".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "performance_check",
                "性能检查规则",
                vec![
                    AutomationCondition::FieldBelow {
                        field: "frame_rate".to_string(),
                        threshold: 30.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "game_scene".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "dev_team".to_string(),
                    message: "场景帧率低于 30 FPS，需要优化".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "build_validation",
                "构建验证规则",
                vec![
                    AutomationCondition::StatusIs { status: "build_failed".to_string() },
                    AutomationCondition::EntityTypeIs { entity_type: "game_build".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "ci_team".to_string(),
                    message: "游戏构建失败".to_string(),
                }],
            ),
        ]
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "active_builds".to_string(),
                title: "活跃游戏版本".to_string(),
                kpi_key: "active_builds_count".to_string(),
            },
            DashboardCardDef {
                id: "player_metrics".to_string(),
                title: "玩家数据".to_string(),
                kpi_key: "daily_active_users".to_string(),
            },
            DashboardCardDef {
                id: "performance".to_string(),
                title: "性能指标".to_string(),
                kpi_key: "average_frame_rate".to_string(),
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
            "game_project" => {
                if entity_data
                    .get("game_title")
                    .is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("game_title", "游戏名称不能为空"));
                }
                if entity_data
                    .get("game_engine")
                    .is_none_or(|e| e.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("game_engine", "游戏引擎不能为空"));
                }
                if let Some(genere) = entity_data.get("genre") {
                    let valid_genres = [
                        "action",
                        "adventure",
                        "rpg",
                        "strategy",
                        "simulation",
                        "sports",
                        "puzzle",
                        "platformer",
                    ];
                    if genere.as_str().is_none_or(|g| !valid_genres.contains(&g)) {
                        errors.push(ValidationError::new("genre", "无效的游戏类型"));
                    }
                }
            },
            "game_build" => {
                if entity_data
                    .get("version")
                    .is_none_or(|v| v.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("version", "版本号不能为空"));
                }
                if let Some(platform) = entity_data.get("platform") {
                    let valid_platforms = ["pc", "playstation", "xbox", "switch", "ios", "android"];
                    if platform.as_str().is_none_or(|p| !valid_platforms.contains(&p)) {
                        errors.push(ValidationError::new("platform", "无效的游戏平台"));
                    }
                }
            },
            "game_asset" => {
                if entity_data
                    .get("asset_path")
                    .is_none_or(|a| a.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("asset_path", "资源路径不能为空"));
                }
                if let Some(asset_type) = entity_data.get("asset_type") {
                    let valid_types = ["model", "texture", "audio", "animation", "shader"];
                    if asset_type.as_str().is_none_or(|t| !valid_types.contains(&t)) {
                        errors.push(ValidationError::new("asset_type", "无效的资源类型"));
                    }
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "game_project".to_string(),
            "game_build".to_string(),
            "game_asset".to_string(),
            "game_scene".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("concept_design", "概念设计", "确定游戏核心玩法和美术风格")
                .with_order(1),
            WorkflowStep::new("prototype_dev", "原型开发", "开发游戏核心玩法原型").with_order(2),
            WorkflowStep::new("content_production", "内容生产", "关卡、角色、道具等游戏内容开发")
                .with_order(3),
            WorkflowStep::new("testing_optimization", "测试优化", "功能测试、性能优化和Bug修复")
                .with_order(4),
            WorkflowStep::new("launch_operations", "上线运营", "正式上线和后续运营支持")
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
            KpiDefinition::new("daily_active_users", "日活跃用户数", "人", MetricType::Counter),
            KpiDefinition::new("average_session_time", "平均游戏时长", "分钟", MetricType::Gauge),
            KpiDefinition::new("monetization_rate", "付费转化率", "%", MetricType::Percentage),
            KpiDefinition::new("player_retention", "玩家留存率", "%", MetricType::Percentage),
        ]
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let _ = time_range;
        let now = chrono::Utc::now().timestamp();
        Ok(vec![
            KpiValue {
                key: "daily_active_users".to_string(),
                value: 0.0,
                target: None,
                unit: Some("人".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "average_session_time".to_string(),
                value: 0.0,
                target: None,
                unit: Some("分钟".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "monetization_rate".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "player_retention".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
        ])
    }
}
