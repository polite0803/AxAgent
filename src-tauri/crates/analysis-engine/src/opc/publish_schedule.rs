// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use async_trait::async_trait;

use axagent_entities::opc_publish_schedules;

use super::error::OpcResult;

/// 发布计划 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishSchedule {
    pub id: String,
    pub content_ref_type: String,
    pub content_ref_id: String,
    pub scheduled_at: i64,
    pub status: String,
    pub published_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePublishScheduleInput {
    pub content_ref_type: String,
    pub content_ref_id: String,
    pub scheduled_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePublishScheduleInput {
    pub scheduled_at: Option<i64>,
    pub status: Option<String>,
}

// ── Entity ↔ DTO 转换 ──────────────────────────────────────────

pub(crate) fn publish_schedule_entity_to_dto(e: opc_publish_schedules::Model) -> PublishSchedule {
    PublishSchedule {
        id: e.id,
        content_ref_type: e.content_ref_type,
        content_ref_id: e.content_ref_id,
        scheduled_at: e.scheduled_at,
        status: e.status,
        published_at: e.published_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

// ── PublishScheduleService trait ─────────────────────────────────────

/// 发布计划相关方法
#[async_trait]
pub trait PublishScheduleService: Send + Sync {
    async fn create_publish_schedule(
        &self,
        input: CreatePublishScheduleInput,
    ) -> OpcResult<PublishSchedule>;
    async fn get_publish_schedule(&self, id: &str) -> OpcResult<PublishSchedule>;
    async fn list_publish_schedules(&self) -> OpcResult<Vec<PublishSchedule>>;
    async fn update_publish_schedule(
        &self,
        id: &str,
        input: UpdatePublishScheduleInput,
    ) -> OpcResult<PublishSchedule>;
    async fn delete_publish_schedule(&self, id: &str) -> OpcResult<()>;

    /// 处理到期的发布计划
    async fn process_due_schedules(&self) -> OpcResult<Vec<PublishSchedule>>;
}
