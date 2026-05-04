use sea_orm::*;

use crate::entity::rl_policies;
use crate::error::{AxAgentError, Result};
use crate::utils::gen_id;

// ---------------------------------------------------------------------------
// Domain types (mirror axagent_agent::rl_optimizer for serialization)
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlPolicyInfo {
    pub id: String,
    pub name: String,
    pub policy_type: String,
    pub model_id: String,
    pub total_experiences: u64,
    pub avg_reward: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlStats {
    pub total_policies: usize,
    pub total_experiences: u64,
    pub avg_reward: f32,
    pub policies: Vec<RlPolicyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlPolicyFull {
    pub id: String,
    pub name: String,
    pub policy_type: String,
    pub model_id: String,
    pub reward_signals: Vec<serde_json::Value>,
    pub experiences: Vec<serde_json::Value>,
    pub training_stats: RlTrainingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlTrainingStats {
    pub total_experiences: u64,
    pub episodes_completed: u64,
    pub avg_reward: f32,
    pub last_update: String,
}

// ---------------------------------------------------------------------------
// Mappings
// ---------------------------------------------------------------------------

fn model_to_policy_info(m: &rl_policies::Model) -> RlPolicyInfo {
    RlPolicyInfo {
        id: m.id.clone(),
        name: m.name.clone(),
        policy_type: m.policy_type.clone(),
        model_id: m.model_id.clone(),
        total_experiences: m.total_experiences as u64,
        avg_reward: m.avg_reward,
    }
}

fn model_to_policy_full(m: rl_policies::Model) -> RlPolicyFull {
    let reward_signals: Vec<serde_json::Value> =
        serde_json::from_str(&m.reward_signals_json).unwrap_or_default();
    let experiences: Vec<serde_json::Value> =
        serde_json::from_str(&m.experiences_json).unwrap_or_default();

    RlPolicyFull {
        id: m.id,
        name: m.name,
        policy_type: m.policy_type,
        model_id: m.model_id,
        reward_signals,
        experiences,
        training_stats: RlTrainingStats {
            total_experiences: m.total_experiences as u64,
            episodes_completed: m.episodes_completed as u64,
            avg_reward: m.avg_reward,
            last_update: m.last_update,
        },
    }
}

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

pub async fn list_rl_policies(db: &DatabaseConnection) -> Result<Vec<RlPolicyInfo>> {
    let rows = rl_policies::Entity::find()
        .order_by_desc(rl_policies::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows.iter().map(model_to_policy_info).collect())
}

pub async fn get_rl_policy(db: &DatabaseConnection, id: &str) -> Result<Option<RlPolicyFull>> {
    let row = rl_policies::Entity::find_by_id(id).one(db).await?;
    Ok(row.map(model_to_policy_full))
}

pub async fn create_rl_policy(
    db: &DatabaseConnection,
    name: &str,
    policy_type: &str,
    model_id: &str,
) -> Result<RlPolicyFull> {
    let id = gen_id();
    let now = chrono::Utc::now().to_rfc3339();

    rl_policies::Entity::insert(rl_policies::ActiveModel {
        id: Set(id.clone()),
        name: Set(name.to_string()),
        policy_type: Set(policy_type.to_string()),
        model_id: Set(model_id.to_string()),
        reward_signals_json: Set("[]".to_string()),
        experiences_json: Set("[]".to_string()),
        total_experiences: Set(0),
        episodes_completed: Set(0),
        avg_reward: Set(0.0),
        last_update: Set(now.clone()),
        created_at: Set(now),
    })
    .exec(db)
    .await?;

    get_rl_policy(db, &id)
        .await
        .map(|opt| opt.expect("just inserted"))
}

pub async fn delete_rl_policy(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = rl_policies::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("RL Policy {}", id)));
    }
    Ok(())
}

pub async fn get_rl_stats(db: &DatabaseConnection) -> Result<RlStats> {
    let rows = rl_policies::Entity::find().all(db).await?;
    let total_experiences: u64 = rows.iter().map(|r| r.total_experiences as u64).sum();
    let total_policies = rows.len();
    let avg_reward = if total_policies > 0 {
        rows.iter().map(|r| r.avg_reward).sum::<f32>() / total_policies as f32
    } else {
        0.0
    };
    let policies: Vec<RlPolicyInfo> = rows.iter().map(model_to_policy_info).collect();

    Ok(RlStats {
        total_policies,
        total_experiences,
        avg_reward,
        policies,
    })
}

// ---------------------------------------------------------------------------
// Experience recording
// ---------------------------------------------------------------------------

pub async fn record_rl_experience(
    db: &DatabaseConnection,
    policy_id: &str,
    experience_json: &str,
    reward: f32,
) -> Result<()> {
    let row = match rl_policies::Entity::find_by_id(policy_id).one(db).await? {
        Some(r) => r,
        None => return Err(AxAgentError::NotFound(format!("RL Policy {}", policy_id))),
    };

    let mut experiences: Vec<serde_json::Value> =
        serde_json::from_str(&row.experiences_json).unwrap_or_default();
    let exp: serde_json::Value =
        serde_json::from_str(experience_json).unwrap_or(serde_json::Value::Null);
    experiences.push(exp);
    if experiences.len() > 1000 {
        experiences.drain(0..experiences.len() - 1000);
    }

    let new_total = row.total_experiences + 1;
    let new_avg = if new_total > 0 {
        (row.avg_reward * (new_total - 1) as f32 + reward) / new_total as f32
    } else {
        reward
    };
    let now = chrono::Utc::now().to_rfc3339();

    rl_policies::Entity::update(rl_policies::ActiveModel {
        id: Set(policy_id.to_string()),
        name: Set(row.name),
        policy_type: Set(row.policy_type),
        model_id: Set(row.model_id),
        reward_signals_json: Set(row.reward_signals_json),
        experiences_json: Set(serde_json::to_string(&experiences).unwrap_or_else(|_| "[]".to_string())),
        total_experiences: Set(new_total),
        episodes_completed: Set(row.episodes_completed),
        avg_reward: Set(new_avg),
        last_update: Set(now),
        created_at: Set(row.created_at),
    })
    .exec(db)
    .await?;

    Ok(())
}

pub async fn update_policy_training_stats(
    db: &DatabaseConnection,
    policy_id: &str,
    stats: &RlTrainingStats,
) -> Result<()> {
    let row = match rl_policies::Entity::find_by_id(policy_id).one(db).await? {
        Some(r) => r,
        None => return Err(AxAgentError::NotFound(format!("RL Policy {}", policy_id))),
    };

    let now = chrono::Utc::now().to_rfc3339();
    rl_policies::Entity::update(rl_policies::ActiveModel {
        id: Set(policy_id.to_string()),
        name: Set(row.name),
        policy_type: Set(row.policy_type),
        model_id: Set(row.model_id),
        reward_signals_json: Set(row.reward_signals_json),
        experiences_json: Set(row.experiences_json),
        total_experiences: Set(stats.total_experiences as i32),
        episodes_completed: Set(stats.episodes_completed as i32),
        avg_reward: Set(stats.avg_reward),
        last_update: Set(now),
        created_at: Set(row.created_at),
    })
    .exec(db)
    .await?;

    Ok(())
}
