// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::sea_query::Expr;
use sea_orm::*;

use axagent_entities::index_jobs;
pub use axagent_entities::index_jobs::{Column, Entity};
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::util_fns::gen_id;

pub const INDEX_JOB_STATUS_PENDING: &str = "pending";
pub const INDEX_JOB_STATUS_PROCESSING: &str = "processing";
pub const INDEX_JOB_STATUS_COMPLETED: &str = "completed";
pub const INDEX_JOB_STATUS_FAILED: &str = "failed";
pub const INDEX_JOB_STATUS_RETRYING: &str = "retrying";
pub const INDEX_JOB_STATUS_CANCELLED: &str = "cancelled";

pub const JOB_TYPE_INDEX_DOCUMENT: &str = "index_document";
pub const JOB_TYPE_INDEX_MEMORY: &str = "index_memory";
pub const JOB_TYPE_INDEX_WIKI_NOTE: &str = "index_wiki_note";
pub const JOB_TYPE_REBUILD_CONTAINER: &str = "rebuild_container";
pub const JOB_TYPE_REINDEX_DOCUMENT: &str = "reindex_document";
pub const JOB_TYPE_EXTRACT_ENTITIES: &str = "extract_entities";

pub const STAGE_PARSING: &str = "parsing";
pub const STAGE_CHUNKING: &str = "chunking";
pub const STAGE_EMBEDDING: &str = "embedding";
pub const STAGE_STORING: &str = "storing";
pub const STAGE_EXTRACTING: &str = "extracting";

pub fn model_to_job(m: index_jobs::Model) -> IndexJob {
    IndexJob {
        id: m.id,
        job_type: m.job_type,
        container_type: m.container_type,
        container_id: m.container_id,
        item_id: m.item_id,
        status: m.status,
        current_stage: m.current_stage,
        progress: m.progress,
        error_message: m.error_message,
        retry_count: m.retry_count,
        max_retries: m.max_retries,
        priority: m.priority,
        created_at: m.created_at,
        started_at: m.started_at,
        completed_at: m.completed_at,
        metadata: m.metadata,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexJob {
    pub id: String,
    pub job_type: String,
    pub container_type: String,
    pub container_id: String,
    pub item_id: String,
    pub status: String,
    pub current_stage: Option<String>,
    pub progress: i32,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub priority: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateIndexJobInput {
    pub job_type: String,
    pub container_type: String,
    pub container_id: String,
    pub item_id: String,
    pub max_retries: Option<i32>,
    pub priority: Option<i32>,
    pub metadata: Option<String>,
}

pub async fn enqueue_job(db: &DatabaseConnection, input: CreateIndexJobInput) -> Result<IndexJob> {
    // 去重检查：同一 container_type + item_id 已有活跃 job（pending / processing / retrying）则跳过
    if let Some(existing) =
        get_active_job_for_item(db, &input.container_type, &input.item_id).await?
    {
        tracing::debug!(
            container_type = %existing.container_type,
            item_id = %existing.item_id,
            existing_job_id = %existing.id,
            "[index_queue] 跳过重复入队，已有活跃 job"
        );
        return Ok(existing);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let id = gen_id();
    let am = index_jobs::ActiveModel {
        id: Set(id.clone()),
        job_type: Set(input.job_type),
        container_type: Set(input.container_type),
        container_id: Set(input.container_id),
        item_id: Set(input.item_id),
        status: Set(INDEX_JOB_STATUS_PENDING.to_string()),
        current_stage: Set(None),
        progress: Set(0),
        error_message: Set(None),
        retry_count: Set(0),
        max_retries: Set(input.max_retries.unwrap_or(3)),
        priority: Set(input.priority.unwrap_or(0)),
        created_at: Set(now),
        started_at: Set(None),
        completed_at: Set(None),
        metadata: Set(input.metadata),
    };

    am.insert(db).await?;
    get_job(db, &id).await
}

pub async fn get_job(db: &DatabaseConnection, id: &str) -> Result<IndexJob> {
    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;
    Ok(model_to_job(model))
}

pub async fn list_pending_jobs(db: &DatabaseConnection, limit: u64) -> Result<Vec<IndexJob>> {
    let models = index_jobs::Entity::find()
        .filter(index_jobs::Column::Status.eq(INDEX_JOB_STATUS_PENDING))
        .order_by_desc(index_jobs::Column::Priority)
        .order_by_asc(index_jobs::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_job).collect())
}

pub async fn list_jobs_by_status(
    db: &DatabaseConnection,
    status: &str,
    limit: u64,
) -> Result<Vec<IndexJob>> {
    let models = index_jobs::Entity::find()
        .filter(index_jobs::Column::Status.eq(status))
        .order_by_desc(index_jobs::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_job).collect())
}

pub async fn list_retryable_failed_jobs(db: &DatabaseConnection) -> Result<Vec<IndexJob>> {
    let retry_col = Expr::col(index_jobs::Column::RetryCount);
    let max_col = Expr::col(index_jobs::Column::MaxRetries);
    let models = index_jobs::Entity::find()
        .filter(index_jobs::Column::Status.eq(INDEX_JOB_STATUS_FAILED).and(retry_col.lt(max_col)))
        .order_by_asc(index_jobs::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_job).collect())
}

pub async fn list_jobs_by_container(
    db: &DatabaseConnection,
    container_type: &str,
    container_id: &str,
) -> Result<Vec<IndexJob>> {
    let models = index_jobs::Entity::find()
        .filter(
            index_jobs::Column::ContainerType
                .eq(container_type)
                .and(index_jobs::Column::ContainerId.eq(container_id)),
        )
        .order_by_desc(index_jobs::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_job).collect())
}

pub async fn list_jobs_by_item(
    db: &DatabaseConnection,
    container_type: &str,
    item_id: &str,
) -> Result<Vec<IndexJob>> {
    let models = index_jobs::Entity::find()
        .filter(
            index_jobs::Column::ContainerType
                .eq(container_type)
                .and(index_jobs::Column::ItemId.eq(item_id)),
        )
        .order_by_desc(index_jobs::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_job).collect())
}

pub async fn mark_job_processing(
    db: &DatabaseConnection,
    id: &str,
    stage: Option<&str>,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;

    let mut am: index_jobs::ActiveModel = model.into();
    am.status = Set(INDEX_JOB_STATUS_PROCESSING.to_string());
    am.current_stage = Set(stage.map(|s| s.to_string()));
    am.started_at = Set(Some(now));
    am.error_message = Set(None);
    am.update(db).await?;
    Ok(())
}

pub async fn update_job_progress(
    db: &DatabaseConnection,
    id: &str,
    stage: Option<&str>,
    progress: i32,
) -> Result<()> {
    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;

    let mut am: index_jobs::ActiveModel = model.into();
    if let Some(s) = stage {
        am.current_stage = Set(Some(s.to_string()));
    }
    am.progress = Set(progress.clamp(0, 100));
    am.update(db).await?;
    Ok(())
}

pub async fn mark_job_completed(db: &DatabaseConnection, id: &str) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;

    let mut am: index_jobs::ActiveModel = model.into();
    am.status = Set(INDEX_JOB_STATUS_COMPLETED.to_string());
    am.progress = Set(100);
    am.current_stage = Set(None);
    am.completed_at = Set(Some(now));
    am.error_message = Set(None);
    am.update(db).await?;
    Ok(())
}

pub async fn mark_job_failed(db: &DatabaseConnection, id: &str, error: &str) -> Result<IndexJob> {
    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;

    let mut am: index_jobs::ActiveModel = model.into();
    let retry_count = am.retry_count.take().expect("IndexJob 缺少 retry_count");
    let next_retry = retry_count + 1;

    let max_retries = am.max_retries.take().expect("IndexJob 缺少 max_retries");
    if next_retry < max_retries {
        am.status = Set(INDEX_JOB_STATUS_RETRYING.to_string());
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        am.status = Set(INDEX_JOB_STATUS_FAILED.to_string());
        am.completed_at = Set(Some(now));
    }
    am.retry_count = Set(next_retry);
    am.current_stage = Set(None);
    am.error_message = Set(Some(error.to_string()));
    let updated = am.update(db).await?;
    Ok(model_to_job(updated))
}

/// 标记任务为终态失败且不重试。
///
/// 用于确定性配置错误（如 embedding provider 未配置，R9）：这类错误重试
/// max_retries 次结果必然相同，直接进入 failed 终态避免指数退避空转。
pub async fn mark_job_failed_no_retry(
    db: &DatabaseConnection,
    id: &str,
    error: &str,
) -> Result<IndexJob> {
    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let mut am: index_jobs::ActiveModel = model.into();
    am.status = Set(INDEX_JOB_STATUS_FAILED.to_string());
    am.completed_at = Set(Some(now));
    am.current_stage = Set(None);
    am.error_message = Set(Some(error.to_string()));
    let updated = am.update(db).await?;
    Ok(model_to_job(updated))
}

pub async fn reset_job_for_retry(db: &DatabaseConnection, id: &str) -> Result<()> {
    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;

    let mut am: index_jobs::ActiveModel = model.into();
    am.status = Set(INDEX_JOB_STATUS_PENDING.to_string());
    am.progress = Set(0);
    am.current_stage = Set(None);
    am.started_at = Set(None);
    am.error_message = Set(None);
    am.update(db).await?;
    Ok(())
}

pub async fn cancel_job(db: &DatabaseConnection, id: &str) -> Result<()> {
    let model = index_jobs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("IndexJob {}", id)))?;

    let mut am: index_jobs::ActiveModel = model.into();
    let status = am.status.take().expect("IndexJob 缺少 status");
    if status != INDEX_JOB_STATUS_PROCESSING && status != INDEX_JOB_STATUS_PENDING {
        return Err(AxAgentError::Validation(
            "Can only cancel pending or processing jobs".to_string(),
        ));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    am.status = Set(INDEX_JOB_STATUS_CANCELLED.to_string());
    am.completed_at = Set(Some(now));
    am.current_stage = Set(None);
    am.update(db).await?;
    Ok(())
}

pub async fn cancel_pending_jobs_for_item(
    db: &DatabaseConnection,
    container_type: &str,
    item_id: &str,
) -> Result<u64> {
    let result = index_jobs::Entity::update_many()
        .col_expr(index_jobs::Column::Status, Expr::value(INDEX_JOB_STATUS_CANCELLED))
        .filter(
            index_jobs::Column::ContainerType
                .eq(container_type)
                .and(index_jobs::Column::ItemId.eq(item_id))
                .and(
                    index_jobs::Column::Status
                        .is_in([INDEX_JOB_STATUS_PENDING, INDEX_JOB_STATUS_RETRYING]),
                ),
        )
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// 取消指定容器下所有未完成（pending / retrying / processing）的索引任务。
///
/// 用于删除知识容器（wiki / knowledge_base / memory namespace）时清理残留任务，
/// 避免队列继续轮询已删除容器导致 NotFound 错误刷屏。
pub async fn cancel_jobs_by_container(
    db: &DatabaseConnection,
    container_type: &str,
    container_id: &str,
) -> Result<u64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let result = index_jobs::Entity::update_many()
        .col_expr(index_jobs::Column::Status, Expr::value(INDEX_JOB_STATUS_CANCELLED))
        .col_expr(index_jobs::Column::CompletedAt, Expr::value(now))
        .filter(
            index_jobs::Column::ContainerType
                .eq(container_type)
                .and(index_jobs::Column::ContainerId.eq(container_id))
                .and(index_jobs::Column::Status.is_in([
                    INDEX_JOB_STATUS_PENDING,
                    INDEX_JOB_STATUS_RETRYING,
                    INDEX_JOB_STATUS_PROCESSING,
                ])),
        )
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// 物理删除指定容器下的所有索引任务。
/// 用于删除知识容器（wiki / knowledge_base / memory namespace）时彻底清理残留 job 数据。
pub async fn delete_jobs_by_container(
    db: &DatabaseConnection,
    container_type: &str,
    container_id: &str,
) -> Result<u64> {
    let result = index_jobs::Entity::delete_many()
        .filter(
            index_jobs::Column::ContainerType
                .eq(container_type)
                .and(index_jobs::Column::ContainerId.eq(container_id)),
        )
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn count_jobs_by_status(db: &DatabaseConnection, status: &str) -> Result<u64> {
    let count =
        index_jobs::Entity::find().filter(index_jobs::Column::Status.eq(status)).count(db).await?;
    Ok(count)
}

pub async fn cleanup_completed_jobs(db: &DatabaseConnection, older_than_ms: i64) -> Result<u64> {
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
        - older_than_ms;

    let result = index_jobs::Entity::delete_many()
        .filter(
            index_jobs::Column::Status
                .is_in([INDEX_JOB_STATUS_COMPLETED, INDEX_JOB_STATUS_CANCELLED])
                .and(index_jobs::Column::CompletedAt.lte(cutoff)),
        )
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn get_active_job_for_item(
    db: &DatabaseConnection,
    container_type: &str,
    item_id: &str,
) -> Result<Option<IndexJob>> {
    let model = index_jobs::Entity::find()
        .filter(
            index_jobs::Column::ContainerType
                .eq(container_type)
                .and(index_jobs::Column::ItemId.eq(item_id))
                .and(index_jobs::Column::Status.is_in([
                    INDEX_JOB_STATUS_PENDING,
                    INDEX_JOB_STATUS_PROCESSING,
                    INDEX_JOB_STATUS_RETRYING,
                ])),
        )
        .order_by_desc(index_jobs::Column::CreatedAt)
        .one(db)
        .await?;
    Ok(model.map(model_to_job))
}

pub async fn list_all_jobs(
    db: &DatabaseConnection,
    limit: u64,
    offset: u64,
) -> Result<(Vec<IndexJob>, u64)> {
    let total = index_jobs::Entity::find().count(db).await?;
    let models = index_jobs::Entity::find()
        .order_by_desc(index_jobs::Column::CreatedAt)
        .limit(limit)
        .offset(offset)
        .all(db)
        .await?;
    Ok((models.into_iter().map(model_to_job).collect(), total))
}
