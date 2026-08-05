// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业注册中心
//!
//! 本 crate 聚合所有 9 个行业适配器，提供统一的注册入口。
//!
//! ```ignore
//! use axagent_opc_runtime::IndustryAdapterRegistry;
//! use axagent_opc_industries::register_all_industries;
//!
//! let registry = IndustryAdapterRegistry::new();
//! register_all_industries(&registry).await;
//! ```

use std::sync::Arc;

use axagent_opc_runtime::IndustryAdapterRegistry;
use axagent_opc_types::OpcIndustryAdapter;

// ── 行业适配器引用 ──────────────────────────────────────────────

use axagent_opc_industry_accounting::AccountingAdapter;
use axagent_opc_industry_ai_research::AiResearchAdapter;
use axagent_opc_industry_content_media::ContentMediaAdapter;
use axagent_opc_industry_ecommerce::EcommerceAdapter;
use axagent_opc_industry_education::EducationAdapter;
use axagent_opc_industry_finance_invest::FinanceInvestAdapter;
use axagent_opc_industry_industry_consulting::IndustryConsultingAdapter;
use axagent_opc_industry_sales_growth::SalesGrowthAdapter;
use axagent_opc_industry_software_dev::SoftwareDevAdapter;

/// 注册所有行业适配器到注册中心
///
/// 在应用启动时调用，将所有 9 个行业的适配器注册到 `IndustryAdapterRegistry`。
pub async fn register_all_industries(registry: &IndustryAdapterRegistry) {
    let adapters: Vec<Arc<dyn OpcIndustryAdapter>> = vec![
        Arc::new(AccountingAdapter::new()),
        Arc::new(AiResearchAdapter::new()),
        Arc::new(ContentMediaAdapter::new()),
        Arc::new(EcommerceAdapter::new()),
        Arc::new(EducationAdapter::new()),
        Arc::new(FinanceInvestAdapter::new()),
        Arc::new(IndustryConsultingAdapter::new()),
        Arc::new(SalesGrowthAdapter::new()),
        Arc::new(SoftwareDevAdapter::new()),
    ];

    for adapter in adapters {
        let id = adapter.industry_id().to_string();
        let name = adapter.industry_name().to_string();
        tracing::info!("注册行业适配器: id={id}, name={name}");
        registry.register(adapter).await;
    }

    let registered = registry.list_ids().await;
    tracing::info!("行业适配器注册完成: {} 个行业", registered.len());
}

/// 获取所有行业 ID 列表
pub fn all_industry_ids() -> Vec<&'static str> {
    vec![
        "accounting",
        "ai_research",
        "content_media",
        "ecommerce",
        "education",
        "finance_invest",
        "industry_consulting",
        "sales_growth",
        "software_dev",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_all_industries() {
        let registry = IndustryAdapterRegistry::new();
        register_all_industries(&registry).await;

        let ids = registry.list_ids().await;
        assert_eq!(ids.len(), 9, "应注册 9 个行业适配器");

        // 验证所有行业都已注册
        for id in all_industry_ids() {
            assert!(registry.contains(id).await, "行业 {id} 未注册");
        }
    }

    #[tokio::test]
    async fn test_each_adapter_has_workflow_steps() {
        let registry = IndustryAdapterRegistry::new();
        register_all_industries(&registry).await;

        for id in all_industry_ids() {
            let adapter = registry.get(id).await.expect("适配器未找到");
            let steps = adapter.workflow_steps();
            assert!(!steps.is_empty(), "行业 {id} 应有工作流步骤");
        }
    }

    #[tokio::test]
    async fn test_each_adapter_has_kpis() {
        let registry = IndustryAdapterRegistry::new();
        register_all_industries(&registry).await;

        for id in all_industry_ids() {
            let adapter = registry.get(id).await.expect("适配器未找到");
            let kpis = adapter.kpi_definitions();
            assert!(!kpis.is_empty(), "行业 {id} 应有 KPI 定义");
        }
    }

    #[tokio::test]
    async fn test_each_adapter_has_dashboard_cards() {
        let registry = IndustryAdapterRegistry::new();
        register_all_industries(&registry).await;

        for id in all_industry_ids() {
            let adapter = registry.get(id).await.expect("适配器未找到");
            let cards = adapter.dashboard_cards();
            assert!(!cards.is_empty(), "行业 {id} 应有仪表盘卡片");
        }
    }
}
