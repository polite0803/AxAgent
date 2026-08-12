//! 行业 KPI 计算服务 — 迁移自 OpcIndustryAdapter::compute_kpis
//!
//! 替代行业适配器中的 compute_kpis() 方法，保留动态业务逻辑。

use std::sync::Arc;

use super::analytics::{KpiDefinition, KpiValue};
use super::data_service::{OpcDataService, TimeRange};
use super::error::OpcResult;
use super::invoice::InvoiceStatus;
use super::project::ProjectStatus;

/// 会计 KPI 计算
pub async fn compute_accounting_kpis(
    data_service: &Arc<dyn OpcDataService>,
    time_range: &TimeRange,
) -> OpcResult<Vec<KpiValue>> {
    let (from, to) = (time_range.start, time_range.end);
    let now = chrono::Utc::now().timestamp();

    let revenue =
        data_service.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;
    let outstanding = data_service
        .count_invoices(&[InvoiceStatus::Sent, InvoiceStatus::Overdue], from, to)
        .await? as f64;
    let total = data_service.count_invoices(&[], from, to).await? as f64;

    let collection_rate = if total > 0.0 {
        let paid = data_service.count_invoices(&[InvoiceStatus::Paid], from, to).await? as f64;
        paid / total
    } else {
        0.0
    };

    Ok(vec![
        KpiValue {
            key: "total_revenue".to_string(),
            value: revenue,
            target: None,
            unit: Some("元".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "outstanding_invoices".to_string(),
            value: outstanding,
            target: None,
            unit: Some("张".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "collection_rate".to_string(),
            value: collection_rate * 100.0,
            target: None,
            unit: Some("%".to_string()),
            timestamp: now,
        },
    ])
}

/// 金融投资 KPI 计算
pub async fn compute_finance_invest_kpis(
    data_service: &Arc<dyn OpcDataService>,
    time_range: &TimeRange,
) -> OpcResult<Vec<KpiValue>> {
    let now = chrono::Utc::now().timestamp();
    let (from, to) = (time_range.start, time_range.end);

    let portfolio_value =
        data_service.aggregate_project_budgets(&[ProjectStatus::Active], from, to).await?.total;
    let orders = data_service.count_invoices(&[], from, to).await? as f64;

    Ok(vec![
        KpiValue {
            key: "portfolio_value".to_string(),
            value: portfolio_value,
            target: None,
            unit: Some("元".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "transaction_count".to_string(),
            value: orders,
            target: None,
            unit: Some("笔".to_string()),
            timestamp: now,
        },
    ])
}

/// 软件研发 KPI 计算
pub async fn compute_software_dev_kpis(
    data_service: &Arc<dyn OpcDataService>,
    time_range: &TimeRange,
) -> OpcResult<Vec<KpiValue>> {
    let now = chrono::Utc::now().timestamp();
    let (from, to) = (time_range.start, time_range.end);

    let completed =
        data_service.count_projects(&[ProjectStatus::Completed], from, to).await? as f64;
    let total = data_service.count_projects(&[], from, to).await? as f64;
    let active = data_service.count_projects(&[ProjectStatus::Active], from, to).await? as f64;

    let completion_rate = if total > 0.0 {
        completed / total * 100.0
    } else {
        0.0
    };

    Ok(vec![
        KpiValue {
            key: "completed_projects".to_string(),
            value: completed,
            target: None,
            unit: Some("个".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "active_projects".to_string(),
            value: active,
            target: None,
            unit: Some("个".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "completion_rate".to_string(),
            value: completion_rate,
            target: None,
            unit: Some("%".to_string()),
            timestamp: now,
        },
    ])
}

/// 内容媒体 KPI 计算
pub async fn compute_content_media_kpis(
    data_service: &Arc<dyn OpcDataService>,
    time_range: &TimeRange,
) -> OpcResult<Vec<KpiValue>> {
    let now = chrono::Utc::now().timestamp();
    let (from, to) = (time_range.start, time_range.end);

    let publish_count = data_service.count_blog_posts(from, to).await? as f64;
    let total_views = data_service.sum_blog_post_views(from, to).await?;

    Ok(vec![
        KpiValue {
            key: "publish_count".to_string(),
            value: publish_count,
            target: None,
            unit: Some("篇".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "total_views".to_string(),
            value: total_views,
            target: None,
            unit: Some("次".to_string()),
            timestamp: now,
        },
    ])
}

/// 电子商务 KPI 计算
pub async fn compute_ecommerce_kpis(
    data_service: &Arc<dyn OpcDataService>,
    time_range: &TimeRange,
) -> OpcResult<Vec<KpiValue>> {
    let now = chrono::Utc::now().timestamp();
    let (from, to) = (time_range.start, time_range.end);

    let gmv = data_service.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;
    let orders = data_service
        .count_invoices(&[InvoiceStatus::Paid, InvoiceStatus::Sent], from, to)
        .await? as f64;

    Ok(vec![
        KpiValue {
            key: "total_gmv".to_string(),
            value: gmv,
            target: None,
            unit: Some("元".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "order_count".to_string(),
            value: orders,
            target: None,
            unit: Some("笔".to_string()),
            timestamp: now,
        },
    ])
}

/// 通用 KPI 计算（无特殊逻辑的行业）
pub async fn compute_generic_kpis(
    data_service: &Arc<dyn OpcDataService>,
    time_range: &TimeRange,
    _kpi_definitions: &[KpiDefinition],
) -> OpcResult<Vec<KpiValue>> {
    let now = chrono::Utc::now().timestamp();
    let (from, to) = (time_range.start, time_range.end);

    // 使用通用指标：客户数、项目数、发票总额
    let customer_count = data_service.count_customers(&[], from, to).await? as f64;
    let project_count = data_service.count_projects(&[], from, to).await? as f64;
    let invoice_total = data_service.aggregate_invoice_amounts(&[], from, to).await?.total;

    Ok(vec![
        KpiValue {
            key: "customer_count".to_string(),
            value: customer_count,
            target: None,
            unit: Some("个".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "project_count".to_string(),
            value: project_count,
            target: None,
            unit: Some("个".to_string()),
            timestamp: now,
        },
        KpiValue {
            key: "invoice_total".to_string(),
            value: invoice_total,
            target: None,
            unit: Some("元".to_string()),
            timestamp: now,
        },
    ])
}

/// 计算行业 KPI（统一入口）
pub async fn compute_kpis(
    industry_id: &str,
    data_service: &Arc<dyn OpcDataService>,
    time_range: &TimeRange,
) -> OpcResult<Vec<KpiValue>> {
    match industry_id.replace('-', "_").as_str() {
        "accounting" => compute_accounting_kpis(data_service, time_range).await,
        "finance_invest" => compute_finance_invest_kpis(data_service, time_range).await,
        "software_dev" => compute_software_dev_kpis(data_service, time_range).await,
        "content_media" => compute_content_media_kpis(data_service, time_range).await,
        "ecommerce" => compute_ecommerce_kpis(data_service, time_range).await,
        _ => {
            let config = super::industry_config::get_config(industry_id);
            let definitions = config.map(|c| c.kpi_definitions).unwrap_or_default();
            compute_generic_kpis(data_service, time_range, &definitions).await
        },
    }
}
