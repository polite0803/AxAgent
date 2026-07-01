// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_core::repo::index_jobs::{
    self, INDEX_JOB_STATUS_CANCELLED, INDEX_JOB_STATUS_COMPLETED, INDEX_JOB_STATUS_FAILED,
    INDEX_JOB_STATUS_PENDING, INDEX_JOB_STATUS_PROCESSING, IndexJob, JOB_TYPE_INDEX_DOCUMENT,
    JOB_TYPE_INDEX_MEMORY, JOB_TYPE_INDEX_WIKI_NOTE,
};
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

    let models = select
        .order_by_desc(index_jobs::Column::CreatedAt)
        .limit(lim)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(models
        .into_iter()
        .map(|m| index_jobs::model_to_job(m))
        .collect())
}

#[tauri::command]
pub async fn index_jobs_stats(state: State<'_, AppState>) -> Result<IndexQueueStats, String> {
    let db = state.harness.db();
    let pending = index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_PENDING)
        .await
        .map_err(|e| e.to_string())?;
    let running = index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_PROCESSING)
        .await
        .map_err(|e| e.to_string())?;
    let completed = index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_COMPLETED)
        .await
        .map_err(|e| e.to_string())?;
    let failed = index_jobs::count_jobs_by_status(db, INDEX_JOB_STATUS_FAILED)
        .await
        .map_err(|e| e.to_string())?;
    Ok(IndexQueueStats {
        pending,
        running,
        completed,
        failed,
    })
}

#[tauri::command]
pub async fn index_jobs_retry(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<IndexJob, String> {
    index_jobs::reset_job_for_retry(state.harness.db(), &job_id)
        .await
        .map_err(|e| e.to_string())?;
    let job = index_jobs::get_job(state.harness.db(), &job_id)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(service) = state.index_job_service.upgrade() {
        service.notify_new_job();
    }
    let _ = state.app.emit(
        "index-job-updated",
        serde_json::json!({ "jobId": job_id, "status": INDEX_JOB_STATUS_PENDING }),
    );
    Ok(job)
}

#[tauri::command]
pub async fn index_jobs_cancel(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<IndexJob, String> {
    index_jobs::cancel_job(state.harness.db(), &job_id)
        .await
        .map_err(|e| e.to_string())?;
    let job = index_jobs::get_job(state.harness.db(), &job_id)
        .await
        .map_err(|e| e.to_string())?;
    let _ = state.app.emit(
        "index-job-updated",
        serde_json::json!({ "jobId": job_id, "status": INDEX_JOB_STATUS_CANCELLED }),
    );
    Ok(job)
}

#[tauri::command]
pub async fn index_jobs_retry_all_failed(state: State<'_, AppState>) -> Result<u64, String> {
    let jobs = index_jobs::list_retryable_failed_jobs(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    let count = jobs.len() as u64;
    for job in &jobs {
        let _ = index_jobs::reset_job_for_retry(state.harness.db(), &job.id).await;
    }
    if count > 0 {
        if let Some(service) = state.index_job_service.upgrade() {
            service.notify_new_job();
        }
    }
    Ok(count)
}

#[tauri::command]
pub async fn index_jobs_clear_completed(state: State<'_, AppState>) -> Result<u64, String> {
    index_jobs::cleanup_completed_jobs(state.harness.db(), 0)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn index_jobs_reindex_collection(
    app: AppHandle,
    state: State<'_, AppState>,
    source_type: String,
    source_id: String,
) -> Result<u64, String> {
    match source_type.as_str() {
        "kb" => {
            let docs =
                axagent_core::repo::knowledge::list_documents(state.harness.db(), &source_id)
                    .await
                    .map_err(|e| e.to_string())?;
            let count = docs.len() as u64;
            for doc in docs {
                let _ = axagent_core::repo::knowledge::update_document_status(
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
            let items =
                axagent_core::repo::memory::list_namespace_items(state.harness.db(), &source_id)
                    .await
                    .map_err(|e| e.to_string())?;
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
            let notes = axagent_core::repo::note::list_notes(state.harness.db(), &source_id)
                .await
                .map_err(|e| e.to_string())?;
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
