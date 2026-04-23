use chrono;
use crate::AppState;

pub fn start_background_services(app: &tauri::AppHandle, state: &AppState, app_dir: std::path::PathBuf, tray_language: String) {
    start_auto_backup(app, state, app_dir.clone());
    start_webdav_sync(app, state, app_dir);
    start_tray(app, &tray_language);
    start_closed_loop_service(app, state);
    start_insight_generation(state);
    start_pattern_learning(state);
    start_cross_session_learning(state);
    start_rl_reward_computation(state);
    start_batch_processing(state);
    start_user_profile_persistence(state);
    start_skill_evolution(state);
}

fn start_auto_backup(_app: &tauri::AppHandle, state: &AppState, app_dir: std::path::PathBuf) {
    let db = state.sea_db.clone();
    let app_data = app_dir.clone();
    let handle = state.auto_backup_handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_core::repo::settings::get_settings(&db).await {
            if settings.auto_backup_enabled && settings.auto_backup_interval_hours > 0 {
                let backup_dir_setting = axagent_core::path_vars::decode_path_opt(&settings.backup_dir);
                let interval = settings.auto_backup_interval_hours;
                let max_count = settings.auto_backup_max_count;
                let interval_secs = interval as u64 * 3600;
                let db2 = db.clone();
                let app_dir2 = app_data.clone();

                let initial_delay_secs = match axagent_core::repo::backup::list_backups(&db).await {
                    Ok(backups) if !backups.is_empty() => {
                        let last_ts = &backups[0].created_at;
                        if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(last_ts, "%Y-%m-%d %H:%M:%S") {
                            let elapsed = chrono::Utc::now()
                                .naive_utc()
                                .signed_duration_since(last_time)
                                .num_seconds()
                                .max(0) as u64;
                            if elapsed >= interval_secs { 0 } else { interval_secs - elapsed }
                        } else { interval_secs }
                    }
                    _ => interval_secs,
                };

                let task = tokio::spawn(async move {
                    let dur = std::time::Duration::from_secs(interval_secs);
                    tokio::time::sleep(std::time::Duration::from_secs(initial_delay_secs)).await;
                    loop {
                        let backup_dir = axagent_core::repo::backup::resolve_backup_dir(
                            backup_dir_setting.as_deref(),
                            &app_dir2,
                        );
                        if let Err(e) = axagent_core::repo::backup::create_backup(&db2, "sqlite", &backup_dir).await {
                            tracing::warn!("Auto-backup failed: {}", e);
                        } else {
                            tracing::info!("Auto-backup created");
                            let _ = axagent_core::repo::backup::cleanup_old_backups(&db2, max_count).await;
                        }
                        tokio::time::sleep(dur).await;
                    }
                });
                *handle.lock().await = Some(task);
            }
        }
    });
}

fn start_webdav_sync(_app: &tauri::AppHandle, state: &AppState, app_dir: std::path::PathBuf) {
    let db = state.sea_db.clone();
    let master_key = state.master_key;
    let app_data_dir = app_dir.clone();
    let handle = state.webdav_sync_handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_core::repo::settings::get_settings(&db).await {
            if settings.webdav_sync_enabled && settings.webdav_sync_interval_minutes > 0 {
                let db2 = db.clone();
                let interval = settings.webdav_sync_interval_minutes;
                let interval_secs = interval as u64 * 60;

                let initial_delay_secs = match axagent_core::repo::settings::get_setting(&db, "webdav_last_sync_time").await {
                    Ok(Some(ts)) => {
                        if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(&ts) {
                            let elapsed = chrono::Utc::now()
                                .signed_duration_since(last_time)
                                .num_seconds()
                                .max(0) as u64;
                            if elapsed >= interval_secs { 0 } else { interval_secs - elapsed }
                        } else { interval_secs }
                    }
                    _ => interval_secs,
                };

                let task = crate::commands::webdav::spawn_webdav_sync_task(db2, master_key, app_data_dir, interval as u32, initial_delay_secs);
                *handle.lock().await = Some(task);
            }
        }
    });
}

fn start_tray(app: &tauri::AppHandle, tray_language: &str) {
    if let Err(e) = crate::tray::create_tray(app, tray_language) {
        tracing::warn!("Failed to create system tray: {}", e);
    }
}

fn start_closed_loop_service(_app: &tauri::AppHandle, state: &AppState) {
    let db = state.sea_db.clone();
    let closed_loop = state.closed_loop_service.clone();
    let nudge_service = state.nudge_service.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_core::repo::settings::get_settings(&db).await {
            if settings.closed_loop_enabled {
                closed_loop.start();
                let interval_minutes = settings.closed_loop_interval_minutes.max(1);
                let interval = std::time::Duration::from_secs(interval_minutes as u64 * 60);
                loop {
                    tokio::time::sleep(interval).await;
                    let new_nudges: Vec<axagent_trajectory::PeriodicNudge> = closed_loop.tick().await;
                    if !new_nudges.is_empty() {
                        tracing::info!("[closed_loop] Generated {} periodic nudges", new_nudges.len());
                        let candidates: Vec<axagent_trajectory::NudgeCandidate> = new_nudges.iter().map(|pn| {
                            axagent_trajectory::NudgeCandidate {
                                entity: axagent_trajectory::NudgeEntity {
                                    id: pn.id.clone(),
                                    name: pn.title.clone(),
                                    entity_type: format!("{:?}", pn.nudge_type),
                                    confidence: if pn.urgency == "high" { 0.9 } else if pn.urgency == "medium" { 0.7 } else { 0.5 },
                                },
                                reason: pn.description.clone(),
                                urgency: match pn.urgency.as_str() {
                                    "high" => axagent_trajectory::Urgency::High,
                                    "medium" => axagent_trajectory::Urgency::Medium,
                                    _ => axagent_trajectory::Urgency::Low,
                                },
                                suggested_action: Some(pn.suggested_action.clone()),
                            }
                        }).collect();
                        let mut ns: tokio::sync::MutexGuard<'_, axagent_trajectory::NudgeService> = nudge_service.lock().await;
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
                let rl: tokio::sync::MutexGuard<'_, axagent_trajectory::RealTimeLearning> = realtime_learning.lock().await;
                rl.generate_insights()
            };
            if !new_insights.is_empty() {
                tracing::info!("[insight] Generated {} learning insights from feedback", new_insights.len());
                let mut is = insight_system.write().unwrap();
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
            let trajectories: Vec<axagent_trajectory::Trajectory> = match trajectory_storage.get_trajectories(Some(20)) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("[pattern] Failed to fetch trajectories: {}", e);
                    continue;
                }
            };
            if trajectories.is_empty() { continue; }
            let mut pl = pattern_learner.write().unwrap();
            let new_patterns = pl.update_from_batch(&trajectories);
            drop(pl);
            if !new_patterns.is_empty() {
                tracing::info!("[pattern] Learned {} new patterns from {} trajectories", new_patterns.len(), trajectories.len());
                for pattern in &new_patterns {
                    if let Err(e) = trajectory_storage.save_pattern(pattern) {
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
            let trajectories: Vec<axagent_trajectory::Trajectory> = match trajectory_storage.get_trajectories(Some(50)) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("[cross_session] Failed to fetch trajectories: {}", e);
                    continue;
                }
            };
            if trajectories.len() < 3 { continue; }
            let mut by_session: std::collections::HashMap<String, Vec<axagent_trajectory::Trajectory>> = std::collections::HashMap::new();
            for t in trajectories {
                by_session.entry(t.session_id.clone()).or_default().push(t);
            }
            if by_session.len() < 2 { continue; }
            let mut csl = cross_session_learner.write().unwrap();
            let new_patterns = csl.learn_from_sessions(by_session);
            drop(csl);
            if !new_patterns.is_empty() {
                tracing::info!("[cross_session] Discovered {} cross-session patterns", new_patterns.len());
                let mut is = insight_system.write().unwrap();
                for pattern in &new_patterns {
                    if let Err(e) = trajectory_storage.save_pattern(pattern) {
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
                            suggested_action: Some("Consider creating a skill for this recurring pattern".to_string()),
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
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(20 * 60);
        let mut reward_normalizer = axagent_trajectory::RewardNormalizer::new();
        loop {
            tokio::time::sleep(interval).await;
            let mut trajectories: Vec<axagent_trajectory::Trajectory> = match trajectory_storage.get_trajectories(Some(15)) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("[rl] Failed to fetch trajectories: {}", e);
                    continue;
                }
            };
            if trajectories.is_empty() { continue; }
            let rl = rl_engine.read().unwrap();
            let mut total_rewards = 0;
            let mut total_advantages = 0;
            for trajectory in &mut trajectories {
                if trajectory.rewards.is_empty() {
                    let mut rewards = rl.compute_rewards(trajectory);
                    total_rewards += rewards.len();
                    rl.shape_rewards(&mut rewards);
                    reward_normalizer.normalize(&mut rewards);
                    trajectory.rewards = rewards;
                    let values = rl.estimate_value_function(trajectory);
                    if !values.is_empty() {
                        let advantages = rl.compute_advantages(&trajectory.rewards, &values);
                        total_advantages += advantages.len();
                        let avg_advantage: f64 = if !advantages.is_empty() {
                            advantages.iter().sum::<f64>() / advantages.len() as f64
                        } else { 0.0 };
                        if avg_advantage > 0.3 {
                            let gradients = rl.compute_policy_gradient(trajectory, &advantages);
                            tracing::debug!("[rl] High-advantage trajectory {}: avg_adv={:.3}, gradients={:?}",
                                &trajectory.id[..trajectory.id.len().min(8)], avg_advantage, gradients);
                        }
                    }
                    let total_reward: f64 = trajectory.rewards.iter().map(|r| r.value).sum();
                    trajectory.value_score = (trajectory.value_score + total_reward) / 2.0;
                    if let Err(e) = trajectory_storage.save_trajectory(trajectory) {
                        tracing::warn!("[rl] Failed to update trajectory: {}", e);
                    }
                }
            }
            drop(rl);
            if total_rewards > 0 {
                tracing::info!("[rl] Computed {} rewards, {} advantages across {} trajectories",
                    total_rewards, total_advantages, trajectories.len());
                let reward_trajectories: Vec<_> = trajectories.iter().filter(|t| !t.rewards.is_empty()).collect();
                if reward_trajectories.len() >= 3 {
                    let avg_reward: f64 = reward_trajectories.iter()
                        .map(|t| t.rewards.iter().map(|r| r.value).sum::<f64>()).sum::<f64>() / reward_trajectories.len() as f64;
                    let high_reward_count = reward_trajectories.iter()
                        .filter(|t| t.rewards.iter().map(|r| r.value).sum::<f64>() > avg_reward).count();
                    let mut is = insight_system.write().unwrap();
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
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(60 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let bp = &*batch_processor;
            let trajectories: Vec<axagent_trajectory::Trajectory> = match trajectory_storage.get_trajectories(Some(50)) {
                Ok(t) => t,
                Err(_) => continue,
            };
            if trajectories.len() < 5 { continue; }
            let quality_filtered = bp.filter_by_quality(&trajectories, 0.3);
            if quality_filtered.is_empty() { continue; }
            let analysis = bp.analyze_batch(&quality_filtered);
            let mut is = insight_system.write().unwrap();
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
    });
}

fn start_user_profile_persistence(state: &AppState) {
    let user_profile = state.user_profile.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(10 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let profile = user_profile.read().unwrap();
            let md_content = profile.to_user_md();
            drop(profile);
            if let Some(home) = dirs::home_dir() {
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
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(45 * 60);
        let success_threshold = 0.5;
        let min_usages = 3;
        loop {
            tokio::time::sleep(interval).await;
            let skills: Vec<axagent_trajectory::Skill> = match trajectory_storage.get_skills() {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("[evolution] Failed to fetch skills: {}", e);
                    continue;
                }
            };
            let weak_skills: Vec<_> = skills.into_iter()
                .filter(|s| s.total_usages >= min_usages && s.success_rate < success_threshold)
                .collect();
            if weak_skills.is_empty() { continue; }
            tracing::info!("[evolution] Found {} skills below success threshold ({:.0}%)",
                weak_skills.len(), success_threshold * 100.0);
            let test_trajectories: Vec<axagent_trajectory::Trajectory> = match trajectory_storage.get_trajectories(Some(30)) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let test_refs: Vec<&axagent_trajectory::Trajectory> = test_trajectories.iter().collect();
            for skill in weak_skills.iter().take(2) {
                let mut engine: tokio::sync::MutexGuard<'_, axagent_trajectory::SkillEvolutionEngine> = skill_evolution_engine.lock().await;
                let result = engine.run(skill, &test_refs);
                if let Some(modification) = result {
                    if modification.validation_result.as_ref().map_or(false, |v| v.success) {
                        tracing::info!("[evolution] Skill '{}' evolved: {} (confidence={:.3})",
                            skill.name, modification.reason, modification.confidence);
                        let mut updated_skill = skill.clone();
                        updated_skill.content = modification.new_content.clone();
                        updated_skill.quality_score = modification.confidence;
                        updated_skill.version = format!("{}.e{}",
                            updated_skill.version.trim_end_matches(|c: char| c == '.' || c.is_ascii_digit()),
                            chrono::Utc::now().timestamp_millis() % 10000);
                        if let Err(e) = trajectory_storage.save_skill(&updated_skill) {
                            tracing::warn!("[evolution] Failed to save evolved skill: {}", e);
                        }
                        let mut is = insight_system.write().unwrap();
                        is.add_insight(axagent_trajectory::LearningInsight {
                            id: format!("evo_{}", chrono::Utc::now().timestamp_millis()),
                            category: axagent_trajectory::InsightCategory::Improvement,
                            title: format!("Skill '{}' auto-evolved", skill.name),
                            description: modification.reason.clone(),
                            confidence: modification.confidence,
                            evidence: vec![],
                            suggested_action: Some(format!("Review evolved skill '{}' for correctness", skill.name)),
                            created_at: chrono::Utc::now().timestamp_millis(),
                        });
                    } else {
                        tracing::info!("[evolution] Skill '{}' evolution did not improve fitness", skill.name);
                    }
                }
            }
        }
    });
}
