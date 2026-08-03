// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 强化学习经验持久化 DAO
//!
//! 提供 RL 经验的 SQLite 读写能力，供 IndustryLearningEngine 使用。

use std::sync::Arc;

use sea_orm::*;

use axagent_entities::{opc_rl_experience, opc_rl_training_stats};

/// RL 经验 DAO
pub struct RlExperienceDao {
    db: Arc<DatabaseConnection>,
}

impl RlExperienceDao {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 插入一条 RL 经验记录
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_experience(
        &self,
        id: &str,
        industry_id: &str,
        workflow_id: &str,
        timestamp_ms: i64,
        quality_score: f64,
        efficiency_score: f64,
        cost_score: f64,
        innovation_score: f64,
        satisfaction_score: f64,
        total_reward: f64,
        step_count: i32,
        success: bool,
        metadata: &str,
    ) -> Result<(), DbErr> {
        let active = opc_rl_experience::ActiveModel {
            id: Set(id.to_string()),
            industry_id: Set(industry_id.to_string()),
            workflow_id: Set(workflow_id.to_string()),
            timestamp_ms: Set(timestamp_ms),
            quality_score: Set(quality_score),
            efficiency_score: Set(efficiency_score),
            cost_score: Set(cost_score),
            innovation_score: Set(innovation_score),
            satisfaction_score: Set(satisfaction_score),
            total_reward: Set(total_reward),
            step_count: Set(step_count),
            success: Set(success),
            metadata: Set(metadata.to_string()),
        };
        active.insert(self.db.as_ref()).await?;
        Ok(())
    }

    /// 查询指定行业的经验池
    pub async fn get_experiences_by_industry(
        &self,
        industry_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<opc_rl_experience::Model>, DbErr> {
        let mut query = opc_rl_experience::Entity::find()
            .filter(opc_rl_experience::Column::IndustryId.eq(industry_id))
            .order_by_desc(opc_rl_experience::Column::TimestampMs);

        if let Some(lim) = limit {
            query = query.limit(lim);
        }

        query.all(self.db.as_ref()).await
    }

    /// 查询指定行业的经验数量
    pub async fn count_experiences_by_industry(&self, industry_id: &str) -> Result<u64, DbErr> {
        opc_rl_experience::Entity::find()
            .filter(opc_rl_experience::Column::IndustryId.eq(industry_id))
            .count(self.db.as_ref())
            .await
    }

    /// 获取所有行业的统计数据
    pub async fn get_global_stats(&self) -> Result<Vec<opc_rl_training_stats::Model>, DbErr> {
        opc_rl_training_stats::Entity::find().all(self.db.as_ref()).await
    }

    /// 获取指定行业的统计数据
    pub async fn get_industry_stats(
        &self,
        industry_id: &str,
    ) -> Result<Option<opc_rl_training_stats::Model>, DbErr> {
        opc_rl_training_stats::Entity::find_by_id(industry_id).one(self.db.as_ref()).await
    }

    /// 初始化或更新行业训练统计
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_training_stats(
        &self,
        industry_id: &str,
        total_experiences: i32,
        total_reward: f64,
        avg_reward: f64,
        success_rate: f64,
        last_trained_at: Option<i64>,
        policy_updated_at: Option<i64>,
        optimization_goals: &str,
    ) -> Result<(), DbErr> {
        let existing =
            opc_rl_training_stats::Entity::find_by_id(industry_id).one(self.db.as_ref()).await?;

        let mut active = match existing {
            Some(model) => model.into(),
            None => opc_rl_training_stats::ActiveModel {
                industry_id: Set(industry_id.to_string()),
                total_experiences: Set(0),
                total_reward: Set(0.0),
                avg_reward: Set(0.0),
                success_rate: Set(0.0),
                last_trained_at: Set(None),
                policy_updated_at: Set(None),
                optimization_goals: Set("[]".to_string()),
            },
        };

        active.total_experiences = Set(total_experiences);
        active.total_reward = Set(total_reward);
        active.avg_reward = Set(avg_reward);
        active.success_rate = Set(success_rate);
        active.last_trained_at = Set(last_trained_at);
        active.policy_updated_at = Set(policy_updated_at);
        active.optimization_goals = Set(optimization_goals.to_string());

        active.save(self.db.as_ref()).await?;
        Ok(())
    }

    /// 删除指定行业的所有经验记录
    pub async fn clear_experiences_by_industry(&self, industry_id: &str) -> Result<(), DbErr> {
        opc_rl_experience::Entity::delete_many()
            .filter(opc_rl_experience::Column::IndustryId.eq(industry_id))
            .exec(self.db.as_ref())
            .await?;
        Ok(())
    }
}
