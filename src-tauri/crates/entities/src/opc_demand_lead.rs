// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 需求线索表实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_demand_lead")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub platform: String,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub budget_min: Option<f64>,
    pub budget_max: Option<f64>,
    pub budget_currency: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub source_url: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub raw_snapshot_json: String,
    #[sea_orm(column_type = "Text")]
    pub matched_capabilities_json: String,
    #[sea_orm(column_type = "Text")]
    pub ai_analysis_json: String,
    pub recommended_workflow_id: Option<String>,
    pub status: String,
    pub priority: i32,
    pub confidence: f64,
    #[sea_orm(column_type = "Text")]
    pub notes: String,
    pub project_id: Option<String>,
    pub customer_id: Option<String>,
    pub expires_at: Option<i64>,
    pub claimed_by: Option<String>,
    // 需求价值评估字段（v222 新增）
    pub pain_score: f64,
    pub market_gap_score: f64,
    pub commercial_value_score: f64,
    pub opportunity_level: String,
    pub demand_type: String,
    pub evaluated_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
