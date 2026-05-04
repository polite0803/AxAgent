use crate::AppState;
use axagent_core::repo::agent_role;
use axagent_core::types::AgentRoleDef;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ImportAgentRolesResult {
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}

/// 列出所有 AgentRole（内置 + 导入）
#[tauri::command]
pub async fn list_agent_roles(
    app_state: State<'_, AppState>,
    source: Option<String>,
) -> Result<Vec<AgentRoleDef>, String> {
    agent_role::list_agent_roles(&app_state.sea_db, source.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 从 Open Agent Spec 目录导入 AgentRole
#[tauri::command]
pub async fn import_agent_roles(
    app_state: State<'_, AppState>,
    path: String,
) -> Result<ImportAgentRolesResult, String> {
    let db = &app_state.sea_db;
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("路径不存在或不是目录: {}", path));
    }

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();

    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();

        if file_path
            .extension()
            .is_none_or(|e| e != "yaml" && e != "yml")
        {
            continue;
        }

        let content = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;

        match parse_open_agent_spec(&content) {
            Ok(role) => {
                match agent_role::upsert_agent_role(
                    db,
                    &role.id,
                    &role.name,
                    role.description.as_deref(),
                    &role.system_prompt,
                    &role.default_tools,
                    role.max_concurrent as i32,
                    role.timeout_seconds as i64,
                    &role.source,
                )
                .await
                {
                    Ok(_) => imported += 1,
                    Err(e) => errors.push(format!("{}: {}", file_path.display(), e)),
                }
            },
            Err(e) => {
                skipped += 1;
                errors.push(format!("{}: {}", file_path.display(), e));
            },
        }
    }

    Ok(ImportAgentRolesResult {
        imported,
        skipped,
        errors,
    })
}

/// 删除导入的 AgentRole（builtin 不可删除）
#[tauri::command]
pub async fn delete_agent_role(app_state: State<'_, AppState>, id: String) -> Result<(), String> {
    let role = agent_role::get_agent_role(&app_state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Role {} not found", id))?;

    if role.source == "builtin" {
        return Err("内置角色不可删除".to_string());
    }

    agent_role::delete_agent_role(&app_state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 解析 Open Agent Spec YAML 文件
fn parse_open_agent_spec(yaml_str: &str) -> Result<AgentRoleDef, String> {
    let doc: serde_yaml::Value =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))?;

    let agent = doc
        .get("agent")
        .ok_or_else(|| "Missing 'agent' block".to_string())?;
    let name = agent
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing agent.name".to_string())?;

    let role_id = name.to_lowercase().replace(' ', "-");

    let system_prompt = doc
        .get("prompts")
        .and_then(|p| p.get("system"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tools: Vec<String> = doc
        .get("tools")
        .and_then(|t| t.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|t| {
                    t.get("native")
                        .or_else(|| t.get("mcp"))
                        .or_else(|| t.get("name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    let max_concurrent = doc
        .get("max_concurrent")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let timeout_seconds = doc
        .get("timeout_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(600);

    let description = agent
        .get("description")
        .or_else(|| doc.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let now = axagent_core::utils::now_ts();

    Ok(AgentRoleDef {
        id: role_id,
        name: name.to_string(),
        description,
        system_prompt,
        default_tools: tools,
        max_concurrent,
        timeout_seconds,
        source: "imported".to_string(),
        sort_order: 0,
        created_at: now,
        updated_at: now,
    })
}
