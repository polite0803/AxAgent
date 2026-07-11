// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use sea_orm::EntityTrait;
use tauri::State;

/// 获取学习图数据（技能 + 记忆 + 洞察的节点/边关系）。
#[tauri::command]
pub async fn get_learning_graph(
    app_state: State<'_, AppState>,
) -> Result<axagent_trajectory::LearningGraph, String> {
    // 1. 获取记忆
    let memories = {
        let mem = app_state.auto_memory_extractor.read().await;
        mem.get_recent_extractions()
    };

    // 2. 获取洞察
    let insights = {
        let is = app_state.insight_system.read().await;
        is.get_insights().to_vec()
    };

    // 3. 从 trajectory_skills 表读取技能数据
    let skills: Vec<axagent_trajectory::Skill> = {
        let db = app_state.harness.db();
        let rows = axagent_entities::trajectory_skills::Entity::find()
            .all(db)
            .await
            .map_err(|e| format!("Failed to load skills: {}", e))?;
        rows.into_iter()
            .map(|r| {
                axagent_trajectory::Skill::new(
                    r.name,
                    r.description,
                    r.content,
                    r.category,
                )
            })
            .collect()
    };

    let graph = axagent_trajectory::build_learning_graph(&skills, &memories, &insights);
    Ok(graph)
}
