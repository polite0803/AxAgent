// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流触发器持久化恢复。
//!
//! 启动时扫描 `workflow_templates` 表的 `trigger_config` 字段,
//! 对非 Manual 类型的触发器批量调用 `TriggerManager.register_*`,
//! 保证进程重启后定时 / Webhook / 事件触发器立即恢复运行。
//!
//! 注意:本恢复只重建运行时注册状态;`trigger_config` 字段本身已经
//! 持久化在 DB 中,无需额外落盘。

use axagent_dao::repo::workflow_template as db_repo;
use axagent_harness::workflow_types::{
    EventTriggerConfig, ScheduleTriggerConfig, TriggerConfig, TriggerType, WebhookTriggerConfig,
};
use axagent_rt_workflow::trigger::TriggerManager;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 从 DB 恢复所有非 Manual 触发器到运行时 `TriggerManager`。
///
/// 在 `start_background_services` 中调用一次,必须在 `WorkEngine::init_trigger_manager`
/// 之后执行 — 否则 `TriggerManager.engine` 为 None,触发时调用 `run_workflow` 会失败。
///
/// 返回成功恢复的触发器数量 (schedule, webhook, event)。
pub async fn recover_workflow_triggers(
    db: &DatabaseConnection,
    trigger_manager: &Arc<TriggerManager>,
) -> (usize, usize, usize) {
    let templates = match db_repo::list_workflow_templates(db, None).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("[trigger_recovery] 加载 workflow_templates 失败,跳过触发器恢复: {}", e);
            return (0, 0, 0);
        },
    };

    let mut sched_count = 0;
    let mut webhook_count = 0;
    let mut event_count = 0;

    for tpl in templates {
        let Some(cfg_str) = tpl.trigger_config.as_ref() else {
            continue;
        };
        let cfg: TriggerConfig = match serde_json::from_str(cfg_str) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    "[trigger_recovery] 模板 {} 的 trigger_config 解析失败,跳过: {}",
                    tpl.id,
                    e
                );
                continue;
            },
        };

        match cfg.trigger_type {
            TriggerType::Manual => {},
            TriggerType::Schedule => {
                // AxInvest 兼容：stock-analysis 等模板使用 `schedules` 多时段格式
                // （{ "schedules": { "morning": "0 9 * * 1-5", ... } }），由独立调度器
                // （如 start_stock_pipeline）管理，不通过 TriggerManager 注册。
                // 上游 ScheduleTriggerConfig 要求单个 `cron` 字段，解析会失败。
                // 此处检测到 `schedules` 字段时记 debug 跳过，不记 warn。
                if cfg.config.get("schedules").is_some() {
                    tracing::debug!(
                        "[trigger_recovery] 模板 {} 使用 AxInvest schedules 多时段格式,由独立调度器管理,跳过 TriggerManager 注册",
                        tpl.id
                    );
                    continue;
                }
                let sched: ScheduleTriggerConfig = match serde_json::from_value(cfg.config) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(
                            "[trigger_recovery] 模板 {} 的 schedule config 解析失败: {}",
                            tpl.id,
                            e
                        );
                        continue;
                    },
                };
                if !sched.enabled {
                    tracing::debug!(
                        "[trigger_recovery] 模板 {} 的 schedule 触发器已禁用,跳过",
                        tpl.id
                    );
                    continue;
                }
                match trigger_manager
                    .register_schedule(
                        &tpl.id,
                        &sched.cron,
                        &sched.timezone,
                        sched.input_params.clone(),
                    )
                    .await
                {
                    Ok(_) => {
                        sched_count += 1;
                        tracing::info!(
                            "[trigger_recovery] 恢复 schedule 触发器: workflow={} cron='{}' tz='{}'",
                            tpl.id,
                            sched.cron,
                            sched.timezone
                        );
                    },
                    Err(e) => {
                        tracing::warn!(
                            "[trigger_recovery] 注册 schedule 触发器失败 (workflow={}): {}",
                            tpl.id,
                            e
                        );
                    },
                }
            },
            TriggerType::Webhook => {
                let wh: WebhookTriggerConfig = match serde_json::from_value(cfg.config) {
                    Ok(w) => w,
                    Err(e) => {
                        tracing::warn!(
                            "[trigger_recovery] 模板 {} 的 webhook config 解析失败: {}",
                            tpl.id,
                            e
                        );
                        continue;
                    },
                };
                let mode = wh.response_mode.clone().unwrap_or_else(|| "async".to_string());
                trigger_manager.register_webhook(&tpl.id, &wh.path, &wh.method, &mode).await;
                webhook_count += 1;
                tracing::info!(
                    "[trigger_recovery] 恢复 webhook 触发器: workflow={} path='{}' method='{}'",
                    tpl.id,
                    wh.path,
                    wh.method
                );
            },
            TriggerType::Event => {
                let ev: EventTriggerConfig = match serde_json::from_value(cfg.config) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(
                            "[trigger_recovery] 模板 {} 的 event config 解析失败: {}",
                            tpl.id,
                            e
                        );
                        continue;
                    },
                };
                trigger_manager.register_event(&tpl.id, &ev.event_type).await;
                event_count += 1;
                tracing::info!(
                    "[trigger_recovery] 恢复 event 触发器: workflow={} event_type='{}'",
                    tpl.id,
                    ev.event_type
                );
            },
        }
    }

    tracing::info!(
        "[trigger_recovery] 恢复完成: {} schedule, {} webhook, {} event",
        sched_count,
        webhook_count,
        event_count
    );

    (sched_count, webhook_count, event_count)
}

/// 同步单个模板的触发器到运行时 (供 create/update 命令调用)。
///
/// - `template_id`: 工作流模板 ID
/// - `trigger_config`: 持久化后的触发器配置 (None 表示 Manual / 无触发器)
///
/// 内部逻辑:
/// 1. 先清理旧的运行时注册状态 (schedule + webhook)
/// 2. 若新配置非 Manual,按类型 register_*
///
/// Event 订阅因 `TriggerManager` 未提供按 workflow_id 反查 event_type 的接口,
/// 只能"追加注册",可能残留旧订阅 — 接受此风险(Event 触发时 run_workflow
/// 会返回 NOT_FOUND,warn 日志吞掉,无 panic 或数据损坏)。
///
/// 失败仅 warn 日志,不阻断命令返回 — DB 已持久化,下次启动恢复时会重新注册。
pub async fn sync_workflow_trigger(
    trigger_manager: &Arc<TriggerManager>,
    template_id: &str,
    trigger_config: Option<&TriggerConfig>,
) {
    // 先清理旧的运行时注册状态。
    // Schedule: 直接按 workflow_id 注销(TriggerManager 内部按 workflow_id 索引)。
    trigger_manager.unregister_schedule(template_id).await;

    // Webhook: 按 workflow_id 反查 path(模板数量有限,全表扫描可接受)。
    let routes = trigger_manager.get_webhook_routes().await;
    let paths_to_remove: Vec<String> = routes
        .into_iter()
        .filter_map(|(path, route)| {
            if route.workflow_id == template_id {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    for path in paths_to_remove {
        trigger_manager.unregister_webhook(&path).await;
    }
    // Event: 见上方注释,接受残留风险。

    let Some(cfg) = trigger_config else {
        return;
    };

    match cfg.trigger_type {
        TriggerType::Manual => {},
        TriggerType::Schedule => {
            match serde_json::from_value::<ScheduleTriggerConfig>(cfg.config.clone()) {
                Ok(sched) if sched.enabled => {
                    if let Err(e) = trigger_manager
                        .register_schedule(
                            template_id,
                            &sched.cron,
                            &sched.timezone,
                            sched.input_params.clone(),
                        )
                        .await
                    {
                        tracing::warn!(
                            "[trigger_sync] 注册 schedule 失败 (workflow={}): {}",
                            template_id,
                            e
                        );
                    }
                },
                Ok(_) => {
                    tracing::debug!(
                        "[trigger_sync] schedule 触发器已禁用,不注册 (workflow={})",
                        template_id
                    );
                },
                Err(e) => {
                    tracing::warn!(
                        "[trigger_sync] schedule config 解析失败 (workflow={}): {}",
                        template_id,
                        e
                    );
                },
            }
        },
        TriggerType::Webhook => {
            match serde_json::from_value::<WebhookTriggerConfig>(cfg.config.clone()) {
                Ok(wh) => {
                    let mode = wh.response_mode.clone().unwrap_or_else(|| "async".to_string());
                    trigger_manager
                        .register_webhook(template_id, &wh.path, &wh.method, &mode)
                        .await;
                },
                Err(e) => {
                    tracing::warn!(
                        "[trigger_sync] webhook config 解析失败 (workflow={}): {}",
                        template_id,
                        e
                    );
                },
            }
        },
        TriggerType::Event => {
            match serde_json::from_value::<EventTriggerConfig>(cfg.config.clone()) {
                Ok(ev) => {
                    trigger_manager.register_event(template_id, &ev.event_type).await;
                },
                Err(e) => {
                    tracing::warn!(
                        "[trigger_sync] event config 解析失败 (workflow={}): {}",
                        template_id,
                        e
                    );
                },
            }
        },
    }
}

/// 注销指定模板的所有运行时触发器 (供 delete 命令调用)。
///
/// 与 `sync_workflow_trigger` 的清理逻辑相同,但不重新注册。
/// Event 订阅可能残留,见 `sync_workflow_trigger` 的注释。
pub async fn unregister_workflow_triggers(
    trigger_manager: &Arc<TriggerManager>,
    template_id: &str,
) {
    // Schedule
    trigger_manager.unregister_schedule(template_id).await;

    // Webhook: 反向匹配 path
    let routes = trigger_manager.get_webhook_routes().await;
    let paths_to_remove: Vec<String> = routes
        .into_iter()
        .filter_map(|(path, route)| {
            if route.workflow_id == template_id {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    for path in paths_to_remove {
        trigger_manager.unregister_webhook(&path).await;
    }

    tracing::info!(
        "[trigger_sync] 已注销 workflow {} 的 schedule/webhook 运行时触发器 \
         (event 订阅可能残留,将在下次重启时清理)",
        template_id
    );
}
