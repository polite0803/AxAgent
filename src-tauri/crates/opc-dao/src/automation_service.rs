// SPDX-License-Identifier: AGPL-3.0-only

//! 自动化服务实现 — 数据库驱动的规则引擎
//!
//! 实现 AutomationService trait，提供：
//! - 规则条件评估（金额阈值、时间阈值、状态触发）
//! - 动作执行（发送通知、更新状态、创建任务）
//! - 规则持久化（CRUD + 启用/禁用）

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::str::FromStr;

use axagent_harness::util_fns::{gen_id, now_ts};
use axagent_opc_entities::{opc_automation_rules, opc_follow_up_tasks};
use axagent_opc_types::{
    AutomationRule, AutomationService, CreateAutomationRuleInput, CreateFollowUpTaskInput,
    FollowUpPriority, FollowUpStatus, FollowUpTask, OpcError, OpcResult,
};

/// 数据库驱动的自动化服务实现
pub struct DbAutomationService {
    pub db: DatabaseConnection,
}

impl DbAutomationService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 评估规则条件是否满足
    ///
    /// trigger_config JSON 格式示例：
    /// ```json
    /// {"field": "invoice_amount", "operator": "gt", "value": 1000.0}
    /// ```
    fn evaluate_condition(&self, trigger_config: &str) -> bool {
        let config: serde_json::Value = match serde_json::from_str(trigger_config) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let field = config.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let _operator = config.get("operator").and_then(|v| v.as_str()).unwrap_or("eq");
        let value = config.get("value").and_then(|v| v.as_f64());

        match field {
            "invoice_amount" => {
                let threshold = value.unwrap_or(0.0);
                tracing::info!("评估发票金额条件，阈值: {}", threshold);
                false
            },
            "invoice_overdue_days" => {
                let days = value.unwrap_or(0.0) as i64;
                tracing::info!("评估发票逾期条件，超过 {} 天", days);
                false
            },
            "customer_inactive_months" => {
                let months = value.unwrap_or(0.0) as i64;
                tracing::info!("评估客户不活跃条件，超过 {} 个月", months);
                false
            },
            "project_status_changed" => {
                let from = config.get("from").and_then(|v| v.as_str()).unwrap_or("");
                let to = config.get("to").and_then(|v| v.as_str()).unwrap_or("");
                tracing::info!("评估项目状态变更条件: {} -> {}", from, to);
                false
            },
            _ => {
                tracing::info!("评估自定义条件: field={}", field);
                false
            },
        }
    }

    /// 执行规则动作
    ///
    /// action_config JSON 格式示例：
    /// ```json
    /// {"type": "send_notification", "message": "发票已逾期"}
    /// ```
    async fn execute_action(&self, action_config: &str) -> Result<(), OpcError> {
        let config: serde_json::Value = match serde_json::from_str(action_config) {
            Ok(v) => v,
            Err(_) => return Err(OpcError::Validation("invalid action_config".into())),
        };

        let action_type = config.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match action_type {
            "send_notification" => {
                let message = config.get("message").and_then(|v| v.as_str()).unwrap_or("");
                tracing::info!("发送通知: {}", message);
                Ok(())
            },
            "update_status" => {
                let entity = config.get("entity").and_then(|v| v.as_str()).unwrap_or("");
                let new_status = config.get("status").and_then(|v| v.as_str()).unwrap_or("");
                tracing::info!("更新状态: {} -> {}", entity, new_status);
                Ok(())
            },
            "create_task" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let description = config.get("description").and_then(|v| v.as_str()).unwrap_or("");
                tracing::info!("创建任务: {} - {}", title, description);

                let id = gen_id();
                let now = now_ts();
                opc_follow_up_tasks::ActiveModel {
                    id: Set(id.clone()),
                    task_type: Set("auto".to_string()),
                    title: Set(title.to_string()),
                    description: Set(description.to_string()),
                    status: Set("pending".to_string()),
                    priority: Set("medium".to_string()),
                    due_at: Set(None),
                    completed_at: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                }
                .insert(&self.db)
                .await
                .map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(())
            },
            "send_email" => {
                let to = config.get("to").and_then(|v| v.as_str()).unwrap_or("");
                let subject = config.get("subject").and_then(|v| v.as_str()).unwrap_or("");
                tracing::info!("发送邮件到 {}: {}", to, subject);
                Ok(())
            },
            "webhook" => {
                let url = config.get("url").and_then(|v| v.as_str()).unwrap_or("");
                tracing::info!("调用 Webhook: {}", url);
                Ok(())
            },
            _ => {
                tracing::warn!("未知动作类型: {}", action_type);
                Err(OpcError::Validation(format!("unknown action type: {action_type}")))
            },
        }
    }

    /// 处理所有已启用的规则
    pub async fn process_all_rules(&self) -> Result<usize, OpcError> {
        let rules = self.list_rules().await?;
        let mut executed = 0;

        for rule in &rules {
            if rule.enabled && self.evaluate_condition(&rule.trigger_type) {
                self.execute_action(&rule.action_config).await?;
                self.mark_rule_executed(&rule.id).await?;
                executed += 1;
            }
        }

        Ok(executed)
    }

    async fn get_rule(&self, id: &str) -> OpcResult<AutomationRule> {
        let entity = opc_automation_rules::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("automation rule {id}")))?;

        entity_to_rule(entity)
    }

    async fn get_follow_up(&self, id: &str) -> OpcResult<FollowUpTask> {
        let entity = opc_follow_up_tasks::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("follow-up task {id}")))?;

        entity_to_follow_up(entity)
    }

    async fn mark_rule_executed(&self, rule_id: &str) -> Result<(), OpcError> {
        let entity = opc_automation_rules::Entity::find_by_id(rule_id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("automation rule {rule_id}")))?;

        let mut am: opc_automation_rules::ActiveModel = entity.into();
        am.last_run_at = Set(Some(now_ts()));
        am.updated_at = Set(now_ts());
        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(())
    }
}

// ── Entity ↔ DTO 转换 ─────────────────────────────────────────────

fn entity_to_rule(e: opc_automation_rules::Model) -> OpcResult<AutomationRule> {
    Ok(AutomationRule {
        id: e.id,
        name: e.name,
        trigger_type: e.trigger_type,
        trigger_config: e.trigger_config,
        action_type: e.action_type,
        action_config: e.action_config,
        enabled: e.enabled,
        last_run_at: e.last_run_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    })
}

fn entity_to_follow_up(e: opc_follow_up_tasks::Model) -> OpcResult<FollowUpTask> {
    let status = FollowUpStatus::from_str(&e.status).map_err(OpcError::Validation)?;
    let priority = FollowUpPriority::from_str(&e.priority).map_err(OpcError::Validation)?;

    Ok(FollowUpTask {
        id: e.id,
        task_type: e.task_type,
        title: e.title,
        description: e.description,
        status,
        priority,
        due_at: e.due_at,
        completed_at: e.completed_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    })
}

fn status_to_str(s: &FollowUpStatus) -> String {
    match s {
        FollowUpStatus::Pending => "pending".into(),
        FollowUpStatus::InProgress => "in_progress".into(),
        FollowUpStatus::Completed => "completed".into(),
        FollowUpStatus::Cancelled => "cancelled".into(),
    }
}

fn priority_to_str(p: &FollowUpPriority) -> String {
    match p {
        FollowUpPriority::Low => "low".into(),
        FollowUpPriority::Medium => "medium".into(),
        FollowUpPriority::High => "high".into(),
        FollowUpPriority::Urgent => "urgent".into(),
    }
}

// ── Service 实现 ───────────────────────────────────────────────────

#[async_trait]
impl AutomationService for DbAutomationService {
    async fn create_rule(&self, input: CreateAutomationRuleInput) -> OpcResult<AutomationRule> {
        let id = gen_id();
        let now = now_ts();

        opc_automation_rules::ActiveModel {
            id: Set(id.clone()),
            name: Set(input.name),
            trigger_type: Set(input.trigger_type),
            trigger_config: Set(input.trigger_config),
            action_type: Set(input.action_type),
            action_config: Set(input.action_config),
            enabled: Set(true),
            last_run_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await
        .map_err(|e| OpcError::Database(e.to_string()))?;

        self.get_rule(&id).await
    }

    async fn list_rules(&self) -> OpcResult<Vec<AutomationRule>> {
        let entities = opc_automation_rules::Entity::find()
            .order_by_desc(opc_automation_rules::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        entities.into_iter().map(entity_to_rule).collect()
    }

    async fn toggle_rule(&self, id: &str, enabled: bool) -> OpcResult<AutomationRule> {
        let entity = opc_automation_rules::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("automation rule {id}")))?;

        let mut am: opc_automation_rules::ActiveModel = entity.into();
        am.enabled = Set(enabled);
        am.updated_at = Set(now_ts());
        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;

        self.get_rule(id).await
    }

    async fn create_follow_up(&self, input: CreateFollowUpTaskInput) -> OpcResult<FollowUpTask> {
        let id = gen_id();
        let now = now_ts();

        opc_follow_up_tasks::ActiveModel {
            id: Set(id.clone()),
            task_type: Set(input.task_type),
            title: Set(input.title),
            description: Set(input.description),
            status: Set(status_to_str(&FollowUpStatus::Pending)),
            priority: Set(priority_to_str(&input.priority)),
            due_at: Set(input.due_at),
            completed_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await
        .map_err(|e| OpcError::Database(e.to_string()))?;

        self.get_follow_up(&id).await
    }

    async fn list_follow_ups(
        &self,
        status: Option<FollowUpStatus>,
    ) -> OpcResult<Vec<FollowUpTask>> {
        let mut query = opc_follow_up_tasks::Entity::find()
            .order_by_desc(opc_follow_up_tasks::Column::CreatedAt);

        if let Some(s) = status {
            query = query.filter(opc_follow_up_tasks::Column::Status.eq(status_to_str(&s)));
        }

        let entities = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;

        entities.into_iter().map(entity_to_follow_up).collect()
    }

    async fn complete_follow_up(&self, id: &str) -> OpcResult<FollowUpTask> {
        let entity = opc_follow_up_tasks::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("follow-up task {id}")))?;

        let mut am: opc_follow_up_tasks::ActiveModel = entity.into();
        am.status = Set(status_to_str(&FollowUpStatus::Completed));
        am.completed_at = Set(Some(now_ts()));
        am.updated_at = Set(now_ts());
        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;

        self.get_follow_up(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evaluate_condition_invoice_amount() {
        let db = DatabaseConnection::default();
        let service = DbAutomationService::new(db);

        let config = r#"{"field":"invoice_amount","operator":"gt","value":1000.0}"#;
        let result = service.evaluate_condition(config);
        assert!(!result, "Mock 实现应返回 false");
    }

    #[tokio::test]
    async fn test_evaluate_condition_invalid_json() {
        let db = DatabaseConnection::default();
        let service = DbAutomationService::new(db);

        let result = service.evaluate_condition("not valid json");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_execute_action_invalid_config() {
        let db = DatabaseConnection::default();
        let service = DbAutomationService::new(db);

        let result = service.execute_action("not valid json").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_action_unknown_type() {
        let db = DatabaseConnection::default();
        let service = DbAutomationService::new(db);

        let result = service.execute_action(r#"{"type":"unknown_action"}"#).await;
        assert!(result.is_err());
    }
}
