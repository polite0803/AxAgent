// 地理信息行业适配器
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
        vec![
            WorkflowStepDef {
                name: "数据采集".to_string(),
                description: "采集原始地理空间数据".to_string(),
                order: 1,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "数据处理".to_string(),
                description: "清洗、转换和标准化地理数据".to_string(),
                order: 2,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "空间分析".to_string(),
                description: "进行空间查询、分析和建模".to_string(),
                order: 3,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "可视化渲染".to_string(),
                description: "生成地图和空间可视化".to_string(),
                order: 4,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "发布共享".to_string(),
                description: "发布地理信息服务和应用".to_string(),
                order: 5,
                ..Default::default()
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
                    if longitude.as_f64().is_none_or(|l| l < -180.0 || l > 180.0) {
                        errors
                            .push(ValidationError::new("longitude", "经度必须在 -180 到 180 之间"));
                    }
                }
                if let Some(latitude) = entity_data.get("latitude") {
                    if latitude.as_f64().is_none_or(|l| l < -90.0 || l > 90.0) {
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
            "spatial_query" => {
                if entity_data.get("query_geometry").is_none_or(|g| g.as_object().is_none()) {
                    errors.push(ValidationError::new("query_geometry", "查询几何不能为空"));
                }
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
