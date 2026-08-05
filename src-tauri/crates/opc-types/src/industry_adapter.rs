// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业适配器 — 行业差异化扩展点
//!
//! 每个行业实现 `OpcIndustryAdapter` trait，提供：
//! - 行业特有的校验规则
//! - 行业特有的 KPI 指标
//! - 行业特有的工作流步骤
//! - 行业特有的自动化规则
//! - 行业特有的仪表盘聚合

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::OpcResult;
use crate::data_service::{OpcDataService, RuleContext};

// ── 时间范围 ────────────────────────────────────────────────────

/// 时间范围，用于 KPI 计算和数据聚合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

impl TimeRange {
    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }

    /// 最近 N 天
    pub fn days(n: i64) -> Self {
        let end = chrono::Utc::now().timestamp();
        let start = end - n * 86400;
        Self { start, end }
    }

    /// 最近 N 月（近似 30 天/月）
    pub fn months(n: i64) -> Self {
        Self::days(n * 30)
    }

    /// 最近一年
    pub fn year() -> Self {
        Self::days(365)
    }
}

// ── 校验错误 ────────────────────────────────────────────────────

/// 字段级校验错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    pub fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field: field.into(), message: message.into() }
    }

    pub fn general(message: impl Into<String>) -> Self {
        Self { field: String::new(), message: message.into() }
    }
}

// ── KPI 指标 ────────────────────────────────────────────────────

/// KPI 指标类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Count,
    Currency,
    Percentage,
    Ratio,
    Duration,
    Boolean,
}

/// KPI 定义（元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiDefinition {
    pub key: String,
    pub name: String,
    pub unit: String,
    pub metric_type: MetricType,
}

impl KpiDefinition {
    pub fn new(
        key: impl Into<String>,
        name: impl Into<String>,
        unit: impl Into<String>,
        metric_type: MetricType,
    ) -> Self {
        Self { key: key.into(), name: name.into(), unit: unit.into(), metric_type }
    }
}

/// KPI 实际值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiValue {
    pub key: String,
    pub value: f64,
    pub unit: String,
    pub recorded_at: i64,
}

impl KpiValue {
    pub fn new(key: impl Into<String>, value: f64) -> Self {
        Self {
            key: key.into(),
            value,
            unit: String::new(),
            recorded_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }
}

// ── 工作流 ──────────────────────────────────────────────────────

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

// ── 自动化规则 ──────────────────────────────────────────────────

/// 自动化条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutomationCondition {
    /// 逾期天数 >= 指定值
    OverdueDaysGte { days: u32 },
    /// 实体类型匹配
    EntityTypeIs { entity_type: String },
    /// 字段值超过阈值
    FieldExceeds { field: String, threshold: f64 },
    /// 字段值低于阈值
    FieldBelow { field: String, threshold: f64 },
    /// 状态匹配
    StatusIs { status: String },
    /// 创建时间超过 N 天
    CreatedDaysGte { days: u32 },
    /// 自定义条件（JSON 表达式）
    Custom { expression: String },
}

/// 自动化动作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutomationAction {
    /// 更新字段值
    UpdateField { field: String, value: serde_json::Value },
    /// 更新状态
    UpdateStatus { status: String },
    /// 发送通知
    SendNotification { target: String, message: String },
    /// 标记为已处理
    MarkProcessed,
    /// 创建关联记录
    CreateRecord { entity_type: String, data: serde_json::Value },
}

/// 行业自动化规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryAutomationRule {
    pub id: String,
    pub name: String,
    pub conditions: Vec<AutomationCondition>,
    pub actions: Vec<AutomationAction>,
    pub enabled: bool,
}

impl IndustryAutomationRule {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        conditions: Vec<AutomationCondition>,
        actions: Vec<AutomationAction>,
    ) -> Self {
        Self { id: id.into(), name: name.into(), conditions, actions, enabled: true }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

// ── 仪表盘 ──────────────────────────────────────────────────────

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

// ── OpcIndustryAdapter Trait ─────────────────────────────────────

/// OPC 行业适配器 trait
///
/// 每个行业实现此 trait，提供差异化的校验、KPI、工作流、规则和仪表盘能力。
/// 通过 `set_data_service` 注入数据服务后，适配器可执行真实的业务逻辑。
///
/// # 生命周期
/// 1. 创建适配器实例（无数据服务）
/// 2. 调用 `set_data_service()` 注入 `Arc<dyn OpcDataService>`
/// 3. 通过 `OpcRuntime` 调度调用各方法
#[async_trait]
pub trait OpcIndustryAdapter: Send + Sync {
    // ── 基本信息 ──

    /// 行业唯一 ID（如 "finance_invest"）
    fn industry_id(&self) -> &str;

    /// 行业显示名称（如 "金融投资分析"）
    fn industry_name(&self) -> &str;

    /// 适配器版本号
    fn version(&self) -> u32 {
        1
    }

    /// 是否启用
    fn enabled(&self) -> bool {
        true
    }

    // ── 数据服务注入 ──

    /// 注入数据服务
    ///
    /// 在注册到 `IndustryAdapterRegistry` 之前或之后调用均可。
    /// 注入后，`compute_kpis`、`evaluate_rule` 等方法可访问真实数据。
    fn set_data_service(&self, _data_service: Arc<dyn OpcDataService>) {}

    /// 获取数据服务引用（如果已注入）
    fn data_service(&self) -> Option<Arc<dyn OpcDataService>> {
        None
    }

    // ── 校验规则 ──

    /// 行业特有的实体校验
    ///
    /// 在通用 CRUD 校验之前调用。返回空 Vec 表示无额外校验。
    /// 可通过 `data_service()` 访问数据库进行唯一性/关联性检查。
    async fn validate(
        &self,
        _entity_type: &str,
        _entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        Ok(Vec::new())
    }

    /// 批量校验（用于规则引擎触发前的预检查）
    async fn validate_batch(
        &self,
        _entities: &[(String, serde_json::Value)],
    ) -> OpcResult<Vec<(String, Vec<ValidationError>)>> {
        Ok(Vec::new())
    }

    // ── KPI 指标 ──

    /// 行业特有的 KPI 定义列表
    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        Vec::new()
    }

    /// 计算行业特有的 KPI 值
    ///
    /// 如果已注入数据服务，可通过 `data_service()` 查询真实数据。
    async fn compute_kpis(&self, _time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        Ok(Vec::new())
    }

    // ── 工作流 ──

    /// 行业特有的工作流步骤
    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        Vec::new()
    }

    /// 行业特有的状态转换规则
    fn state_transitions(&self) -> Vec<StateTransition> {
        Vec::new()
    }

    // ── 自动化规则 ──

    /// 行业特有的自动化规则定义
    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        Vec::new()
    }

    /// 评估单个规则的条件是否满足
    ///
    /// 默认实现仅检查条件是否在上下文中匹配。
    /// 行业可覆盖此方法实现更复杂的业务逻辑判断。
    async fn evaluate_rule(
        &self,
        _rule: &IndustryAutomationRule,
        _context: &RuleContext,
    ) -> OpcResult<bool> {
        Ok(false)
    }

    /// 执行规则动作
    ///
    /// 在规则条件满足后调用，执行动作如更新字段、发送通知等。
    async fn execute_rule_actions(
        &self,
        _rule: &IndustryAutomationRule,
        _context: &RuleContext,
    ) -> OpcResult<()> {
        Ok(())
    }

    // ── 仪表盘 ──

    /// 行业特有的仪表盘卡片
    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        Vec::new()
    }

    /// 聚合行业仪表盘数据
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_new() {
        let range = TimeRange::new(1000, 2000);
        assert_eq!(range.start, 1000);
        assert_eq!(range.end, 2000);
    }

    #[test]
    fn test_time_range_days() {
        let range = TimeRange::days(30);
        let now = chrono::Utc::now().timestamp();
        let expected_start = now - 30 * 86400;
        assert!(range.end <= now);
        assert!(range.start >= expected_start - 1);
    }

    #[test]
    fn test_time_range_months() {
        let range = TimeRange::months(3);
        let now = chrono::Utc::now().timestamp();
        let expected_start = now - 90 * 86400;
        assert!(range.end <= now);
        assert!(range.start >= expected_start - 1);
    }

    #[test]
    fn test_time_range_year() {
        let range = TimeRange::year();
        let now = chrono::Utc::now().timestamp();
        let expected_start = now - 365 * 86400;
        assert!(range.end <= now);
        assert!(range.start >= expected_start - 1);
    }

    #[test]
    fn test_validation_error_field() {
        let err = ValidationError::field("name", "名称不能为空");
        assert_eq!(err.field, "name");
        assert_eq!(err.message, "名称不能为空");
    }

    #[test]
    fn test_validation_error_general() {
        let err = ValidationError::general("数据验证失败");
        assert!(err.field.is_empty());
        assert_eq!(err.message, "数据验证失败");
    }

    #[test]
    fn test_kpi_definition_new() {
        let kpi = KpiDefinition::new("revenue", "营收", "万元", MetricType::Currency);
        assert_eq!(kpi.key, "revenue");
        assert_eq!(kpi.name, "营收");
        assert_eq!(kpi.unit, "万元");
        assert!(matches!(kpi.metric_type, MetricType::Currency));
    }

    #[test]
    fn test_kpi_value_new() {
        let value = KpiValue::new("revenue", 12345.67);
        assert_eq!(value.key, "revenue");
        assert_eq!(value.value, 12345.67);
        assert!(value.unit.is_empty());
    }

    #[test]
    fn test_kpi_value_with_unit() {
        let value = KpiValue::new("revenue", 100.0).with_unit("万元");
        assert_eq!(value.unit, "万元");
    }

    #[test]
    fn test_workflow_step_new() {
        let step = WorkflowStep::new("step1", "步骤一", "第一个步骤");
        assert_eq!(step.id, "step1");
        assert_eq!(step.name, "步骤一");
        assert_eq!(step.description, "第一个步骤");
        assert_eq!(step.order, 0);
    }

    #[test]
    fn test_workflow_step_with_order() {
        let step = WorkflowStep::new("step1", "步骤一", "描述").with_order(5);
        assert_eq!(step.order, 5);
    }

    #[test]
    fn test_state_transition() {
        let transition = StateTransition {
            entity_type: "invoice".to_string(),
            from: "draft".to_string(),
            to: "sent".to_string(),
            allowed: true,
        };
        assert_eq!(transition.entity_type, "invoice");
        assert!(transition.allowed);
    }

    #[test]
    fn test_automation_condition_entity_type_is() {
        let cond = AutomationCondition::EntityTypeIs { entity_type: "invoice".to_string() };
        assert!(matches!(cond, AutomationCondition::EntityTypeIs { .. }));
    }

    #[test]
    fn test_automation_condition_overdue_days_gte() {
        let cond = AutomationCondition::OverdueDaysGte { days: 30 };
        assert!(matches!(cond, AutomationCondition::OverdueDaysGte { days: 30 }));
    }

    #[test]
    fn test_automation_condition_field_exceeds() {
        let cond =
            AutomationCondition::FieldExceeds { field: "amount".to_string(), threshold: 1000.0 };
        assert!(
            matches!(cond, AutomationCondition::FieldExceeds { threshold, .. } if threshold == 1000.0)
        );
    }

    #[test]
    fn test_automation_action_update_status() {
        let action = AutomationAction::UpdateStatus { status: "paid".to_string() };
        assert!(matches!(action, AutomationAction::UpdateStatus { status } if status == "paid"));
    }

    #[test]
    fn test_automation_action_mark_processed() {
        let action = AutomationAction::MarkProcessed;
        assert!(matches!(action, AutomationAction::MarkProcessed));
    }

    #[test]
    fn test_industry_automation_rule_new() {
        let rule = IndustryAutomationRule::new(
            "rule1",
            "规则一",
            vec![AutomationCondition::EntityTypeIs { entity_type: "invoice".to_string() }],
            vec![AutomationAction::UpdateStatus { status: "active".to_string() }],
        );
        assert_eq!(rule.id, "rule1");
        assert_eq!(rule.name, "规则一");
        assert!(rule.enabled);
        assert_eq!(rule.conditions.len(), 1);
        assert_eq!(rule.actions.len(), 1);
    }

    #[test]
    fn test_industry_automation_rule_disabled() {
        let rule = IndustryAutomationRule::new("rule1", "规则一", vec![], vec![]).disabled();
        assert!(!rule.enabled);
    }

    #[test]
    fn test_dashboard_card_new() {
        let card = DashboardCard::new("card1", "营收总额", "revenue", "¥100万");
        assert_eq!(card.id, "card1");
        assert_eq!(card.title, "营收总额");
        assert_eq!(card.kpi_key, "revenue");
        assert_eq!(card.display_value, "¥100万");
    }

    #[test]
    fn test_industry_dashboard_default() {
        let dashboard = IndustryDashboard::default();
        assert!(dashboard.industry_id.is_empty());
        assert!(dashboard.kpis.is_empty());
        assert!(dashboard.cards.is_empty());
        assert!(dashboard.summary.is_none());
    }

    #[test]
    fn test_metric_type_deserialization() {
        let json = r#"{"type": "field_exceeds", "field": "amount", "threshold": 1000.0}"#;
        let cond: AutomationCondition = serde_json::from_str(json).unwrap();
        assert!(
            matches!(cond, AutomationCondition::FieldExceeds { field, threshold } if field == "amount" && threshold == 1000.0)
        );
    }
}
