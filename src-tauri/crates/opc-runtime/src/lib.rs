// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 运行时 — 行业适配器注册中心 + 规则调度引擎
//!
//! 本 crate 提供：
//! - `IndustryAdapterRegistry`：行业适配器注册与查找
//! - `OpcRuntime`：统一运行时入口，调度行业校验、KPI 计算、规则执行
//!
//! 架构定位：consumer 层，仅依赖 `opc-types`（契约层）。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_opc_types::{
    AutomationAction, AutomationCondition, IndustryAutomationRule, KpiDefinition, KpiValue,
    OpcDataService, OpcIndustryAdapter, OpcResult, RuleContext, TimeRange, ValidationError,
    WorkflowStep,
};
use tokio::sync::RwLock;

// ── IndustryAdapterRegistry ─────────────────────────────────────

/// 行业适配器注册中心
///
/// 行业 crate 在启动时注册其 `OpcIndustryAdapter` 实现，
/// 运行时通过 `get()` 查找对应行业的适配器。
pub struct IndustryAdapterRegistry {
    adapters: RwLock<HashMap<String, Arc<dyn OpcIndustryAdapter>>>,
}

impl IndustryAdapterRegistry {
    pub fn new() -> Self {
        Self { adapters: RwLock::new(HashMap::new()) }
    }

    /// 注册行业适配器
    pub async fn register(&self, adapter: Arc<dyn OpcIndustryAdapter>) {
        let id = adapter.industry_id().to_string();
        tracing::info!(
            "[opc-runtime] 注册行业适配器: id={}, name={}, version={}",
            id,
            adapter.industry_name(),
            adapter.version()
        );
        self.adapters.write().await.insert(id, adapter);
    }

    /// 按 ID 查找适配器
    pub async fn get(&self, industry_id: &str) -> Option<Arc<dyn OpcIndustryAdapter>> {
        self.adapters.read().await.get(industry_id).cloned()
    }

    /// 列出所有已注册行业
    pub async fn list_all(&self) -> Vec<(String, String)> {
        self.adapters
            .read()
            .await
            .values()
            .map(|a| (a.industry_id().to_string(), a.industry_name().to_string()))
            .collect()
    }

    /// 列出所有已注册行业 ID
    pub async fn list_ids(&self) -> Vec<String> {
        self.adapters.read().await.keys().cloned().collect()
    }

    /// 检查行业是否已注册
    pub async fn contains(&self, industry_id: &str) -> bool {
        self.adapters.read().await.contains_key(industry_id)
    }

    /// 向所有已注册的适配器注入数据服务
    pub async fn inject_data_service(&self, data_service: Arc<dyn OpcDataService>) {
        let adapters = self.adapters.read().await;
        for adapter in adapters.values() {
            adapter.set_data_service(data_service.clone());
        }
        tracing::info!("[opc-runtime] 数据服务已注入所有行业适配器");
    }
}

impl Default for IndustryAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── OpcRuntime ───────────────────────────────────────────────────

/// OPC 统一运行时
///
/// 提供行业适配器调度的统一入口，包括：
/// - 数据服务注入
/// - 实体校验（行业特有的 + 通用的）
/// - KPI 计算
/// - 工作流步骤获取
/// - 自动化规则评估与执行
pub struct OpcRuntime {
    registry: Arc<IndustryAdapterRegistry>,
    data_service: Option<Arc<dyn OpcDataService>>,
}

impl OpcRuntime {
    pub fn new(registry: Arc<IndustryAdapterRegistry>) -> Self {
        Self { registry, data_service: None }
    }

    /// 注入数据服务并同步到所有适配器
    pub async fn with_data_service(mut self, data_service: Arc<dyn OpcDataService>) -> Self {
        self.data_service = Some(data_service.clone());
        self.registry.inject_data_service(data_service).await;
        self
    }

    pub fn registry(&self) -> &IndustryAdapterRegistry {
        &self.registry
    }

    pub fn data_service(&self) -> Option<&Arc<dyn OpcDataService>> {
        self.data_service.as_ref()
    }

    // ── 校验 ──

    /// 对指定行业的实体执行行业特有校验
    pub async fn validate_entity(
        &self,
        industry_id: &str,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let adapter = self.get_adapter(industry_id).await?;
        adapter.validate(entity_type, entity_data).await
    }

    /// 批量校验多个实体
    pub async fn validate_batch(
        &self,
        industry_id: &str,
        entities: &[(String, serde_json::Value)],
    ) -> OpcResult<Vec<(String, Vec<ValidationError>)>> {
        let adapter = self.get_adapter(industry_id).await?;
        adapter.validate_batch(entities).await
    }

    // ── KPI ──

    /// 获取指定行业的所有 KPI 定义
    pub async fn get_kpi_definitions(&self, industry_id: &str) -> OpcResult<Vec<KpiDefinition>> {
        let adapter = self.get_adapter(industry_id).await?;
        Ok(adapter.kpi_definitions())
    }

    /// 计算指定行业在给定时间范围的 KPI 值
    pub async fn compute_kpis(
        &self,
        industry_id: &str,
        time_range: &TimeRange,
    ) -> OpcResult<Vec<KpiValue>> {
        let adapter = self.get_adapter(industry_id).await?;
        adapter.compute_kpis(time_range).await
    }

    // ── 工作流 ──

    /// 获取指定行业的工作流步骤
    pub async fn get_workflow_steps(&self, industry_id: &str) -> OpcResult<Vec<WorkflowStep>> {
        let adapter = self.get_adapter(industry_id).await?;
        let mut steps = adapter.workflow_steps();
        steps.sort_by_key(|s| s.order);
        Ok(steps)
    }

    // ── 自动化规则 ──

    /// 获取指定行业的所有启用自动化规则
    pub async fn get_enabled_rules(
        &self,
        industry_id: &str,
    ) -> OpcResult<Vec<IndustryAutomationRule>> {
        let adapter = self.get_adapter(industry_id).await?;
        Ok(adapter.automation_rules().into_iter().filter(|r| r.enabled).collect())
    }

    /// 评估规则是否满足条件
    ///
    /// 先调用适配器的行业特有评估逻辑，回退到通用条件匹配。
    pub async fn evaluate_rule(
        &self,
        industry_id: &str,
        rule: &IndustryAutomationRule,
        context: &RuleContext,
    ) -> OpcResult<bool> {
        let adapter = self.get_adapter(industry_id).await?;

        // 优先使用行业适配器的评估逻辑
        match adapter.evaluate_rule(rule, context).await {
            Ok(true) => return Ok(true),
            Ok(false) => {}, // 回退到通用逻辑
            Err(e) => {
                tracing::warn!("[opc-runtime] 行业规则评估出错，回退到通用逻辑: {}", e);
            },
        }

        // 通用条件评估（纯函数匹配）
        let entity_ctx = self.context_to_hashmap(context);
        Ok(Self::evaluate_conditions(&rule.conditions, &entity_ctx))
    }

    /// 执行规则动作
    ///
    /// 优先使用行业适配器的特有执行逻辑；若适配器返回默认空实现（Ok(())），
    /// 则回退到运行时的通用动作处理（日志记录 + DAO 实际数据库操作）。
    pub async fn execute_rule(
        &self,
        industry_id: &str,
        rule: &IndustryAutomationRule,
        context: &RuleContext,
    ) -> OpcResult<()> {
        let adapter = self.get_adapter(industry_id).await?;

        // 1. 先尝试适配器的特有执行逻辑
        adapter.execute_rule_actions(rule, context).await?;

        // 2. 执行通用动作处理（适用于所有行业的默认行为）
        for action in &rule.actions {
            match action {
                AutomationAction::SendNotification { target, message } => {
                    tracing::info!("[opc-runtime] 通知: target={}, message={}", target, message);
                    // 预留扩展点：可接入 opc-dao 的通知服务
                },
                AutomationAction::UpdateStatus { status } => {
                    tracing::info!(
                        "[opc-runtime] 状态更新: entity={}/{} -> status={}",
                        context.entity_type,
                        context.entity_id,
                        status
                    );
                    if let Some(ds) = &self.data_service {
                        ds.update_entity_status(&context.entity_type, &context.entity_id, status)
                            .await?;
                    }
                },
                AutomationAction::UpdateField { field, value } => {
                    tracing::info!(
                        "[opc-runtime] 字段更新: entity={}/{} , field={}, value={}",
                        context.entity_type,
                        context.entity_id,
                        field,
                        value
                    );
                    // 预留扩展点：可接入 opc-dao 的 DAO 更新方法
                },
                AutomationAction::MarkProcessed => {
                    tracing::info!(
                        "[opc-runtime] 标记已处理: entity={}/{}",
                        context.entity_type,
                        context.entity_id
                    );
                },
                AutomationAction::CreateRecord { entity_type, data } => {
                    tracing::info!(
                        "[opc-runtime] 创建记录: entity_type={}, data={}",
                        entity_type,
                        data
                    );
                    if let Some(ds) = &self.data_service {
                        let new_id = ds.create_entity_record(entity_type, data).await?;
                        tracing::info!(
                            "[opc-runtime] 记录创建成功: id={}, entity_type={}",
                            new_id,
                            entity_type
                        );
                    }
                },
            }
        }

        Ok(())
    }

    /// 评估并执行规则（原子操作）
    ///
    /// 如果规则条件满足，执行规则动作。返回是否执行了动作。
    pub async fn evaluate_and_execute_rule(
        &self,
        industry_id: &str,
        rule: &IndustryAutomationRule,
        context: &RuleContext,
    ) -> OpcResult<bool> {
        if self.evaluate_rule(industry_id, rule, context).await? {
            self.execute_rule(industry_id, rule, context).await?;
            tracing::info!(
                "[opc-runtime] 规则触发并执行: industry={}, rule={}, entity={}/{}",
                industry_id,
                rule.id,
                context.entity_type,
                context.entity_id
            );
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 批量评估并执行指定行业的所有规则
    ///
    /// 返回被触发的规则 ID 列表。
    pub async fn run_all_rules(
        &self,
        industry_id: &str,
        context: &RuleContext,
    ) -> OpcResult<Vec<String>> {
        let rules = self.get_enabled_rules(industry_id).await?;
        let mut triggered = Vec::new();

        for rule in &rules {
            match self.evaluate_and_execute_rule(industry_id, rule, context).await {
                Ok(true) => {
                    triggered.push(rule.id.clone());
                },
                Ok(false) => {},
                Err(e) => {
                    tracing::error!(
                        "[opc-runtime] 规则执行失败: industry={}, rule={}, error={}",
                        industry_id,
                        rule.id,
                        e
                    );
                },
            }
        }

        if !triggered.is_empty() {
            tracing::info!(
                "[opc-runtime] 行业 {} 规则执行完成: {} 条规则被触发",
                industry_id,
                triggered.len()
            );
        }

        Ok(triggered)
    }

    /// 检查条件是否满足（通用纯函数）
    pub fn evaluate_conditions(
        conditions: &[AutomationCondition],
        entity_context: &HashMap<String, serde_json::Value>,
    ) -> bool {
        conditions.iter().all(|cond| Self::eval_condition(cond, entity_context))
    }

    fn eval_condition(
        cond: &AutomationCondition,
        ctx: &HashMap<String, serde_json::Value>,
    ) -> bool {
        match cond {
            AutomationCondition::OverdueDaysGte { days } => {
                let overdue = ctx.get("overdue_days").and_then(|v| v.as_f64()).unwrap_or(0.0);
                overdue >= *days as f64
            },
            AutomationCondition::EntityTypeIs { entity_type } => {
                ctx.get("entity_type").and_then(|v| v.as_str()).is_some_and(|v| v == entity_type)
            },
            AutomationCondition::FieldExceeds { field, threshold } => {
                ctx.get(field).and_then(|v| v.as_f64()).is_some_and(|v| v >= *threshold)
            },
            AutomationCondition::FieldBelow { field, threshold } => {
                ctx.get(field).and_then(|v| v.as_f64()).is_some_and(|v| v <= *threshold)
            },
            AutomationCondition::StatusIs { status } => {
                ctx.get("status").and_then(|v| v.as_str()).is_some_and(|v| v == status)
            },
            AutomationCondition::CreatedDaysGte { days } => {
                let created = ctx.get("created_days").and_then(|v| v.as_f64()).unwrap_or(0.0);
                created >= *days as f64
            },
            AutomationCondition::Custom { expression } => {
                tracing::debug!(
                    "[opc-runtime] 跳过自定义条件表达式（需 Rhai 支持）: {}",
                    expression
                );
                false
            },
        }
    }

    fn context_to_hashmap(&self, context: &RuleContext) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("entity_type".to_string(), serde_json::json!(context.entity_type));
        map.insert("entity_id".to_string(), serde_json::json!(context.entity_id));
        if let Some(ref status) = context.status {
            map.insert("status".to_string(), serde_json::json!(status));
        }
        if let Some(overdue) = context.overdue_days {
            map.insert("overdue_days".to_string(), serde_json::json!(overdue));
        }
        if let Some(created) = context.created_days {
            map.insert("created_days".to_string(), serde_json::json!(created));
        }
        // 合并自定义字段
        if let Some(fields) = context.fields.as_object() {
            for (key, value) in fields {
                map.insert(key.clone(), value.clone());
            }
        }
        map
    }

    // ── 仪表盘 ──

    /// 获取指定行业的仪表盘数据
    pub async fn get_industry_dashboard(
        &self,
        industry_id: &str,
        time_range: &TimeRange,
    ) -> OpcResult<axagent_opc_types::IndustryDashboard> {
        let adapter = self.get_adapter(industry_id).await?;
        adapter.aggregate_dashboard(time_range).await
    }

    // ── 内部工具 ──

    async fn get_adapter(&self, industry_id: &str) -> OpcResult<Arc<dyn OpcIndustryAdapter>> {
        self.registry.get(industry_id).await.ok_or_else(|| {
            axagent_opc_types::OpcError::NotFound(format!("行业适配器未注册: {industry_id}"))
        })
    }
}

// ── 便捷构造函数 ──────────────────────────────────────────────────

/// 创建带空注册中心的运行时（测试用）
pub fn create_empty_runtime() -> Arc<OpcRuntime> {
    let registry = Arc::new(IndustryAdapterRegistry::new());
    Arc::new(OpcRuntime::new(registry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_opc_types::{
        AutomationAction, AutomationCondition, DashboardCard, IndustryAutomationRule,
        KpiDefinition, KpiValue, OpcIndustryAdapter, ValidationError, WorkflowStep,
    };
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 测试用空适配器
    struct NoopAdapter {
        id: String,
        name: String,
        call_count: AtomicU32,
    }

    impl NoopAdapter {
        fn new(id: &str, name: &str) -> Self {
            Self { id: id.to_string(), name: name.to_string(), call_count: AtomicU32::new(0) }
        }
    }

    #[async_trait::async_trait]
    impl OpcIndustryAdapter for NoopAdapter {
        fn industry_id(&self) -> &str {
            &self.id
        }

        fn industry_name(&self) -> &str {
            &self.name
        }

        async fn validate(
            &self,
            _entity_type: &str,
            _entity_data: &serde_json::Value,
        ) -> OpcResult<Vec<ValidationError>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }

        fn kpi_definitions(&self) -> Vec<KpiDefinition> {
            vec![KpiDefinition::new(
                "test_kpi",
                "测试KPI",
                "个",
                axagent_opc_types::MetricType::Count,
            )]
        }

        async fn compute_kpis(&self, _range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
            Ok(vec![KpiValue::new("test_kpi", 42.0)])
        }

        fn workflow_steps(&self) -> Vec<WorkflowStep> {
            vec![
                WorkflowStep::new("step1", "步骤1", "描述1").with_order(1),
                WorkflowStep::new("step2", "步骤2", "描述2").with_order(2),
            ]
        }

        fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
            vec![IndustryAutomationRule::new(
                "test_rule",
                "测试规则",
                vec![AutomationCondition::EntityTypeIs { entity_type: "invoice".into() }],
                vec![AutomationAction::UpdateStatus { status: "active".into() }],
            )]
        }

        fn dashboard_cards(&self) -> Vec<DashboardCard> {
            vec![DashboardCard::new("card1", "卡片1", "test_kpi", "42")]
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_get() {
        let registry = IndustryAdapterRegistry::new();
        let adapter = Arc::new(NoopAdapter::new("test", "测试行业"));

        registry.register(adapter.clone()).await;

        assert!(registry.contains("test").await);
        let ids = registry.list_ids().await;
        assert_eq!(ids, vec!["test"]);
    }

    #[tokio::test]
    async fn test_runtime_validate_entity() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("finance", "金融"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let data = serde_json::json!({"amount": 100});
        let errors = runtime.validate_entity("finance", "invoice", &data).await.unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_runtime_kpis() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("sales", "销售"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let definitions = runtime.get_kpi_definitions("sales").await.unwrap();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].key, "test_kpi");

        let range = TimeRange::days(30);
        let values = runtime.compute_kpis("sales", &range).await.unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].value, 42.0);
    }

    #[tokio::test]
    async fn test_runtime_workflow_steps_sorted() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("ai", "AI"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let steps = runtime.get_workflow_steps("ai").await.unwrap();
        assert_eq!(steps.len(), 2);
        assert!(steps[0].order <= steps[1].order);
    }

    #[tokio::test]
    async fn test_runtime_rules() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("software", "软件"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let rules = runtime.get_enabled_rules("software").await.unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].enabled);
    }

    #[test]
    fn test_evaluate_conditions_entity_type() {
        let cond = AutomationCondition::EntityTypeIs { entity_type: "invoice".into() };
        let mut ctx = HashMap::new();
        ctx.insert("entity_type".to_string(), serde_json::json!("invoice"));
        assert!(OpcRuntime::evaluate_conditions(&[cond], &ctx));
    }

    #[test]
    fn test_evaluate_conditions_field_exceeds() {
        let cond = AutomationCondition::FieldExceeds { field: "amount".into(), threshold: 1000.0 };
        let mut ctx = HashMap::new();
        ctx.insert("amount".to_string(), serde_json::json!(5000.0));
        assert!(OpcRuntime::evaluate_conditions(std::slice::from_ref(&cond), &ctx));

        ctx.insert("amount".to_string(), serde_json::json!(500.0));
        assert!(!OpcRuntime::evaluate_conditions(&[cond], &ctx));
    }

    #[test]
    fn test_evaluate_conditions_overdue_days() {
        let cond = AutomationCondition::OverdueDaysGte { days: 30 };
        let mut ctx = HashMap::new();
        ctx.insert("overdue_days".to_string(), serde_json::json!(45));
        assert!(OpcRuntime::evaluate_conditions(std::slice::from_ref(&cond), &ctx));

        ctx.insert("overdue_days".to_string(), serde_json::json!(10));
        assert!(!OpcRuntime::evaluate_conditions(&[cond], &ctx));
    }

    #[tokio::test]
    async fn test_runtime_industry_not_found() {
        let runtime = create_empty_runtime();
        let result = runtime.validate_entity("nonexistent", "test", &serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_evaluate_and_execute_rule_triggered() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("test", "测试"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let rule = IndustryAutomationRule::new(
            "trigger_rule",
            "触发规则",
            vec![AutomationCondition::EntityTypeIs { entity_type: "invoice".into() }],
            vec![AutomationAction::UpdateStatus { status: "paid".into() }],
        );
        let ctx = RuleContext::new("invoice", "inv-001");
        let result = runtime.evaluate_and_execute_rule("test", &rule, &ctx).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_evaluate_and_execute_rule_not_triggered() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("test", "测试"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let rule = IndustryAutomationRule::new(
            "no_trigger",
            "不触发规则",
            vec![AutomationCondition::EntityTypeIs { entity_type: "customer".into() }],
            vec![AutomationAction::UpdateStatus { status: "active".into() }],
        );
        let ctx = RuleContext::new("invoice", "inv-001");
        let result = runtime.evaluate_and_execute_rule("test", &rule, &ctx).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_run_all_rules() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("test", "测试"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let ctx = RuleContext::new("invoice", "inv-001");
        let triggered = runtime.run_all_rules("test", &ctx).await.unwrap();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], "test_rule");
    }

    #[tokio::test]
    async fn test_run_all_rules_no_match() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("test", "测试"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let ctx = RuleContext::new("customer", "cust-001");
        let triggered = runtime.run_all_rules("test", &ctx).await.unwrap();
        assert!(triggered.is_empty());
    }

    #[tokio::test]
    async fn test_get_dashboard() {
        let registry = Arc::new(IndustryAdapterRegistry::new());
        let adapter = Arc::new(NoopAdapter::new("test", "测试"));
        registry.register(adapter).await;

        let runtime = OpcRuntime::new(registry);
        let range = TimeRange::days(30);
        let dashboard = runtime.get_industry_dashboard("test", &range).await.unwrap();
        assert_eq!(dashboard.industry_id, "test");
        assert_eq!(dashboard.kpis.len(), 1);
        assert_eq!(dashboard.cards.len(), 1);
    }

    #[test]
    fn test_evaluate_conditions_multiple() {
        let conditions = vec![
            AutomationCondition::EntityTypeIs { entity_type: "invoice".into() },
            AutomationCondition::StatusIs { status: "pending".into() },
            AutomationCondition::FieldExceeds { field: "amount".into(), threshold: 100.0 },
        ];

        let mut ctx = HashMap::new();
        ctx.insert("entity_type".to_string(), serde_json::json!("invoice"));
        ctx.insert("status".to_string(), serde_json::json!("pending"));
        ctx.insert("amount".to_string(), serde_json::json!(500.0));

        assert!(OpcRuntime::evaluate_conditions(&conditions, &ctx));

        // 不满足其中一个条件
        ctx.insert("amount".to_string(), serde_json::json!(50.0));
        assert!(!OpcRuntime::evaluate_conditions(&conditions, &ctx));
    }

    #[test]
    fn test_evaluate_conditions_status_is() {
        let cond = AutomationCondition::StatusIs { status: "active".into() };
        let mut ctx = HashMap::new();
        ctx.insert("status".to_string(), serde_json::json!("active"));
        assert!(OpcRuntime::evaluate_conditions(std::slice::from_ref(&cond), &ctx));

        ctx.insert("status".to_string(), serde_json::json!("inactive"));
        assert!(!OpcRuntime::evaluate_conditions(&[cond], &ctx));
    }

    #[test]
    fn test_evaluate_conditions_field_below() {
        let cond = AutomationCondition::FieldBelow { field: "stock".into(), threshold: 10.0 };
        let mut ctx = HashMap::new();
        ctx.insert("stock".to_string(), serde_json::json!(5.0));
        assert!(OpcRuntime::evaluate_conditions(std::slice::from_ref(&cond), &ctx));

        ctx.insert("stock".to_string(), serde_json::json!(20.0));
        assert!(!OpcRuntime::evaluate_conditions(&[cond], &ctx));
    }

    #[test]
    fn test_evaluate_conditions_created_days() {
        let cond = AutomationCondition::CreatedDaysGte { days: 90 };
        let mut ctx = HashMap::new();
        ctx.insert("created_days".to_string(), serde_json::json!(120));
        assert!(OpcRuntime::evaluate_conditions(std::slice::from_ref(&cond), &ctx));

        ctx.insert("created_days".to_string(), serde_json::json!(30));
        assert!(!OpcRuntime::evaluate_conditions(&[cond], &ctx));
    }

    #[test]
    fn test_context_to_hashmap() {
        let ctx = RuleContext::new("invoice", "inv-001")
            .with_status("pending")
            .with_overdue_days(45)
            .with_created_days(60)
            .with_field("amount", serde_json::json!(1000.0));

        let runtime = create_empty_runtime();
        let map = runtime.context_to_hashmap(&ctx);

        assert_eq!(map.get("entity_type").unwrap().as_str().unwrap(), "invoice");
        assert_eq!(map.get("entity_id").unwrap().as_str().unwrap(), "inv-001");
        assert_eq!(map.get("status").unwrap().as_str().unwrap(), "pending");
        assert_eq!(map.get("overdue_days").unwrap().as_f64().unwrap(), 45.0);
        assert_eq!(map.get("created_days").unwrap().as_f64().unwrap(), 60.0);
        assert_eq!(map.get("amount").unwrap().as_f64().unwrap(), 1000.0);
    }
}
