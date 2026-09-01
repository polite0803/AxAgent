// SPDX-License-Identifier: AGPL-3.0-only

//! 持久化索引队列服务。
//!
//! 解决三个核心问题：
//! 1. 应用重启后索引任务不丢失（持久化到 `index_jobs` 表）
//! 2. 失败任务自动重试（指数退避，最多 max_retries 次）
//! 3. 细粒度进度事件（parsing → chunking → embedding → storing）

use crate::AppState;
use axagent_dao::repo::index_jobs as jobs;
use axagent_harness::ExtractEntitiesResult;
use axagent_harness::prompt_provider::PromptLang;
use axagent_harness::util_fns::truncate_to_char_boundary;
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

        jobs::mark_job_processing(&self.db, &job.id, Some(jobs::STAGE_PARSING))
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(&job, jobs::STAGE_PARSING, 5).await;

        let result = match job.job_type.as_str() {
            jobs::JOB_TYPE_EXTRACT_ENTITIES => self.run_entity_extraction(&job).await,
            _ => self.run_indexing(&job).await,
        };

        match result {
            Ok(()) => {
                let _ = jobs::mark_job_completed(&self.db, &job.id).await;
                self.emit_completed(&job).await;
                tracing::info!(job_id = %job.id, "[index_queue] 任务完成");
            },
            Err(e) => {
                let err_msg = e.to_string();

                // R9: embedding provider 未配置属确定性配置错误，重试结果必然相同，
                // 直接进入 failed 终态，避免指数退避空转 max_retries 次。
                if err_msg.contains(axagent_search::rag::ERR_NO_EMBEDDING_PROVIDER) {
                    match jobs::mark_job_failed_no_retry(&self.db, &job.id, &err_msg).await {
                        Ok(_) => {
                            self.emit_failed(&job, &err_msg).await;
                            self.mark_item_error(&job, &err_msg).await;
                            tracing::error!(
                                job_id = %job.id,
                                error = %err_msg,
                                "[index_queue] embedding 未配置，任务不重试直接失败"
                            );
                        },
                        Err(e2) => {
                            tracing::error!(
                                job_id = %job.id,
                                error = %e2,
                                "[index_queue] 标记任务终态失败时出错"
                            );
                        },
                    }
                    return Ok(());
                }

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

        // 知识库文档 / Wiki 笔记索引完成后自动触发实体抽取
        // （R6：Wiki 此前无自动抽取，编辑后图谱长期陈旧；enqueue_job 按
        //   container_type+item_id 活跃去重，同一 vault 只有一个进行中的抽取任务）
        if matches!(
            container_type,
            rag::ContainerType::KnowledgeBase | rag::ContainerType::WikiVault
        ) {
            self.enqueue_entity_extraction(job).await;
        }

        Ok(())
    }

    /// 执行实体抽取任务：按容器类型分发 —— KB 从向量库 chunks 抽取，
    /// Wiki 从笔记表抽取（写入各带来源标记）。
    async fn run_entity_extraction(&self, job: &jobs::IndexJob) -> Result<(), String> {
        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_EXTRACTING), 5)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_EXTRACTING, 5).await;

        let result = match job.container_type.as_str() {
            "wiki" => {
                run_wiki_entity_extraction_core(&self.db, &self.master_key, &job.container_id, None)
                    .await
            },
            _ => {
                run_entity_extraction_core(
                    &self.db,
                    &self.vector_store,
                    &self.master_key,
                    &job.container_id,
                )
                .await
            },
        };

        jobs::update_job_progress(&self.db, &job.id, Some(jobs::STAGE_EXTRACTING), 100)
            .await
            .map_err(|e| e.to_string())?;
        self.emit_progress(job, jobs::STAGE_EXTRACTING, 100).await;

        result.map(|_| ())
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

    /// 文档索引完成后自动入队实体抽取任务（幂等：同一 KB 只有一个活跃的抽取任务）
    async fn enqueue_entity_extraction(&self, job: &jobs::IndexJob) {
        let metadata = serde_json::json!({
            "auto_extract": true,
        });
        let meta_str = serde_json::to_string(&metadata).unwrap_or_default();

        let input = jobs::CreateIndexJobInput {
            job_type: jobs::JOB_TYPE_EXTRACT_ENTITIES.to_string(),
            container_type: job.container_type.clone(),
            container_id: job.container_id.clone(),
            item_id: job.container_id.clone(),
            max_retries: Some(1),
            priority: Some(0),
            metadata: Some(meta_str),
        };

        match jobs::enqueue_job(&self.db, input).await {
            Ok(j) => {
                tracing::info!(
                    job_id = %j.id,
                    container_id = %job.container_id,
                    "[index_queue] 已入队实体抽取任务",
                );
                let _ = self.app.emit(
                    "index-job-queued",
                    serde_json::json!({
                        "jobId": j.id,
                        "jobType": jobs::JOB_TYPE_EXTRACT_ENTITIES,
                        "containerType": job.container_type,
                        "containerId": job.container_id,
                        "itemId": job.container_id,
                    }),
                );
            },
            Err(e) => {
                tracing::warn!(
                    container_id = %job.container_id,
                    error = %e,
                    "[index_queue] 实体抽取入队失败",
                );
            },
        }
    }
}

/// 跨文档实体抽取核心逻辑：加载 KB 下所有 ready 文档，分批调用 LLM 抽取实体/关系并写入 DB。
///
/// 手动触发命令（`extract_entities_for_kb`）与索引队列 worker 共用，避免逻辑重复。
/// 返回聚合的抽取结果，含新增/更新实体与关系计数。
pub(crate) async fn run_entity_extraction_core(
    db: &DatabaseConnection,
    vector_store: &Arc<VectorStore>,
    master_key: &[u8; 32],
    kb_id: &str,
) -> Result<ExtractEntitiesResult, String> {
    let collection_id = format!("kb_{}", kb_id);

    // 1. 获取该 KB 下所有已索引的文档
    let docs = axagent_dao::repo::knowledge::list_documents(db, kb_id)
        .await
        .map_err(|e| format!("获取文档列表失败: {}", e))?;

    let ready_docs: Vec<_> = docs.iter().filter(|d| d.indexing_status == "ready").collect();

    if ready_docs.is_empty() {
        tracing::info!(kb_id = %kb_id, "[index_queue] 没有可抽取的文档");
        return Ok(ExtractEntitiesResult {
            new_entities: Vec::new(),
            updated_entities: Vec::new(),
            new_relations: Vec::new(),
            skipped_chunks: 0,
            elapsed_ms: 0,
        });
    }

    // 2. 分批组装文本（每批 BATCH_SIZE 个文档，16KB 截断防 context 爆炸）
    const MAX_EXTRACT_TEXT_BYTES: usize = 16_000;
    const BATCH_SIZE: usize = 20;

    let mut batch_texts: Vec<String> = Vec::new();
    for doc_batch in ready_docs.chunks(BATCH_SIZE) {
        let mut all_text = String::new();
        'doc_batch: for doc in doc_batch {
            let chunks = vector_store
                .list_document_chunks(&collection_id, &doc.id)
                .await
                .map_err(|e| format!("加载 chunks 失败: {}", e))?;
            for chunk in &chunks {
                all_text.push_str(&chunk.content);
                all_text.push_str("\n\n");
                if all_text.len() >= MAX_EXTRACT_TEXT_BYTES {
                    let truncated = truncate_to_char_boundary(&all_text, MAX_EXTRACT_TEXT_BYTES);
                    all_text = truncated.to_string();
                    break 'doc_batch;
                }
            }
        }
        batch_texts.push(all_text);
    }

    // 3. 共享执行器：LLM 抽取 + 写入（KB 来源）
    run_entity_extraction_batches(db, master_key, kb_id, "knowledge_base", "", batch_texts).await
}

/// 实体抽取共享执行器：对预组装的文本批次逐批调用 LLM 抽取实体/关系并写入 DB。
///
/// KB 抽取（`run_entity_extraction_core`，文本来自向量库 chunks）与
/// Wiki 抽取（`run_wiki_entity_extraction_core`，文本来自笔记表）共用，
/// 写入时通过 `source_type` / `source_id` 落 v113 统一图谱来源字段
/// （KB 传 `("knowledge_base", "")`，Wiki 传 `("wiki", wiki_id)`），
/// 避免 Wiki 实体被误标为 knowledge_base 而混入 KB 图谱。
async fn run_entity_extraction_batches(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    graph_kb_id: &str,
    source_type: &str,
    source_id: &str,
    batch_texts: Vec<String>,
) -> Result<ExtractEntitiesResult, String> {
    let started = std::time::Instant::now();
    let mut aggregate = ExtractEntitiesResult {
        new_entities: Vec::new(),
        updated_entities: Vec::new(),
        new_relations: Vec::new(),
        skipped_chunks: 0,
        elapsed_ms: 0,
    };

    if batch_texts.is_empty() {
        return Ok(aggregate);
    }

    let existing_entities =
        axagent_dao::repo::knowledge_graph::get_all_entities_by_kb(db, graph_kb_id)
            .await
            .map_err(|e| e.to_string())?;
    let existing_names: Vec<String> =
        existing_entities.iter().take(50).map(|e| e.name.clone()).collect();

    let system_prompt = axagent_kit::prompts::PromptRegistry::get(
        "entity_extraction.document_system_prompt",
        PromptLang::ZhCN,
    );
    let user_template = axagent_kit::prompts::PromptRegistry::get(
        "entity_extraction.document_user_template",
        PromptLang::ZhCN,
    );
    let existing_hint = if existing_names.is_empty() {
        String::new()
    } else {
        format!("\n\n[已有实体名称（可引用，勿重复抽取）]\n{}", existing_names.join(", "))
    };

    let bridge = axagent_runtime::llm_bridge::build_llm_bridge_from_db(master_key)
        .await
        .ok_or_else(|| "未找到启用的 LLM Provider，无法执行实体抽取".to_string())?;

    for batch_text in batch_texts {
        if batch_text.trim().is_empty() {
            continue;
        }

        let user_prompt = user_template
            .replace("{document_content}", &format!("{}{}", batch_text, existing_hint));
        let llm_response = bridge
            .call_llm(system_prompt, &user_prompt)
            .await
            .map_err(|e| format!("LLM 实体抽取调用失败: {}", e))?;

        let (entities, relations) =
            crate::commands::knowledge_graph::parse_entity_extraction_response(&llm_response)?;

        if !entities.is_empty() || !relations.is_empty() {
            let batch_result =
                axagent_dao::repo::knowledge_graph::batch_upsert_entities_and_relations(
                    db,
                    graph_kb_id,
                    source_type,
                    source_id,
                    entities,
                    relations,
                )
                .await
                .map_err(|e| format!("批量写入实体/关系失败: {}", e))?;
            aggregate.new_entities.extend(batch_result.new_entities);
            aggregate.updated_entities.extend(batch_result.updated_entities);
            aggregate.new_relations.extend(batch_result.new_relations);
            aggregate.skipped_chunks += batch_result.skipped_chunks;
        }
    }

    aggregate.elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(aggregate)
}

/// Wiki 实体抽取核心逻辑：加载 vault 笔记，分批调用 LLM 抽取实体/关系并写入 DB。
///
/// 手动触发命令（`extract_entities_from_wiki`）与索引队列 worker 共用。
/// 与 KB 抽取的差异：文本来自笔记表（非向量库 chunks），写入带 `("wiki", wiki_id)`
/// 来源标记（v113 统一图谱字段），与真实 KB 实体可区分。
pub(crate) async fn run_wiki_entity_extraction_core(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    wiki_id: &str,
    note_ids: Option<Vec<String>>,
) -> Result<ExtractEntitiesResult, String> {
    const MAX_EXTRACT_TEXT_BYTES: usize = 16_000;

    // 1. 加载 notes（指定 ids 或整个 vault）
    let notes = match note_ids {
        Some(ids) if ids.is_empty() => Vec::new(),
        Some(ids) => axagent_dao::repo::note::get_notes_by_ids(db, &ids)
            .await
            .map_err(|e| format!("加载笔记失败: {}", e))?,
        None => axagent_dao::repo::note::list_notes(db, wiki_id)
            .await
            .map_err(|e| format!("加载笔记列表失败: {}", e))?,
    };

    if notes.is_empty() {
        tracing::info!(wiki_id = %wiki_id, "[index_queue] 没有可抽取的 Wiki 笔记");
        return Ok(ExtractEntitiesResult {
            new_entities: Vec::new(),
            updated_entities: Vec::new(),
            new_relations: Vec::new(),
            skipped_chunks: 0,
            elapsed_ms: 0,
        });
    }

    // 2. 组装批次文本：逐笔记拼接（标题作上下文），单笔记超限多字节安全截断，16KB 一批
    let mut batch_texts: Vec<String> = Vec::new();
    let mut current = String::new();
    for note in &notes {
        let mut piece = format!("# {}\n\n{}\n\n---\n\n", note.title, note.content);
        if piece.len() >= MAX_EXTRACT_TEXT_BYTES {
            let truncated = truncate_to_char_boundary(&piece, MAX_EXTRACT_TEXT_BYTES);
            piece = truncated.to_string();
        }
        if !current.is_empty() && current.len() + piece.len() >= MAX_EXTRACT_TEXT_BYTES {
            batch_texts.push(std::mem::take(&mut current));
        }
        current.push_str(&piece);
    }
    if !current.trim().is_empty() {
        batch_texts.push(current);
    }

    // 3. 共享执行器：LLM 抽取 + 写入（Wiki 来源）
    run_entity_extraction_batches(db, master_key, wiki_id, "wiki", wiki_id, batch_texts).await
}

#[allow(clippy::too_many_arguments)]
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
