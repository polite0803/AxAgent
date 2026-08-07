// 行业适配器模块 - 9 个行业独立手写 adapter

pub mod accounting;
pub mod ai_research;
pub mod content_media;
pub mod ecommerce;
pub mod education;
pub mod finance_invest;
pub mod industry_consulting;
pub mod sales_growth;
pub mod software_dev;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use super::analytics::{KpiDefinition, KpiValue};
use super::automation::IndustryAutomationRule;
use super::data_service::OpcDataService;
use super::data_service::TimeRange;
use super::error::OpcResult;
use super::rules::ValidationError;
use super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};

// ── 行业适配器共享基类 ────────────────────────────────────────
// 收敛 9 个行业 adapter 中重复的数据服务注入与基本信息存储逻辑。

/// 行业适配器共享基类 — 统一封装行业 ID/名称与数据服务注入
pub struct BaseIndustryAdapter {
    pub(crate) id: String,
    pub(crate) name: String,
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl BaseIndustryAdapter {
    pub(crate) fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into(), data_service: Mutex::new(None) }
    }

    pub(crate) fn industry_id(&self) -> &str {
        &self.id
    }

    pub(crate) fn industry_name(&self) -> &str {
        &self.name
    }

    pub(crate) fn set_data_service(&self, data_service: Arc<dyn OpcDataService>) {
        // 锁中毒时静默忽略注入，避免 panic 毒化整个进程
        if let Ok(mut guard) = self.data_service.lock() {
            *guard = Some(data_service);
        }
    }

    pub(crate) fn data_service(&self) -> Option<Arc<dyn OpcDataService>> {
        self.data_service.lock().ok().and_then(|guard| guard.clone())
    }
}

/// 为行业 adapter 生成 `OpcIndustryAdapter` trait 中与其他行业完全一致的通用方法。
/// 调用方需在作用域内可见 `Arc`、`OpcDataService`、`KpiDefinition`。
macro_rules! impl_industry_base {
    () => {
        fn industry_id(&self) -> &str {
            self.base.industry_id()
        }

        fn industry_name(&self) -> &str {
            self.base.industry_name()
        }

        fn set_data_service(&self, data_service: Arc<dyn OpcDataService>) {
            self.base.set_data_service(data_service);
        }

        fn data_service(&self) -> Option<Arc<dyn OpcDataService>> {
            self.base.data_service()
        }

        fn kpi_definitions(&self) -> Vec<KpiDefinition> {
            self.default_kpi_definitions()
        }
    };
}
pub(crate) use impl_industry_base;

// ── 行业通用类型 ──────────────────────────────────────────────────

/// 工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub order: u32,
}

impl WorkflowStep {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self { id: id.into(), name: name.into(), description: description.into(), order: 0 }
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }
}

/// 状态转换规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub entity_type: String,
    pub from: String,
    pub to: String,
    pub allowed: bool,
}

/// 仪表盘卡片定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardCard {
    pub id: String,
    pub title: String,
    pub kpi_key: String,
    pub display_value: String,
}

impl DashboardCard {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        kpi_key: impl Into<String>,
        display_value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            kpi_key: kpi_key.into(),
            display_value: display_value.into(),
        }
    }
}

/// 行业仪表盘摘要
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndustryDashboard {
    pub industry_id: String,
    pub kpis: Vec<KpiValue>,
    pub cards: Vec<DashboardCard>,
    pub summary: Option<String>,
}

// ── OpcIndustryAdapter Trait ────────────────────────────────────

/// OPC 行业适配器 trait
///
/// 每个行业实现此 trait，提供差异化的校验、KPI、工作流、规则和仪表盘能力。
/// 通过 `set_data_service` 注入数据服务后，适配器可执行真实的业务逻辑。
#[async_trait]
pub trait OpcIndustryAdapter: Send + Sync {
    // ── 基本信息 ──

    fn industry_id(&self) -> &str;
    fn industry_name(&self) -> &str;

    /// 适配器版本号
    fn version(&self) -> u32 {
        1
    }

    // ── 数据服务注入 ──

    fn set_data_service(&self, _data_service: Arc<dyn OpcDataService>) {}

    fn data_service(&self) -> Option<Arc<dyn OpcDataService>> {
        None
    }

    // ── 校验规则 ──

    async fn validate(
        &self,
        _entity_type: &str,
        _entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        Ok(Vec::new())
    }

    async fn validate_batch(
        &self,
        _entities: &[(String, serde_json::Value)],
    ) -> OpcResult<Vec<(String, Vec<ValidationError>)>> {
        Ok(Vec::new())
    }

    // ── KPI 指标 ──

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        Vec::new()
    }

    /// 默认 KPI 定义（行业特有）
    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        Vec::new()
    }

    async fn compute_kpis(&self, _time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        Ok(Vec::new())
    }

    // ── 实体类型 ──

    /// 行业支持的实体类型列表
    fn entity_types(&self) -> Vec<String> {
        Vec::new()
    }

    // ── 工作流 ──

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        Vec::new()
    }

    // ── 动态工作流元素定义（供工作流引擎编排，映射为标准 WorkflowNode） ──

    /// 定义数据验证规则（映射为 Validation 节点）
    fn define_validations(&self) -> Vec<ValidationDef> {
        Vec::new()
    }

    /// 定义业务步骤（映射为 Code/SubWorkflow 节点）
    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        Vec::new()
    }

    /// 定义 KPI 计算项（映射为 Code/DatabaseQuery 节点）
    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        Vec::new()
    }

    /// 定义自动化规则（映射为 Condition + Notification 节点组合）
    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        Vec::new()
    }

    /// 定义仪表盘卡片配置（映射为 Aggregator 节点）
    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        Vec::new()
    }

    /// 行业是否需要审批流程
    fn requires_approval(&self) -> bool {
        false
    }

    // ── 自动化规则 ──

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        Vec::new()
    }

    // ── 仪表盘 ──

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        Vec::new()
    }

    async fn aggregate_dashboard(&self, time_range: &TimeRange) -> OpcResult<IndustryDashboard> {
        let kpis = self.compute_kpis(time_range).await?;
        Ok(IndustryDashboard {
            industry_id: self.industry_id().to_string(),
            kpis,
            cards: self.dashboard_cards(),
            summary: None,
        })
    }
}

// ── 行业适配器工厂 ───────────────────────────────────────────────

/// 行业适配器工厂（用于注册/查询内建行业）
pub struct IndustryAdapterFactory;

impl IndustryAdapterFactory {
    pub fn create(industry_id: &str) -> Option<Arc<dyn OpcIndustryAdapter>> {
        match industry_id {
            "accounting" => Some(Arc::new(accounting::AccountingIndustryAdapter::new())),
            "ai_research" => Some(Arc::new(ai_research::AiResearchIndustryAdapter::new())),
            "content_media" => Some(Arc::new(content_media::ContentMediaIndustryAdapter::new())),
            "ecommerce" => Some(Arc::new(ecommerce::EcommerceIndustryAdapter::new())),
            "education" => Some(Arc::new(education::EducationIndustryAdapter::new())),
            "finance_invest" => Some(Arc::new(finance_invest::FinanceInvestIndustryAdapter::new())),
            "industry_consulting" => {
                Some(Arc::new(industry_consulting::IndustryConsultingIndustryAdapter::new()))
            },
            "sales_growth" => Some(Arc::new(sales_growth::SalesGrowthIndustryAdapter::new())),
            "software_dev" => Some(Arc::new(software_dev::SoftwareDevIndustryAdapter::new())),
            _ => None,
        }
    }

    pub fn list_all() -> Vec<(&'static str, &'static str)> {
        vec![
            ("accounting", "会计与财务管理"),
            ("ai_research", "AI 研究与咨询"),
            ("content_media", "内容与媒体"),
            ("ecommerce", "电子商务"),
            ("education", "教育培训"),
            ("finance_invest", "金融投资"),
            ("industry_consulting", "行业咨询"),
            ("sales_growth", "销售增长与营销"),
            ("software_dev", "软件开发"),
        ]
    }
}
