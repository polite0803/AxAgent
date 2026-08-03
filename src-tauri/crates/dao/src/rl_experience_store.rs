// SPDX-License-Identifier: AGPL-3.0-only

//! RlExperienceStore trait 的 DAO 实现

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::*;

use axagent_harness::rl::{RlExperienceRecord, RlExperienceStore, RlIndustryStats};

use crate::repo::rl_experience::RlExperienceDao;

/// RlExperienceStore 的 SQLite 实现
pub struct RlExperienceStoreImpl {
    dao: RlExperienceDao,
}

impl RlExperienceStoreImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { dao: RlExperienceDao::new(db) }
    }
}

#[async_trait]
impl RlExperienceStore for RlExperienceStoreImpl {
    async fn save_experience(&self, record: &RlExperienceRecord) -> Result<(), String> {
        self.dao
            .insert_experience(
                &record.id,
                &record.industry_id,
                &record.workflow_id,
                record.timestamp_ms,
                record.quality_score,
                record.efficiency_score,
                record.cost_score,
                record.innovation_score,
                record.satisfaction_score,
                record.total_reward,
                record.step_count,
                record.success,
                &record.metadata,
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_experiences(
        &self,
        industry_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<RlExperienceRecord>, String> {
        let models = self
            .dao
            .get_experiences_by_industry(industry_id, limit)
            .await
            .map_err(|e| e.to_string())?;

        Ok(models
            .into_iter()
            .map(|m| RlExperienceRecord {
                id: m.id,
                industry_id: m.industry_id,
                workflow_id: m.workflow_id,
                timestamp_ms: m.timestamp_ms,
                quality_score: m.quality_score,
                efficiency_score: m.efficiency_score,
                cost_score: m.cost_score,
                innovation_score: m.innovation_score,
                satisfaction_score: m.satisfaction_score,
                total_reward: m.total_reward,
                step_count: m.step_count,
                success: m.success,
                metadata: m.metadata,
            })
            .collect())
    }

    async fn count_experiences(&self, industry_id: &str) -> Result<u64, String> {
        self.dao.count_experiences_by_industry(industry_id).await.map_err(|e| e.to_string())
    }

    async fn get_global_stats(&self) -> Result<Vec<RlIndustryStats>, String> {
        let models = self.dao.get_global_stats().await.map_err(|e| e.to_string())?;

        Ok(models
            .into_iter()
            .map(|m| {
                let goals: Vec<String> =
                    serde_json::from_str(&m.optimization_goals).unwrap_or_default();
                RlIndustryStats {
                    industry_id: m.industry_id,
                    total_experiences: m.total_experiences,
                    total_reward: m.total_reward,
                    avg_reward: m.avg_reward,
                    success_rate: m.success_rate,
                    last_trained_at: m.last_trained_at,
                    policy_updated_at: m.policy_updated_at,
                    optimization_goals: goals,
                }
            })
            .collect())
    }

    async fn get_industry_stats(
        &self,
        industry_id: &str,
    ) -> Result<Option<RlIndustryStats>, String> {
        let model = self.dao.get_industry_stats(industry_id).await.map_err(|e| e.to_string())?;

        Ok(model.map(|m| {
            let goals: Vec<String> =
                serde_json::from_str(&m.optimization_goals).unwrap_or_default();
            RlIndustryStats {
                industry_id: m.industry_id,
                total_experiences: m.total_experiences,
                total_reward: m.total_reward,
                avg_reward: m.avg_reward,
                success_rate: m.success_rate,
                last_trained_at: m.last_trained_at,
                policy_updated_at: m.policy_updated_at,
                optimization_goals: goals,
            }
        }))
    }

    async fn upsert_stats(&self, stats: &RlIndustryStats) -> Result<(), String> {
        let goals_str =
            serde_json::to_string(&stats.optimization_goals).unwrap_or_else(|_| "[]".to_string());

        self.dao
            .upsert_training_stats(
                &stats.industry_id,
                stats.total_experiences,
                stats.total_reward,
                stats.avg_reward,
                stats.success_rate,
                stats.last_trained_at,
                stats.policy_updated_at,
                &goals_str,
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn clear_experiences(&self, industry_id: &str) -> Result<(), String> {
        self.dao.clear_experiences_by_industry(industry_id).await.map_err(|e| e.to_string())
    }
}
