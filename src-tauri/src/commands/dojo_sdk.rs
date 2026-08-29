// SPDX-License-Identifier: AGPL-3.0-only
//! G19 Plan 三件套 — 分层执行计划创建、控制、重规划
//!
//! 复用 `axagent_agent::HierarchicalPlanner`，通过全局 `PLANS_REGISTRY` 管理
//! `plan_id → planner`，配合 TTL 清理机制防止内存膨胀。
//!
//! ## 三件套
//! - `dojo_create_plan` — 创建分层执行计划
//! - `dojo_execute_plan` — 控制执行（start/pause/resume/cancel/progress/next_tasks/complete_task/fail_task）
//! - `dojo_revise_plan` — 重规划（Retry/Skip/Insert/Remove/Reorder/AddPhase/ModifyTask + Rollback）
//!
//! ## 启动注册
//!
//! 在 `init::services` 中调用 `spawn_plan_ttl_cleanup` 启动后台清理任务。

use axagent_harness::plan_types::{
    ActionType, Phase, PhaseStatus, PlannedTask, ReplanAction, ReplanReason, TaskStatus,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

// ── G19 Plan 三件套工具实现 ──────────────────────────────────────────────
//
// 复用 axagent_agent::HierarchicalPlanner，通过全局 PLANS_REGISTRY 管理 plan_id → planner。
// 三件套：
// - dojo_create_plan: 创建分层执行计划
// - dojo_execute_plan: 控制执行（start/pause/resume/cancel/progress/next_tasks/complete_task/fail_task）
// - dojo_revise_plan: 重规划（Retry/Skip/Insert/Remove/Reorder/AddPhase/ModifyTask + Rollback）
//
// P2-9: 引入 TTL 清理机制，避免长期运行导致 PLANS_REGISTRY 内存膨胀。
// - 每个 PlanEntry 记录 last_accessed_at（create/execute/revise 时更新）
// - 默认 TTL = 24 小时，超期未访问的计划会被 cleanup_expired_plans 删除
// - 在 init::services 中通过 tokio::spawn 启动后台定时清理任务

/// 默认 TTL：24 小时（单位：秒）
pub const PLAN_DEFAULT_TTL_SECS: u64 = 24 * 60 * 60;

/// 清理任务执行间隔：1 小时
pub const PLAN_CLEANUP_INTERVAL_SECS: u64 = 60 * 60;

/// Plan Registry 表项：planner + 最后访问时间
struct PlanEntry {
    planner: Arc<Mutex<axagent_agent::HierarchicalPlanner>>,
    last_accessed_at: std::time::Instant,
}

/// 全局 Plan Registry：plan_id → PlanEntry
#[allow(clippy::type_complexity)]
static PLANS_REGISTRY: LazyLock<Arc<Mutex<HashMap<String, PlanEntry>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

/// 清理过期的 plan（由后台定时任务调用）
///
/// 删除 `last_accessed_at` 距今超过 `ttl_secs` 的所有 plan。
/// 返回被清理的 plan_id 列表。
pub async fn cleanup_expired_plans(ttl_secs: u64) -> Vec<String> {
    let ttl = std::time::Duration::from_secs(ttl_secs);
    let now = std::time::Instant::now();
    let mut registry = PLANS_REGISTRY.lock().await;
    let expired_ids: Vec<String> = registry
        .iter()
        .filter_map(|(id, entry)| {
            if now.duration_since(entry.last_accessed_at) > ttl {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    for id in &expired_ids {
        registry.remove(id);
    }
    if !expired_ids.is_empty() {
        tracing::info!(expired_count = expired_ids.len(), ttl_secs, "[G19 TTL] 清理过期 plan");
    }
    expired_ids
}

/// 启动 plan TTL 清理后台任务（在 init::services 中调用一次）
///
/// 接受 shutdown_token 以便应用关闭时优雅退出，与项目其它后台任务保持一致。
///
/// 使用 `tauri::async_runtime::spawn` 而非 `tokio::spawn`，因为本函数在
/// `start_background_services`（setup hook 同步上下文）中调用，此时可能没有
/// tokio runtime。`tauri::async_runtime::spawn` 会自动选择合适的运行时。
pub fn spawn_plan_ttl_cleanup(shutdown_token: tokio_util::sync::CancellationToken) {
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(PLAN_CLEANUP_INTERVAL_SECS);
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("[G19 TTL] 收到关闭信号，停止 plan TTL 清理任务");
                    break;
                },
                _ = tokio::time::sleep(interval) => {
                    // 静默清理，失败仅记录日志
                    let expired = cleanup_expired_plans(PLAN_DEFAULT_TTL_SECS).await;
                    if !expired.is_empty() {
                        tracing::debug!(
                            expired_ids = ?expired,
                            "[G19 TTL] 后台清理任务完成"
                        );
                    }
                },
            }
        }
    });
}

/// 生成 plan_id（短 UUID）
fn generate_plan_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string().split('-').next().unwrap_or("plan").to_string()
}

/// 从 JSON 构造 Phase（自动生成 ID）
fn phase_from_json(phase_json: &Value, phase_idx: usize) -> Result<Phase, String> {
    let name =
        phase_json["name"].as_str().ok_or_else(|| "phase.name 缺失".to_string())?.to_string();
    let description = phase_json["description"].as_str().unwrap_or("").to_string();
    let dependencies: Vec<String> = phase_json["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    // 兼容整数索引（1=第一Phase）与字符串 ID
                    if let Some(n) = v.as_u64() {
                        Some(format!("phase_{}", n))
                    } else {
                        v.as_str().map(String::from)
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let phase_id = format!("phase_{}", phase_idx + 1);
    let tasks_json =
        phase_json["tasks"].as_array().ok_or_else(|| format!("phase '{name}' 缺少 tasks 数组"))?;

    let mut tasks = Vec::with_capacity(tasks_json.len());
    for (ti, tj) in tasks_json.iter().enumerate() {
        let task = task_from_json(tj, ti)?;
        tasks.push(task);
    }

    Ok(Phase { id: phase_id, name, description, tasks, dependencies, status: PhaseStatus::Pending })
}

/// 从 JSON 构造 PlannedTask（自动生成 ID）
fn task_from_json(task_json: &Value, task_idx: usize) -> Result<PlannedTask, String> {
    let description = task_json["description"]
        .as_str()
        .ok_or_else(|| "task.description 缺失".to_string())?
        .to_string();
    let action_type = ActionType::from_str(task_json["action_type"].as_str().unwrap_or("agent"))
        .unwrap_or(ActionType::Agent);
    let parameters = task_json["parameters"].clone();
    let dependencies: Vec<String> = task_json["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    if let Some(n) = v.as_u64() {
                        Some(format!("task_{}", n))
                    } else {
                        v.as_str().map(String::from)
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let max_retries = task_json["max_retries"].as_u64().unwrap_or(3) as u32;
    let assigned_role = task_json["assigned_role"].as_str().map(String::from);

    Ok(PlannedTask {
        id: format!("task_{}", task_idx + 1),
        description,
        action_type,
        parameters,
        dependencies,
        status: TaskStatus::Pending,
        result: None,
        error: None,
        retry_count: 0,
        max_retries,
        assigned_role,
        compensation: None,
    })
}

/// 创建分层执行计划
#[tauri::command]
pub async fn execute_create_plan(arguments: Value) -> Result<String, String> {
    let goal = arguments["goal"].as_str().ok_or_else(|| "goal 参数缺失".to_string())?.to_string();
    let phases_json =
        arguments["phases"].as_array().ok_or_else(|| "phases 参数缺失或非数组".to_string())?;

    if phases_json.is_empty() {
        return Err("phases 不能为空".to_string());
    }

    let mut phases = Vec::with_capacity(phases_json.len());
    for (i, pj) in phases_json.iter().enumerate() {
        phases.push(phase_from_json(pj, i)?);
    }

    let mut planner = axagent_agent::HierarchicalPlanner::new();
    let plan_ref = planner.create_plan(&goal, phases);
    let plan_id = generate_plan_id();
    let plan_snapshot = plan_ref.clone();

    let planner_arc = Arc::new(Mutex::new(planner));
    PLANS_REGISTRY.lock().await.insert(
        plan_id.clone(),
        PlanEntry { planner: planner_arc, last_accessed_at: std::time::Instant::now() },
    );

    let response = json!({
        "status": "ok",
        "plan_id": plan_id,
        "plan": plan_snapshot,
        "message": "计划已创建。调用 dojo_execute_plan(action='start') 开始执行。",
    });
    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 执行计划控制
#[tauri::command]
pub async fn execute_execute_plan(arguments: Value) -> Result<String, String> {
    let plan_id =
        arguments["plan_id"].as_str().ok_or_else(|| "plan_id 参数缺失".to_string())?.to_string();
    let action = arguments["action"].as_str().unwrap_or("start").to_string();

    // P2-9: 在持锁期间一并更新 last_accessed_at，避免双重锁
    let planner_arc = {
        let mut registry = PLANS_REGISTRY.lock().await;
        let entry = registry
            .get_mut(&plan_id)
            .ok_or_else(|| format!("计划 {plan_id} 不存在（可能已过期或未创建）"))?;
        entry.last_accessed_at = std::time::Instant::now();
        entry.planner.clone()
    };

    let mut planner = planner_arc.lock().await;

    let response = match action.as_str() {
        "start" => {
            planner.start_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "start",
                "plan_status": plan.status,
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "pause" => {
            planner.pause_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "pause",
                "plan_status": plan.status,
            })
        },
        "resume" => {
            planner.resume_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "resume",
                "plan_status": plan.status,
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "cancel" => {
            planner.cancel_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "cancel",
                "plan_status": plan.status,
            })
        },
        "progress" => {
            let progress = planner.get_progress();
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "progress",
                "plan_status": plan.status,
                "progress": progress,
            })
        },
        "next_tasks" => {
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "next_tasks",
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "complete_task" => {
            let task_id = arguments["task_id"]
                .as_str()
                .ok_or_else(|| "complete_task 需要 task_id 参数".to_string())?
                .to_string();
            let result =
                arguments.get("result").cloned().unwrap_or_else(|| json!({"status": "done"}));
            planner.mark_task_started(&task_id)?;
            planner.mark_task_completed(&task_id, result)?;
            let progress = planner.get_progress();
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "complete_task",
                "task_id": task_id,
                "plan_status": plan.status,
                "progress": progress,
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "fail_task" => {
            let task_id = arguments["task_id"]
                .as_str()
                .ok_or_else(|| "fail_task 需要 task_id 参数".to_string())?
                .to_string();
            let error = arguments["error"].as_str().unwrap_or("未知错误").to_string();
            // 若任务未启动则先标记 started（HierarchicalPlanner 要求）
            let _ = planner.mark_task_started(&task_id);
            planner.mark_task_failed(&task_id, &error)?;
            let progress = planner.get_progress();
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "fail_task",
                "task_id": task_id,
                "error": error,
                "plan_status": plan.status,
                "progress": progress,
            })
        },
        other => {
            return Err(format!(
                "未知 action: {other}（支持 start/pause/resume/cancel/progress/next_tasks/complete_task/fail_task）"
            ));
        },
    };

    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 重规划（修订计划）
#[tauri::command]
pub async fn execute_revise_plan(arguments: Value) -> Result<String, String> {
    let plan_id =
        arguments["plan_id"].as_str().ok_or_else(|| "plan_id 参数缺失".to_string())?.to_string();

    // P2-9: 在持锁期间一并更新 last_accessed_at，避免双重锁
    let planner_arc = {
        let mut registry = PLANS_REGISTRY.lock().await;
        let entry = registry.get_mut(&plan_id).ok_or_else(|| format!("计划 {plan_id} 不存在"))?;
        entry.last_accessed_at = std::time::Instant::now();
        entry.planner.clone()
    };

    let mut planner = planner_arc.lock().await;

    // 回滚模式
    if let Some(version) = arguments["rollback_to_version"].as_u64() {
        let restored_plan = planner.rollback(version as u32)?;
        let response = json!({
            "status": "ok",
            "action": "rollback",
            "rollback_to_version": version,
            "plan": restored_plan,
        });
        return serde_json::to_string(&response).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        });
    }

    // 重规划模式
    let reason_str = arguments["reason"].as_str().ok_or_else(|| "reason 参数缺失".to_string())?;
    let actions_json =
        arguments["actions"].as_array().ok_or_else(|| "actions 参数缺失或非数组".to_string())?;

    let reason = parse_replan_reason(reason_str, &arguments)?;
    let actions = parse_replan_actions(actions_json)?;

    let record = planner.replan(reason, actions)?;
    let plan = planner.get_plan().ok_or("计划不存在")?.clone();
    let progress = planner.get_progress();

    let response = json!({
        "status": "ok",
        "action": "revise",
        "plan": plan,
        "progress": progress,
        "record": record,
        "version_history_count": planner.get_plan_versions().len(),
    });
    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 解析 ReplanReason
fn parse_replan_reason(reason_str: &str, args: &Value) -> Result<ReplanReason, String> {
    match reason_str {
        "StepFailed" => {
            let task_id = args["task_id"]
                .as_str()
                .or_else(|| args["details"]["task_id"].as_str())
                .ok_or_else(|| "StepFailed 需要 task_id 参数".to_string())?
                .to_string();
            let error = args["error"]
                .as_str()
                .or_else(|| args["details"]["error"].as_str())
                .unwrap_or("unknown error")
                .to_string();
            Ok(ReplanReason::StepFailed { task_id, error })
        },
        "NewDependencyDiscovered" => {
            let task_id = args["task_id"]
                .as_str()
                .or_else(|| args["details"]["task_id"].as_str())
                .ok_or_else(|| "NewDependencyDiscovered 需要 task_id 参数".to_string())?
                .to_string();
            let dependency = args["dependency"]
                .as_str()
                .or_else(|| args["details"]["dependency"].as_str())
                .ok_or_else(|| "NewDependencyDiscovered 需要 dependency 参数".to_string())?
                .to_string();
            Ok(ReplanReason::NewDependencyDiscovered { task_id, dependency })
        },
        "GoalChanged" => {
            let old_goal = args["old_goal"]
                .as_str()
                .or_else(|| args["details"]["old_goal"].as_str())
                .unwrap_or("")
                .to_string();
            let new_goal = args["new_goal"]
                .as_str()
                .or_else(|| args["details"]["new_goal"].as_str())
                .unwrap_or("")
                .to_string();
            Ok(ReplanReason::GoalChanged { old_goal, new_goal })
        },
        "ResourceConstraint" => {
            let constraint = args["constraint"]
                .as_str()
                .or_else(|| args["details"]["constraint"].as_str())
                .unwrap_or("unspecified constraint")
                .to_string();
            Ok(ReplanReason::ResourceConstraint { constraint })
        },
        "ManualIntervention" => {
            let reason = args["detail"]
                .as_str()
                .or_else(|| args["details"]["reason"].as_str())
                .unwrap_or("manual intervention")
                .to_string();
            Ok(ReplanReason::ManualIntervention { reason })
        },
        other => Err(format!(
            "未知 reason: {other}（支持 StepFailed/NewDependencyDiscovered/GoalChanged/ResourceConstraint/ManualIntervention）"
        )),
    }
}

/// 解析 ReplanAction 数组
fn parse_replan_actions(actions_json: &[Value]) -> Result<Vec<ReplanAction>, String> {
    let mut actions = Vec::with_capacity(actions_json.len());
    for aj in actions_json {
        let action_type = aj["type"].as_str().ok_or_else(|| "action.type 缺失".to_string())?;
        let action = match action_type {
            "Retry" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Retry 需要 task_id".to_string())?
                    .to_string();
                let modified_parameters = aj.get("modified_parameters").cloned();
                ReplanAction::Retry { task_id, modified_parameters }
            },
            "Skip" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Skip 需要 task_id".to_string())?
                    .to_string();
                let reason = aj["reason"].as_str().unwrap_or("").to_string();
                ReplanAction::Skip { task_id, reason }
            },
            "Insert" => {
                let phase_id = aj["phase_id"]
                    .as_str()
                    .ok_or_else(|| "Insert 需要 phase_id".to_string())?
                    .to_string();
                let task_json =
                    aj.get("task").ok_or_else(|| "Insert 需要 task 定义".to_string())?;
                let task = task_from_json(task_json, 0)?;
                let position = aj["position"].as_u64().unwrap_or(0) as usize;
                ReplanAction::Insert { phase_id, task, position }
            },
            "Remove" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Remove 需要 task_id".to_string())?
                    .to_string();
                let reason = aj["reason"].as_str().unwrap_or("").to_string();
                ReplanAction::Remove { task_id, reason }
            },
            "Reorder" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Reorder 需要 task_id".to_string())?
                    .to_string();
                let new_position =
                    aj["new_position"].as_u64().or_else(|| aj["position"].as_u64()).unwrap_or(0)
                        as usize;
                ReplanAction::Reorder { task_id, new_position }
            },
            "AddPhase" => {
                let phase_json =
                    aj.get("phase").ok_or_else(|| "AddPhase 需要 phase 定义".to_string())?;
                let phase = phase_from_json(phase_json, 0)?;
                let position = aj["position"].as_u64().unwrap_or(0) as usize;
                ReplanAction::AddPhase { phase, position }
            },
            "ModifyTask" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "ModifyTask 需要 task_id".to_string())?
                    .to_string();
                let modifications = aj["modifications"].clone();
                ReplanAction::ModifyTask { task_id, modifications }
            },
            other => return Err(format!("未知 action type: {other}")),
        };
        actions.push(action);
    }
    Ok(actions)
}

/// 任务简略信息（用于响应返回）
fn task_brief(t: &PlannedTask) -> Value {
    json!({
        "id": t.id,
        "description": t.description,
        "action_type": t.action_type,
        "status": format!("{:?}", t.status),
        "assigned_role": t.assigned_role,
        "dependencies": t.dependencies,
        "retry_count": t.retry_count,
        "max_retries": t.max_retries,
    })
}
