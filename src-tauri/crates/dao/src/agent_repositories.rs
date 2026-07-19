// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 系列 repository 的 dao 实现 + 全局注册。
//!
//! harness 的 repository 注册约定是"owner crate 在自身初始化时注册"；
//! dao 拥有的 agent 系列（profile / expert / role）与 workflow 系列此前未注册，
//! 导致 consumer crate 调用 `xxx_repository()` 访问器时 panic。
//! `register_repositories` 在 DB 初始化后统一注册这些 repo。

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::*;

use axagent_entities::{agency_experts, agent_profiles, agent_roles};
use axagent_harness::repo_dtos::{AgencyExpertDto, AgentRoleDto};
use axagent_harness::repositories::{
    AgencyExpertRepository, AgentProfileRepository, AgentRoleRepository,
};
use axagent_harness::types::AgentProfile;

pub struct DaoAgentProfileRepository {
    pub db: DatabaseConnection,
}

pub struct DaoAgencyExpertRepository {
    pub db: DatabaseConnection,
}

pub struct DaoAgentRoleRepository {
    pub db: DatabaseConnection,
}

fn parse_json_arr(raw: &Option<String>) -> Vec<String> {
    raw.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default()
}

#[async_trait]
impl AgentProfileRepository for DaoAgentProfileRepository {
    async fn get_agent_profile(&self, id: &str) -> Result<Option<AgentProfile>, String> {
        let row = agent_profiles::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|m| AgentProfile {
            id: m.id,
            name: m.name,
            description: m.description,
            category: m.category,
            icon: m.icon,
            agent_role: m.agent_role,
            source: m.source,
            tags: parse_json_arr(&m.tags),
            suggested_provider_id: m.suggested_provider_id,
            suggested_model_id: m.suggested_model_id,
            suggested_temperature: m.suggested_temperature,
            suggested_max_tokens: m.suggested_max_tokens.map(|v| v as u32),
            search_enabled: m.search_enabled.map(|v| v != 0),
            recommend_permission_mode: m.recommend_permission_mode,
            recommended_tools: parse_json_arr(&m.recommended_tools),
            disallowed_tools: parse_json_arr(&m.disallowed_tools),
            recommended_workflows: parse_json_arr(&m.recommended_workflows),
            sort_order: m.sort_order,
            is_enabled: m.is_enabled != 0,
            expert_id: m.expert_id,
            business_role_id: m.business_role_id,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }))
    }
}

/// 把 agency_experts::Model 转换为 AgencyExpertDto（含 v101 新增的 6 个字段）。
fn expert_from_model(m: agency_experts::Model) -> AgencyExpertDto {
    AgencyExpertDto {
        id: m.id,
        name: m.name,
        description: m.description,
        category: m.category,
        system_prompt: m.system_prompt,
        color: m.color,
        source_dir: m.source_dir,
        is_enabled: m.is_enabled != 0,
        imported_at: m.imported_at,
        recommended_workflows: m.recommended_workflows,
        recommended_tools: m.recommended_tools,
        active_domains: m.active_domains,
        seniority: m.seniority,
        specialties: m.specialties,
        parent_role_id: m.parent_role_id,
        success_rate: m.success_rate,
        avg_latency_ms: m.avg_latency_ms,
        avg_token_cost: m.avg_token_cost,
    }
}

#[async_trait]
impl AgencyExpertRepository for DaoAgencyExpertRepository {
    async fn get_agency_expert(&self, id: &str) -> Result<Option<AgencyExpertDto>, String> {
        let row = agency_experts::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(expert_from_model))
    }

    async fn list_agency_experts(&self) -> Result<Vec<AgencyExpertDto>, String> {
        // 仅返回 is_enabled=true 的记录，按 name 排序
        let rows = agency_experts::Entity::find()
            .filter(agency_experts::Column::IsEnabled.eq(1))
            .order_by_asc(agency_experts::Column::Name)
            .all(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(expert_from_model).collect())
    }
}

#[async_trait]
impl AgentRoleRepository for DaoAgentRoleRepository {
    async fn get_agent_role(&self, id: &str) -> Result<Option<AgentRoleDto>, String> {
        let row =
            agent_roles::Entity::find_by_id(id).one(&self.db).await.map_err(|e| e.to_string())?;
        Ok(row.map(|m| {
            let tools: Vec<String> = parse_json_arr(&m.default_tools);
            let domains: Vec<String> = parse_json_arr(&m.active_domains);
            AgentRoleDto {
                id: m.id,
                name: m.name,
                description: m.description,
                system_prompt: m.system_prompt,
                default_tools: tools,
                active_domains: domains,
                max_concurrent: m.max_concurrent,
                timeout_seconds: m.timeout_seconds,
                source: m.source,
            }
        }))
    }
}

/// 将 dao 实现的 repository 注册进 harness 全局服务注册表。
///
/// 在 DB 初始化后调用（见 `src/init/database.rs`），统一注册：
/// - agent 系列：agent_profile / agency_expert / agent_role
/// - workflow 系列：workflow_execution / loop_checkpoint / workflow_template
/// - settings/tools 系列：settings
pub fn register_repositories(db: &DatabaseConnection) {
    axagent_harness::repositories::set_agent_profile_repository(Arc::new(
        DaoAgentProfileRepository { db: db.clone() },
    ));
    axagent_harness::repositories::set_agency_expert_repository(Arc::new(
        DaoAgencyExpertRepository { db: db.clone() },
    ));
    axagent_harness::repositories::set_agent_role_repository(Arc::new(DaoAgentRoleRepository {
        db: db.clone(),
    }));
    axagent_harness::repositories::set_workflow_execution_repository(Arc::new(
        crate::workflow_execution_repository::DaoWorkflowExecutionRepository {
            db: Arc::new(db.clone()),
        },
    ));
    axagent_harness::repositories::set_loop_checkpoint_repository(Arc::new(
        crate::loop_checkpoint_repository::DaoLoopCheckpointRepository { db: Arc::new(db.clone()) },
    ));
    axagent_harness::repositories::set_workflow_template_repository(Arc::new(
        crate::workflow_template_repository::DaoWorkflowTemplateRepository {
            db: Arc::new(db.clone()),
        },
    ));
    // wiki / note 域 repository（dao 实现，wiki_dtos 为权威 trait）
    axagent_harness::repositories::set_note_repository(Arc::new(
        crate::repo::note_repository::DaoNoteRepository::new(Arc::new(db.clone())),
    ));
    axagent_harness::repositories::set_wiki_repository(Arc::new(
        crate::repo::wiki_repository::DaoWikiRepository::new(Arc::new(db.clone())),
    ));
    axagent_harness::repositories::set_wiki_page_repository(Arc::new(
        crate::repo::wiki_page_repository::DaoWikiPageRepository::new(Arc::new(db.clone())),
    ));
    axagent_harness::repositories::set_wiki_source_repository(Arc::new(
        crate::repo::wiki_source_repository::DaoWikiSourceRepository::new(Arc::new(db.clone())),
    ));
    axagent_harness::repositories::set_wiki_operation_repository(Arc::new(
        crate::repo::wiki_operation_repository::DaoWikiOperationRepository::new(Arc::new(
            db.clone(),
        )),
    ));
    axagent_harness::repositories::set_note_backlink_repository(Arc::new(
        crate::repo::note_backlink_repository::DaoNoteBacklinkRepository::new(Arc::new(db.clone())),
    ));
    axagent_harness::repositories::set_settings_repository(Arc::new(
        crate::settings_repository::DaoSettingsRepository::new(db.clone()),
    ));
    axagent_harness::repositories::set_provider_repository(Arc::new(
        crate::provider_repository::DaoProviderRepository::new(db.clone()),
    ));
}
