use crate::app_state::{PlannerAction, PlannerSession, PlannerVersion, TotNode, TotSession};
use crate::AppState;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tracing::info;

// Tree of Thoughts commands
#[tauri::command]
pub async fn tot_get_state(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, "tot_get_state invoked");
    let mut sessions = app_state.tot_sessions.lock().await;
    let session = sessions.entry(session_id).or_insert_with(|| TotSession {
        nodes: HashMap::new(),
        current_node_id: None,
        root_node_id: None,
    });
    serde_json::to_value(session).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub async fn tot_backtrack(
    session_id: String,
    node_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    info!(session_id = %session_id, node_id = %node_id, "tot_backtrack invoked");
    let mut sessions = app_state.tot_sessions.lock().await;
    let session = sessions.entry(session_id).or_insert_with(|| TotSession {
        nodes: HashMap::new(),
        current_node_id: None,
        root_node_id: None,
    });
    if session.nodes.contains_key(&node_id) {
        session.current_node_id = Some(node_id);
        Ok(())
    } else {
        Err(format!("Node not found: {}", node_id))
    }
}

#[tauri::command]
pub async fn tot_explore(
    session_id: String,
    node_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    info!(session_id = %session_id, node_id = %node_id, "tot_explore invoked");
    let mut sessions = app_state.tot_sessions.lock().await;
    let session = sessions.entry(session_id).or_insert_with(|| TotSession {
        nodes: HashMap::new(),
        current_node_id: None,
        root_node_id: None,
    });

    // Create children nodes (simple implementation for now)
    let parent_node = if let Some(n) = session.nodes.get(&node_id) {
        n.clone()
    } else {
        // If node doesn't exist, create root or add to existing tree
        let root_node = TotNode {
            id: node_id.clone(),
            parent_id: None,
            content: format!("Root node: {}", node_id),
            score: Some(0.5),
            children: Vec::new(),
        };
        session.nodes.insert(node_id.clone(), root_node);
        if session.root_node_id.is_none() {
            session.root_node_id = Some(node_id.clone());
        }
        if session.current_node_id.is_none() {
            session.current_node_id = Some(node_id.clone());
        }
        session.nodes.get(&node_id).unwrap().clone()
    };

    let mut children = Vec::new();
    for i in 0..3 {
        let child_id = format!("{}-{}", node_id, i);
        let child_node = TotNode {
            id: child_id.clone(),
            parent_id: Some(node_id.clone()),
            content: format!("Child {} of {}", i, node_id),
            score: Some(0.3 + (i as f64) * 0.2),
            children: Vec::new(),
        };
        session.nodes.insert(child_id.clone(), child_node);
        children.push(child_id);
    }

    // Update parent's children
    if let Some(parent) = session.nodes.get_mut(&node_id) {
        parent.children.extend(children.clone());
    }

    Ok(children)
}

// Replanning commands
#[tauri::command]
pub async fn planner_replan(
    session_id: String,
    reason: String,
    actions: Vec<Value>,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        session_id = %session_id,
        reason = %reason,
        action_count = actions.len(),
        "planner_replan invoked"
    );
    let mut sessions = app_state.planner_sessions.lock().await;
    let session = sessions.entry(session_id.clone()).or_insert_with(|| PlannerSession {
        actions: Vec::new(),
        versions: Vec::new(),
        current_version: 0,
    });

    // Record actions
    for action_data in actions {
        let action = PlannerAction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            action_type: "replan_action".to_string(),
            data: action_data,
        };
        session.actions.push(action);
    }

    // Create new version
    let new_version_id = session.current_version + 1;
    let version = PlannerVersion {
        id: new_version_id,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        reason,
        state: serde_json::json!({
            "session_id": session_id,
            "version": new_version_id,
            "action_count": actions.len()
        }),
    };
    session.versions.push(version);
    session.current_version = new_version_id;

    serde_json::to_value(session).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub async fn planner_rollback(
    session_id: String,
    version: u32,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    info!(session_id = %session_id, version = version, "planner_rollback invoked");
    let mut sessions = app_state.planner_sessions.lock().await;
    let session = sessions.entry(session_id).or_insert_with(|| PlannerSession {
        actions: Vec::new(),
        versions: Vec::new(),
        current_version: 0,
    });
    if session.versions.iter().any(|v| v.id == version) {
        session.current_version = version;
        Ok(())
    } else {
        Err(format!("Version not found: {}", version))
    }
}

#[tauri::command]
pub async fn planner_get_history(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, "planner_get_history invoked");
    let mut sessions = app_state.planner_sessions.lock().await;
    let session = sessions.entry(session_id).or_insert_with(|| PlannerSession {
        actions: Vec::new(),
        versions: Vec::new(),
        current_version: 0,
    });
    serde_json::to_value(&session.actions).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub async fn planner_get_versions(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, "planner_get_versions invoked");
    let mut sessions = app_state.planner_sessions.lock().await;
    let session = sessions.entry(session_id).or_insert_with(|| PlannerSession {
        actions: Vec::new(),
        versions: Vec::new(),
        current_version: 0,
    });
    serde_json::to_value(&session.versions).map_err(|e| format!("Serialization error: {}", e))
}

// Semantic cache commands
#[tauri::command]
pub async fn semantic_cache_stats(app_state: State<'_, AppState>) -> Result<Value, String> {
    info!("semantic_cache_stats invoked");
    let cache_state = app_state.semantic_cache.lock().await;
    let stats = cache_state.cache.stats().await?;
    serde_json::to_value(serde_json::json!({
        "enabled": cache_state.enabled,
        "stats": stats
    }))
    .map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub async fn semantic_cache_clear(app_state: State<'_, AppState>) -> Result<(), String> {
    info!("semantic_cache_clear invoked");
    // Note: Current SemanticCache doesn't have a clear method, so we just log for now
    Ok(())
}

#[tauri::command]
pub async fn semantic_cache_set_enabled(
    enabled: bool,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    info!(enabled = enabled, "semantic_cache_set_enabled invoked");
    let mut cache_state = app_state.semantic_cache.lock().await;
    cache_state.enabled = enabled;
    Ok(())
}

// Error context commands
#[tauri::command]
pub async fn error_get_report(error_json: Value) -> Result<Value, String> {
    info!("error_get_report invoked");
    // Basic error analysis
    let report = serde_json::json!({
        "original_error": error_json,
        "analysis": {
            "error_type": match error_json.get("type") {
                Some(t) => t.as_str().unwrap_or("unknown"),
                None => "unknown"
            },
            "severity": "medium",
            "suggestions": [
                "Check logs for more details",
                "Verify configuration",
                "Restart the application"
            ]
        },
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    });
    Ok(report)
}
