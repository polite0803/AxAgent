// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use sea_orm::EntityTrait;
use tauri::State;

/// 获取学习图数据（技能 + 记忆 + 洞察的节点/边关系）。
#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "获取学习图数据")]
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

    // 3. 从 trajectory_skills 表读取技能数据——使用 DB 的实际时间戳和使用次数
    let skills: Vec<axagent_trajectory::Skill> = {
        let db = app_state.harness.db();
        let rows = axagent_entities::trajectory_skills::Entity::find()
            .all(db)
            .await
            .map_err(|e| format!("Failed to load skills: {}", e))?;
        rows.into_iter()
            .map(|r| {
                let created_at =
                    chrono::NaiveDateTime::parse_from_str(&r.created_at, "%Y-%m-%d %H:%M:%S%.f")
                        .map(|n| n.and_utc())
                        .or_else(|_| {
                            chrono::DateTime::parse_from_rfc3339(&r.created_at)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                        })
                        .unwrap_or_else(|_| chrono::Utc::now());
                let updated_at =
                    chrono::NaiveDateTime::parse_from_str(&r.updated_at, "%Y-%m-%d %H:%M:%S%.f")
                        .map(|n| n.and_utc())
                        .or_else(|_| {
                            chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                        })
                        .unwrap_or_else(|_| chrono::Utc::now());
                axagent_trajectory::Skill {
                    id: r.id,
                    name: r.name,
                    description: r.description,
                    version: "1.0.0".to_string(),
                    content: r.content,
                    category: r.category.clone(),
                    tags: r
                        .tags
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    platforms: Vec::new(),
                    scenarios: r
                        .scenarios
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    quality_score: 0.0,
                    success_rate: r.success_rate,
                    avg_execution_time_ms: r.avg_execution_time_ms as u64,
                    total_usages: r.usage_count as u32,
                    successful_usages: (r.success_rate * r.usage_count as f64) as u32,
                    created_at,
                    updated_at,
                    last_used_at: None,
                    consecutive_failures: 0,
                    last_failure_at: None,
                    metadata: axagent_trajectory::SkillMetadata {
                        hermes: axagent_trajectory::HermesMetadata {
                            tags: Vec::new(),
                            category: r.category.clone(),
                            fallback_for_toolsets: Vec::new(),
                            requires_toolsets: Vec::new(),
                            config: Vec::new(),
                            source_kind: None,
                            source_ref: None,
                            commit: None,
                            skill_dependencies: None,
                        },
                        references: Vec::new(),
                    },
                }
            })
            .collect()
    };

    // 4. 读取真实实体关系数据（trajectory_entities / trajectory_relationships 表）
    let trajectory_storage = app_state.trajectory_storage.clone();
    let entities = trajectory_storage.get_all_entities().await.unwrap_or_default();
    let relationships = trajectory_storage.get_all_relationships().await.unwrap_or_default();

    let graph = axagent_trajectory::build_learning_graph(
        &skills,
        &memories,
        &insights,
        &entities,
        &relationships,
    );
    Ok(graph)
}
