// 地理信息行业适配器
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

pub struct GeospatialIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl GeospatialIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("geospatial", "地理信息") }
    }
}

impl Default for GeospatialIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for GeospatialIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "longitude".to_string(),
                r#type: "range".to_string(),
                error_message: "经度必须在 -180 到 180 之间".to_string(),
            },
            ValidationDef {
                field: "latitude".to_string(),
                r#type: "range".to_string(),
                error_message: "纬度必须在 -90 到 90 之间".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        // 用户输入变量通过 input_mapping 注入 AgentNode context
        let user_inputs = HashMap::from([
            ("region".to_string(), "region".to_string()),
            ("data_type".to_string(), "data_type".to_string()),
            ("output_format".to_string(), "output_format".to_string()),
        ]);
        vec![
            WorkflowStepDef {
                name: "空间分析".to_string(),
                description: "对目标区域进行空间数据分析".to_string(),
                prompt: Some(
                    "你是一名 GIS 分析师。请对目标区域进行空间数据分析。\
                     输出 JSON {analysis_result, spatial_patterns, key_layers, insights}"
                        .to_string(),
                ),
                tools: vec!["OpcSearchWiki".to_string(), "WebSearch".to_string()],
                agent_profile_id: None,
                error_handling: "stop".to_string(),
                order: 1,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "地图制作".to_string(),
                description: "制作专题地图与可视化".to_string(),
                prompt: Some(
                    "你是一名制图专家。请制作专题地图与可视化。\
                     输出 JSON {map_spec, layers, symbology, output_assets}"
                        .to_string(),
                ),
                tools: vec!["OpcCreateContentAsset".to_string(), "FileWrite".to_string()],
                agent_profile_id: None,
                error_handling: "continue".to_string(),
                order: 2,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "GIS 应用开发".to_string(),
                description: "规划 GIS 应用功能与部署".to_string(),
                prompt: Some(
                    "你是一名 GIS 应用工程师。请规划 GIS 应用功能与部署。\
                     输出 JSON {app_features, data_pipeline, deployment_plan, api_design}"
                        .to_string(),
                ),
                tools: vec!["OpcCreateProject".to_string(), "OpcCreateContentAsset".to_string()],
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
                key: "region".to_string(),
                label: "目标区域".to_string(),
                field_type: "string".to_string(),
                required: true,
                placeholder: Some("如：深圳市南山区".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "data_type".to_string(),
                label: "数据类型".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：矢量、栅格、点云、3D 模型".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "output_format".to_string(),
                label: "输出格式".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：GeoJSON、Shapefile、Web 地图服务".to_string()),
                default: None,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "data_accuracy".to_string(), name: "数据精度".to_string()
            },
            KpiCalculationDef {
                key: "update_frequency".to_string(),
                name: "数据更新频率".to_string(),
            },
            KpiCalculationDef {
                key: "service_availability".to_string(),
                name: "服务可用性".to_string(),
            },
            KpiCalculationDef {
                key: "analysis_turnaround".to_string(),
                name: "分析响应时间".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "data_stale_alert",
                "数据过期告警",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 1 },
                    AutomationCondition::EntityTypeIs {
                        entity_type: "geospatial_dataset".to_string(),
                    },
                ],
                vec![AutomationAction::SendNotification {
                    target: "geo_team".to_string(),
                    message: "地理数据已超过 24 小时未更新".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "service_health_check",
                "服务健康检查",
                vec![
                    AutomationCondition::StatusIs { status: "unhealthy".to_string() },
                    AutomationCondition::EntityTypeIs { entity_type: "geo_service".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "geo_team".to_string(),
                    message: "地理信息服务异常".to_string(),
                }],
            ),
        ]
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "active_datasets".to_string(),
                title: "活跃数据集".to_string(),
                kpi_key: "active_datasets_count".to_string(),
            },
            DashboardCardDef {
                id: "service_status".to_string(),
                title: "服务可用性".to_string(),
                kpi_key: "service_availability".to_string(),
            },
            DashboardCardDef {
                id: "data_quality".to_string(),
                title: "数据精度".to_string(),
                kpi_key: "data_accuracy".to_string(),
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
            "location" => {
                if let Some(longitude) = entity_data.get("longitude") {
                    if longitude.as_f64().is_none_or(|l| !(-180.0..=180.0).contains(&l)) {
                        errors
                            .push(ValidationError::new("longitude", "经度必须在 -180 到 180 之间"));
                    }
                }
                if let Some(latitude) = entity_data.get("latitude") {
                    if latitude.as_f64().is_none_or(|l| !(-90.0..=90.0).contains(&l)) {
                        errors.push(ValidationError::new("latitude", "纬度必须在 -90 到 90 之间"));
                    }
                }
                if entity_data.get("name").is_none_or(|n| n.as_str().is_none_or(|s| s.is_empty())) {
                    errors.push(ValidationError::new("name", "地点名称不能为空"));
                }
            },
            "geospatial_dataset" => {
                if entity_data
                    .get("dataset_id")
                    .is_none_or(|d| d.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("dataset_id", "数据集 ID 不能为空"));
                }
                if let Some(data_type) = entity_data.get("data_type") {
                    let valid_types = ["vector", "raster", "point_cloud", "3d_model"];
                    if data_type.as_str().is_none_or(|t| !valid_types.contains(&t)) {
                        errors.push(ValidationError::new("data_type", "无效的数据类型"));
                    }
                }
            },
            "spatial_query"
                if entity_data.get("query_geometry").is_none_or(|g| g.as_object().is_none()) =>
            {
                errors.push(ValidationError::new("query_geometry", "查询几何不能为空"));
            },
            _ => {},
        }

        Ok(errors)
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "location".to_string(),
            "geospatial_dataset".to_string(),
            "spatial_query".to_string(),
            "map_layer".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("data_collection", "数据采集", "采集原始地理空间数据").with_order(1),
            WorkflowStep::new("data_processing", "数据处理", "清洗、转换和标准化地理数据")
                .with_order(2),
            WorkflowStep::new("spatial_analysis", "空间分析", "进行空间查询、分析和建模")
                .with_order(3),
            WorkflowStep::new("visualization", "可视化渲染", "生成地图和空间可视化").with_order(4),
            WorkflowStep::new("publish_share", "发布共享", "发布地理信息服务和应用").with_order(5),
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
            KpiDefinition::new("data_accuracy", "数据精度", "米", MetricType::Gauge),
            KpiDefinition::new("update_frequency", "数据更新频率", "次/天", MetricType::Rate),
            KpiDefinition::new("service_availability", "服务可用性", "%", MetricType::Percentage),
            KpiDefinition::new("analysis_turnaround", "分析响应时间", "秒", MetricType::Gauge),
        ]
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let _ = time_range;
        let now = chrono::Utc::now().timestamp();
        Ok(vec![
            KpiValue {
                key: "data_accuracy".to_string(),
                value: 0.0,
                target: None,
                unit: Some("米".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "update_frequency".to_string(),
                value: 0.0,
                target: None,
                unit: Some("次/天".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "service_availability".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "analysis_turnaround".to_string(),
                value: 0.0,
                target: None,
                unit: Some("秒".to_string()),
                timestamp: now,
            },
        ])
    }
}
