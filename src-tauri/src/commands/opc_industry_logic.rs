// SPDX-License-Identifier: AGPL-3.0-only

//! 行业规则引擎（无运行时容器，命令直调 — 与股票业务同架构）
//!
//! 宏观要求：OPC 行业与股票业务**同架构**。股票业务 = 引擎 + 命令直调（无业务
//! 运行时容器）；本模块同理——OPC 行业 = 内建手写 adapter（`analysis-engine`）+ 命令直调，
//! **没有 opc-runtime / registry / 数据驱动 adapter 注册表**（已移除）。
//!
//! 本模块仅保留通用规则引擎的纯函数，供命令层对 `IndustryAutomationRule` 做条件求值
//! 与动作分发：
//! - `evaluate_conditions` / `eval_condition`：条件求值（通用纯函数）
//! - `context_to_hashmap`：将 `RuleContext` 展开为求值用键值映射
//! - `execute_rule_actions`：动作分发（日志 + 按需数据服务的实际数据库操作）

use std::collections::HashMap;
use std::sync::Arc;

use axagent_analysis_engine::opc::*;

// ── 通用规则引擎（自由函数，命令直调） ─────────────────────────

/// 检查自动化规则条件是否全部满足（通用纯函数）
pub fn evaluate_conditions(
    conditions: &[AutomationCondition],
    entity_context: &HashMap<String, serde_json::Value>,
) -> bool {
    conditions.iter().all(|cond| eval_condition(cond, entity_context))
}

fn eval_condition(cond: &AutomationCondition, ctx: &HashMap<String, serde_json::Value>) -> bool {
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
            tracing::debug!("[opc-industry] 跳过自定义条件表达式（需 Rhai 支持）: {}", expression);
            false
        },
    }
}

/// 将规则上下文展开为条件求值用的键值映射
pub fn context_to_hashmap(context: &RuleContext) -> HashMap<String, serde_json::Value> {
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
    if let Some(fields) = context.fields.as_object() {
        for (key, value) in fields {
            map.insert(key.clone(), value.clone());
        }
    }
    map
}

/// 执行规则动作（通用处理：日志 + 可选数据服务的实际数据库操作）
pub async fn execute_rule_actions(
    data_service: Option<&Arc<dyn OpcDataService>>,
    rule: &IndustryAutomationRule,
    context: &RuleContext,
) -> OpcResult<()> {
    for action in &rule.actions {
        match action {
            AutomationAction::SendNotification { target, message } => {
                tracing::info!("[opc-industry] 通知: target={}, message={}", target, message);
            },
            AutomationAction::UpdateStatus { status } => {
                tracing::info!(
                    "[opc-industry] 状态更新: entity={}/{} -> status={}",
                    context.entity_type,
                    context.entity_id,
                    status
                );
                if let Some(ds) = data_service {
                    ds.update_entity_status(&context.entity_type, &context.entity_id, status)
                        .await?;
                }
            },
            AutomationAction::UpdateField { field, value } => {
                tracing::info!(
                    "[opc-industry] 字段更新: entity={}/{} , field={}, value={}",
                    context.entity_type,
                    context.entity_id,
                    field,
                    value
                );
            },
            AutomationAction::MarkProcessed => {
                tracing::info!(
                    "[opc-industry] 标记已处理: entity={}/{}",
                    context.entity_type,
                    context.entity_id
                );
            },
            AutomationAction::CreateRecord { entity_type, data } => {
                tracing::info!(
                    "[opc-industry] 创建记录: entity_type={}, data={}",
                    entity_type,
                    data
                );
                if let Some(ds) = data_service {
                    let new_id = ds.create_entity_record(entity_type, data).await?;
                    tracing::info!(
                        "[opc-industry] 记录创建成功: id={}, entity_type={}",
                        new_id,
                        entity_type
                    );
                }
            },
        }
    }
    Ok(())
}
