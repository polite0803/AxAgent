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

    let trajectories = state
        .trajectory_storage
        .get_trajectories(Some(1000))
        .unwrap_or_default();

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
