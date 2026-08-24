// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_dao::repo::index_jobs::{
    self, INDEX_JOB_STATUS_CANCELLED, INDEX_JOB_STATUS_COMPLETED, INDEX_JOB_STATUS_FAILED,
    INDEX_JOB_STATUS_PENDING, INDEX_JOB_STATUS_PROCESSING, IndexJob, JOB_TYPE_INDEX_DOCUMENT,
    JOB_TYPE_INDEX_MEMORY, JOB_TYPE_INDEX_WIKI_NOTE,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexQueueStats {
    pub pending: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "列出索引任务")]
#[tauri::command]
pub async fn index_jobs_list(
    state: State<'_, AppState>,
    status: Option<String>,
    source_type: Option<String>,
    source_id: Option<String>,
    target_id: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<IndexJob>, String> {
    let db = state.harness.db();
    let lim = limit.unwrap_or(50);

    let status_str = status.as_deref();
    let select = index_jobs::Entity::find();

    let select = if let Some(s) = status_str {
        select.filter(index_jobs::Column::Status.eq(s))
    } else {
        select
    };
    let select = if let Some(st) = &source_type {
        select.filter(index_jobs::Column::ContainerType.eq(st))
    } else {
        select
    };
    let select = if let Some(si) = &source_id {
        select.filter(index_jobs::Column::ContainerId.eq(si))
    } else {
        select
    };
    let select = if let Some(ti) = &target_id {
        select.filter(index_jobs::Column::ItemId.eq(ti))
    } else {
        select
    };

    let models =
        select.order_by_desc(index_jobs::Column::CreatedAt).limit(lim).all(db).await.map_err(
            |e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            },
        )?;

    Ok(models.into_iter().map(index_jobs::model_to_job).collect())
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateOnly, description = "获取索引任务统计")]
#[tauri::command]
pub async fn index_jobs_stats(state: State<'_, AppState>) -> Result<IndexQueueStats, String> {
    let db = state.harness.db();
    let pending =
        index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_PENDING).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let running =
        index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_PROCESSING).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let completed =
        index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_COMPLETED).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    let failed =
        index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_FAILED).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    Ok(IndexQueueStats { pending, running, completed, failed })
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = Manual, description = "重试索引任务")]
#[tauri::command]
pub async fn index_jobs_retry(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: String,
) -> Result<IndexJob, String> {
    index_jobs::reset_job_for_retry(state.harness.db(), &job_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let job = index_jobs::get_job(state.harness.db(), &job_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let _ = app.emit(
        "index-job-updated",
        serde_json::json!({ "jobId": job_id, "status": INDEX_JOB_STATUS_PENDING }),
    );
    Ok(job)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = Manual, description = "取消索引任务")]
#[tauri::command]
pub async fn index_jobs_cancel(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: String,
) -> Result<IndexJob, String> {
    index_jobs::cancel_job(state.harness.db(), &job_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let job = index_jobs::get_job(state.harness.db(), &job_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let _ = app.emit(
        "index-job-updated",
        serde_json::json!({ "jobId": job_id, "status": INDEX_JOB_STATUS_CANCELLED }),
    );
    Ok(job)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "重试所有失败任务")]
#[tauri::command]
pub async fn index_jobs_retry_all_failed(state: State<'_, AppState>) -> Result<u64, String> {
    let jobs = index_jobs::list_retryable_failed_jobs(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let count = jobs.len() as u64;
    for job in &jobs {
        let _ = index_jobs::reset_job_for_retry(state.harness.db(), &job.id).await;
    }
    Ok(count)
}

#[agent_command(domain = knowledge, safety = Dangerous, call_mode = StateOnly, description = "清除已完成任务")]
#[tauri::command]
pub async fn index_jobs_clear_completed(state: State<'_, AppState>) -> Result<u64, String> {
    index_jobs::cleanup_completed_jobs(state.harness.db(), 0).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = Manual, description = "重建集合索引")]
#[tauri::command]
pub async fn index_jobs_reindex_collection(
    app: AppHandle,
    state: State<'_, AppState>,
    source_type: String,
    source_id: String,
) -> Result<u64, String> {
    match source_type.as_str() {
        "kb" => {
            let docs = axagent_dao::repo::knowledge::list_documents(state.harness.db(), &source_id)
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
            let count = docs.len() as u64;
            for doc in docs {
                let _ = axagent_dao::repo::knowledge::update_document_status(
                    state.harness.db(),
                    &doc.id,
                    "pending",
                )
                .await;
                let _ = crate::index_queue::enqueue_job_sync(
                    &state,
                    &app,
                    JOB_TYPE_INDEX_DOCUMENT,
                    "kb",
                    &source_id,
                    &doc.id,
                    None,
                    None,
                );
            }
            Ok(count)
        },
        "memory" => {
            let items = axagent_dao::repo::memory::list_namespaces(state.harness.db())
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
            let count = items.len() as u64;
            for item in items {
                let _ = crate::index_queue::enqueue_job_sync(
                    &state,
                    &app,
                    JOB_TYPE_INDEX_MEMORY,
                    "memory",
                    &source_id,
                    &item.id,
                    None,
                    None,
                );
            }
            Ok(count)
        },
        "wiki" => {
            let notes = axagent_dao::repo::note::list_notes(state.harness.db(), &source_id)
                .await
                .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            let count = notes.len() as u64;
            for note in notes {
                let _ = crate::index_queue::enqueue_job_sync(
                    &state,
                    &app,
                    JOB_TYPE_INDEX_WIKI_NOTE,
                    "wiki",
                    &source_id,
                    &note.id,
                    None,
                    None,
                );
            }
            Ok(count)
        },
        _ => Err(format!("Unknown source type: {}", source_type)),
    }
}
