// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::app_state::{
    InMemoryCacheEntry, PlannerAction, PlannerSession, PlannerVersion, PlannerVersionDiff, TotNode,
    TotSession,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tracing::info;

fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn ensure_tot_session(sessions: &mut HashMap<String, TotSession>, session_id: &str) {
    sessions.entry(session_id.to_string()).or_default();
}

fn ensure_planner_session(sessions: &mut HashMap<String, PlannerSession>, session_id: &str) {
    sessions
        .entry(session_id.to_string())
        .or_insert_with(|| PlannerSession {
            actions: Vec::new(),
            versions: Vec::new(),
            current_version: 0,
        });
}

// ---------------------------------------------------------------------------
// Tree of Thoughts commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn tot_get_state(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, "tot_get_state invoked");
    let mut sessions = app_state.tot_sessions.lock().await;
    ensure_tot_session(&mut sessions, &session_id);
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;
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
    ensure_tot_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;
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
    prompt: Option<String>,
    thoughts: Option<Vec<String>>,
    num_branches: Option<u32>,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        session_id = %session_id,
        node_id = %node_id,
        has_prompt = prompt.is_some(),
        thoughts_count = thoughts.as_ref().map(|t| t.len()).unwrap_or(0),
        "tot_explore invoked"
    );
    let mut sessions = app_state.tot_sessions.lock().await;
    ensure_tot_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let parent_depth = if let Some(n) = session.nodes.get(&node_id) {
        n.depth
    } else {
        let root_node = TotNode {
            id: node_id.clone(),
            content: prompt
                .clone()
                .unwrap_or_else(|| format!("Root node: {}", node_id)),
            score: Some(0.5),
            ..TotNode::default()
        };
        session.nodes.insert(node_id.clone(), root_node);
        if session.root_node_id.is_none() {
            session.root_node_id = Some(node_id.clone());
        }
        if session.current_node_id.is_none() {
            session.current_node_id = Some(node_id.clone());
        }
        0u32
    };

    if parent_depth >= session.max_depth {
        return Err(format!(
            "Maximum depth {} reached for session {}",
            session.max_depth, session_id
        ));
    }

    let branch_count = num_branches
        .unwrap_or(session.max_branches)
        .min(session.max_branches);

    let thought_contents = if let Some(ts) = thoughts {
        ts
    } else if let Some(ref p) = prompt {
        (0..branch_count)
            .map(|i| {
                let approach = match i {
                    0 => "Direct approach",
                    1 => "Alternative perspective",
                    2 => "Decomposed sub-problem",
                    _ => "Additional branch",
                };
                format!("{}: Exploring '{}' — {}", approach, p, generate_thought_content(p, i))
            })
            .collect()
    } else {
        (0..branch_count)
            .map(|i| format!("Branch {} exploration of node {}", i, node_id))
            .collect()
    };

    let thought_types = [
        "reasoning",
        "evaluation",
        "planning",
        "creative",
        "critical",
    ];

    let mut children = Vec::new();
    for (i, content) in thought_contents.into_iter().enumerate() {
        let child_id = format!("{}-{}", node_id, i);
        let child_node = TotNode {
            id: child_id.clone(),
            parent_id: Some(node_id.clone()),
            content,
            score: None,
            children: Vec::new(),
            depth: parent_depth + 1,
            thought_type: thought_types[i % thought_types.len()].to_string(),
            metadata: serde_json::json!({
                "branch_index": i,
                "prompt": prompt,
            }),
        };
        session.nodes.insert(child_id.clone(), child_node);
        children.push(child_id);
    }

    if let Some(parent) = session.nodes.get_mut(&node_id) {
        parent.children.extend(children.clone());
    }

    let total_nodes = session.nodes.len();

    Ok(serde_json::json!({
        "children": children,
        "parent_id": node_id,
        "depth": parent_depth + 1,
        "total_nodes": total_nodes,
        "llm_prompt": prompt.map(|p| format!(
            "Given the problem: '{}'\nGenerate {} different approaches to solve it. Each approach should explore a distinct strategy.",
            p, branch_count
        )),
    }))
}

fn generate_thought_content(prompt: &str, branch_index: u32) -> String {
    let strategies = [
        "Break the problem into smaller sub-problems and solve each independently",
        "Consider edge cases and boundary conditions that may affect the solution",
        "Apply analogical reasoning from similar solved problems",
        "Use a step-by-step deductive approach from first principles",
        "Evaluate the problem from the opposite direction (work backwards)",
    ];
    let strategy = strategies[(branch_index as usize) % strategies.len()];
    format!("Strategy: {} — Applied to: {}", strategy, prompt)
}

#[tauri::command]
pub async fn tot_score_node(
    session_id: String,
    node_id: String,
    criteria: Option<Value>,
    score: Option<f64>,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        session_id = %session_id,
        node_id = %node_id,
        has_criteria = criteria.is_some(),
        provided_score = score,
        "tot_score_node invoked"
    );
    let mut sessions = app_state.tot_sessions.lock().await;
    ensure_tot_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let node = session
        .nodes
        .get_mut(&node_id)
        .ok_or_else(|| format!("Node not found: {}", node_id))?;

    let final_score = if let Some(s) = score {
        s.clamp(0.0, 1.0)
    } else {
        compute_heuristic_score(node, criteria.as_ref())
    };

    let old_score = node.score;
    node.score = Some(final_score);

    Ok(serde_json::json!({
        "node_id": node_id,
        "previous_score": old_score,
        "new_score": final_score,
        "scoring_method": if score.is_some() { "provided" } else { "heuristic" },
    }))
}

fn compute_heuristic_score(node: &TotNode, criteria: Option<&Value>) -> f64 {
    let content_len = node.content.len() as f64;
    let depth_penalty = 0.05 * node.depth as f64;
    let has_children_bonus = if node.children.is_empty() { 0.0 } else { 0.1 };
    let content_score = (content_len / 200.0).min(1.0) * 0.4;
    let type_bonus = match node.thought_type.as_str() {
        "reasoning" => 0.2,
        "planning" => 0.25,
        "evaluation" => 0.15,
        "creative" => 0.1,
        "critical" => 0.2,
        _ => 0.1,
    };

    let criteria_bonus = if let Some(c) = criteria {
        let weight = c.get("weight").and_then(|w| w.as_f64()).unwrap_or(0.0);
        let relevance = c.get("relevance").and_then(|r| r.as_f64()).unwrap_or(0.5);
        weight * relevance
    } else {
        0.0
    };

    (content_score + type_bonus + has_children_bonus + criteria_bonus - depth_penalty)
        .clamp(0.0, 1.0)
}

#[tauri::command]
pub async fn tot_traverse(
    session_id: String,
    strategy: Option<String>,
    max_nodes: Option<usize>,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        session_id = %session_id,
        strategy = ?strategy,
        max_nodes = ?max_nodes,
        "tot_traverse invoked"
    );
    let mut sessions = app_state.tot_sessions.lock().await;
    ensure_tot_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let root_id = match &session.root_node_id {
        Some(id) => id.clone(),
        None => {
            return Ok(
                serde_json::json!({"nodes": [], "strategy": "none", "message": "No root node found"}),
            );
        },
    };

    let strat = strategy
        .as_deref()
        .unwrap_or(&session.traversal_strategy)
        .to_string();
    session.traversal_strategy = strat.clone();

    let limit = max_nodes.unwrap_or(usize::MAX);
    let visited = match strat.as_str() {
        "bfs" => traverse_bfs(&session.nodes, &root_id, limit),
        "dfs" => traverse_dfs(&session.nodes, &root_id, limit),
        "best_first" => traverse_best_first(&session.nodes, &root_id, limit),
        _ => traverse_bfs(&session.nodes, &root_id, limit),
    };

    let scored_count = visited
        .iter()
        .filter(|id| session.nodes.get(*id).and_then(|n| n.score).is_some())
        .count();

    Ok(serde_json::json!({
        "nodes": visited,
        "strategy": strat,
        "total_visited": visited.len(),
        "scored_nodes": scored_count,
    }))
}

fn traverse_bfs(nodes: &HashMap<String, TotNode>, root_id: &str, limit: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back(root_id.to_string());

    while let Some(id) = queue.pop_front() {
        if result.len() >= limit {
            break;
        }
        if let Some(node) = nodes.get(&id) {
            result.push(id.clone());
            for child in &node.children {
                queue.push_back(child.clone());
            }
        }
    }

    result
}

fn traverse_dfs(nodes: &HashMap<String, TotNode>, root_id: &str, limit: usize) -> Vec<String> {
    let mut result = Vec::new();
    dfs_recursive(nodes, root_id, &mut result, limit);
    result
}

fn dfs_recursive(
    nodes: &HashMap<String, TotNode>,
    current_id: &str,
    result: &mut Vec<String>,
    limit: usize,
) {
    if result.len() >= limit {
        return;
    }
    if let Some(node) = nodes.get(current_id) {
        result.push(current_id.to_string());
        for child in &node.children {
            dfs_recursive(nodes, child, result, limit);
        }
    }
}

fn traverse_best_first(
    nodes: &HashMap<String, TotNode>,
    root_id: &str,
    limit: usize,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut frontier: Vec<String> = vec![root_id.to_string()];

    while !frontier.is_empty() && result.len() < limit {
        frontier.sort_by(|a, b| {
            let score_a = nodes.get(a).and_then(|n| n.score).unwrap_or(0.0);
            let score_b = nodes.get(b).and_then(|n| n.score).unwrap_or(0.0);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let best_id = frontier.remove(0);
        if let Some(node) = nodes.get(&best_id) {
            result.push(best_id.clone());
            for child in &node.children {
                if !result.contains(child) {
                    frontier.push(child.clone());
                }
            }
        }
    }

    result
}

#[tauri::command]
pub async fn tot_prune(
    session_id: String,
    threshold: Option<f64>,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        session_id = %session_id,
        threshold = ?threshold,
        "tot_prune invoked"
    );
    let mut sessions = app_state.tot_sessions.lock().await;
    ensure_tot_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let prune_threshold = threshold.unwrap_or(session.pruning_threshold);
    session.pruning_threshold = prune_threshold;

    let root_id = match &session.root_node_id {
        Some(id) => id.clone(),
        None => return Ok(serde_json::json!({"pruned": 0, "remaining": 0})),
    };

    let mut to_prune = Vec::new();
    collect_prunable_nodes(&session.nodes, &root_id, prune_threshold, &mut to_prune);

    let pruned_count = to_prune.len();
    for node_id in &to_prune {
        if let Some(node) = session.nodes.remove(node_id) {
            if let Some(ref parent_id) = node.parent_id {
                if let Some(parent) = session.nodes.get_mut(parent_id) {
                    parent.children.retain(|c| c != node_id);
                }
            }
        }
    }

    let remaining = session.nodes.len();

    Ok(serde_json::json!({
        "pruned_count": pruned_count,
        "remaining_nodes": remaining,
        "threshold": prune_threshold,
        "pruned_ids": to_prune,
    }))
}

fn collect_prunable_nodes(
    nodes: &HashMap<String, TotNode>,
    current_id: &str,
    threshold: f64,
    to_prune: &mut Vec<String>,
) {
    if let Some(node) = nodes.get(current_id) {
        let children = node.children.clone();
        for child_id in &children {
            if let Some(child) = nodes.get(child_id) {
                let child_score = child.score.unwrap_or(0.0);
                if child_score < threshold && child.children.is_empty() {
                    to_prune.push(child_id.clone());
                } else {
                    collect_prunable_nodes(nodes, child_id, threshold, to_prune);
                }
            }
        }
    }
}

#[tauri::command]
pub async fn tot_get_best_path(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, "tot_get_best_path invoked");
    let mut sessions = app_state.tot_sessions.lock().await;
    ensure_tot_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let root_id = match &session.root_node_id {
        Some(id) => id.clone(),
        None => return Ok(serde_json::json!({"path": [], "scores": [], "total_score": 0.0})),
    };

    let mut path = vec![root_id.clone()];
    let mut scores = Vec::new();
    let mut current_id = root_id;

    while let Some(n) = session.nodes.get(&current_id) {
        let children = n.children.clone();

        if children.is_empty() {
            break;
        }

        let best_child = children.iter().max_by(|a, b| {
            let sa = session.nodes.get(*a).and_then(|n| n.score).unwrap_or(0.0);
            let sb = session.nodes.get(*b).and_then(|n| n.score).unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });

        match best_child {
            Some(id) => {
                let score = session.nodes.get(id).and_then(|n| n.score).unwrap_or(0.0);
                scores.push(score);
                path.push(id.clone());
                current_id = id.clone();
            },
            None => break,
        }
    }

    let total_score = if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f64>() / scores.len() as f64
    };

    Ok(serde_json::json!({
        "path": path,
        "scores": scores,
        "total_score": total_score,
    }))
}

// ---------------------------------------------------------------------------
// Replanning commands
// ---------------------------------------------------------------------------

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
    ensure_planner_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let previous_action_snapshot: Vec<PlannerAction> = session.actions.clone();

    for action_data in actions {
        let action = PlannerAction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now_epoch_secs(),
            action_type: "replan_action".to_string(),
            data: action_data,
        };
        session.actions.push(action);
    }

    let new_version_id = session.current_version + 1;

    let diff = compute_version_diff(
        session.current_version,
        new_version_id,
        &previous_action_snapshot,
        &session.actions,
    );

    let version = PlannerVersion {
        id: new_version_id,
        timestamp: now_epoch_secs(),
        reason,
        state: serde_json::json!({
            "session_id": session_id,
            "version": new_version_id,
            "action_count": session.actions.len(),
        }),
        action_snapshot: session.actions.clone(),
        diff_from_previous: Some(diff),
    };
    session.versions.push(version);
    session.current_version = new_version_id;

    serde_json::to_value(session).map_err(|e| format!("Serialization error: {}", e))
}

fn compute_version_diff(
    from_version: u32,
    to_version: u32,
    previous: &[PlannerAction],
    current: &[PlannerAction],
) -> PlannerVersionDiff {
    let previous_ids: std::collections::HashSet<String> =
        previous.iter().map(|a| a.id.clone()).collect();
    let current_ids: std::collections::HashSet<String> =
        current.iter().map(|a| a.id.clone()).collect();

    let actions_added: Vec<PlannerAction> = current
        .iter()
        .filter(|a| !previous_ids.contains(&a.id))
        .cloned()
        .collect();

    let actions_removed: Vec<String> = previous
        .iter()
        .filter(|a| !current_ids.contains(&a.id))
        .map(|a| a.id.clone())
        .collect();

    let summary = format!(
        "v{} → v{}: +{} actions, -{} actions",
        from_version,
        to_version,
        actions_added.len(),
        actions_removed.len()
    );

    PlannerVersionDiff {
        from_version,
        to_version,
        actions_added,
        actions_removed,
        summary,
    }
}

#[tauri::command]
pub async fn planner_rollback(
    session_id: String,
    version: u32,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, version = version, "planner_rollback invoked");
    let mut sessions = app_state.planner_sessions.lock().await;
    ensure_planner_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let target_version = session
        .versions
        .iter()
        .find(|v| v.id == version)
        .ok_or_else(|| format!("Version not found: {}", version))?;

    let restored_actions = target_version.action_snapshot.clone();
    let restored_count = restored_actions.len();

    session.actions = restored_actions;
    session.current_version = version;

    Ok(serde_json::json!({
        "rolled_back_to": version,
        "actions_restored": restored_count,
        "current_version": version,
    }))
}

#[tauri::command]
pub async fn planner_diff_versions(
    session_id: String,
    from_version: u32,
    to_version: u32,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        session_id = %session_id,
        from_version = from_version,
        to_version = to_version,
        "planner_diff_versions invoked"
    );
    let mut sessions = app_state.planner_sessions.lock().await;
    ensure_planner_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let from_snap = session
        .versions
        .iter()
        .find(|v| v.id == from_version)
        .map(|v| v.action_snapshot.clone())
        .ok_or_else(|| format!("Version {} not found", from_version))?;

    let to_snap = session
        .versions
        .iter()
        .find(|v| v.id == to_version)
        .map(|v| v.action_snapshot.clone())
        .ok_or_else(|| format!("Version {} not found", to_version))?;

    let diff = compute_version_diff(from_version, to_version, &from_snap, &to_snap);

    Ok(serde_json::json!({
        "from_version": from_version,
        "to_version": to_version,
        "from_action_count": from_snap.len(),
        "to_action_count": to_snap.len(),
        "actions_added": diff.actions_added,
        "actions_removed": diff.actions_removed,
        "summary": diff.summary,
    }))
}

#[tauri::command]
pub async fn planner_get_history(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, "planner_get_history invoked");
    let mut sessions = app_state.planner_sessions.lock().await;
    ensure_planner_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;
    serde_json::to_value(&session.actions).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub async fn planner_get_versions(
    session_id: String,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(session_id = %session_id, "planner_get_versions invoked");
    let mut sessions = app_state.planner_sessions.lock().await;
    ensure_planner_session(&mut sessions, &session_id);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;
    serde_json::to_value(&session.versions).map_err(|e| format!("Serialization error: {}", e))
}

// ---------------------------------------------------------------------------
// Semantic Cache commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn semantic_cache_stats(app_state: State<'_, AppState>) -> Result<Value, String> {
    info!("semantic_cache_stats invoked");
    let cache_state = app_state.semantic_cache.lock().await;
    let db_stats = cache_state.cache.stats().await?;
    serde_json::to_value(serde_json::json!({
        "enabled": cache_state.enabled,
        "similarity_threshold": cache_state.similarity_threshold,
        "in_memory_entries": cache_state.in_memory_entries.len(),
        "db_stats": db_stats,
    }))
    .map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub async fn semantic_cache_clear(app_state: State<'_, AppState>) -> Result<(), String> {
    info!("semantic_cache_clear invoked");
    let mut cache_state = app_state.semantic_cache.lock().await;
    cache_state.in_memory_entries.clear();
    info!("Semantic cache in-memory entries cleared");
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

#[tauri::command]
pub async fn semantic_cache_lookup(
    query_embedding: Vec<f32>,
    model_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        embedding_dim = query_embedding.len(),
        model_id = ?model_id,
        "semantic_cache_lookup invoked"
    );
    let mut cache_state = app_state.semantic_cache.lock().await;

    if !cache_state.enabled {
        return Ok(serde_json::json!({"hit": false, "reason": "cache_disabled"}));
    }

    let now = now_epoch_secs();
    let threshold = cache_state.similarity_threshold;

    cache_state
        .in_memory_entries
        .retain(|entry| (now - entry.created_at) as u64 <= entry.ttl_secs);

    let mut best_entry: Option<&mut InMemoryCacheEntry> = None;
    let mut best_similarity = 0.0f32;

    for entry in &mut cache_state.in_memory_entries {
        let is_expired = (now - entry.created_at) as u64 > entry.ttl_secs;
        if is_expired {
            continue;
        }
        if let Some(ref mid) = model_id {
            if entry.model_id.as_ref() != Some(mid) {
                continue;
            }
        }

        let sim = cosine_similarity(&query_embedding, &entry.query_embedding);
        if sim >= threshold && sim > best_similarity {
            best_similarity = sim;
            best_entry = Some(entry);
        }
    }

    match best_entry {
        Some(entry) => {
            entry.access_count += 1;
            Ok(serde_json::json!({
                "hit": true,
                "query_hash": entry.query_hash,
                "query_text": entry.query_text,
                "response": entry.response,
                "similarity": best_similarity,
                "access_count": entry.access_count,
            }))
        },
        None => Ok(serde_json::json!({
            "hit": false,
            "best_similarity": best_similarity,
            "threshold": threshold,
        })),
    }
}

#[tauri::command]
pub async fn semantic_cache_store(
    query_text: String,
    query_embedding: Vec<f32>,
    response: String,
    model_id: Option<String>,
    ttl_secs: Option<u64>,
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    info!(
        query_len = query_text.len(),
        embedding_dim = query_embedding.len(),
        "semantic_cache_store invoked"
    );
    let mut cache_state = app_state.semantic_cache.lock().await;

    if !cache_state.enabled {
        return Ok(serde_json::json!({"stored": false, "reason": "cache_disabled"}));
    }

    let query_hash = hash_embedding(&query_embedding);
    let ttl = ttl_secs.unwrap_or(3600);

    let entry = InMemoryCacheEntry {
        query_hash: query_hash.clone(),
        query_text,
        query_embedding,
        response,
        model_id,
        created_at: now_epoch_secs(),
        access_count: 0,
        ttl_secs: ttl,
    };

    cache_state.in_memory_entries.push(entry);

    let total = cache_state.in_memory_entries.len();

    Ok(serde_json::json!({
        "stored": true,
        "query_hash": query_hash,
        "total_entries": total,
    }))
}

#[tauri::command]
pub async fn semantic_cache_set_threshold(
    threshold: f32,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    info!(threshold = threshold, "semantic_cache_set_threshold invoked");
    let mut cache_state = app_state.semantic_cache.lock().await;
    cache_state.similarity_threshold = threshold.clamp(0.0, 1.0);
    Ok(())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (va, vb) in a.iter().zip(b.iter()) {
        dot += va * vb;
        norm_a += va * va;
        norm_b += vb * vb;
    }
    let magnitude = norm_a.sqrt() * norm_b.sqrt();
    if magnitude < 1e-10 {
        return 0.0;
    }
    dot / magnitude
}

fn hash_embedding(embedding: &[f32]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for value in embedding {
        hasher.update(value.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Error Context commands
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum ErrorCategory {
    Network,
    Auth,
    Timeout,
    RateLimit,
    Validation,
    NotFound,
    Permission,
    Configuration,
    Internal,
    Unknown,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::Network => write!(f, "network"),
            ErrorCategory::Auth => write!(f, "auth"),
            ErrorCategory::Timeout => write!(f, "timeout"),
            ErrorCategory::RateLimit => write!(f, "rate_limit"),
            ErrorCategory::Validation => write!(f, "validation"),
            ErrorCategory::NotFound => write!(f, "not_found"),
            ErrorCategory::Permission => write!(f, "permission"),
            ErrorCategory::Configuration => write!(f, "configuration"),
            ErrorCategory::Internal => write!(f, "internal"),
            ErrorCategory::Unknown => write!(f, "unknown"),
        }
    }
}

fn categorize_error(error_json: &Value) -> ErrorCategory {
    let error_type = error_json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_lowercase();

    let error_code = error_json
        .get("code")
        .or_else(|| error_json.get("status"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);

    let error_message = error_json
        .get("message")
        .or_else(|| error_json.get("error"))
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_lowercase();

    if error_type.contains("network")
        || error_type.contains("connection")
        || error_type.contains("dns")
        || error_message.contains("connection refused")
        || error_message.contains("network unreachable")
        || error_message.contains("econnrefused")
        || error_message.contains("econnreset")
        || error_message.contains("enetunreach")
    {
        return ErrorCategory::Network;
    }

    if error_type.contains("auth")
        || error_type.contains("unauthorized")
        || error_type.contains("forbidden")
        || error_code == 401
        || error_code == 403
        || error_message.contains("invalid api key")
        || error_message.contains("authentication failed")
        || error_message.contains("unauthorized")
        || error_message.contains("invalid token")
    {
        return ErrorCategory::Auth;
    }

    if error_type.contains("timeout")
        || error_type.contains("timed out")
        || error_message.contains("timeout")
        || error_message.contains("timed out")
        || error_message.contains("deadline exceeded")
    {
        return ErrorCategory::Timeout;
    }

    if error_type.contains("rate_limit")
        || error_type.contains("rate limit")
        || error_type.contains("throttl")
        || error_code == 429
        || error_message.contains("rate limit")
        || error_message.contains("too many requests")
        || error_message.contains("throttl")
    {
        return ErrorCategory::RateLimit;
    }

    if error_type.contains("validat")
        || error_code == 400
        || error_code == 422
        || error_message.contains("invalid")
        || error_message.contains("validation")
        || error_message.contains("malformed")
    {
        return ErrorCategory::Validation;
    }

    if error_type.contains("not_found")
        || error_code == 404
        || error_message.contains("not found")
        || error_message.contains("does not exist")
    {
        return ErrorCategory::NotFound;
    }

    if error_type.contains("permission")
        || error_type.contains("access denied")
        || error_message.contains("permission denied")
        || error_message.contains("access denied")
        || error_message.contains("insufficient permissions")
    {
        return ErrorCategory::Permission;
    }

    if error_type.contains("config")
        || error_message.contains("configuration")
        || error_message.contains("misconfigured")
        || error_message.contains("missing config")
    {
        return ErrorCategory::Configuration;
    }

    if error_type.contains("internal")
        || error_code >= 500
        || error_message.contains("internal server error")
        || error_message.contains("unexpected error")
    {
        return ErrorCategory::Internal;
    }

    ErrorCategory::Unknown
}

fn get_severity(category: &ErrorCategory, error_json: &Value) -> &'static str {
    let error_code = error_json
        .get("code")
        .or_else(|| error_json.get("status"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);

    match category {
        ErrorCategory::Auth => "high",
        ErrorCategory::Network => "high",
        ErrorCategory::RateLimit => "low",
        ErrorCategory::Timeout => "medium",
        ErrorCategory::Validation => "low",
        ErrorCategory::NotFound => "low",
        ErrorCategory::Permission => "high",
        ErrorCategory::Configuration => "high",
        ErrorCategory::Internal => "critical",
        ErrorCategory::Unknown => {
            if error_code >= 500 {
                "critical"
            } else {
                "medium"
            }
        },
    }
}

fn get_remediation(category: &ErrorCategory) -> Vec<&'static str> {
    match category {
        ErrorCategory::Network => vec![
            "Check your internet connection and DNS settings",
            "Verify the target server is reachable (try ping or curl)",
            "Check firewall rules and proxy configuration",
            "Retry with exponential backoff after a brief wait",
            "If using a VPN, verify it is connected properly",
        ],
        ErrorCategory::Auth => vec![
            "Verify your API key or credentials are correct",
            "Check if the API key has expired or been revoked",
            "Ensure the credentials have the required permissions/scopes",
            "Re-authenticate and obtain a fresh token",
            "Check if the account is locked or suspended",
        ],
        ErrorCategory::Timeout => vec![
            "Increase the request timeout setting",
            "Reduce the payload size or simplify the request",
            "Check server load — it may be under heavy load",
            "Retry the request after a brief delay",
            "Consider splitting the operation into smaller chunks",
        ],
        ErrorCategory::RateLimit => vec![
            "Implement exponential backoff with jitter in your retry logic",
            "Reduce the request frequency to stay within rate limits",
            "Check the Retry-After header for the recommended wait time",
            "Consider upgrading your API plan for higher rate limits",
            "Batch multiple operations into fewer requests",
        ],
        ErrorCategory::Validation => vec![
            "Review the request payload against the API schema",
            "Check for missing or malformed required fields",
            "Ensure data types match the expected format",
            "Validate enum values are within the allowed set",
            "Trim whitespace and normalize input strings",
        ],
        ErrorCategory::NotFound => vec![
            "Verify the resource ID or path is correct",
            "Check if the resource has been deleted or moved",
            "Ensure you are targeting the correct environment/region",
            "List available resources to confirm the target exists",
        ],
        ErrorCategory::Permission => vec![
            "Verify your account has the required role or permissions",
            "Contact an administrator to request access",
            "Check if the resource has access control restrictions",
            "Ensure you are operating within the correct scope/organization",
        ],
        ErrorCategory::Configuration => vec![
            "Review the configuration file for missing or incorrect values",
            "Check environment variables are set correctly",
            "Verify all required configuration keys are present",
            "Compare with a known-working configuration template",
            "Check for typos in configuration key names",
        ],
        ErrorCategory::Internal => vec![
            "This is a server-side error — retry after a brief wait",
            "Check the server logs for more details",
            "Report this issue to the service provider if it persists",
            "Verify you are using a compatible API version",
            "Check the service status page for ongoing incidents",
        ],
        ErrorCategory::Unknown => vec![
            "Check logs for more detailed error information",
            "Verify your request matches the API documentation",
            "Try reproducing the error with minimal parameters",
            "Contact support if the error persists",
        ],
    }
}

#[tauri::command]
pub async fn error_get_report(error_json: Value) -> Result<Value, String> {
    info!("error_get_report invoked");
    let category = categorize_error(&error_json);
    let severity = get_severity(&category, &error_json);
    let remediation = get_remediation(&category);

    let error_code = error_json
        .get("code")
        .or_else(|| error_json.get("status"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);

    let error_message = error_json
        .get("message")
        .or_else(|| error_json.get("error"))
        .and_then(|m| m.as_str())
        .unwrap_or("No message provided");

    let is_retryable = matches!(
        category,
        ErrorCategory::Network
            | ErrorCategory::Timeout
            | ErrorCategory::RateLimit
            | ErrorCategory::Internal
    );

    let retry_after = match &category {
        ErrorCategory::RateLimit => error_json
            .get("retry_after")
            .or_else(|| error_json.get("retryAfter"))
            .and_then(|v| v.as_i64())
            .or(if error_code == 429 { Some(60) } else { None }),
        ErrorCategory::Timeout => Some(5),
        ErrorCategory::Network => Some(10),
        _ => None,
    };

    Ok(serde_json::json!({
        "original_error": error_json,
        "analysis": {
            "error_type": category.to_string(),
            "severity": severity,
            "error_code": error_code,
            "error_message": error_message,
            "is_retryable": is_retryable,
            "retry_after_secs": retry_after,
            "remediation": remediation,
        },
        "timestamp": now_epoch_secs(),
    }))
}

// ---------------------------------------------------------------------------
// Prompt Cache commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_prompt_cache_state(app_state: State<'_, AppState>) -> Result<Value, String> {
    let state = app_state.prompt_cache.get_state().await;
    let pending = app_state.prompt_cache.has_pending_changes().await;
    serde_json::to_value(serde_json::json!({
        "cacheValid": state.cache_valid,
        "hasPendingChanges": pending,
        "tokensSaved": state.tokens_saved_estimate,
        "cacheHits": state.cache_hits,
    }))
    .map_err(|e| format!("Serialization error: {}", e))
}
