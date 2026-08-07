// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

// ── 分析指标类型（stock-analysis 域） ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum MetricType {
    #[default]
    Counter,
    Count,
    Gauge,
    Histogram,
    Rate,
    Ratio,
    Currency,
    Percentage,
    Boolean,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Counter => write!(f, "counter"),
            MetricType::Count => write!(f, "count"),
            MetricType::Gauge => write!(f, "gauge"),
            MetricType::Histogram => write!(f, "histogram"),
            MetricType::Rate => write!(f, "rate"),
            MetricType::Ratio => write!(f, "ratio"),
            MetricType::Currency => write!(f, "currency"),
            MetricType::Percentage => write!(f, "percentage"),
            MetricType::Boolean => write!(f, "boolean"),
        }
    }
}

impl std::str::FromStr for MetricType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "counter" => Ok(MetricType::Counter),
            "count" => Ok(MetricType::Count),
            "gauge" => Ok(MetricType::Gauge),
            "histogram" => Ok(MetricType::Histogram),
            "rate" => Ok(MetricType::Rate),
            "ratio" => Ok(MetricType::Ratio),
            "currency" => Ok(MetricType::Currency),
            "percentage" => Ok(MetricType::Percentage),
            "boolean" => Ok(MetricType::Boolean),
            _ => Err(format!("Unknown metric type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KpiDefinition {
    pub key: String,
    pub name: String,
    pub description: String,
    pub metric_type: MetricType,
    pub target: Option<f64>,
    pub unit: Option<String>,
    pub formula: Option<String>,
    pub calculation_rule: Option<String>,
}

impl KpiDefinition {
    pub fn new(
        key: impl Into<String>,
        name: impl Into<String>,
        unit: impl Into<String>,
        metric_type: MetricType,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            description: String::new(),
            metric_type,
            target: None,
            unit: Some(unit.into()),
            formula: None,
            calculation_rule: None,
        }
    }

    pub fn with_target(mut self, target: f64) -> Self {
        self.target = Some(target);
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiValue {
    pub key: String,
    pub value: f64,
    pub target: Option<f64>,
    pub unit: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    pub chart_type: ChartType,
    pub title: String,
    pub x_axis_label: Option<String>,
    pub y_axis_label: Option<String>,
    pub data_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ChartType {
    #[default]
    Line,
    Bar,
    Pie,
    Area,
    Scatter,
    Table,
    Metric,
}

impl std::fmt::Display for ChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChartType::Line => write!(f, "line"),
            ChartType::Bar => write!(f, "bar"),
            ChartType::Pie => write!(f, "pie"),
            ChartType::Area => write!(f, "area"),
            ChartType::Scatter => write!(f, "scatter"),
            ChartType::Table => write!(f, "table"),
            ChartType::Metric => write!(f, "metric"),
        }
    }
}

impl std::str::FromStr for ChartType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "line" => Ok(ChartType::Line),
            "bar" => Ok(ChartType::Bar),
            "pie" => Ok(ChartType::Pie),
            "area" => Ok(ChartType::Area),
            "scatter" => Ok(ChartType::Scatter),
            "table" => Ok(ChartType::Table),
            "metric" => Ok(ChartType::Metric),
            _ => Err(format!("Unknown chart type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub layout: Vec<DashboardWidget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub id: String,
    pub title: String,
    pub widget_type: ChartType,
    pub data_key: String,
    pub size: WidgetSize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum WidgetSize {
    Small,
    #[default]
    Medium,
    Large,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub id: String,
    pub name: String,
    pub template: String,
    pub sections: Vec<ReportSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub id: String,
    pub title: String,
    pub type_: ReportSectionType,
    pub data_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ReportSectionType {
    #[default]
    Summary,
    Detail,
    Chart,
    Table,
    Text,
}

// ── OPC 分析仪表盘服务 ──────────────────────────────────────────

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use axagent_entities::{
    opc_customers, opc_invoices, opc_kpi_records, opc_projects, opc_revenue_records,
};
use axagent_harness::util_fns::{gen_id, now_ts};

use super::error::{OpcError, OpcResult};

/// KPI 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiRecord {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub period: String,
    pub recorded_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKpiInput {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub period: String,
}

/// 收入记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueRecord {
    pub id: String,
    pub amount: f64,
    pub currency: String,
    pub category: String,
    pub description: String,
    pub recorded_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRevenueInput {
    pub amount: f64,
    pub currency: String,
    pub category: String,
    pub description: String,
}

/// 仪表盘摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_revenue: f64,
    pub total_invoices: u32,
    pub active_projects: u32,
    pub total_customers: u32,
    pub recent_kpis: Vec<KpiRecord>,
    pub revenue_trend: Vec<RevenueRecord>,
}

// ── AnalyticsService Trait ─────────────────────────────────────

#[async_trait]
pub trait AnalyticsService: Send + Sync {
    async fn record_kpi(&self, input: CreateKpiInput) -> OpcResult<KpiRecord>;
    async fn list_kpis(
        &self,
        period: Option<String>,
        limit: Option<u32>,
    ) -> OpcResult<Vec<KpiRecord>>;

    async fn record_revenue(&self, input: CreateRevenueInput) -> OpcResult<RevenueRecord>;
    async fn list_revenue(
        &self,
        category: Option<String>,
        limit: Option<u32>,
    ) -> OpcResult<Vec<RevenueRecord>>;

    async fn get_dashboard_summary(&self) -> OpcResult<DashboardSummary>;
}

/// Noop 实现
#[derive(Debug)]
pub struct NoopAnalyticsService;

#[async_trait]
impl AnalyticsService for NoopAnalyticsService {
    async fn record_kpi(&self, _: CreateKpiInput) -> OpcResult<KpiRecord> {
        Err(OpcError::NotFound("AnalyticsService not implemented".into()))
    }
    async fn list_kpis(&self, _: Option<String>, _: Option<u32>) -> OpcResult<Vec<KpiRecord>> {
        Ok(Vec::new())
    }
    async fn record_revenue(&self, _: CreateRevenueInput) -> OpcResult<RevenueRecord> {
        Err(OpcError::NotFound("AnalyticsService not implemented".into()))
    }
    async fn list_revenue(
        &self,
        _: Option<String>,
        _: Option<u32>,
    ) -> OpcResult<Vec<RevenueRecord>> {
        Ok(Vec::new())
    }
    async fn get_dashboard_summary(&self) -> OpcResult<DashboardSummary> {
        Ok(DashboardSummary {
            total_revenue: 0.0,
            total_invoices: 0,
            active_projects: 0,
            total_customers: 0,
            recent_kpis: Vec::new(),
            revenue_trend: Vec::new(),
        })
    }
}

// ── Entity ↔ DTO 转换 ──────────────────────────────────────────

fn kpi_entity_to_dto(e: opc_kpi_records::Model) -> KpiRecord {
    KpiRecord {
        id: e.id,
        name: e.name,
        value: e.value,
        unit: e.unit,
        period: e.period,
        recorded_at: e.recorded_at,
        created_at: e.created_at,
    }
}

fn revenue_entity_to_dto(e: opc_revenue_records::Model) -> RevenueRecord {
    RevenueRecord {
        id: e.id,
        amount: e.amount,
        currency: e.currency,
        category: e.category,
        description: e.description,
        recorded_at: e.recorded_at,
        created_at: e.created_at,
    }
}

// ── DefaultAnalyticsService (SeaORM) ──────────────────────────

/// 默认分析服务实现
pub struct DefaultAnalyticsService {
    pub db: DatabaseConnection,
}

impl DefaultAnalyticsService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AnalyticsService for DefaultAnalyticsService {
    async fn record_kpi(&self, input: CreateKpiInput) -> OpcResult<KpiRecord> {
        let now = now_ts();
        let am = opc_kpi_records::ActiveModel {
            id: Set(gen_id()),
            name: Set(input.name),
            value: Set(input.value),
            unit: Set(input.unit),
            period: Set(input.period),
            recorded_at: Set(now),
            created_at: Set(now),
        };
        let entity = am.insert(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(kpi_entity_to_dto(entity))
    }

    async fn list_kpis(
        &self,
        period: Option<String>,
        _limit: Option<u32>,
    ) -> OpcResult<Vec<KpiRecord>> {
        let mut query =
            opc_kpi_records::Entity::find().order_by_desc(opc_kpi_records::Column::RecordedAt);
        if let Some(ref p) = period {
            query = query.filter(opc_kpi_records::Column::Period.eq(p));
        }
        let entities = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(entities.into_iter().map(kpi_entity_to_dto).collect())
    }

    async fn record_revenue(&self, input: CreateRevenueInput) -> OpcResult<RevenueRecord> {
        let now = now_ts();
        let am = opc_revenue_records::ActiveModel {
            id: Set(gen_id()),
            amount: Set(input.amount),
            currency: Set(input.currency),
            category: Set(input.category),
            description: Set(input.description),
            recorded_at: Set(now),
            created_at: Set(now),
        };
        let entity = am.insert(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(revenue_entity_to_dto(entity))
    }

    async fn list_revenue(
        &self,
        category: Option<String>,
        _limit: Option<u32>,
    ) -> OpcResult<Vec<RevenueRecord>> {
        let mut query = opc_revenue_records::Entity::find()
            .order_by_desc(opc_revenue_records::Column::RecordedAt);
        if let Some(ref c) = category {
            query = query.filter(opc_revenue_records::Column::Category.eq(c));
        }
        let entities = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(entities.into_iter().map(revenue_entity_to_dto).collect())
    }

    async fn get_dashboard_summary(&self) -> OpcResult<DashboardSummary> {
        let invoices = opc_invoices::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        let customers = opc_customers::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        let projects = opc_projects::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        let kpis = opc_kpi_records::Entity::find()
            .order_by_desc(opc_kpi_records::Column::RecordedAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        let revenue = opc_revenue_records::Entity::find()
            .order_by_desc(opc_revenue_records::Column::RecordedAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        let total_revenue: f64 =
            invoices.iter().filter(|i| i.status == "paid").map(|i| i.total).sum();
        let total_invoices = invoices.len() as u32;
        let active_projects =
            projects.iter().filter(|p| p.status == "active" || p.status == "planning").count()
                as u32;
        let total_customers = customers.len() as u32;

        Ok(DashboardSummary {
            total_revenue,
            total_invoices,
            active_projects,
            total_customers,
            recent_kpis: kpis.into_iter().map(kpi_entity_to_dto).collect(),
            revenue_trend: revenue.into_iter().map(revenue_entity_to_dto).collect(),
        })
    }
}
