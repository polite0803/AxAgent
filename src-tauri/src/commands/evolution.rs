// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct EvolutionEngineStatus {
    pub name: String,
    pub running: bool,
    pub last_run: Option<String>,
    pub items_processed: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub skill_count: usize,
    pub total_trajectories: usize,
    pub evolution_engines: Vec<EvolutionEngineStatus>,
    pub auto_tools_count: usize,
    pub auto_tool_patterns: Vec<String>,
    pub text_grad_nodes: usize,
    pub text_grad_gradients: usize,
    pub constitution_rules: usize,
    pub intrinsic_motivation_active: bool,
    pub coevolution_tasks: usize,
    pub dream_knowledge_count: usize,
    pub prm_enabled: bool,
    pub sandbox_enabled: bool,
    pub llm_provider_connected: bool,
}

#[command]
pub async fn get_evolution_stats(
    state: tauri::State<'_, AppState>,
) -> Result<EvolutionStats, String> {
    let skill_evolution = state.skill_evolution_engine.lock().await;
    let skill_count = skill_evolution.skill_count();
    let llm_connected = skill_evolution.has_llm_provider();
    let sandbox_enabled = skill_evolution.has_sandbox();
    drop(skill_evolution);

    let trajectories =
        state.trajectory_storage.get_trajectories(Some(1000)).await.unwrap_or_default();

    let auto_tool = state.auto_tool_creator.lock().await;
    let auto_tools_count = auto_tool.tool_count();
    let auto_tool_patterns = auto_tool
        .get_frequent_patterns(2)
        .iter()
        .take(5)
        .map(|(p, c)| format!("{} (×{})", p, c))
        .collect();
    drop(auto_tool);

    let text_grad = state.text_grad_engine.lock().await;
    let text_grad_stats = text_grad.stats();
    drop(text_grad);

    let constitution_rules = state.constitution.rule_count();

    let intrinsic = state.intrinsic_motivation.lock().await;
    let intrinsic_motivation_active = intrinsic.has_provider();
    drop(intrinsic);

    let coevolution = state.coevolution_env.lock().await;
    let coevolution_tasks = coevolution.task_count();
    drop(coevolution);

    let dream_knowledge_count = state.dream_consolidator.knowledge_count().await;

    let prm = state.process_reward_model.lock().await;
    let prm_enabled = prm.has_provider();
    drop(prm);

    let evolution_engines = vec![
        EvolutionEngineStatus {
            name: "Skill Evolution".into(),
            running: llm_connected,
            last_run: None,
            items_processed: skill_count as u64,
        },
        EvolutionEngineStatus {
            name: "RL Reward".into(),
            running: true,
            last_run: None,
            items_processed: trajectories.len() as u64,
        },
        EvolutionEngineStatus {
            name: "Process Reward Model".into(),
            running: prm_enabled,
            last_run: None,
            items_processed: 0,
        },
        EvolutionEngineStatus {
            name: "Auto Tool Creator".into(),
            running: auto_tools_count > 0,
            last_run: None,
            items_processed: auto_tools_count as u64,
        },
        EvolutionEngineStatus {
            name: "TextGrad Engine".into(),
            running: text_grad_stats.gradient_count > 0,
            last_run: None,
            items_processed: text_grad_stats.gradient_count as u64,
        },
        EvolutionEngineStatus {
            name: "Dream Consolidator".into(),
            running: dream_knowledge_count > 0,
            last_run: None,
            items_processed: dream_knowledge_count as u64,
        },
        EvolutionEngineStatus {
            name: "Intrinsic Motivation".into(),
            running: intrinsic_motivation_active,
            last_run: None,
            items_processed: 0,
        },
        EvolutionEngineStatus {
            name: "Coevolution".into(),
            running: coevolution_tasks > 0,
            last_run: None,
            items_processed: coevolution_tasks as u64,
        },
    ];

    Ok(EvolutionStats {
        skill_count,
        total_trajectories: trajectories.len(),
        evolution_engines,
        auto_tools_count,
        auto_tool_patterns,
        text_grad_nodes: text_grad_stats.node_count,
        text_grad_gradients: text_grad_stats.gradient_count,
        constitution_rules,
        intrinsic_motivation_active,
        coevolution_tasks,
        dream_knowledge_count,
        prm_enabled,
        sandbox_enabled,
        llm_provider_connected: llm_connected,
    })
}

// ---------------------------------------------------------------------------
// 模式学习命令（从 agent 模块迁移）
// ---------------------------------------------------------------------------

/// 获取学习到的模式（高价值和失败模式）
#[tauri::command]
pub async fn pattern_list(
    app_state: tauri::State<'_, AppState>,
    pattern_type: Option<String>,
    min_success_rate: Option<f64>,
) -> Result<Vec<serde_json::Value>, String> {
    use crate::commands::error::ErrorResponse;
    use crate::commands::error_code::agent as agent_err;

    let pl = app_state.pattern_learner.read().await;
    let patterns = if let Some(pt) = pattern_type {
        let ptype = match pt.as_str() {
            "tool_sequence" => axagent_trajectory::PatternType::ToolSequence,
            "reasoning_chain" => axagent_trajectory::PatternType::ReasoningChain,
            "error_recovery" => axagent_trajectory::PatternType::ErrorRecovery,
            "user_interaction" => axagent_trajectory::PatternType::UserInteraction,
            "context_switch" => axagent_trajectory::PatternType::ContextSwitch,
            "multi_step" => axagent_trajectory::PatternType::MultiStep,
            "goal_oriented" => axagent_trajectory::PatternType::GoalOriented,
            "exploratory" => axagent_trajectory::PatternType::Exploratory,
            _ => {
                return Err(ErrorResponse::new(agent_err::INTERNAL)
                    .with_detail(format!("Unknown pattern type: {}", pt))
                    .to_string());
            },
        };
        pl.get_patterns_by_type(ptype).iter().filter_map(|p| serde_json::to_value(p).ok()).collect()
    } else if let Some(min_sr) = min_success_rate {
        pl.get_high_value_patterns(min_sr)
            .iter()
            .filter_map(|p| serde_json::to_value(p).ok())
            .collect()
    } else {
        // 从存储返回所有模式
        drop(pl);
        let all = app_state.trajectory_storage.get_patterns().await.map_err(|e| e.to_string())?;
        all.iter().filter_map(|p| serde_json::to_value(p).ok()).collect()
    };
    Ok(patterns)
}

/// 获取跨会话洞察
#[tauri::command]
pub async fn cross_session_insights(
    app_state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let csl = app_state.cross_session_learner.read().await;
    let insights = csl.get_cross_session_insights();
    Ok(insights.iter().filter_map(|i| serde_json::to_value(i).ok()).collect())
}

// ---------------------------------------------------------------------------
// 技能进化命令（从 agent 模块迁移）
// ---------------------------------------------------------------------------

/// 启动指定技能的进化
#[tauri::command]
pub async fn skill_evolution_start(
    app_state: tauri::State<'_, AppState>,
    skill_id: String,
) -> Result<serde_json::Value, String> {
    // 获取技能
    let skill = app_state
        .trajectory_storage
        .get_skill(&skill_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;

    // 获取测试轨迹
    let trajectories =
        app_state.trajectory_storage.get_trajectories(Some(30)).await.map_err(|e| e.to_string())?;
    let test_refs: Vec<_> = trajectories.iter().collect();

    // 运行进化
    let mut engine = app_state.skill_evolution_engine.lock().await;
    let result = engine.run(&skill, &test_refs).await;

    match result {
        Some(modification) => {
            let improved = modification.validation_result.as_ref().is_some_and(|v| v.success);

            // 如果改进，修补技能
            if improved {
                let mut updated = skill.clone();
                updated.content = modification.new_content.clone();
                updated.quality_score = modification.confidence;
                if let Err(e) = app_state.trajectory_storage.save_skill(&updated).await {
                    tracing::warn!("[evolution] Failed to save evolved skill: {}", e);
                }
            }

            Ok(serde_json::json!({
                "skill_id": skill_id,
                "improved": improved,
                "reason": modification.reason,
                "confidence": modification.confidence,
                "quality_delta": modification.validation_result.as_ref().map(|v| v.quality_delta),
                "stats": engine.get_stats(),
            }))
        },
        None => Ok(serde_json::json!({
            "skill_id": skill_id,
            "improved": false,
            "reason": "Evolution did not produce a result",
            "confidence": 0.0,
        })),
    }
}

/// 获取当前技能进化状态
#[tauri::command]
pub async fn skill_evolution_status(
    app_state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let engine = app_state.skill_evolution_engine.lock().await;
    let stats = engine.get_stats();
    Ok(serde_json::json!({
        "is_running": engine.is_running(),
        "stats": stats,
    }))
}

/// 获取当前自适应状态
#[tauri::command]
pub async fn adaptation_status(
    app_state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut rl = app_state.realtime_learning.lock().await;
    let adaptation = rl.compute_adaptation();
    Ok(serde_json::json!({
        "response_style": adaptation.response_style,
        "content_adjustments": adaptation.content_adjustments,
        "skill_suggestions": adaptation.skill_suggestions,
        "memory_priorities": adaptation.memory_priorities,
    }))
}