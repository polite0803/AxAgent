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
    init_mcp_oauth(state);
    start_auto_backup(app, state, app_dir.clone());
    start_webdav_sync(app, state, app_dir.clone());
    #[cfg(not(mobile))]
    start_tray(app, &_tray_language);
    start_closed_loop_service(app, state);
    start_insight_generation(state);
    start_pattern_learning(state);
    start_cross_session_learning(state);
    start_rl_reward_computation(state, app_dir);
    start_batch_processing(state);
    start_user_profile_persistence(state);
    start_skill_evolution(state);
    start_dream_consolidation(state);
    start_dream_task_executor(state);
    start_coevolution_task_executor(state);
    start_pattern_analyzer_task_executor(state);
    start_insight_generator_task_executor(state);
    start_auto_tool_observation(state);
    start_text_grad_analysis(state);
    start_cron_scheduler(state);
    start_trigger_recovery(state);
    start_persistent_runner(state);
    start_platform_adapters(state);
    start_skill_watcher(app, state);
    start_memory_decay_tick(state);
    start_memory_maintenance_tick(state);
    start_trajectory_cleanup(state);
    start_index_job_service(app, state);
    start_plugins(state);
}

/// 初始化 MCP OAuth 凭据存储的全局单例。
///
/// 必须在任何 MCP 工具调用前完成，否则 `McpOAuthStore::try_global()` 返回 `None`，
/// 受保护服务器将按匿名方式连接（很可能 401）。
fn init_mcp_oauth(state: &AppState) {
    let master_key = state.harness.master_key_owned();
    let crypto = std::sync::Arc::new(
        axagent_crypto::platform_adapter_impl::DefaultCryptoService::new(master_key),
    );
    let store = std::sync::Arc::new(axagent_mcp::mcp_oauth::McpOAuthStore::new(crypto));
    axagent_mcp::mcp_oauth::McpOAuthStore::init_global(store);
    tracing::info!("[McpOAuth] 全局 OAuth 凭据存储已初始化");
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

fn start_rl_reward_computation(state: &AppState, app_dir: std::path::PathBuf) {
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
                {
                    let mut rewards = rl.compute_rewards(trajectory).await;
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
                            // M7-C: 桥接 compute_policy_gradient → RLOptimizer 权重更新
                            if !gradients.is_empty() {
                                let mut opt =
                                    crate::commands::_shared_state::SHARED_OPTIMIZER.write().await;
                                opt.apply_gradients(&gradients);
                            }
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
            // M7-E: 每轮遍历结束保存 RLOptimizer 状态
            let save_dir = app_dir.clone();
            let _ = tokio::task::spawn_blocking(move || {
                crate::commands::_shared_state::save_rl_optimizer(&save_dir);
            })
            .await;
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
            engine.set_llm_provider(std::sync::Arc::new(bridge)).await;
            drop(engine);
            tracing::info!("[evolution] LLM provider injected into SkillEvolutionEngine");
        }

        let interval = std::time::Duration::from_secs(45 * 60);
        let success_threshold = 0.5;
        let min_usages = 3;
        // P1: 连续失败次数阈值，达到即触发自动进化
        let auto_trigger_consecutive_failures: u32 = 3;
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
                .filter(|s| {
                    s.consecutive_failures >= auto_trigger_consecutive_failures
                        || (s.total_usages >= min_usages && s.success_rate < success_threshold)
                })
                .collect();
            if weak_skills.is_empty() {
                continue;
            }
            tracing::info!(
                "[evolution] Found {} skills to evolve (consecutive_failures>={} or success<{:.0}%)",
                weak_skills.len(),
                auto_trigger_consecutive_failures,
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

/// Dream 巩固定时任务
///
/// 每 30 分钟检查一次：
/// 1. 通过 trajectory 数量增量检测新会话，调用 record_new_session 累加计数
/// 2. 检查 should_consolidate 门控（启用/未运行/间隔/会话数/锁）
/// 3. 满足门控则执行 consolidate（经验回放→知识蒸馏→对比学习→建议生成）
fn start_dream_consolidation(state: &AppState) {
    let consolidator = state.dream_consolidator.clone();
    let trajectory_storage = state.trajectory_storage.clone();
    let last_trajectory_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(u64::MAX));
    let last_count = last_trajectory_count.clone();
    tauri::async_runtime::spawn(async move {
        // 初始化 trajectory 基线计数
        if let Ok(trajs) = trajectory_storage.get_trajectories(Some(10000)).await {
            last_count.store(trajs.len() as u64, std::sync::atomic::Ordering::Relaxed);
        }

        let interval = std::time::Duration::from_secs(30 * 60);
        loop {
            tokio::time::sleep(interval).await;

            // 检测新会话：trajectory 数量增量即为新会话数
            let current_count = match trajectory_storage.get_trajectories(Some(10000)).await {
                Ok(trajs) => trajs.len() as u64,
                Err(e) => {
                    tracing::warn!("[dream] 获取 trajectory 失败: {}", e);
                    continue;
                },
            };
            let prev_count = last_count.swap(current_count, std::sync::atomic::Ordering::Relaxed);
            // 首次循环 prev_count == u64::MAX（基线），跳过计数
            if prev_count != u64::MAX && current_count > prev_count {
                let new_sessions = (current_count - prev_count) as usize;
                for _ in 0..new_sessions {
                    consolidator.record_new_session().await;
                }
                tracing::info!("[dream] 记录 {} 个新会话", new_sessions);
            }

            // 检查门控条件
            if !consolidator.should_consolidate().await {
                continue;
            }

            tracing::info!("[dream] 开始执行巩固...");
            let result = consolidator
                .consolidate(
                    Some(&|n| tracing::info!("[dream] 提取 {} 条记忆", n)),
                    Some(&|n| tracing::info!("[dream] 发现 {} 个模式", n)),
                    Some(&|n| tracing::info!("[dream] 生成 {} 个建议", n)),
                )
                .await;

            if result.executed {
                tracing::info!(
                    "[dream] 巩固完成: {} 条记忆, {} 个模式, {} 个建议, 耗时 {} 秒",
                    result.memories_extracted,
                    result.patterns_discovered,
                    result.suggestions_generated,
                    result.duration_secs
                );

                // Dream↔Evolution 联动：发现新模式时提示可能需要触发技能进化
                // 注意：不直接调用 SkillEvolutionEngine（避免循环依赖），
                // 仅记录日志，由独立的 start_skill_evolution 定时任务在下一轮自动检测弱技能并进化
                if result.patterns_discovered > 0 {
                    tracing::info!(
                        "[dream] 发现 {} 个新模式，下一轮 skill evolution 将评估是否需要进化相关技能",
                        result.patterns_discovered
                    );
                }
            } else {
                tracing::warn!(
                    "[dream] 巩固未执行: {}",
                    result.error.as_deref().unwrap_or("未知原因")
                );
            }
        }
    });
}

fn start_memory_decay_tick(state: &AppState) {
    let memory_service = state.memory_service.clone();
    let harness_state = state.harness.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(3600);
        loop {
            tokio::time::sleep(interval).await;
            // 1. trajectory working memory 衰减（sentinel namespace）
            let ms = memory_service.read().await;
            let evicted = ms.apply_decay_tick().await;
            drop(ms);
            if evicted > 0 {
                tracing::info!("[memory_decay] trajectory evicted {} entries", evicted);
            }
            // 2. 用户 namespace 全表衰减（三层记忆系统）
            match axagent_dao::repo::memory::apply_decay_tick(harness_state.db()).await {
                Ok((expired, low_score, capacity)) => {
                    if expired + low_score + capacity > 0 {
                        tracing::info!(
                            "[memory_decay] user ns: expired={}, low_score={}, capacity={}",
                            expired,
                            low_score,
                            capacity
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!("[memory_decay] user ns tick failed: {}", e);
                },
            }
        }
    });
}

/// DreamTask 全量清理定时任务
///
/// 每 60 分钟执行一次 FullCleanup，涵盖：
/// - 轨迹整合（consolidator，与 start_dream_consolidation 共享实例，内部有门控）
/// - 记忆压缩（auto_memory_extractor + FTS5 optimize）
/// - 技能更新（skill_evolution_engine，与 start_skill_evolution 共享实例）
/// - 僵尸 SubAgent 清理（sub_agent_registry）
/// - FTS5 索引优化（optimize + vacuum）
///
/// 与 start_dream_consolidation 的关系：
/// - start_dream_consolidation 30 分钟一次，仅做轨迹巩固（轻量）
/// - start_dream_task_executor 60 分钟一次，做全量清理（重量）
///
/// 两者共享 DreamConsolidator 实例，consolidator 内部 should_consolidate 门控
/// 会避免重复执行实际的巩固操作。
fn start_dream_task_executor(state: &AppState) {
    // 组装 DreamTaskContext（所有依赖均为 AppState 中的 Arc 克隆）
    let ctx = axagent_runtime::tasks::dream_task::DreamTaskContext {
        consolidator: Some(state.dream_consolidator.clone()),
        trajectory_storage: Some(state.trajectory_storage.clone()),
        skill_evolution_engine: Some(state.skill_evolution_engine.clone()),
        auto_memory_extractor: Some(state.auto_memory_extractor.clone()),
        sub_agent_registry: Some(state.sub_agent_registry.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 10 分钟首次执行，避免与启动期间的其它密集任务冲突
        let initial_delay = std::time::Duration::from_secs(10 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(60 * 60);
        loop {
            let task = axagent_runtime::tasks::dream_task::DreamTask::on_session_end();
            tracing::info!("[dream_task_executor] 触发全量清理 (scope={:?})", task.scope);
            let result =
                axagent_runtime::tasks::dream_task::DreamTaskExecutor::execute(&task, &ctx).await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[dream_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
        }
    });
}

/// CoevolutionTask 协同进化定时任务
///
/// 每 30 分钟执行一次，根据近期轨迹成功率驱动难度调整 + 生成针对薄弱类别的新任务。
/// 与 `start_skill_evolution` 共享同一 `CoevolutionEnvironment` 实例：
/// - `start_skill_evolution` 在技能进化成功后被动更新性能
/// - 本任务主动周期性地用整体成功率驱动协同进化
///
/// 依赖：`coevolution_env` / `trajectory_storage` / `insight_system`
/// 任一缺失会跳过对应子功能并在 `result.errors` 中记录。
fn start_coevolution_task_executor(state: &AppState) {
    let ctx = axagent_runtime::tasks::coevolution_task::CoevolutionTaskContext {
        coevolution_env: Some(state.coevolution_env.clone()),
        trajectory_storage: Some(state.trajectory_storage.clone()),
        insight_system: Some(state.insight_system.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 10 分钟首次执行，避免与启动期间的其它密集任务冲突
        let initial_delay = std::time::Duration::from_secs(10 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(30 * 60);
        loop {
            tracing::info!("[coevolution_task_executor] 触发协同进化周期任务");
            let result =
                axagent_runtime::tasks::coevolution_task::CoevolutionTaskExecutor::execute(&ctx)
                    .await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[coevolution_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
        }
    });
}

/// PatternAnalyzerTask 跨会话模式分析定时任务
///
/// 每 2 小时执行一次，从近期轨迹提取代码风格 / 工具偏好 / 时间分布模式，
/// 把关键发现作为 `LearningInsight` 写入 `insight_system`。
///
/// 与 `start_pattern_learning` 互补：
/// - `start_pattern_learning` 学习任务级 `TrajectoryPattern`（含 success_rate）
/// - 本任务提取更细粒度的用户行为模式，用于丰富用户画像与行为洞察
///
/// 依赖：`trajectory_storage` / `insight_system`
fn start_pattern_analyzer_task_executor(state: &AppState) {
    let ctx = axagent_runtime::tasks::pattern_task::PatternAnalyzerTaskContext {
        trajectory_storage: Some(state.trajectory_storage.clone()),
        insight_system: Some(state.insight_system.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 15 分钟首次执行，比 coevolution 稍晚以错峰
        let initial_delay = std::time::Duration::from_secs(15 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(2 * 60 * 60);
        loop {
            tracing::info!("[pattern_analyzer_task_executor] 触发模式分析周期任务");
            let result =
                axagent_runtime::tasks::pattern_task::PatternAnalyzerTaskExecutor::execute(&ctx)
                    .await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[pattern_analyzer_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
        }
    });
}

/// InsightGeneratorTask 学习洞察生成定时任务
///
/// 每 6 小时执行一次，从近期轨迹分析整体趋势（成功率 / 质量分布），
/// 生成趋势洞察 + 日报。与 `start_insight_generation` 互补：
/// - `start_insight_generation` 从实时反馈生成洞察（10 分钟一次，关注实时）
/// - 本任务从轨迹存储的整体趋势生成洞察 + 日报（周期更长，关注长期）
///
/// 依赖：`trajectory_storage` / `insight_system`
fn start_insight_generator_task_executor(state: &AppState) {
    let ctx = axagent_runtime::tasks::insight_task::InsightGeneratorTaskContext {
        trajectory_storage: Some(state.trajectory_storage.clone()),
        insight_system: Some(state.insight_system.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 20 分钟首次执行，与其它任务错峰
        let initial_delay = std::time::Duration::from_secs(20 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(6 * 60 * 60);
        loop {
            tracing::info!("[insight_generator_task_executor] 触发洞察生成周期任务");
            let result =
                axagent_runtime::tasks::insight_task::InsightGeneratorTaskExecutor::execute(&ctx)
                    .await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[insight_generator_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
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

    // 保存到 AppState 以便外部控制（停止/重启）
    {
        let mut state_scheduler = tauri::async_runtime::block_on(state.cron_scheduler.write());
        *state_scheduler = Some(scheduler.clone());
    }

    tauri::async_runtime::spawn(async move {
        scheduler.start().await;
    });

    tracing::info!("[CronScheduler] 已启动（统一 Cron + ScheduledTask），每30秒轮询一次");
}

/// 2.7 P1:启动时从 DB 恢复工作流触发器到运行时 `TriggerManager`。
///
/// 在 `start_cron_scheduler` 之后调用 — `init_trigger_manager` 已在
/// `create_app_state` 中执行,这里只需扫描 `workflow_templates.trigger_config`
/// 字段,对非 Manual 类型触发器批量调用 `register_*`。
///
/// 失败仅 warn 日志,不阻断启动 — 即使所有触发器恢复失败,工作流模板
/// 本身仍然可用,用户可手动触发或通过 update 命令重新激活。
fn start_trigger_recovery(state: &AppState) {
    let db = state.harness.db().clone();
    let trigger_manager = state.work_engine.trigger_manager.clone();
    tauri::async_runtime::spawn(async move {
        let (sched, webhook, event) =
            crate::init::trigger_recovery::recover_workflow_triggers(&db, &trigger_manager).await;
        tracing::info!(
            "[start_trigger_recovery] 触发器恢复完成: {} schedule, {} webhook, {} event",
            sched,
            webhook,
            event
        );
    });
}

/// 3.3 P2:启动 PersistentRunner 后台守护线程。
///
/// 守护线程每 60 秒检查一次 pending session。默认 `enabled: false` 时
/// 守护线程空转 sleep,不会有任何调度行为。
///
/// **注意**:当前 executor 闭包为占位实现,返回 `Err("not implemented")`。
/// 真正的 SessionManager 适配器需后续实现 — 实现后即可通过配置启用持久化重试。
fn start_persistent_runner(state: &AppState) {
    let Some(runner) = state.persistent_runner.clone() else {
        tracing::debug!("[start_persistent_runner] PersistentRunner 未构造,跳过");
        return;
    };

    // 占位 executor — 真正的 SessionManager 适配器需后续实现。
    // 当前返回 Err,让 PersistentRunner 记录 warn 日志但不 panic。
    let executor: axagent_runtime::persistent_runner::SessionExecutor = Arc::new(|_session| {
        Box::pin(async {
            tracing::warn!("[PersistentRunner] SessionExecutor 适配器尚未实现,session 执行被跳过");
            Err("SessionExecutor adapter not yet implemented".to_string())
        })
    });

    // spawn_daemon 内部第一行即 tokio::spawn,要求 tokio runtime 上下文。
    // Tauri setup 闭包是同步执行,不在 runtime 上下文中,直接调用会 panic
    // ("there is no reactor running")。用 tauri::async_runtime::spawn 包裹
    // 进入 runtime 上下文后再执行 spawn_daemon。参考 start_trigger_recovery。
    tauri::async_runtime::spawn(async move {
        let handle = runner.spawn_daemon(60, executor);
        tracing::info!(
            "[start_persistent_runner] 守护线程已启动(默认 enabled=false,空转等待配置启用)"
        );
        // JoinHandle 被 drop 时 tokio 不会取消任务(detach),守护线程继续运行。
        // 若需要优雅关闭,可后续把 handle 挂到 task_manager。
        drop(handle);
    });
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
