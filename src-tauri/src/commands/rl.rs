// SPDX-License-Identifier: AGPL-3.0-only

//! RL（强化学习）相关 Tauri 命令
//!
//! P1-1 修复：所有命令原本为 stub（返回假数据），现改为返回明确 "未实现" 错误，
//! 避免误导调用方。等 `axagent_agent::rl_optimizer` 暴露持久化能力后再接入。

use axagent_agent::rl_optimizer::{Policy, TrainingStats};
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct RLPolicyInfo {
    pub id: String,
    pub name: String,
    pub policy_type: String,
    pub total_experiences: u64,
    pub avg_reward: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RLStats {
    pub total_policies: usize,
    pub total_experiences: u64,
    pub avg_reward: f32,
    pub policies: Vec<RLPolicyInfo>,
}

fn not_implemented(op: &str) -> String {
    format!("RL 功能尚未实现: {}（等待 axagent_agent::rl_optimizer 接入持久化后启用）", op)
}

#[command]
pub fn rl_list_policies() -> Result<Vec<RLPolicyInfo>, String> {
    Err(not_implemented("rl_list_policies"))
}

#[command]
pub fn rl_get_policy(policy_id: String) -> Result<Option<Policy>, String> {
    Err(not_implemented(&format!("rl_get_policy({})", policy_id)))
}

#[command]
pub fn rl_create_policy(
    name: String,
    policy_type: String,
    model_id: String,
) -> Result<Policy, String> {
    Err(not_implemented(&format!("rl_create_policy({}, {}, {})", name, policy_type, model_id)))
}

#[command]
pub fn rl_delete_policy(policy_id: String) -> Result<(), String> {
    Err(not_implemented(&format!("rl_delete_policy({})", policy_id)))
}

#[command]
pub fn rl_get_stats() -> Result<RLStats, String> {
    Err(not_implemented("rl_get_stats"))
}

#[command]
pub fn rl_record_experience(
    task_id: String,
    task_type: String,
    tool_id: String,
    tool_name: String,
    reward: f32,
) -> Result<(), String> {
    let _ = task_id;
    let _ = task_type;
    let _ = tool_id;
    let _ = tool_name;
    let _ = reward;
    Err(not_implemented("rl_record_experience"))
}

#[command]
pub fn rl_train_policy(policy_id: String) -> Result<TrainingStats, String> {
    Err(not_implemented(&format!("rl_train_policy({})", policy_id)))
}

#[command]
pub fn rl_export_model(policy_id: String, path: String) -> Result<String, String> {
    Err(not_implemented(&format!("rl_export_model({}, {})", policy_id, path)))
}

#[command]
pub fn rl_import_model(path: String) -> Result<Policy, String> {
    Err(not_implemented(&format!("rl_import_model({})", path)))
}
