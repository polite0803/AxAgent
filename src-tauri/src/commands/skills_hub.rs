// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::paths::axagent_home;
use axagent_agent_macro::agent_command;
use axagent_trajectory::{Skill, SkillsHubAdapter};
use serde::{Deserialize, Serialize};
use tauri::State;

// ── review / export / import — 纯本地操作（无网络依赖） ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsHubReviewResult {
    pub skill_name: String,
    pub action: String,
    pub message: String,
}

#[agent_command(domain = skills, safety = Caution, call_mode = StateInput, description = "审查隔离区技能")]
#[tauri::command]
pub async fn skills_hub_review(
    state: State<'_, AppState>,
    name: String,
    action: Option<String>,
) -> Result<SkillsHubReviewResult, String> {
    let hub_base = axagent_home().join("skills").join(".hub");
    let q_dir = hub_base.join("quarantine").join(&name);

    if !q_dir.exists() {
        return Err(format!("隔离区中未找到技能 '{}'。请先将技能文件放入隔离区后重试。", name));
    }

    let action_str = action.as_deref().unwrap_or("");

    match action_str {
        "approve" => {
            let skills_dir = axagent_home().join("skills");
            std::fs::create_dir_all(&skills_dir)
                .map_err(|e| format!("创建 skills 目录失败: {e}"))?;

            let skill_dir = skills_dir.join(&name);
            if skill_dir.exists() {
                return Err(format!("技能 '{}' 已安装，无法重复安装", name));
            }

            let manifest_path = q_dir.join("skill-manifest.json");
            let manifest: serde_json::Value = if manifest_path.exists() {
                let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                serde_json::json!({})
            };

            let skill_md_content = std::fs::read_to_string(q_dir.join("SKILL.md"))
                .map_err(|e| format!("读取 SKILL.md 失败: {e}"))?;

            std::fs::create_dir_all(&skill_dir).map_err(|e| format!("创建 skill 目录失败: {e}"))?;
            std::fs::write(skill_dir.join("SKILL.md"), &skill_md_content)
                .map_err(|e| format!("写入 SKILL.md 失败: {e}"))?;
            std::fs::write(
                skill_dir.join("skill-manifest.json"),
                serde_json::to_string_pretty(&manifest).unwrap_or_default(),
            )
            .map_err(|e| format!("写入 skill-manifest.json 失败: {e}"))?;

            std::fs::remove_dir_all(&q_dir).map_err(|e| format!("清理隔离目录失败: {e}"))?;

            let mut adapter = SkillsHubAdapter::new();
            adapter.parse_hermes_skill_md(&skill_md_content)?;
            let axagent_skill = adapter.to_axagent_skill()?;

            state
                .trajectory_storage
                .save_skill(&axagent_skill)
                .await
                .map_err(|e| format!("保存 skill 到存储失败: {e}"))?;

            let skill_content = axagent_skill.content.clone();
            let skill_name_for_tool = format!("Skill::{}", axagent_skill.name);
            {
                let mut reg = state.local_tool_registry.lock().await;
                reg.register_skill_tool(
                    skill_name_for_tool,
                    Box::new(move |_input: &str| Ok(skill_content.clone())),
                );
            }

            if let Some(workflow) = manifest.get("workflow") {
                if let (Some(nodes), Some(edges)) = (
                    workflow.get("nodes").and_then(|v| v.as_array()),
                    workflow.get("edges").and_then(|v| v.as_array()),
                ) {
                    use axagent_entities::workflow_template;
                    use sea_orm::Set;

                    let template_id = format!("skill_wf_{}", axagent_skill.name);
                    let nodes_json = serde_json::to_string(nodes).unwrap_or_default();
                    let edges_json = serde_json::to_string(edges).unwrap_or_default();
                    let now = chrono::Utc::now().timestamp_millis();

                    let tmpl = workflow_template::ActiveModel {
                        id: Set(template_id.clone()),
                        name: Set(format!("Skill: {}", axagent_skill.name)),
                        description: Set(Some(axagent_skill.description.clone())),
                        icon: Set("⚡".to_string()),
                        tags: Set(Some(axagent_skill.tags.join(","))),
                        version: Set(1i32),
                        is_preset: Set(false),
                        is_editable: Set(true),
                        is_public: Set(false),
                        trigger_config: Set(None),
                        nodes: Set(nodes_json),
                        edges: Set(edges_json),
                        input_schema: Set(None),
                        output_schema: Set(None),
                        variables: Set(None),
                        error_config: Set(None),
                        composite_source: Set(None),
                        tool_defs: Set(None),
                        mission_hash: Set(None),
                        cluster_id: Set(None),
                        route_path: Set(None),
                        created_at: Set(now),
                        updated_at: Set(now),
                    };
                    if let Err(e) = axagent_dao::repo::workflow_template::upsert_workflow_template(
                        state.harness.db(),
                        tmpl,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to register workflow template from skill '{}': {}",
                            axagent_skill.name,
                            e
                        );
                    }
                }
            }

            tracing::info!(
                "Skill '{}' approved and installed to {}",
                axagent_skill.name,
                skill_dir.display()
            );

            Ok(SkillsHubReviewResult {
                skill_name: name.clone(),
                action: "approve".to_string(),
                message: format!("技能 '{}' 已批准安装", name),
            })
        },
        "reject" => {
            std::fs::remove_dir_all(&q_dir).map_err(|e| format!("删除隔离区技能失败: {e}"))?;

            Ok(SkillsHubReviewResult {
                skill_name: name.clone(),
                action: "reject".to_string(),
                message: format!("技能 '{}' 已被拒绝并从隔离区删除", name),
            })
        },
        _ => {
            let skill_md_content = std::fs::read_to_string(q_dir.join("SKILL.md"))
                .unwrap_or_else(|_| "(未找到 SKILL.md)".to_string());

            Ok(SkillsHubReviewResult {
                skill_name: name.clone(),
                action: "preview".to_string(),
                message: format!(
                    "隔离区技能审查: {}\n\n{}\n\n使用 action=approve 批准安装，或 action=reject 拒绝删除。",
                    name, skill_md_content
                ),
            })
        },
    }
}

#[derive(Debug, Serialize)]
pub struct SkillExportResult {
    pub hermes_json: String,
    pub skill_name: String,
    pub version: String,
    pub manifest: serde_json::Value,
}

/// 导出本地 skill 为可发布的格式（Hermes JSON + manifest 摘要）
#[agent_command(domain = skills, safety = Safe, call_mode = StateInput, description = "导出技能为可发布格式")]
#[tauri::command]
pub async fn skills_hub_export(skill_name: String) -> Result<SkillExportResult, String> {
    let home = dirs::home_dir().ok_or("无法确定 home 目录")?;
    let skill_dirs = vec![
        axagent_home().join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ];

    let mut adapter = SkillsHubAdapter::new();
    let mut found_skill: Option<(String, serde_json::Value)> = None;

    // 搜索 skill
    for dir in &skill_dirs {
        let skill_path = dir.join(&skill_name);
        if !skill_path.exists() {
            continue;
        }

        // 读取 SKILL.md
        let skill_md_path = skill_path.join("SKILL.md");
        if skill_md_path.exists() {
            let content = std::fs::read_to_string(&skill_md_path)
                .map_err(|e| format!("读取 SKILL.md 失败: {}", e))?;
            adapter.parse_hermes_skill_md(&content)?;
        }

        // 读取 manifest.json
        let manifest_path = skill_path
            .join("manifest.json")
            .exists()
            .then(|| skill_path.join("manifest.json"))
            .or_else(|| {
                skill_path
                    .join("skill-manifest.json")
                    .exists()
                    .then(|| skill_path.join("skill-manifest.json"))
            });

        if let Some(mpath) = manifest_path {
            let manifest_content = std::fs::read_to_string(&mpath)
                .map_err(|e| format!("读取 manifest 失败: {}", e))?;
            let manifest_json: serde_json::Value = serde_json::from_str(&manifest_content)
                .map_err(|e| format!("manifest JSON 解析失败: {}", e))?;
            found_skill = Some((skill_name.clone(), manifest_json));
        }
        break;
    }

    let (name, manifest) = found_skill.ok_or_else(|| format!("Skill '{}' 未找到", skill_name))?;

    // 转换为 Hermes 格式
    let hermes = adapter.to_hermes_md();
    let version = manifest.get("version").and_then(|v| v.as_str()).unwrap_or("0.1.0").to_string();

    Ok(SkillExportResult {
        hermes_json: serde_json::to_string_pretty(&hermes)
            .map_err(|e| format!("序列化失败: {}", e))?,
        skill_name: name,
        version,
        manifest,
    })
}

#[agent_command(domain = skills, safety = Safe, call_mode = StateInput, description = "导入技能清单")]
#[tauri::command]
pub async fn skills_hub_import(manifest_json: String) -> Result<Skill, String> {
    let mut adapter = SkillsHubAdapter::new();
    adapter.parse_hermes_manifest(&manifest_json)?;

    adapter.to_axagent_skill()
}
