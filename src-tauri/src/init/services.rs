// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::index_queue::IndexJobService;
use chrono;
use notify::{Event, RecursiveMode, Watcher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

pub fn start_background_services(
    app: &tauri::AppHandle,
    state: &AppState,
    app_dir: std::path::PathBuf,
    _tray_language: String,
) {
    start_auto_backup(app, state, app_dir.clone());
    start_webdav_sync(app, state, app_dir);
    #[cfg(not(mobile))]
    start_tray(app, &_tray_language);
    start_closed_loop_service(app, state);
    start_insight_generation(state);
    start_pattern_learning(state);
    start_cross_session_learning(state);
    start_rl_reward_computation(state);
    start_batch_processing(state);
    start_user_profile_persistence(state);
    start_skill_evolution(state);
    start_auto_tool_observation(state);
    start_text_grad_analysis(state);
    start_cron_scheduler(state);
    start_platform_adapters(state);
    start_skill_watcher(app, state);
    start_memory_decay_tick(state);
    start_memory_maintenance_tick(state);
    start_trajectory_cleanup(state);
    start_index_job_service(app, state);
    start_plugins(state);
    // 反思工作流定时任务（方向1：接入上游反思基础设施）
    start_batch_reflection(state);
    start_lesson_validation(state);
    // [方向6] DreamConsolidator：每日 18:00 跨股票蒸馏反思轨迹
    start_dream_consolidation(state);
    // 股票全业务管道：每日 18:00 自动发现+分析+持仓再评估
    start_stock_pipeline(app, state);
}

fn start_plugins(state: &AppState) {
    let plugin_manager = state.plugin_manager.clone();
    let dashboard_registry = state.dashboard_registry.clone();

    tauri::async_runtime::spawn(async move {
        tracing::info!("Initializing plugin system...");

        let mut manager = plugin_manager.write().await;
        let _started = match manager.start_enabled_plugins() {
            Ok(started) => {
                if !started.is_empty() {
                    tracing::info!(
                        "Started {} enabled plugin(s): {}",
                        started.len(),
                        started.join(", ")
                    );
                } else {
                    tracing::info!("No enabled plugins to start");
                }
                started
            },
            Err(e) => {
                tracing::error!("Failed to start enabled plugins: {e}");
                Vec::new()
            },
        };

        drop(manager);

        if let Some(registry) = dashboard_registry {
            if let Err(e) = registry.reload().await {
                tracing::warn!("Failed to reload dashboard plugins: {e}");
            } else {
                let count = registry.list_plugins().await.len();
                tracing::info!("Loaded {count} dashboard plugin(s)");
            }
        }

        tracing::info!("Plugin system initialization complete");
    });
}

fn start_auto_backup(app: &tauri::AppHandle, state: &AppState, app_dir: std::path::PathBuf) {
    let db = state.harness.db().clone();
    let app_data = app_dir.clone();
    let handle = state.auto_backup_handle.clone();
    let shutdown_token = state.shutdown_token.clone();
    let app_for_emit = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_dao::repo::settings::get_settings(&db).await {
            if settings.auto_backup_enabled && settings.auto_backup_interval_hours > 0 {
                let backup_dir_setting =
                    axagent_storage::path_vars::decode_path_opt(&settings.backup_dir);
                let interval = settings.auto_backup_interval_hours;
                let max_count = settings.auto_backup_max_count;
                let interval_secs = interval as u64 * 3600;
                let db2 = db.clone();
                let app_dir2 = app_data.clone();
                let shutdown_token = shutdown_token.clone();
                let app_for_backup = app_for_emit.clone();

                let initial_delay_secs = match axagent_dao::repo::backup::list_backups(
                    &db,
                    &axagent_storage::DefaultPathEncoder,
                )
                .await
                {
                    Ok(backups) if !backups.is_empty() => {
                        let last_ts = &backups[0].created_at;
                        if let Ok(last_time) =
                            chrono::NaiveDateTime::parse_from_str(last_ts, "%Y-%m-%d %H:%M:%S")
                        {
                            let elapsed = chrono::Utc::now()
                                .naive_utc()
                                .signed_duration_since(last_time)
                                .num_seconds()
                                .max(0) as u64;
                            interval_secs.saturating_sub(elapsed)
                        } else {
                            interval_secs
                        }
                    },
                    _ => interval_secs,
                };

                let task = tokio::spawn(async move {
                    let dur = std::time::Duration::from_secs(interval_secs);
                    tokio::time::sleep(std::time::Duration::from_secs(initial_delay_secs)).await;
                    loop {
                        tokio::select! {
                            _ = shutdown_token.cancelled() => {
                                tracing::info!("[auto_backup] 收到关闭信号，停止自动备份");
                                break;
                            }
                            _ = tokio::time::sleep(dur) => {
                                let backup_dir = axagent_dao::repo::backup::resolve_backup_dir(
                                    backup_dir_setting.as_deref(),
                                    &app_dir2,
                                );
                                if let Err(e) =
                                    axagent_dao::repo::backup::create_backup(&db2, "sqlite", &backup_dir, &axagent_storage::DefaultPathEncoder)
                                        .await
                                {
                                    tracing::warn!("Auto-backup failed: {}", e);
                                    let _ = app_for_backup.emit("auto-backup-completed", serde_json::json!({
                                        "success": false,
                                        "error": e.to_string(),
                                    }));
                                } else {
                                    tracing::info!("Auto-backup created");
                                    let _ = app_for_backup.emit("auto-backup-completed", serde_json::json!({
                                        "success": true,
                                        "message": "Auto-backup created successfully",
                                    }));
                                    let _ =
                                        axagent_dao::repo::backup::cleanup_old_backups(&db2, max_count, &axagent_storage::DefaultPathEncoder)
                                            .await;
                                }
                            }
                        }
                    }
                });
                *handle.lock().await = Some(task);
            }
        }
    });
}

fn start_memory_maintenance_tick(state: &AppState) {
    let memory_service = state.memory_service.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("memory_maintenance", async move {
        let interval = std::time::Duration::from_secs(7200);
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[memory_maintenance] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    let ms = memory_service.read().await;
                    let disambiguation = ms.disambiguate_entities().await;
                    drop(ms);
                    if disambiguation.merged > 0 {
                        tracing::info!(
                            "[memory_maintenance] Disambiguated entities: merged {} of {}",
                            disambiguation.merged,
                            disambiguation.total
                        );
                    }
                    let ms = memory_service.read().await;
                    let clusters = ms.find_similar_clusters(0.75).await;
                    drop(ms);
                    if !clusters.is_empty() {
                        tracing::info!(
                            "[memory_maintenance] Found {} similar memory clusters (potential duplicates)",
                            clusters.len()
                        );
                    }
                }
            }
        }
    });
}

/// 反思工作流定时任务：定期扫描 pending row 并执行反思。
///
/// 每 6 小时运行一次，首次延迟 60 秒（避免启动时抢资源）。
/// 调用 `run_batch_reflection_inner` 处理最多 20 条 pending row。
/// 监听 `shutdown_token` 支持优雅关闭。
fn start_batch_reflection(state: &AppState) {
    let db = state.harness.db().clone();
    let client = state.astock_client.clone();
    let engine = state.work_engine.clone();
    let vector_store = state.vector_store.clone();
    let master_key = state.harness.master_key_owned();
    // [方向3] 透传 TrajectoryStorage，让批量反思也持久化执行轨迹
    let trajectory_storage = state.trajectory_storage.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("batch_reflection", async move {
        let initial_delay = std::time::Duration::from_secs(60);
        let interval = std::time::Duration::from_secs(6 * 3600); // 6 小时
        tokio::time::sleep(initial_delay).await;
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[batch_reflection] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    tracing::info!("[batch_reflection] 开始执行批量反思");
                    match crate::commands::stock_workflow::run_batch_reflection_inner(
                        &db,
                        &client,
                        &engine,
                        &vector_store,
                        &master_key,
                        None,
                        Some(&trajectory_storage),
                    ).await {
                        Ok(result) => {
                            tracing::info!(
                                "[batch_reflection] 完成: {}",
                                serde_json::to_string(&result).unwrap_or_default()
                            );
                        }
                        Err(e) => {
                            tracing::error!("[batch_reflection] 失败: {e}");
                        }
                    }
                }
            }
        }
    });
}

/// 反思规则有效性验证定时任务：定期校证 `reflection_lessons` 表的 confidence。
///
/// 每 24 小时运行一次，首次延迟 120 秒。
/// 调用 `run_lesson_validation` 基于 `times_applied` 和 `actual_success_rate`
/// 自动调整每条 lesson 的 confidence（×1.1 提升 / ×0.8 衰减 / 0.1 强废弃）。
/// 监听 `shutdown_token` 支持优雅关闭。
fn start_lesson_validation(state: &AppState) {
    let db = state.harness.db().clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("lesson_validation", async move {
        let initial_delay = std::time::Duration::from_secs(120);
        let interval = std::time::Duration::from_secs(24 * 3600); // 24 小时
        tokio::time::sleep(initial_delay).await;
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[lesson_validation] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    tracing::info!("[lesson_validation] 开始校证反思规则置信度");
                    match crate::commands::stock_workflow::run_lesson_validation(&db).await {
                        Ok(result) => {
                            tracing::info!(
                                "[lesson_validation] 完成: validated={} deprecated={} avg_success_rate={:.2}",
                                result.get("validatedLessons").and_then(|v| v.as_i64()).unwrap_or(0),
                                result.get("deprecatedLessons").and_then(|v| v.as_i64()).unwrap_or(0),
                                result.get("avgSuccessRate").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            );
                        }
                        Err(e) => {
                            tracing::error!("[lesson_validation] 失败: {e}");
                        }
                    }
                }
            }
        }
    });
}

/// [方向6] DreamConsolidator 定时任务：每日 18:00 跨股票蒸馏反思轨迹。
///
/// 调用 `state.dream_consolidator.consolidate_force()` 强制执行蒸馏（绕过时间门控）。
/// - 从 TrajectoryStorage 拉取最近 N 条反思轨迹（N=experience_replay_sample_size，默认 50）
/// - 按 topic（股票代码）分组，蒸馏 ToolUsagePattern / ReasoningStrategy / ErrorRecovery
/// - 蒸馏知识写入 `trajectory_skills` 表 + FTS 索引
/// - 建议仅存内存（进程内 cache），重启丢失
///
/// 首次延迟：距离今日 18:00 的秒数（若已过 18:00 则推迟到次日 18:00）。
/// 监听 `shutdown_token` 支持优雅关闭。
fn start_dream_consolidation(state: &AppState) {
    let consolidator = state.dream_consolidator.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("dream_consolidation", async move {
        // 计算距离今日 18:00 的初始延迟（Asia/Shanghai 时区）
        let now = chrono::Local::now();
        let today_18 = now
            .date_naive()
            .and_hms_opt(18, 0, 0)
            .unwrap_or_else(|| now.naive_local());
        let initial_delay_secs = if now.naive_local() < today_18 {
            (today_18 - now.naive_local()).num_seconds().max(60) as u64
        } else {
            // 已过 18:00，推迟到次日 18:00
            ((today_18 + chrono::Duration::days(1) - now.naive_local())
                .num_seconds()
                .max(60)) as u64
        };

        let initial_delay = std::time::Duration::from_secs(initial_delay_secs);
        let interval = std::time::Duration::from_secs(24 * 3600); // 24 小时

        tracing::info!(
            "[dream_consolidation] 首次运行延迟 {} 秒（约 {} 小时），之后每 24 小时运行一次",
            initial_delay_secs,
            initial_delay_secs / 3600
        );

        tokio::time::sleep(initial_delay).await;
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[dream_consolidation] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    tracing::info!("[dream_consolidation] 开始跨股票蒸馏反思轨迹");
                    let result = consolidator.consolidate_force().await;
                    if result.executed {
                        tracing::info!(
                            "[dream_consolidation] 完成: memories={} patterns={} suggestions={} duration={}s",
                            result.memories_extracted,
                            result.patterns_discovered,
                            result.suggestions_generated,
                            result.duration_secs
                        );
                    } else if let Some(reason) = &result.skip_reason {
                        tracing::info!("[dream_consolidation] 跳过: {}", reason);
                    } else {
                        tracing::info!("[dream_consolidation] 未执行（无明确原因）");
                    }
                }
            }
        }
    });
}

fn start_platform_adapters(state: &AppState) {
    let platform_manager = state.platform_manager.clone();
    let db = state.harness.db().clone();

    tauri::async_runtime::spawn(async move {
        let config = axagent_dao::repo::platform_config::get_platform_config(&db).await;
        match platform_manager.reconcile(&config).await {
            Ok(report) => {
                if !report.started.is_empty() {
                    tracing::info!(
                        "[PlatformManager] boot reconcile: started {:?}",
                        report.started
                    );
                }
                if !report.errors.is_empty() {
                    for (name, err) in &report.errors {
                        tracing::error!(
                            "[PlatformManager] boot reconcile: {} error: {}",
                            name,
                            err
                        );
                    }
                }
            },
            Err(e) => {
                tracing::error!("[PlatformManager] boot reconcile failed: {}", e);
            },
        }
    });
}

fn start_webdav_sync(app: &tauri::AppHandle, state: &AppState, app_dir: std::path::PathBuf) {
    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let app_data_dir = app_dir.clone();
    let handle = state.webdav_sync_handle.clone();
    let shutdown_token = state.shutdown_token.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_dao::repo::settings::get_settings(&db).await {
            if settings.webdav_sync_enabled && settings.webdav_sync_interval_minutes > 0 {
                let db2 = db.clone();
                let interval = settings.webdav_sync_interval_minutes;
                let interval_secs = interval as u64 * 60;

                let initial_delay_secs =
                    match axagent_dao::repo::settings::get_setting(&db, "webdav_last_sync_time")
                        .await
                    {
                        Ok(Some(ts)) => {
                            if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(&ts) {
                                let elapsed = chrono::Utc::now()
                                    .signed_duration_since(last_time)
                                    .num_seconds()
                                    .max(0) as u64;
                                interval_secs.saturating_sub(elapsed)
                            } else {
                                interval_secs
                            }
                        },
                        _ => interval_secs,
                    };

                let task = crate::commands::webdav::spawn_webdav_sync_task(
                    app_clone,
                    db2,
                    master_key,
                    app_data_dir,
                    interval,
                    initial_delay_secs,
                    shutdown_token,
                );
                *handle.lock().await = Some(task);
            }
        }
    });
}

#[cfg(not(mobile))]
fn start_tray(app: &tauri::AppHandle, tray_language: &str) {
    if let Err(e) = crate::tray::create_tray(app, tray_language) {
        tracing::warn!("Failed to create system tray: {}", e);
    }
}

fn start_closed_loop_service(_app: &tauri::AppHandle, state: &AppState) {
    let db = state.harness.db().clone();
    let closed_loop = state.closed_loop_service.clone();
    let nudge_service = state.nudge_service.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_dao::repo::settings::get_settings(&db).await {
            if settings.closed_loop_enabled {
                closed_loop.start();
                let interval_minutes = settings.closed_loop_interval_minutes.max(1);
                let interval = std::time::Duration::from_secs(interval_minutes as u64 * 60);
                loop {
                    tokio::time::sleep(interval).await;
                    let new_nudges: Vec<axagent_trajectory::PeriodicNudge> =
                        closed_loop.tick().await;
                    if !new_nudges.is_empty() {
                        tracing::info!(
                            "[closed_loop] Generated {} periodic nudges",
                            new_nudges.len()
                        );
                        let candidates: Vec<axagent_trajectory::NudgeCandidate> = new_nudges
                            .iter()
                            .map(|pn| axagent_trajectory::NudgeCandidate {
                                entity: axagent_trajectory::NudgeEntity {
                                    id: pn.id.clone(),
                                    name: pn.title.clone(),
                                    entity_type: format!("{:?}", pn.nudge_type),
                                    confidence: if pn.urgency == "high" {
                                        0.9
                                    } else if pn.urgency == "medium" {
                                        0.7
                                    } else {
                                        0.5
                                    },
                                },
                                reason: pn.description.clone(),
                                urgency: match pn.urgency.as_str() {
                                    "high" => axagent_trajectory::Urgency::High,
                                    "medium" => axagent_trajectory::Urgency::Medium,
                                    _ => axagent_trajectory::Urgency::Low,
                                },
                                suggested_action: Some(pn.suggested_action.clone()),
                            })
                            .collect();
                        let mut ns: tokio::sync::MutexGuard<'_, axagent_trajectory::NudgeService> =
                            nudge_service.lock().await;
                        let ctx = axagent_trajectory::NudgeContext {
                            current_task: None,
                            recent_entities: None,
                            session_id: "closed_loop_bg".to_string(),
                        };
                        ns.generate_nudges(ctx, candidates);
                    }
                }
            }
        }
    });
}

fn start_insight_generation(state: &AppState) {
    let realtime_learning = state.realtime_learning.clone();
    let insight_system = state.insight_system.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(10 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let new_insights = {
                let rl: tokio::sync::MutexGuard<'_, axagent_trajectory::RealTimeLearning> =
                    realtime_learning.lock().await;
                rl.generate_insights()
            };
            if !new_insights.is_empty() {
                tracing::info!(
                    "[insight] Generated {} learning insights from feedback",
                    new_insights.len()
                );
                let mut is = insight_system.write().await;
                for insight in new_insights {
                    is.add_insight(insight);
                }
            }
        }
    });
}

fn start_pattern_learning(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let pattern_learner = state.pattern_learner.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(15 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(20)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[pattern] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };
            if trajectories.is_empty() {
                continue;
            }
            let mut pl = pattern_learner.write().await;
            let new_patterns = pl.update_from_batch(&trajectories);
            drop(pl);
            if !new_patterns.is_empty() {
                tracing::info!(
                    "[pattern] Learned {} new patterns from {} trajectories",
                    new_patterns.len(),
                    trajectories.len()
                );
                for pattern in &new_patterns {
                    if let Err(e) = trajectory_storage.save_pattern(pattern).await {
                        tracing::warn!("[pattern] Failed to persist pattern: {}", e);
                    }
                }
            }
        }
    });
}

fn start_cross_session_learning(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let cross_session_learner = state.cross_session_learner.clone();
    let insight_system = state.insight_system.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(30 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(50)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[cross_session] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };
            if trajectories.len() < 3 {
                continue;
            }
            let mut by_session: std::collections::HashMap<
                String,
                Vec<axagent_trajectory::Trajectory>,
            > = std::collections::HashMap::new();
            for t in trajectories {
                by_session.entry(t.session_id.clone()).or_default().push(t);
            }
            if by_session.len() < 2 {
                continue;
            }
            let mut csl = cross_session_learner.write().await;
            let new_patterns = csl.learn_from_sessions(by_session);
            drop(csl);
            if !new_patterns.is_empty() {
                tracing::info!(
                    "[cross_session] Discovered {} cross-session patterns",
                    new_patterns.len()
                );
                let mut is = insight_system.write().await;
                for pattern in &new_patterns {
                    if let Err(e) = trajectory_storage.save_pattern(pattern).await {
                        tracing::warn!("[cross_session] Failed to persist pattern: {}", e);
                    }
                    if pattern.success_rate >= 0.7 && pattern.frequency >= 3 {
                        is.add_insight(axagent_trajectory::LearningInsight {
                            id: format!("cs_{}", pattern.id),
                            category: axagent_trajectory::InsightCategory::Pattern,
                            title: format!("Cross-session pattern: {}", pattern.name),
                            description: pattern.description.clone(),
                            confidence: pattern.success_rate,
                            evidence: pattern.trajectory_ids.iter().take(3).cloned().collect(),
                            suggested_action: Some(
                                "Consider creating a skill for this recurring pattern".to_string(),
                            ),
                            created_at: chrono::Utc::now().timestamp_millis(),
                        });
                    }
                }
            }
        }
    });
}

fn start_rl_reward_computation(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let rl_engine = state.rl_engine.clone();
    let insight_system = state.insight_system.clone();
    let process_reward_model = state.process_reward_model.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            {
                let mut rl = rl_engine.write().await;
                rl.set_llm_judge(Box::new(bridge.clone()));
            }
            tracing::info!("[rl] LLM judge injected into RLEngine");

            {
                let mut prm = process_reward_model.lock().await;
                prm.set_provider(Box::new(bridge));
            }
            tracing::info!("[rl] LLM PRM provider injected into ProcessRewardModel");
        }

        let interval = std::time::Duration::from_secs(20 * 60);
        let mut reward_normalizer = axagent_trajectory::RewardNormalizer::new();
        loop {
            tokio::time::sleep(interval).await;
            let mut trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(15)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[rl] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };
            if trajectories.is_empty() {
                continue;
            }
            let rl = rl_engine.read().await;
            let mut total_rewards = 0;
            let mut total_advantages = 0;
            let mut total_prm_rewards = 0;
            for trajectory in &mut trajectories {
                if trajectory.rewards.is_empty() {
                    let mut rewards = rl.compute_rewards(trajectory);
                    total_rewards += rewards.len();
                    rl.shape_rewards(&mut rewards);
                    reward_normalizer.normalize(&mut rewards);
                    trajectory.rewards = rewards;

                    {
                        let prm = process_reward_model.lock().await;
                        let prm_result = prm.compute_trajectory_rewards(trajectory).await;
                        if !prm_result.step_rewards.is_empty() {
                            total_prm_rewards += prm_result.step_rewards.len();
                            let combined_value =
                                trajectory.value_score * 0.5 + prm_result.weighted_reward * 0.5;
                            trajectory.value_score = combined_value;
                            tracing::debug!(
                                "[rl] PRM for trajectory {}: aggregate={:.3}, outcome={:.3}, weighted={:.3}",
                                &trajectory.id[..trajectory.id.len().min(8)],
                                prm_result.aggregate_reward,
                                prm_result.outcome_reward,
                                prm_result.weighted_reward
                            );
                        }
                    }

                    let values = rl.estimate_value_function(trajectory);
                    if !values.is_empty() {
                        let advantages = rl.compute_advantages(&trajectory.rewards, &values);
                        total_advantages += advantages.len();
                        let avg_advantage: f64 = if !advantages.is_empty() {
                            advantages.iter().sum::<f64>() / advantages.len() as f64
                        } else {
                            0.0
                        };
                        if avg_advantage > 0.3 {
                            let gradients = rl.compute_policy_gradient(trajectory, &advantages);
                            tracing::debug!(
                                "[rl] High-advantage trajectory {}: avg_adv={:.3}, gradients={:?}",
                                &trajectory.id[..trajectory.id.len().min(8)],
                                avg_advantage,
                                gradients
                            );
                        }
                    }
                    let total_reward: f64 = trajectory.rewards.iter().map(|r| r.value).sum();
                    trajectory.value_score = (trajectory.value_score + total_reward) / 2.0;
                    if let Err(e) = trajectory_storage.save_trajectory(trajectory).await {
                        tracing::warn!("[rl] Failed to update trajectory: {}", e);
                    }
                }
            }
            drop(rl);
            if total_rewards > 0 {
                tracing::info!(
                    "[rl] Computed {} rewards, {} advantages, {} PRM step-evals across {} trajectories",
                    total_rewards,
                    total_advantages,
                    total_prm_rewards,
                    trajectories.len()
                );
                let reward_trajectories: Vec<_> =
                    trajectories.iter().filter(|t| !t.rewards.is_empty()).collect();
                if reward_trajectories.len() >= 3 {
                    let avg_reward: f64 = reward_trajectories
                        .iter()
                        .map(|t| t.rewards.iter().map(|r| r.value).sum::<f64>())
                        .sum::<f64>()
                        / reward_trajectories.len() as f64;
                    let high_reward_count = reward_trajectories
                        .iter()
                        .filter(|t| t.rewards.iter().map(|r| r.value).sum::<f64>() > avg_reward)
                        .count();
                    let mut is = insight_system.write().await;
                    is.add_insight(axagent_trajectory::LearningInsight {
                        id: format!("rl_{}", chrono::Utc::now().timestamp_millis()),
                        category: if avg_reward > 0.0 { axagent_trajectory::InsightCategory::Pattern } else { axagent_trajectory::InsightCategory::Warning },
                        title: format!("RL reward analysis: avg={:.2}", avg_reward),
                        description: format!("{} trajectories analyzed, {} above average reward. Average reward: {:.3}",
                            reward_trajectories.len(), high_reward_count, avg_reward),
                        confidence: (avg_reward.abs() * 2.0).min(0.9),
                        evidence: vec![],
                        suggested_action: if avg_reward < 0.0 {
                            Some("Recent interactions have negative reward signals. Consider adjusting tool usage patterns.".to_string())
                        } else { None },
                        created_at: chrono::Utc::now().timestamp_millis(),
                    });
                }
            }
        }
    });
}

fn start_batch_processing(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let batch_processor = state.batch_processor.clone();
    let insight_system = state.insight_system.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("batch_processing", async move {
        let interval = std::time::Duration::from_secs(60 * 60);
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[batch_processing] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
            let bp = &*batch_processor;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(50)).await {
                    Ok(t) => t,
                    Err(_) => continue,
                };
            if trajectories.len() < 5 {
                continue;
            }
            let quality_filtered = bp.filter_by_quality(&trajectories, 0.3);
            if quality_filtered.is_empty() {
                continue;
            }
            let analysis = bp.analyze_batch(&quality_filtered);
            let mut is = insight_system.write().await;
            is.add_insight(axagent_trajectory::LearningInsight {
                id: format!("batch_{}", chrono::Utc::now().timestamp_millis()),
                category: axagent_trajectory::InsightCategory::Improvement,
                title: format!("Batch analysis: {} trajectories, {:.0}% success",
                    analysis.total,
                    if analysis.total > 0 { analysis.outcome_counts.values().sum::<usize>() as f64 / analysis.total as f64 * 100.0 } else { 0.0 }),
                description: format!("Quality: avg={:.2}, value={:.2}. Patterns: {}.",
                    analysis.avg_quality, analysis.avg_value, analysis.top_patterns.len().min(5)),
                confidence: (analysis.avg_quality * 1.5).min(0.9),
                evidence: vec![],
                suggested_action: if analysis.avg_quality < 0.4 {
                    Some("Batch quality is low. Consider reviewing recent interactions for improvement opportunities.".to_string())
                } else { None },
                created_at: chrono::Utc::now().timestamp_millis(),
            });
                }
            }
        }
    });
}

fn start_user_profile_persistence(state: &AppState) {
    let user_profile = state.user_profile.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(10 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let profile = user_profile.read().await;
            let md_content = profile.to_user_md();
            drop(profile);
            if let Some(home) = {
                #[cfg(mobile)]
                {
                    dirs::data_dir().or_else(dirs::home_dir)
                }
                #[cfg(not(mobile))]
                {
                    dirs::home_dir()
                }
            } {
                let user_md_path = home.join(".axagent").join("USER.md");
                if let Some(parent) = user_md_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(&user_md_path, &md_content) {
                    tracing::warn!("[user-profile] Failed to write USER.md: {}", e);
                }
            }
        }
    });
}

fn start_skill_evolution(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let skill_evolution_engine = state.skill_evolution_engine.clone();
    let insight_system = state.insight_system.clone();
    let constitution = state.constitution.clone();
    let intrinsic_motivation = state.intrinsic_motivation.clone();
    let coevolution_env = state.coevolution_env.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            let engine = skill_evolution_engine.lock().await;
            engine.set_llm_provider(std::sync::Arc::new(bridge));
            drop(engine);
            tracing::info!("[evolution] LLM provider injected into SkillEvolutionEngine");
        }

        let interval = std::time::Duration::from_secs(45 * 60);
        let success_threshold = 0.5;
        let min_usages = 3;
        loop {
            tokio::time::sleep(interval).await;
            let skills: Vec<axagent_trajectory::Skill> = match trajectory_storage.get_skills().await
            {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("[evolution] Failed to fetch skills: {}", e);
                    continue;
                },
            };
            let weak_skills: Vec<_> = skills
                .into_iter()
                .filter(|s| s.total_usages >= min_usages && s.success_rate < success_threshold)
                .collect();
            if weak_skills.is_empty() {
                continue;
            }
            tracing::info!(
                "[evolution] Found {} skills below success threshold ({:.0}%)",
                weak_skills.len(),
                success_threshold * 100.0
            );
            let test_trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(30)).await {
                    Ok(t) => t,
                    Err(_) => continue,
                };
            let test_refs: Vec<&axagent_trajectory::Trajectory> =
                test_trajectories.iter().collect();
            for skill in weak_skills.iter().take(2) {
                let mut engine: tokio::sync::MutexGuard<
                    '_,
                    axagent_trajectory::SkillEvolutionEngine,
                > = skill_evolution_engine.lock().await;
                let result = engine.run(skill, &test_refs).await;
                if let Some(modification) = result {
                    if let Err(violations) = constitution.validate_evolution(&modification) {
                        let has_fatal = violations
                            .iter()
                            .any(|v| v.severity == axagent_trajectory::ViolationSeverity::Fatal);
                        let has_critical = violations
                            .iter()
                            .any(|v| v.severity == axagent_trajectory::ViolationSeverity::Critical);
                        if has_fatal || has_critical {
                            tracing::warn!(
                                "[evolution] Constitution blocked skill '{}' evolution: {} violations (fatal={}, critical={})",
                                skill.name,
                                violations.len(),
                                has_fatal,
                                has_critical
                            );
                            continue;
                        }
                        tracing::info!(
                            "[evolution] Constitution warnings for skill '{}' evolution: {:?}",
                            skill.name,
                            violations.iter().map(|v| &v.description).collect::<Vec<_>>()
                        );
                    }
                    if modification.validation_result.as_ref().is_some_and(|v| v.success) {
                        tracing::info!(
                            "[evolution] Skill '{}' evolved: {} (confidence={:.3})",
                            skill.name,
                            modification.reason,
                            modification.confidence
                        );
                        let mut updated_skill = skill.clone();
                        updated_skill.content = modification.new_content.clone();
                        updated_skill.quality_score = modification.confidence;
                        updated_skill.version = format!(
                            "{}.e{}",
                            updated_skill
                                .version
                                .trim_end_matches(|c: char| c == '.' || c.is_ascii_digit()),
                            chrono::Utc::now().timestamp_millis() % 10000
                        );
                        if let Err(e) = trajectory_storage.save_skill(&updated_skill).await {
                            tracing::warn!("[evolution] Failed to save evolved skill: {}", e);
                        }

                        {
                            let mut im = intrinsic_motivation.lock().await;
                            for traj in &test_trajectories {
                                let _intrinsic_reward = im.compute_intrinsic_reward(traj);
                            }
                        }

                        {
                            let mut env = coevolution_env.lock().await;
                            env.update_performance(modification.confidence);
                        }

                        let mut is = insight_system.write().await;
                        is.add_insight(axagent_trajectory::LearningInsight {
                            id: format!("evo_{}", chrono::Utc::now().timestamp_millis()),
                            category: axagent_trajectory::InsightCategory::Improvement,
                            title: format!("Skill '{}' auto-evolved", skill.name),
                            description: modification.reason.clone(),
                            confidence: modification.confidence,
                            evidence: vec![],
                            suggested_action: Some(format!(
                                "Review evolved skill '{}' for correctness",
                                skill.name
                            )),
                            created_at: chrono::Utc::now().timestamp_millis(),
                        });
                    } else {
                        tracing::info!(
                            "[evolution] Skill '{}' evolution did not improve fitness",
                            skill.name
                        );
                    }
                }
            }
        }
    });
}

fn start_skill_watcher(app: &tauri::AppHandle, state: &AppState) {
    let home = {
        #[cfg(mobile)]
        {
            dirs::data_dir().or_else(dirs::home_dir).unwrap_or_default()
        }
        #[cfg(not(mobile))]
        {
            dirs::home_dir().unwrap_or_default()
        }
    };
    let skill_dirs: Vec<std::path::PathBuf> = vec![
        home.join(".axagent").join("skills"),
        home.join(".claude").join("skills"),
        home.join(".trae").join("skills"),
        home.join(".codebuddy").join("skills"),
        home.join(".workbuddy").join("skills"),
        home.join(".agents").join("skills"),
    ];

    let app_handle = app.clone();
    let shutdown = Arc::new(AtomicBool::new(false));
    let _ = state.skill_watcher_shutdown.set(shutdown.clone());
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher =
            match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("Failed to create skill watcher: {}", e);
                    return;
                },
            };

        for dir in &skill_dirs {
            if dir.exists() {
                if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
                    tracing::warn!("Failed to watch skill dir {:?}: {}", dir, e);
                }
            }
        }

        tracing::info!("Skill file watcher started");

        let mut pending: std::collections::HashMap<String, std::time::Instant> =
            std::collections::HashMap::new();
        let debounce = std::time::Duration::from_secs(2);

        loop {
            if shutdown.load(Ordering::Relaxed) {
                tracing::info!("Skill file watcher 收到关闭信号");
                return;
            }
            match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(event) => {
                    if !event.kind.is_modify() {
                        continue;
                    }
                    for path in &event.paths {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        let is_skill_file = matches!(
                            name,
                            "SKILL.md" | "manifest.json" | "skill-manifest.json" | "frontend.json"
                        );
                        if !is_skill_file {
                            continue;
                        }

                        if let Some(parent) = path.parent() {
                            if let Some(skill_name) = parent.file_name().and_then(|n| n.to_str()) {
                                pending
                                    .entry(skill_name.to_string())
                                    .or_insert(std::time::Instant::now());
                            }
                        }
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // 检查是否有到期的事件需要发送
                    let now = std::time::Instant::now();
                    let mut ready: Vec<String> = vec![];
                    pending.retain(|name, ts| {
                        if now.duration_since(*ts) >= debounce {
                            ready.push(name.clone());
                            false
                        } else {
                            true
                        }
                    });

                    if ready.is_empty() {
                        continue;
                    }

                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        for name in ready {
                            let _ = app.emit("skill:file-changed", name);
                        }
                    });
                },
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    tracing::info!("Skill file watcher stopped");
                    return;
                },
            }
        }
    });
}

fn start_memory_decay_tick(state: &AppState) {
    let memory_service = state.memory_service.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(3600);
        loop {
            tokio::time::sleep(interval).await;
            let ms = memory_service.read().await;
            let evicted = ms.apply_decay_tick().await;
            drop(ms);
            if evicted > 0 {
                tracing::info!("[memory_decay] Evicted {} expired/decayed memories", evicted);
            }
        }
    });
}

fn start_auto_tool_observation(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let auto_tool_creator = state.auto_tool_creator.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            let mut atc = auto_tool_creator.lock().await;
            atc.set_llm_provider(Box::new(bridge));
            drop(atc);
            tracing::info!("[auto_tool] LLM provider injected into AutoToolCreator");
        }

        let interval = std::time::Duration::from_secs(60 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(30)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[auto_tool] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };

            let mut atc = auto_tool_creator.lock().await;
            for trajectory in &trajectories {
                atc.observe_trajectory(trajectory);
            }

            let frequent = atc.get_frequent_patterns(3);
            if !frequent.is_empty() {
                tracing::info!(
                    "[auto_tool] Observed {} frequent tool patterns (top: {:?})",
                    frequent.len(),
                    &frequent[..frequent.len().min(5)]
                );

                for (pattern, count) in frequent.iter().take(2) {
                    if atc.get_tool(&axagent_trajectory::slugify(pattern)).is_none() {
                        match atc
                            .create_tool_from_pattern(
                                pattern,
                                &format!("Auto-observed pattern ({} occurrences)", count),
                                vec![],
                            )
                            .await
                        {
                            Ok(tool) => {
                                tracing::info!(
                                    "[auto_tool] Created tool '{}' from pattern '{}' (freq={})",
                                    tool.name,
                                    pattern,
                                    count
                                );
                            },
                            Err(e) => {
                                tracing::debug!(
                                    "[auto_tool] Could not create tool from '{}': {}",
                                    pattern,
                                    e
                                );
                            },
                        }
                    }
                }
            }
        }
    });
}

fn start_text_grad_analysis(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let text_grad_engine = state.text_grad_engine.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            let mut engine = text_grad_engine.lock().await;
            engine.set_provider(bridge);
            drop(engine);
            tracing::info!("[text_grad] LLM provider injected into TextGradEngine");
        }

        let interval = std::time::Duration::from_secs(2 * 60 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(10)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[text_grad] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };

            let mut engine = text_grad_engine.lock().await;
            for trajectory in &trajectories {
                if trajectory.steps.len() < 3 {
                    continue;
                }
                let session_id = &trajectory.session_id;
                let topic = &trajectory.topic;

                for (i, step) in trajectory.steps.iter().enumerate() {
                    let content_summary: String = step.content.chars().take(200).collect();
                    let node_id = format!("{}_{}", &session_id[..session_id.len().min(8)], i);
                    engine.add_node(node_id.clone(), content_summary, Some(format!("step_{}", i)));
                    if i > 0 {
                        let prev_id =
                            format!("{}_{}", &session_id[..session_id.len().min(8)], i - 1);
                        engine.add_edge(prev_id, node_id, 1.0);
                    }
                }

                if !trajectory.steps.is_empty() {
                    if let Some(last_step) = trajectory.steps.last() {
                        let feedback = match trajectory.outcome {
                            axagent_trajectory::TrajectoryOutcome::Success => {
                                format!("Task succeeded: {}", topic)
                            },
                            axagent_trajectory::TrajectoryOutcome::Failure => {
                                format!(
                                    "Task failed: {} - last step: {}",
                                    topic,
                                    last_step.content.chars().take(100).collect::<String>()
                                )
                            },
                            axagent_trajectory::TrajectoryOutcome::Partial => {
                                format!("Task partially completed: {}", topic)
                            },
                            axagent_trajectory::TrajectoryOutcome::Abandoned => {
                                format!("Task abandoned: {}", topic)
                            },
                        };
                        let last_id = format!(
                            "{}_{}",
                            &session_id[..session_id.len().min(8)],
                            trajectory.steps.len() - 1
                        );
                        let _ = engine.forward();
                        let _ = engine.backward(&last_id, &feedback).await;
                    }
                }
            }

            let stats = engine.stats();
            tracing::info!(
                "[text_grad] Graph stats: {} nodes, {} edges, {} gradients computed",
                stats.node_count,
                stats.edge_count,
                stats.gradient_count
            );
        }
    });
}

fn start_cron_scheduler(state: &AppState) {
    use axagent_runtime::cron::{CronExecutor, CronScheduler};
    use std::sync::Arc;

    let store = state.cron_job_store.clone();

    // 注入共享存储到 tools crate，使 CronCreateTool 等可用
    axagent_tools::tools::cron::init_cron_store(store.clone());

    // 设置工具解析器（从全局 registry 按需自动注册工作流中引用的工具）
    {
        let registry = state.local_tool_registry.clone();
        let work_engine = state.work_engine.clone();
        let resolver: axagent_runtime::work_engine::ToolResolver =
            std::sync::Arc::new(move |tool_name: String| {
                let registry = registry.clone();
                let work_engine = work_engine.clone();
                Box::pin(async move {
                    let reg = registry.lock().await;
                    let known = reg.list_all_tool_names().contains(&tool_name)
                        || reg.mcp.mcp_tools.contains_key(&tool_name);
                    if known {
                        let registry = registry.clone();
                        let cb: axagent_runtime::work_engine::ToolCallback =
                            std::sync::Arc::new(move |tn: String, args: serde_json::Value| {
                                let registry = registry.clone();
                                Box::pin(async move {
                                    let reg = registry.lock().await;
                                    let input_str = serde_json::to_string(&args)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    match reg.execute(&tn, &input_str).await {
                                        Ok(output) => {
                                            Ok(serde_json::json!({"content": output.content}))
                                        },
                                        Err(e) => Err(format!("Tool execution error: {}", e)),
                                    }
                                })
                            });
                        Some(cb)
                    } else if let Some(template_id) = tool_name.strip_prefix("workflow::") {
                        // 工作流注册为工具：workflow::<template_id>
                        let engine = work_engine.clone();
                        let template_id = template_id.to_string();
                        let cb: axagent_runtime::work_engine::ToolCallback =
                            std::sync::Arc::new(move |_tn: String, args: serde_json::Value| {
                                let engine = engine.clone();
                                let tid = template_id.clone();
                                Box::pin(async move {
                                    let mut opts =
                                        axagent_runtime::work_engine::RunOptions::default();
                                    if let Some(input) = args.get("input") {
                                        opts.input = Some(input.clone());
                                    }
                                    match engine.run_workflow(&tid, opts).await {
                                        Ok(wf) => Ok(serde_json::json!({
                                            "content": serde_json::json!({
                                                "status": format!("{:?}", wf.status),
                                                "results": wf.results,
                                            }).to_string()
                                        })),
                                        Err(e) => {
                                            Err(format!("Workflow tool '{}' failed: {:?}", tid, e))
                                        },
                                    }
                                })
                            });
                        Some(cb)
                    } else {
                        None
                    }
                })
            });
        // 复用 Tauri 全局 runtime，避免一次性创建/销毁 runtime 的开销。
        tauri::async_runtime::block_on(state.work_engine.set_tool_resolver(resolver));
    }

    // 设置 RAG 知识源检索回调（供工作流 Agent 节点从知识库/记忆/Wiki 检索上下文）
    {
        let db = state.harness.db().clone();
        let master_key = state.harness.master_key_owned();
        let vector_store = state.vector_store.clone();
        let rag_callback: axagent_rt_workflow::work_engine::RagCallback = std::sync::Arc::new(
            move |kb_ids: Vec<String>,
                  mem_ids: Vec<String>,
                  wiki_ids: Vec<String>,
                  query: String| {
                let db = db.clone();
                let vector_store = vector_store.clone();
                Box::pin(async move {
                    let embed_fn = crate::indexing::ProviderEmbedFn;
                    let result = axagent_search::rag::collect_rag_context(
                        &db,
                        &master_key,
                        &vector_store,
                        &kb_ids,
                        &mem_ids,
                        &wiki_ids,
                        &query,
                        5,
                        embed_fn,
                    )
                    .await;
                    Ok(result)
                })
            },
        );
        // 复用 Tauri 全局 runtime，避免一次性创建/销毁 runtime 的开销。
        tauri::async_runtime::block_on(state.work_engine.set_rag_callback(rag_callback));
    }

    let work_engine = state.work_engine.clone();
    let cron_store = state.cron_job_store.clone();
    let mut executor = CronExecutor::new();
    executor.set_handler(move |job| {
        if let Some(ref wf_id) = job.workflow_id {
            let engine = work_engine.clone();
            let store = cron_store.clone();
            let wf_id = wf_id.clone();
            let job_id = job.id.clone();
            let job_name = job.name.clone();
            let recurring = job.recurring;
            tokio::task::spawn(async move {
                let started = axagent_runtime_core::cron_job::now_millis();
                let opts = axagent_runtime::work_engine::RunOptions::default();
                let result = match engine.run_workflow(&wf_id, opts).await {
                    Ok(workflow) => {
                        tracing::info!(
                            "[CronScheduler] 工作流任务 '{}' 完成: {:?}",
                            job_name,
                            workflow.status
                        );
                        axagent_runtime_core::TaskRunResult {
                            success: true,
                            output: Some(format!("{:?}", workflow.status)),
                            error: None,
                            duration_ms: (axagent_runtime_core::cron_job::now_millis() - started)
                                as u64,
                            executed_at: started,
                        }
                    },
                    Err(e) => {
                        let err_msg = format!("{:?}", e);
                        tracing::error!(
                            "[CronScheduler] 工作流任务 '{}' 失败: {err_msg}",
                            job_name
                        );
                        axagent_runtime_core::TaskRunResult {
                            success: false,
                            output: None,
                            error: Some(err_msg),
                            duration_ms: (axagent_runtime_core::cron_job::now_millis() - started)
                                as u64,
                            executed_at: started,
                        }
                    },
                };
                store.record_run(&job_id, result).await;
                // 非循环任务执行后禁用
                if !recurring {
                    let _ = store
                        .set_status(&job_id, axagent_runtime_core::CronJobStatus::Disabled)
                        .await;
                }
            });
        } else {
            tracing::info!(
                "[CronScheduler] 触发任务 '{}': {}",
                job.name,
                &job.prompt[..std::cmp::min(job.prompt.len(), 200)]
            );
        }
    });

    let scheduler = Arc::new(CronScheduler::new(store, Arc::new(executor)));

    tauri::async_runtime::spawn(async move {
        scheduler.start().await;
    });

    tracing::info!("[CronScheduler] 已启动（统一 Cron + ScheduledTask），每30秒轮询一次");
}

fn start_trajectory_cleanup(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let handle = state.trajectory_cleanup_handle.clone();
    let shutdown_token = state.shutdown_token.clone();
    let config = axagent_trajectory::TrajectoryCleanupConfig::default();
    let config_for_log = config.clone();
    let interval = std::time::Duration::from_secs(24 * 3600);

    tauri::async_runtime::spawn(async move {
        let task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        match trajectory_storage.cleanup(&config).await {
                            Ok(count) if count > 0 => {
                                tracing::info!(
                                    "[trajectory_cleanup] Cleaned up {} old trajectories",
                                    count
                                );
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(
                                    "[trajectory_cleanup] cleanup failed: {}",
                                    e
                                );
                            }
                        }
                    }
                    _ = shutdown_token.cancelled() => {
                        tracing::info!(
                            "[trajectory_cleanup] Received shutdown signal, stopping"
                        );
                        break;
                    }
                }
            }
        });
        *handle.lock().await = Some(task);
    });
    tracing::info!(
        "[trajectory_cleanup] Started with max_age_days={:?}, max_trajectories={:?}, interval=24h",
        config_for_log.max_age_days,
        config_for_log.max_trajectories
    );
}

fn start_index_job_service(app: &tauri::AppHandle, state: &AppState) {
    let db = state.harness.db().clone();
    let vector_store = state.vector_store.clone();
    let master_key = state.harness.master_key_owned();
    let shutdown_token = state.shutdown_token.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let service = std::sync::Arc::new(IndexJobService::new(
            db,
            vector_store,
            master_key,
            shutdown_token,
            app_handle,
        ));
        service.start().await;
    });
    tracing::info!("[index_queue] 已启动持久化索引队列 worker");
}

/// 股票全业务管道定时任务：每日 18:00 Asia/Shanghai 自动运行
///
/// 首次延迟 60 秒（等待其他服务初始化），之后计算下一个 18:00 北京时间。
/// 反思阶段由现有 6h cron 接力，此处只管发现+分析+持仓再评估。
fn start_stock_pipeline(app: &tauri::AppHandle, state: &AppState) {
    let db = state.harness.db().clone();
    let client = state.astock_client.clone();
    let engine = state.work_engine.clone();
    let token = state.shutdown_token.clone();
    let app_handle = app.clone();

    state.task_manager.spawn("stock_pipeline", async move {
        // 首次延迟 60 秒，等待其他服务初始化
        let initial_delay = std::time::Duration::from_secs(60);
        tokio::time::sleep(initial_delay).await;

        loop {
            // 计算下一个 18:00 Asia/Shanghai（转为 UTC）
            let next_run = next_18_00_shanghai();
            let wait_duration = next_run.signed_duration_since(chrono::Utc::now());
            let wait_secs = wait_duration.num_seconds().max(60) as u64;

            tracing::info!(
                "[stock_pipeline] 距离下次执行还有 {} 秒（约 {} 小时）",
                wait_secs,
                wait_secs / 3600
            );

            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[stock_pipeline] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(wait_secs)) => {
                    tracing::info!("[stock_pipeline] 开始执行股票管道");

                    let app_for_progress = app_handle.clone();
                    let progress_callback = std::sync::Arc::new(move |step: &str, detail: &str| {
                        let _ = app_for_progress.emit(
                            "pipeline-step",
                            serde_json::json!({
                                "step": step,
                                "detail": detail,
                                "timestamp": chrono::Utc::now().timestamp_millis(),
                            }),
                        );
                    });

                    let config = crate::commands::stock_pipeline::core::PipelineConfig::default();
                    match crate::commands::stock_pipeline::core::run_stock_pipeline_inner(
                        &db,
                        &client,
                        &engine,
                        &config,
                        None,
                        Some(progress_callback),
                    )
                    .await
                    {
                        Ok(result) => {
                            tracing::info!(
                                "[stock_pipeline] 管道执行完成: {} 候选, {} 新分析, {} 持仓再评估",
                                result.candidates.len(),
                                result.new_analyses.len(),
                                result.reassessed.len()
                            );
                        }
                        Err(e) => {
                            tracing::error!("[stock_pipeline] 管道执行失败: {e}");
                        }
                    }
                }
            }
        }
    });
}

/// 计算下一个 18:00 Asia/Shanghai 时间点（转为 UTC 返回）
fn next_18_00_shanghai() -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now_shanghai = chrono::Local::now().with_timezone(&cst);
    let today_18 = now_shanghai.date_naive().and_hms_opt(18, 0, 0).unwrap();
    let today_18_shanghai = cst.from_local_datetime(&today_18).unwrap();

    if now_shanghai < today_18_shanghai {
        today_18_shanghai.with_timezone(&chrono::Utc)
    } else {
        (today_18_shanghai + chrono::Duration::days(1)).with_timezone(&chrono::Utc)
    }
}
