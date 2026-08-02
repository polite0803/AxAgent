// SPDX-License-Identifier: AGPL-3.0-only

//! 分析仪表盘服务实现 — KPI/收益记录 + 仪表盘摘要

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use axagent_harness::util_fns::{gen_id, now_ts};
use axagent_opc_entities::{
    opc_customers, opc_invoices, opc_kpi_records, opc_projects, opc_revenue_records,
};
use axagent_opc_types::{
    AnalyticsService, CreateKpiInput, CreateRevenueInput, DashboardSummary, KpiRecord, OpcError,
    OpcResult, RevenueRecord,
};

pub struct DefaultAnalyticsService {
    pub db: DatabaseConnection,
}

impl DefaultAnalyticsService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

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
        // 查询各业务数据源
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
