// SPDX-License-Identifier: AGPL-3.0-only

//! 持久化索引队列服务。
//!
//! 解决三个核心问题：
//! 1. 应用重启后索引任务不丢失（持久化到 `index_jobs` 表）
//! 2. 失败任务自动重试（指数退避，最多 max_retries 次）
//! 3. 细粒度进度事件（parsing → chunking → embedding → storing）

use crate::AppState;
use axagent_dao::repo::index_jobs as jobs;
use axagent_harness::{ExtractedEntity, ExtractedRelation};
use axagent_search::rag;
use axagent_search::vector_store::VectorStore;
use sea_orm::ConnectionTrait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

const RETRY_BASE_DELAY_MS: u64 = 2_000;
const RETRY_MAX_DELAY_MS: u64 = 60_000;
const POLL_INTERVAL_MS: u64 = 500;
const MAX_CONCURRENT_JOBS: usize = 2;

#[derive(Clone)]
pub struct IndexJobService {
    db: DatabaseConnection,
    vector_store: Arc<VectorStore>,
    master_key: [u8; 32],
    semaphore: Arc<Semaphore>,
    shutdown_token: CancellationToken,
    app: AppHandle,
}

impl IndexJobService {
    pub fn new(
        db: DatabaseConnection,
        vector_store: Arc<VectorStore>,
        master_key: [u8; 32],
        shutdown_token: CancellationToken,
        app: AppHandle,
    ) -> Self {
        Self {
            db,
            vector_store,
            master_key,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_JOBS)),
            shutdown_token,
            app,
        }
    }

    pub async fn start(self: Arc<Self>) {
        tracing::info!("[index_queue] 启动持久化索引队列服务");
        // 防御性建表：确保 index_jobs 表存在（迁移系统可能尚未补跑 v5）
        if let Err(e) = self
            .db
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS index_jobs (\
                 id TEXT NOT NULL PRIMARY KEY, \
                 job_type TEXT NOT NULL, \
                 container_type TEXT NOT NULL, \
                 container_id TEXT NOT NULL, \
                 item_id TEXT NOT NULL, \
                 status TEXT NOT NULL DEFAULT 'pending', \
                 current_stage TEXT, \
                 progress INTEGER NOT NULL DEFAULT 0, \
                 error_message TEXT, \
                 retry_count INTEGER NOT NULL DEFAULT 0, \
                 max_retries INTEGER NOT NULL DEFAULT 3, \
                 priority INTEGER NOT NULL DEFAULT 0, \
                 created_at INTEGER NOT NULL, \
                 started_at INTEGER, \
                 completed_at INTEGER, \
                 metadata TEXT)",
            )
            .await
        {
            tracing::warn!("[index_queue] 防御性建表失败: {}", e);
        }
        self.recover_pending_jobs().await;

        loop {
            tokio::select! {
                _ = self.shutdown_token.cancelled() => {
                    tracing::info!("[index_queue] 收到关闭信号，停止索引队列");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)) => {
                    if let Err(e) = self.process_next_batch().await {
                        tracing::warn!("[index_queue] 处理批次出错: {}", e);
                    }
                }
            }
        }
    }

    async fn recover_pending_jobs(&self) {
        let reset_statuses = [jobs::INDEX_JOB_STATUS_PROCESSING, jobs::INDEX_JOB_STATUS_RETRYING];
        for status in &reset_statuses {
            match jobs::list_jobs_by_status(&self.db, status, 100).await {
                Ok(pending) => {
                    for job in pending {
                        let _ = jobs::reset_job_for_retry(&self.db, &job.id).await;
                        tracing::info!(
                            job_id = %job.id,
                            old_status = %status,
                            "[index_queue] 恢复中断任务，重置为pending"
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!("[index_queue] 恢复{}任务失败: {}", status, e);
                },
            }
        }
    }

    async fn process_next_batch(&self) -> Result<(), String> {
        let pending = jobs::list_pending_jobs(&self.db, MAX_CONCURRENT_JOBS as u64)
            .await
            .map_err(|e| e.to_string())?;

        for job in pending {
            let permit = match self.semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => return Ok(()),
            };

            let service = self.clone();
            let job_id = job.id.clone();

            tokio::spawn(async move {
                let _permit = permit;
                if let Err(e) = service.execute_job(&job_id).await {
                    tracing::error!(job_id = %job_id, error = %e, "[index_queue] 任务执行失败");
                }
            });
        }

        Ok(())
    }

    async fn execute_job(&self, job_id: &str) -> Result<(), String> {
        let job = jobs::get_job(&self.db, job_id).await.map_err(|e| e.to_string())?;

        let delay_ms = if job.retry_count > 0 {
            let backoff = RETRY_BASE_DELAY_MS
                .saturating_mul(2u64.saturating_pow(job.retry_count as u32))
                .min(RETRY_MAX_DELAY_MS);
            tracing::info!(
                job_id = %job.id,
                retry = job.retry_count,
                delay_ms = backoff,
                "[index_queue] 重试任务，等待后退",
            );
            backoff
        } else {
            0
        };

        if delay_ms > 0 {
            tokio::select! {
                _ = self.shutdown_token.cancelled() => return Ok(()),
                _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
            }
        }

        // 根据任务类型路由到不同的处理函数
        let result = match job.job_type.as_str() {
            jobs::JOB_TYPE_EXTRACT_ENTITIES => {
                jobs::mark_job_processing(&self.db, &job.id, Some(jobs::STAGE_EXTRACTING))
                    .await
                    .map_err(|e| e.to_string())?;
                self.emit_progress(&job, jobs::STAGE_EXTRACTING, 10).await;
                self.run_entity_extraction(&job).await
            },
            _ => {
                jobs::mark_job_processing(&self.db, &job.id, Some(jobs::STAGE_PARSING))
                    .await
                    .map_err(|e| e.to_string())?;
                self.emit_progress(&job, jobs::STAGE_PARSING, 5).await;
                self.run_indexing(&job).await
            },
        };

        match result {
            Ok(()) => {
                let _ = jobs::mark_job_completed(&self.db, &job.id).await;
                self.emit_completed(&job).await;
                tracing::info!(job_id = %job.id, "[index_queue] 任务完成");
            },
            Err(e) => {
                let err_msg = e.to_string();
                match jobs::mark_job_failed(&self.db, &job.id, &err_msg).await {
                    Ok(updated) => {
                        if updated.status == jobs::INDEX_JOB_STATUS_RETRYING {
                            self.emit_retrying(&job, &err_msg).await;
                            tracing::warn!(
                                job_id = %job.id,
                                retry = updated.retry_count,
                                error = %err_msg,
                                "[index_queue] 任务将重试",
                            );
                        } else {
                            self.emit_failed(&job, &err_msg).await;
                            tracing::error!(
                                job_id = %job.id,
                                retries = updated.retry_count,
                                error = %err_msg,
                                "[index_queue] 任务最终失败",
                            );
                            self.mark_item_error(&job, &err_msg).await;
                        }
                    },
                    Err(e2) => {
                        tracing::error!(
                            job_id = %job.id,
                            error = %e2,
                            "[index_queue] 更新任务状态失败"
                        );
                    },
                }
            },
        }

        Ok(())
    }

    async fn run_indexing(&self, job: &jobs::IndexJob) -> Result<(), String> {
        let container_type = match job.container_type.as_str() {
            "knowledge" | "kb" => rag::ContainerType::KnowledgeBase,
            "memory" | "mem" => rag::ContainerType::Memory,
            "wiki" => rag::ContainerType::WikiVault,
            other => return Err(format!("未知容器类型: {}", other)),
        };

        let container = self.load_container(&container_type, &job.container_id).await?;

        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_PARSING), 10)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_PARSING, 10).await;

        let (source_path, mime_type, content) =
            self.extract_job_inputs(job, &container_type).await?;

        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_CHUNKING), 30)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_CHUNKING, 30).await;

        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_EMBEDDING), 60)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_EMBEDDING, 60).await;

        crate::indexing::index_source(
            &self.db,
            &self.master_key,
            &self.vector_store,
            &container,
            &job.item_id,
            content.as_deref().unwrap_or(""),
            source_path.as_deref(),
            mime_type.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_STORING), 90)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_STORING, 90).await;

        self.mark_item_ready(&container_type, job).await?;

        // 文档索引完成后，自动入队实体抽取任务
        if container_type == rag::ContainerType::KnowledgeBase {
            let metadata = serde_json::json!({
                "document_ids": [job.item_id.clone()],
                "auto_extract": true,
            });
            let _ = jobs::enqueue_job(
                &self.db,
                jobs::CreateIndexJobInput {
                    job_type: jobs::JOB_TYPE_EXTRACT_ENTITIES.to_string(),
                    container_type: "kb".to_string(),
                    container_id: job.container_id.clone(),
                    item_id: job.item_id.clone(),
                    max_retries: Some(1),
                    priority: Some(1),
                    metadata: serde_json::to_string(&metadata).ok(),
                },
            )
            .await;
        }

        Ok(())
    }

    /// 执行实体抽取任务
    async fn run_entity_extraction(&self, job: &jobs::IndexJob) -> Result<(), String> {
        let kb_id = &job.container_id;

        // 从 metadata 中获取文档 ID 列表
        let document_ids: Vec<String> = if let Some(ref meta) = job.metadata {
            serde_json::from_str(meta)
                .ok()
                .and_then(|v: serde_json::Value| {
                    v.get("document_ids").and_then(|ids| ids.as_array()).map(|arr| {
                        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                    })
                })
                .unwrap_or_default()
        } else {
            // 如果没有指定文档，则处理整个知识库的文档
            let docs = axagent_dao::repo::knowledge::list_documents(&self.db, kb_id)
                .await
                .map_err(|e| e.to_string())?;
            docs.iter().map(|d| d.id.clone()).collect()
        };

        if document_ids.is_empty() {
            tracing::info!(
                job_id = %job.id,
                kb_id = %kb_id,
                "[index_queue] 无文档需要抽取实体，跳过"
            );
            return Ok(());
        }

        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_EXTRACTING), 30)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_EXTRACTING, 30).await;

        // 分批处理（每批最多 20 个文档）
        let chunk_size = 20;
        let mut total_new_entities = 0;
        let mut total_new_relations = 0;

        for chunk in document_ids.chunks(chunk_size) {
            let batch: Vec<String> = chunk.to_vec();

            // 调用实体抽取逻辑（使用 knowledge_graph 命令的核心逻辑）
            let result = self.extract_entities_batch(kb_id, &batch).await?;

            total_new_entities += result.new_entities.len();
            total_new_relations += result.new_relations.len();

            tracing::info!(
                job_id = %job.id,
                kb_id = %kb_id,
                batch_size = batch.len(),
                new_entities = result.new_entities.len(),
                new_relations = result.new_relations.len(),
                "[index_queue] 实体抽取批次完成"
            );
        }

        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_EXTRACTING), 90)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_EXTRACTING, 90).await;

        tracing::info!(
            job_id = %job.id,
            kb_id = %kb_id,
            total_new_entities = total_new_entities,
            total_new_relations = total_new_relations,
            "[index_queue] 实体抽取任务完成"
        );

        Ok(())
    }

    /// 执行一批文档的实体抽取（核心逻辑）
    async fn extract_entities_batch(
        &self,
        kb_id: &str,
        document_ids: &[String],
    ) -> Result<axagent_harness::ExtractEntitiesResult, String> {
        let collection_id = format!("kb_{}", kb_id);

        // 1. 加载所有文档的 chunks 并拼接
        let mut all_text = String::new();
        let mut skipped_chunks: u32 = 0;
        for doc_id in document_ids {
            let chunks = self
                .vector_store
                .list_document_chunks(&collection_id, doc_id)
                .await
                .map_err(|e| e.to_string())?;
            if chunks.is_empty() {
                skipped_chunks += 1;
                continue;
            }
            for chunk in chunks {
                all_text.push_str(&chunk.content);
                all_text.push_str("\n\n");
                // 截断到上限（16k 字节）
                if all_text.len() >= 16_000 {
                    all_text.truncate(16_000);
                    break;
                }
            }
            if all_text.len() >= 16_000 {
                break;
            }
        }

        if all_text.trim().is_empty() {
            return Ok(axagent_harness::ExtractEntitiesResult {
                new_entities: Vec::new(),
                updated_entities: Vec::new(),
                new_relations: Vec::new(),
                skipped_chunks,
                elapsed_ms: 0,
            });
        }

        // 2. 加载已有实体列表（用于去重）
        let existing_entities =
            axagent_dao::repo::knowledge_graph::get_all_entities_by_kb(&self.db, kb_id)
                .await
                .map_err(|e| e.to_string())?;
        let existing_names: Vec<String> =
            existing_entities.iter().take(50).map(|e| e.name.clone()).collect();

        // 3. 构建提示词
        let system_prompt = axagent_kit::prompts::PromptRegistry::get(
            "entity_extraction.system_prompt",
            axagent_harness::prompt_provider::PromptLang::ZhCN,
        );
        let user_template = axagent_kit::prompts::PromptRegistry::get(
            "entity_extraction.user_template",
            axagent_harness::prompt_provider::PromptLang::ZhCN,
        );

        let existing_hint = if existing_names.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n[已存在的实体名称（请勿重复抽取，可在关系中引用）]\n{}",
                existing_names.join(", ")
            )
        };
        let user_prompt = user_template.replace("{0}", &format!("{}{}", all_text, existing_hint));

        // 4. 构建 LLM Bridge
        let bridge = axagent_runtime::llm_bridge::build_llm_bridge_from_db(&self.master_key)
            .await
            .ok_or_else(|| "未找到启用的 LLM Provider，无法执行实体抽取".to_string())?;

        // 5. 调用 LLM
        let llm_response = bridge
            .call_llm(system_prompt, &user_prompt)
            .await
            .map_err(|e| format!("LLM 实体抽取调用失败：{}", e))?;

        // 6. 解析响应
        let (entities, relations) = self.parse_entity_extraction_response(&llm_response)?;

        // 7. 写入 DB
        let result = axagent_dao::repo::knowledge_graph::batch_upsert_entities_and_relations(
            &self.db, kb_id, entities, relations,
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(axagent_harness::ExtractEntitiesResult {
            new_entities: result.new_entities,
            updated_entities: result.updated_entities,
            new_relations: result.new_relations,
            skipped_chunks: result.skipped_chunks + skipped_chunks,
            elapsed_ms: result.elapsed_ms,
        })
    }

    /// 解析 LLM 实体抽取响应
    fn parse_entity_extraction_response(
        &self,
        response: &str,
    ) -> Result<(Vec<ExtractedEntity>, Vec<ExtractedRelation>), String> {
        // 清理 markdown fences
        let cleaned = response
            .trim()
            .trim_start_matches("```")
            .trim_start_matches("json")
            .trim_start_matches("JSON")
            .trim_end_matches("```")
            .trim()
            .to_string();

        // 尝试解析 JSON
        let payload: serde_json::Value =
            serde_json::from_str(&cleaned).unwrap_or(serde_json::json!({}));

        let entities: Vec<ExtractedEntity> = payload
            .get("entities")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| {
                        let name = e.get("name")?.as_str()?.to_string();
                        if name.is_empty() {
                            return None;
                        }
                        Some(ExtractedEntity {
                            name,
                            entity_type: e
                                .get("entity_type")
                                .or_else(|| e.get("type"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("concept")
                                .to_string(),
                            aliases: e
                                .get("aliases")
                                .and_then(|a| a.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            description: e
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let relations: Vec<ExtractedRelation> = payload
            .get("relations")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        let source = r.get("source")?.as_str()?.to_string();
                        let target = r.get("target")?.as_str()?.to_string();
                        if source.is_empty() || target.is_empty() {
                            return None;
                        }
                        Some(ExtractedRelation {
                            source,
                            target,
                            relation_type: r
                                .get("relation")
                                .or_else(|| r.get("relation_type"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("mentions")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok((entities, relations))
    }

    async fn load_container(
        &self,
        container_type: &rag::ContainerType,
        container_id: &str,
    ) -> Result<rag::KnowledgeContainer, String> {
        match container_type {
            rag::ContainerType::KnowledgeBase => {
                let kb = axagent_dao::repo::knowledge::get_knowledge_base(&self.db, container_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(rag::KnowledgeContainer::from_knowledge_base(&kb))
            },
            rag::ContainerType::Memory => {
                let ns = axagent_dao::repo::memory::get_namespace(&self.db, container_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(rag::KnowledgeContainer::from_memory_ns(&ns))
            },
            rag::ContainerType::WikiVault => {
                let wiki = axagent_dao::repo::wiki::get_wiki(&self.db, container_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(rag::KnowledgeContainer::from_wiki(&wiki))
            },
        }
    }

    async fn extract_job_inputs(
        &self,
        job: &jobs::IndexJob,
        container_type: &rag::ContainerType,
    ) -> Result<(Option<String>, Option<String>, Option<String>), String> {
        match container_type {
            rag::ContainerType::KnowledgeBase => {
                let doc = axagent_dao::repo::knowledge::get_document(&self.db, &job.item_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok((Some(doc.source_path), Some(doc.mime_type), None))
            },
            rag::ContainerType::Memory => {
                let item = axagent_dao::repo::memory::get_item(&self.db, &job.item_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok((None, None, Some(item.content)))
            },
            rag::ContainerType::WikiVault => {
                let note = axagent_dao::repo::note::get_note(&self.db, &job.item_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok((None, None, Some(note.content)))
            },
        }
    }

    async fn mark_item_ready(
        &self,
        container_type: &rag::ContainerType,
        job: &jobs::IndexJob,
    ) -> Result<(), String> {
        match container_type {
            rag::ContainerType::KnowledgeBase => {
                axagent_dao::repo::knowledge::update_document_status(
                    &self.db,
                    &job.item_id,
                    "ready",
                )
                .await
                .map_err(|e| e.to_string())?;
            },
            rag::ContainerType::Memory => {
                axagent_dao::repo::memory::update_item_index_status(
                    &self.db,
                    &job.item_id,
                    "ready",
                    None,
                )
                .await
                .map_err(|e| e.to_string())?;
            },
            rag::ContainerType::WikiVault => {},
        }
        Ok(())
    }

    async fn mark_item_error(&self, job: &jobs::IndexJob, error: &str) {
        let ct = match job.container_type.as_str() {
            "knowledge" | "kb" => Some(rag::ContainerType::KnowledgeBase),
            "memory" | "mem" => Some(rag::ContainerType::Memory),
            _ => None,
        };
        if let Some(ct) = ct {
            let _ = match ct {
                rag::ContainerType::KnowledgeBase => {
                    axagent_dao::repo::knowledge::update_document_status_with_error(
                        &self.db,
                        &job.item_id,
                        "failed",
                        Some(error),
                    )
                    .await
                },
                rag::ContainerType::Memory => {
                    axagent_dao::repo::memory::update_item_index_status(
                        &self.db,
                        &job.item_id,
                        "failed",
                        Some(error),
                    )
                    .await
                },
                _ => Ok(()),
            };
        }
    }

    async fn emit_progress(&self, job: &jobs::IndexJob, stage: &str, progress: i32) {
        let _ = self.app.emit(
            "index-job-progress",
            serde_json::json!({
                "jobId": job.id,
                "jobType": job.job_type,
                "containerType": job.container_type,
                "containerId": job.container_id,
                "itemId": job.item_id,
                "stage": stage,
                "progress": progress,
            }),
        );
    }

    async fn emit_completed(&self, job: &jobs::IndexJob) {
        let _ = self.app.emit(
            "index-job-completed",
            serde_json::json!({
                "jobId": job.id,
                "jobType": job.job_type,
                "containerType": job.container_type,
                "containerId": job.container_id,
                "itemId": job.item_id,
            }),
        );
    }

    async fn emit_failed(&self, job: &jobs::IndexJob, error: &str) {
        let _ = self.app.emit(
            "index-job-failed",
            serde_json::json!({
                "jobId": job.id,
                "jobType": job.job_type,
                "containerType": job.container_type,
                "containerId": job.container_id,
                "itemId": job.item_id,
                "error": error,
                "retryCount": job.retry_count,
                "maxRetries": job.max_retries,
            }),
        );
    }

    async fn emit_retrying(&self, job: &jobs::IndexJob, error: &str) {
        let _ = self.app.emit(
            "index-job-retrying",
            serde_json::json!({
                "jobId": job.id,
                "jobType": job.job_type,
                "containerType": job.container_type,
                "containerId": job.container_id,
                "itemId": job.item_id,
                "error": error,
                "retryCount": job.retry_count,
                "maxRetries": job.max_retries,
            }),
        );
    }
}

pub fn enqueue_job_sync(
    state: &AppState,
    app: &AppHandle,
    job_type: &str,
    container_type: &str,
    container_id: &str,
    item_id: &str,
    priority: Option<i32>,
    metadata: Option<serde_json::Value>,
) -> Result<String, String> {
    let db = state.harness.db().clone();
    let app_handle = app.clone();

    let jtype = job_type.to_string();
    let ctype = container_type.to_string();
    let cid = container_id.to_string();
    let iid = item_id.to_string();
    let meta_str = metadata.map(|m| serde_json::to_string(&m).unwrap_or_default());

    tauri::async_runtime::spawn(async move {
        let input = jobs::CreateIndexJobInput {
            job_type: jtype.clone(),
            container_type: ctype.clone(),
            container_id: cid.clone(),
            item_id: iid.clone(),
            max_retries: None,
            priority,
            metadata: meta_str,
        };

        match jobs::enqueue_job(&db, input).await {
            Ok(job) => {
                tracing::debug!(
                    job_id = %job.id,
                    container_type = %ctype,
                    item_id = %iid,
                    "[index_queue] 已入队索引任务",
                );
                let _ = app_handle.emit(
                    "index-job-queued",
                    serde_json::json!({
                        "jobId": job.id,
                        "jobType": jtype,
                        "containerType": ctype,
                        "containerId": cid,
                        "itemId": iid,
                    }),
                );
            },
            Err(e) => {
                tracing::error!(
                    container_type = %ctype,
                    item_id = %iid,
                    error = %e,
                    "[index_queue] 入队失败"
                );
                let _ = app_handle.emit(
                    "index-job-failed",
                    serde_json::json!({
                        "jobId": "",
                        "jobType": jtype,
                        "containerType": ctype,
                        "containerId": cid,
                        "itemId": iid,
                        "error": format!("入队失败: {}", e),
                    }),
                );
            },
        }
    });

    Ok(item_id.to_string())
}
