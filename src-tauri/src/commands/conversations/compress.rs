// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::AppState;
use crate::commands::error_code::conv_err;
#[cfg(test)]
use crate::app_state::SemanticCacheState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::session as session_err;
#[cfg(test)]
use crate::commands::proactive::ProactiveService;
use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;
use axagent_providers::ProviderRequestContext;
#[cfg(test)]
use axagent_runtime_core::prompt_cache::PromptCache;
use sea_orm::*;
#[cfg(test)]
use std::collections::HashMap;
use super::{chat_message_from_message};

use tauri::{Emitter, State};

pub async fn list_message_versions(
    state: State<'_, AppState>,
    conversation_id: String,
    parent_message_id: String,
) -> Result<Vec<Message>, String> {
    axagent_core::repo::message::list_message_versions(
        state.harness.db(),
        &conversation_id,
        &parent_message_id,
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn switch_message_version(
    state: State<'_, AppState>,
    conversation_id: String,
    parent_message_id: String,
    message_id: String,
) -> Result<(), String> {
    axagent_core::repo::message::set_active_version(
        state.harness.db(),
        &conversation_id,
        &parent_message_id,
        &message_id,
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn delete_message_group(
    state: State<'_, AppState>,
    conversation_id: String,
    user_message_id: String,
) -> Result<(), String> {
    let deleted =
        axagent_core::repo::message::delete_message_group(state.harness.db(), &user_message_id)
            .await
            .map_err(|e| e.to_string())?;
    // Decrement message count by deleted count
    for _ in 0..deleted {
        axagent_core::repo::conversation::decrement_message_count(
            state.harness.db(),
            &conversation_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Internal helper: call LLM to compress messages into a summary and persist it.
pub(crate) async fn do_compress(
    db: &sea_orm::DatabaseConnection,
    ctx: CompressContext<'_>,
    provider_info: CompressProviderInfo<'_>,
    harness: &axagent_runtime::harness::RuntimeHarness,
) -> Result<String, String> {
    let CompressContext {
        conversation_id,
        history_messages,
        existing_summary,
        settings,
        master_key,
    } = ctx;
    let CompressProviderInfo {
        provider,
        decrypted_key,
        key_id,
        proxy_config,
        model_id,
        use_max_completion_tokens,
    } = provider_info;
    // Resolve compression model: settings override → fallback to conversation model
    let (comp_provider, comp_key, comp_key_id, comp_proxy, comp_model_id, comp_use_max) = if let (
        Some(pid),
        Some(mid),
    ) =
        (&settings.compression_provider_id, &settings.compression_model_id)
    {
        match axagent_core::repo::provider::get_provider(db, pid).await {
            Ok(p) => match p.keys.first() {
                Some(k) => {
                    let dk = axagent_core::crypto::decrypt_key(&k.key_encrypted, master_key)
                        .map_err(|e| e.to_string())?;
                    let kid = k.id.clone();
                    let proxy = ProviderProxyConfig::resolve(&p.proxy_config, settings);
                    let override_umc = axagent_core::repo::provider::get_model(db, pid, mid)
                        .await
                        .ok()
                        .and_then(|m| m.param_overrides)
                        .and_then(|po| po.use_max_completion_tokens);
                    (p, dk, kid, proxy, mid.clone(), override_umc)
                },
                None => {
                    tracing::warn!(
                        "Compression model provider has no key, falling back to conversation model"
                    );
                    (
                        provider.clone(),
                        decrypted_key.to_string(),
                        key_id.to_string(),
                        proxy_config.clone(),
                        model_id.to_string(),
                        use_max_completion_tokens,
                    )
                },
            },
            Err(_) => {
                tracing::warn!(
                    "Compression model provider not found, falling back to conversation model"
                );
                (
                    provider.clone(),
                    decrypted_key.to_string(),
                    key_id.to_string(),
                    proxy_config.clone(),
                    model_id.to_string(),
                    use_max_completion_tokens,
                )
            },
        }
    } else {
        (
            provider.clone(),
            decrypted_key.to_string(),
            key_id.to_string(),
            proxy_config.clone(),
            model_id.to_string(),
            use_max_completion_tokens,
        )
    };

    let sum_req = crate::context_manager::SummarizationRequest {
        existing_summary: existing_summary.map(|s| s.to_string()),
        messages_to_compress: history_messages.to_vec(),
    };

    let custom_prompt = settings.compression_prompt.as_deref();
    let summary_messages = if let Some(prompt) = custom_prompt {
        crate::context_manager::build_summary_prompt_with_custom(&sum_req, prompt)
    } else {
        crate::context_manager::build_summary_prompt(&sum_req)
    };

    let request = ChatRequest {
        model: comp_model_id.clone(),
        messages: summary_messages,
        stream: false,
        temperature: settings
            .compression_temperature
            .map(|v| v as f64)
            .or(Some(0.3)),
        top_p: settings.compression_top_p.map(|v| v as f64),
        max_tokens: settings.compression_max_tokens.or(Some(1024)),
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: comp_use_max,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };

    let ctx = ProviderRequestContext {
        api_key: comp_key,
        key_id: comp_key_id,
        provider_id: comp_provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(
            &comp_provider.api_host,
            &comp_provider.provider_type,
        )),
        api_path: comp_provider.api_path.clone(),
        proxy_config: comp_proxy,
        custom_headers: comp_provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry_key = comp_provider.provider_type.registry_key();
    let adapter = harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| "Provider adapter not found".to_string())?;

    let response = adapter
        .chat(&ctx, request)
        .await
        .map_err(|e| ErrorResponse::new(conv_err::INTERNAL).with_detail(format!("Summary generation failed: {}", e)))?;

    let token_count = axagent_core::token_counter::estimate_tokens(&response.content);
    axagent_core::repo::conversation::upsert_summary(
        db,
        conversation_id,
        &response.content,
        None,
        Some(token_count as u32),
        Some(&comp_model_id),
    )
    .await
    .map_err(|e| ErrorResponse::new(conv_err::INTERNAL).with_detail(format!("Failed to save summary: {}", e)))?;

    tracing::debug!("Compressed context for {} ({} tokens)", conversation_id, token_count);
    Ok(response.content)
}

/// Tauri command: manually compress the current conversation context.
///
/// Returns the generated summary text and inserts a compression marker.
pub async fn compress_context(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    let conversation =
        axagent_core::repo::conversation::get_conversation(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;

    // Get provider + key
    let provider =
        axagent_core::repo::provider::get_provider(state.harness.db(), &conversation.provider_id)
            .await
            .map_err(|e| e.to_string())?;
    let key_row = provider
        .keys
        .first()
        .ok_or_else(|| "No API key configured".to_string())?;
    let decrypted_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| e.to_string())?;

    let global_settings = axagent_core::repo::settings::get_settings(state.harness.db())
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    // Load messages after last marker
    let db_messages =
        axagent_core::repo::message::list_messages(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?;

    let file_store = axagent_core::file_store::FileStore::new();

    // For manual compression: try messages after last marker first,
    // fall back to ALL messages if nothing after marker
    let marker_idx = db_messages.iter().rposition(|m| {
        m.role == MessageRole::System
            && (m.content == "<!-- context-clear -->"
                || m.content == crate::context_manager::COMPRESSION_MARKER)
    });

    let collect_messages = |msgs: &[Message]| -> Result<Vec<ChatMessage>, String> {
        let mut out = Vec::new();
        for m in msgs {
            if m.role == MessageRole::System
                && (m.content == "<!-- context-clear -->"
                    || m.content == crate::context_manager::COMPRESSION_MARKER)
            {
                continue;
            }
            if m.role == MessageRole::Tool {
                continue;
            }
            if m.role == MessageRole::Assistant && m.tool_calls_json.is_some() {
                continue;
            }
            out.push(chat_message_from_message(&file_store, m).map_err(|e| e.to_string())?);
        }
        Ok(out)
    };

    let mut history_messages = match marker_idx {
        Some(idx) => collect_messages(&db_messages[idx + 1..])?,
        None => collect_messages(&db_messages)?,
    };

    // If nothing after the last marker, try all messages
    if history_messages.is_empty() && marker_idx.is_some() {
        history_messages = collect_messages(&db_messages)?;
    }

    if history_messages.is_empty() {
        return Err(ErrorResponse::err(session_err::NO_MESSAGES));
    }

    // Load existing summary
    let existing_summary =
        axagent_core::repo::conversation::get_summary(state.harness.db(), &conversation_id)
            .await
            .ok()
            .flatten();

    // Compress
    let use_max_completion_tokens = axagent_core::repo::provider::get_model(
        state.harness.db(),
        &conversation.provider_id,
        &conversation.model_id,
    )
    .await
    .ok()
    .and_then(|m| m.param_overrides)
    .and_then(|p| p.use_max_completion_tokens);

    do_compress(
        state.harness.db(),
        CompressContext {
            conversation_id: &conversation_id,
            history_messages: &history_messages,
            existing_summary: existing_summary.as_ref().map(|s| s.summary_text.as_str()),
            settings: &global_settings,
            master_key: state.harness.master_key(),
        },
        CompressProviderInfo {
            provider: &provider,
            decrypted_key: &decrypted_key,
            key_id: &key_row.id,
            proxy_config: &resolved_proxy,
            model_id: &conversation.model_id,
            use_max_completion_tokens,
        },
        &state.harness,
    )
    .await?;

    // Insert compression marker message
    let marker_msg = axagent_core::repo::message::create_message(
        state.harness.db(),
        &conversation_id,
        MessageRole::System,
        crate::context_manager::COMPRESSION_MARKER,
        &[],
        None,
        0,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Emit events to frontend
    let _ = app.emit(&format!("conversation:compressed:{}", conversation_id), &marker_msg);

    // Return the updated summary
    let summary =
        axagent_core::repo::conversation::get_summary(state.harness.db(), &conversation_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Summary not found after compression".to_string())?;

    Ok(summary)
}

/// Tauri command: get the compression summary for a conversation.
pub async fn get_compression_summary(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Option<ConversationSummary>, String> {
    axagent_core::repo::conversation::get_summary(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: delete the compression summary and all marker messages.
pub async fn delete_compression(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    // Delete the summary
    axagent_core::repo::conversation::delete_summary(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    // Delete all compression marker messages
    axagent_core::entity::messages::Entity::delete_many()
        .filter(axagent_core::entity::messages::Column::ConversationId.eq(&conversation_id))
        .filter(
            axagent_core::entity::messages::Column::Content
                .eq(crate::context_manager::COMPRESSION_MARKER),
        )
        .exec(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn send_system_message(
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
) -> Result<Message, String> {
    let msg = axagent_core::repo::message::create_message(
        state.harness.db(),
        &conversation_id,
        MessageRole::System,
        &content,
        &[],
        None,
        0,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(msg)
}

#[cfg(test)]
mod tests_conversation {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use tokio::sync::Mutex;

    #[test]
    fn build_message_content_turns_images_into_multipart_data_urls() {
        let temp_dir = std::env::temp_dir()
            .join(format!("axagent-vision-test-{}", axagent_core::utils::gen_id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let result = {
            let file_store = axagent_core::file_store::FileStore::with_root(temp_dir.clone());
            let saved = file_store
                .save_file(b"abc", "image.png", "image/png")
                .unwrap();
            let message = Message {
                id: "msg-1".into(),
                conversation_id: "conv-1".into(),
                role: MessageRole::User,
                content: "Describe this image".into(),
                provider_id: None,
                model_id: None,
                token_count: None,
                prompt_tokens: None,
                completion_tokens: None,
                cache_creation_tokens: None,
                cache_read_tokens: None,
                attachments: vec![Attachment {
                    id: "att-1".into(),
                    file_type: "image/png".into(),
                    file_name: "image.png".into(),
                    file_path: saved.storage_path,
                    file_size: 3,
                    data: None,
                }],
                thinking: None,
                tool_calls_json: None,
                tool_call_id: None,
                created_at: 0,
                parent_message_id: None,
                version_index: 0,
                is_active: true,
                status: "done".into(),
                tokens_per_second: None,
                first_token_latency_ms: None,
                parts: None,
                blocks: None,
            };

            build_message_content(&file_store, &message).unwrap()
        };

        fs::remove_dir_all(&temp_dir).unwrap();

        match result {
            ChatContent::Multipart(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].text.as_deref(), Some("Describe this image"));
                assert_eq!(
                    parts[1].image_url.as_ref().map(|img| img.url.as_str()),
                    Some("data:image/png;base64,YWJj")
                );
            },
            ChatContent::Text(_) => panic!("expected multipart content"),
        }
    }

    #[test]
    fn build_message_content_uses_inline_attachment_data_when_file_path_is_missing() {
        let temp_dir = std::env::temp_dir()
            .join(format!("axagent-vision-test-{}", axagent_core::utils::gen_id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let result = {
            let file_store = axagent_core::file_store::FileStore::with_root(temp_dir.clone());
            let message = Message {
                id: "msg-1".into(),
                conversation_id: "conv-1".into(),
                role: MessageRole::User,
                content: "Old attachment".into(),
                provider_id: None,
                model_id: None,
                token_count: None,
                prompt_tokens: None,
                completion_tokens: None,
                cache_creation_tokens: None,
                cache_read_tokens: None,
                attachments: vec![Attachment {
                    id: String::new(),
                    file_type: "image/png".into(),
                    file_name: "image.png".into(),
                    file_path: String::new(),
                    file_size: 3,
                    data: Some("YWJj".into()),
                }],
                thinking: None,
                tool_calls_json: None,
                tool_call_id: None,
                created_at: 0,
                parent_message_id: None,
                version_index: 0,
                is_active: true,
                status: "done".into(),
                tokens_per_second: None,
                first_token_latency_ms: None,
                parts: None,
                blocks: None,
            };

            build_message_content(&file_store, &message).unwrap()
        };

        fs::remove_dir_all(&temp_dir).unwrap();

        match result {
            ChatContent::Multipart(parts) => {
                assert_eq!(
                    parts[1].image_url.as_ref().map(|img| img.url.as_str()),
                    Some("data:image/png;base64,YWJj")
                );
            },
            ChatContent::Text(_) => panic!("expected multipart content"),
        }
    }

    #[tokio::test]
    async fn delete_conversation_removes_attached_files_and_records() {
        let db = axagent_core::db::create_test_pool().await.unwrap().conn;
        let temp_dir = std::env::temp_dir()
            .join(format!("axagent-conv-delete-test-{}", axagent_core::utils::gen_id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let conversation = axagent_core::repo::conversation::create_conversation(
            &db,
            "Files cleanup",
            "model-1",
            "provider-1",
            None,
        )
        .await
        .unwrap();

        let file_store = axagent_core::file_store::FileStore::with_root(temp_dir.clone());
        let saved = file_store
            .save_file(b"cleanup me", "cleanup.png", "image/png")
            .unwrap();
        let physical_path = temp_dir.join(&saved.storage_path);
        assert!(
            physical_path.exists(),
            "fixture file must exist before deleting the conversation"
        );

        axagent_core::repo::stored_file::create_stored_file(
            &db,
            "file-1",
            &saved.hash,
            "cleanup.png",
            "image/png",
            saved.size_bytes,
            &saved.storage_path,
            Some(&conversation.id),
        )
        .await
        .unwrap();

        let result =
            delete_conversation_with_attachments_using(&db, &file_store, &conversation.id).await;
        assert!(
            result.is_ok(),
            "deleting a conversation should clean up its attached files, got: {result:?}"
        );
        assert!(
            axagent_core::repo::conversation::get_conversation(&db, &conversation.id)
                .await
                .is_err(),
            "conversation must be deleted"
        );
        assert!(
            axagent_core::repo::stored_file::list_stored_files_by_conversation(
                &db,
                &conversation.id
            )
            .await
            .unwrap()
            .is_empty(),
            "conversation attachments must be removed from the database"
        );
        assert!(
            !physical_path.exists(),
            "conversation deletion must remove the backing attachment file from disk"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::disallowed_types)]
    async fn persist_attachments_registers_stored_files_for_files_page() {
        use base64::Engine;

        let db = axagent_core::db::create_test_pool().await.unwrap().conn;
        let temp_dir = std::env::temp_dir()
            .join(format!("axagent-persist-attachments-test-{}", axagent_core::utils::gen_id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let conversation = axagent_core::repo::conversation::create_conversation(
            &db,
            "Image indexing",
            "model-1",
            "provider-1",
            None,
        )
        .await
        .unwrap();

        let vector_store = Arc::new(axagent_core::vector_store::VectorStore::new(db.clone()));
        let memory_service = {
            let storage =
                axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(db.clone()));
            let ms = axagent_trajectory::MemoryService::new(std::sync::Arc::new(storage))
                .unwrap_or_else(|e| panic!("Failed to create MemoryService: {}", e));
            if let Err(e) = ms.initialize() {
                panic!("Failed to initialize MemoryService: {}", e);
            }
            Arc::new(tokio::sync::RwLock::new(ms))
        };
        let pattern_learner = Arc::new(tokio::sync::RwLock::new(
            axagent_trajectory::PatternLearner::new(axagent_trajectory::PatternConfig::default()),
        ));
        let trajectory_storage =
            Arc::new(axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(db.clone())));
        let semantic_cache = Arc::new(tokio::sync::Mutex::new(SemanticCacheState {
            cache: crate::semantic_cache::SemanticCache::new(
                db.clone(),
                crate::semantic_cache::CacheConfig::default(),
            )
            .await
            .expect("Failed to create semantic cache"),
            enabled: true,
            in_memory_entries: Vec::new(),
            similarity_threshold: 0.85,
        }));
        let state = crate::AppState {
            gateway: Arc::new(Mutex::new(None)),
            close_to_tray: Arc::new(AtomicBool::new(false)),
            app_data_dir: temp_dir.clone(),
            auto_backup_handle: Arc::new(Mutex::new(None)),
            webdav_sync_handle: Arc::new(Mutex::new(None)),
            api_server_handle: Arc::new(Mutex::new(None)),
            trajectory_cleanup_handle: Arc::new(Mutex::new(None)),
            task_manager: Arc::new(axagent_runtime::task_manager::TaskManager::new()),
            skill_watcher_shutdown: std::sync::OnceLock::new(),
            vector_store: vector_store.clone(),
            indexing_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
            stream_cancel_flags: Arc::new(DashMap::new()),
            agent_permission_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            agent_ask_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            agent_always_allowed: Arc::new(Mutex::new(std::collections::HashMap::new())),
            agent_prompters: Arc::new(Mutex::new(std::collections::HashMap::new())),
            agent_session_manager: Arc::new(axagent_agent::SessionManager::new(db.clone())),
            agent_cancel_tokens: Arc::new(DashMap::new()),
            agent_paused: Arc::new(Mutex::new(std::collections::HashSet::new())),
            running_agents: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
            reflector: Arc::new(axagent_agent::Reflector::new()),
            shared_memory: Arc::new(tokio::sync::RwLock::new(
                axagent_runtime::shared_memory::SharedMemory::new(),
            )),
            sub_agent_registry: Arc::new(tokio::sync::RwLock::new(
                axagent_trajectory::SubAgentRegistry::new().unwrap_or_default(),
            )),
            trajectory_storage: trajectory_storage.clone(),
            memory_service: memory_service.clone(),
            nudge_service: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::NudgeService::new(),
            )),
            closed_loop_service: {
                let storage =
                    axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(db.clone()));
                Arc::new(axagent_trajectory::ClosedLoopService::new(std::sync::Arc::new(storage)))
            },
            insight_system: Arc::new(tokio::sync::RwLock::new(
                axagent_trajectory::LearningInsightSystem::new().with_storage_limits(200, 30),
            )),
            realtime_learning: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::RealTimeLearning::new(),
            )),
            pattern_learner: pattern_learner.clone(),
            cross_session_learner: Arc::new(tokio::sync::RwLock::new(
                axagent_trajectory::CrossSessionLearner::new(),
            )),
            rl_engine: Arc::new(tokio::sync::RwLock::new(axagent_trajectory::RLEngine::new(
                axagent_trajectory::RLConfig::default(),
                axagent_trajectory::RewardWeights::default(),
            ))),
            batch_processor: {
                let storage =
                    axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(db.clone()));
                Arc::new(axagent_trajectory::BatchProcessor::new(
                    std::sync::Arc::new(storage),
                    axagent_trajectory::BatchConfig::default(),
                ))
            },
            skill_evolution_engine: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::SkillEvolutionEngine::new(),
            )),
            skill_proposal_service: Arc::new(tokio::sync::RwLock::new(
                axagent_trajectory::SkillProposalService::new(Arc::new(
                    axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(db.clone())),
                )),
            )),
            auto_memory_extractor: Arc::new(tokio::sync::RwLock::new(
                axagent_trajectory::AutoMemoryExtractor::new(
                    Arc::new(axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(
                        db.clone(),
                    ))),
                    memory_service.clone(),
                    pattern_learner.clone(),
                ),
            )),
            parallel_execution_service: Arc::new(tokio::sync::RwLock::new(
                axagent_trajectory::ParallelExecutionService::new(10),
            )),
            cron_job_store: Arc::new(axagent_runtime_core::CronJobStore::new_ephemeral()),
            platform_manager: Arc::new(
                axagent_runtime::message_gateway::platform_manager::PlatformManager::new(),
            ),
            platform_bridge: Arc::new(
                axagent_runtime::message_gateway::platform_bridge::PlatformBridge::new(
                    db.clone(),
                    [0; 32],
                    Arc::new(
                        axagent_runtime::message_gateway::platform_manager::PlatformManager::new(),
                    ),
                ),
            ),
            user_profile: Arc::new(
                tokio::sync::RwLock::new(axagent_trajectory::UserProfile::new()),
            ),
            local_tool_registry: Arc::new(tokio::sync::Mutex::new(
                axagent_tools::registry::UnifiedToolRegistry::new(),
            )),
            work_engine: Arc::new(axagent_runtime::work_engine::WorkEngine::new(
                Arc::new(db.clone()),
                [0; 32],
                Arc::new(axagent_providers::registry::ProviderRegistry::create_default())
                    as Arc<dyn axagent_harness::registry::ProviderRegistry>,
            )),
            skill_decomposer: Arc::new(tokio::sync::RwLock::new(
                axagent_trajectory::SkillDecomposer::new(),
            )),
            proactive_service: Arc::new(tokio::sync::RwLock::new(ProactiveService::new())),
            dashboard_registry: None,
            webhook_subscription_manager: None,
            semantic_cache: semantic_cache.clone(),
            prompt_cache: Arc::new(PromptCache::new()),
            harness: axagent_runtime::harness::RuntimeHarness::new(
                axagent_runtime::harness::HarnessDeps {
                    persistence: Arc::new(axagent_core::db::DbHandle {
                        conn: db.clone(),
                        path: ":memory:".into(),
                    }) as Arc<dyn axagent_harness::Persistence>,
                    master_key: [0; 32],
                    provider_registry: Arc::new(
                        axagent_providers::registry::ProviderRegistry::create_default(),
                    )
                        as Arc<dyn axagent_harness::registry::ProviderRegistry>,
                },
            ),
            tot_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            planner_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            #[cfg(not(target_os = "android"))]
            browser_client: Arc::new(tokio::sync::Mutex::new(None)),
            #[cfg(target_os = "android")]
            browser_client: Arc::new(tokio::sync::Mutex::new(None)),
            dream_consolidator: Arc::new(axagent_trajectory::DreamConsolidator::new()),
            text_grad_engine: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::TextGradEngine::new(
                    axagent_trajectory::ComputationGraph::new(),
                    axagent_trajectory::TextGradConfig::default(),
                ),
            )),
            auto_tool_creator: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::AutoToolCreator::new(
                    axagent_trajectory::AutoToolCreatorConfig::default(),
                    Box::new(axagent_trajectory::DefaultLlmToolProvider::new()),
                    Box::new(axagent_trajectory::DefaultSandboxToolTester),
                ),
            )),
            intrinsic_motivation: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::IntrinsicMotivationEngine::new(
                    axagent_trajectory::IntrinsicMotivationConfig::default(),
                ),
            )),
            coevolution_env: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::CoevolutionEnvironment::new(
                    axagent_trajectory::CoevolutionConfig::default(),
                ),
            )),
            constitution: Arc::new(axagent_trajectory::ImmutableConstitution::new(
                vec![
                    axagent_trajectory::ConstitutionalRule::NoSelfModificationOfReward,
                    axagent_trajectory::ConstitutionalRule::NoCodeExecutionWithoutSandbox,
                    axagent_trajectory::ConstitutionalRule::PreserveUserIntent,
                    axagent_trajectory::ConstitutionalRule::MaxModificationSize(0.5),
                ],
                axagent_trajectory::ConstitutionConfig::default(),
            )),
            process_reward_model: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::ProcessRewardModel::default(),
            )),
            dream_data_provider: Arc::new(axagent_trajectory::TrajectoryDreamDataProvider::new(
                trajectory_storage.clone(),
            )),
            #[cfg(not(target_os = "android"))]
            sandbox_executor: Arc::new(
                axagent_trajectory::SkillSandboxExecutor::with_default_policy(),
            ),
            #[cfg(target_os = "android")]
            sandbox_executor: Arc::new(()),
            sync_engine: None,
            plugin_manager: Arc::new(tokio::sync::RwLock::new(
                axagent_plugins::PluginManager::new(axagent_plugins::PluginManagerConfig::new(
                    temp_dir.clone(),
                )),
            )),
            shutdown_token: tokio_util::sync::CancellationToken::new(),
            file_authorizer: Arc::new(axagent_core::file_authorizer::FileAuthorizer::new()),
            session_share_manager: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            // ── Phase 3 P1 Task 3.1: domain sub-states ──
            infra: crate::state::InfraState::new(
                axagent_runtime::harness::RuntimeHarness::new(
                    axagent_runtime::harness::HarnessDeps {
                        persistence: Arc::new(axagent_core::db::DbHandle {
                            conn: db.clone(),
                            path: ":memory:".into(),
                        })
                            as Arc<dyn axagent_harness::Persistence>,
                        master_key: [0; 32],
                        provider_registry: Arc::new(
                            axagent_providers::registry::ProviderRegistry::create_default(),
                        )
                            as Arc<dyn axagent_harness::registry::ProviderRegistry>,
                    },
                ),
                vector_store.clone(),
                Arc::new(tokio::sync::Semaphore::new(2)),
                Arc::new(axagent_core::file_authorizer::FileAuthorizer::new()),
                temp_dir.clone(),
            ),
            gateway_state: crate::state::GatewayState::new(Arc::new(Mutex::new(None))),
            task: crate::state::TaskState::new(
                Arc::new(axagent_runtime::task_manager::TaskManager::new()),
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                tokio_util::sync::CancellationToken::new(),
                Arc::new(AtomicBool::new(false)),
                Arc::new(DashMap::new()),
                Arc::new(Mutex::new(std::collections::HashMap::new())),
                Arc::new(Mutex::new(std::collections::HashMap::new())),
                Arc::new(Mutex::new(std::collections::HashMap::new())),
                Arc::new(Mutex::new(std::collections::HashMap::new())),
            ),
            agent: crate::state::AgentState::new(
                Arc::new(axagent_agent::SessionManager::new(db.clone())),
                Arc::new(DashMap::new()),
                Arc::new(Mutex::new(std::collections::HashSet::new())),
                Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
                Arc::new(axagent_agent::Reflector::new()),
                Arc::new(axagent_runtime::message_gateway::platform_manager::PlatformManager::new()),
                Arc::new(axagent_runtime::message_gateway::platform_bridge::PlatformBridge::new(
                    db.clone(),
                    [0; 32],
                    Arc::new(
                        axagent_runtime::message_gateway::platform_manager::PlatformManager::new(),
                    ),
                )),
                Arc::new(tokio::sync::Mutex::new(
                    axagent_tools::registry::UnifiedToolRegistry::new(),
                )),
                Arc::new(axagent_runtime::work_engine::WorkEngine::new(
                    Arc::new(db.clone()),
                    [0; 32],
                    Arc::new(axagent_providers::registry::ProviderRegistry::create_default())
                        as Arc<dyn axagent_harness::registry::ProviderRegistry>,
                )),
            ),
            memory: crate::state::MemoryState::new(
                Arc::new(tokio::sync::RwLock::new(
                    axagent_runtime::shared_memory::SharedMemory::new(),
                )),
                Arc::new(tokio::sync::RwLock::new(
                    axagent_trajectory::SubAgentRegistry::new().unwrap_or_default(),
                )),
                memory_service.clone(),
                Arc::new(tokio::sync::Mutex::new(axagent_trajectory::NudgeService::new())),
                {
                    let storage =
                        axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(db.clone()));
                    Arc::new(axagent_trajectory::ClosedLoopService::new(std::sync::Arc::new(
                        storage,
                    )))
                },
                trajectory_storage.clone(),
                Arc::new(tokio::sync::RwLock::new(
                    axagent_trajectory::LearningInsightSystem::new().with_storage_limits(200, 30),
                )),
                Arc::new(tokio::sync::Mutex::new(axagent_trajectory::RealTimeLearning::new())),
                pattern_learner.clone(),
                Arc::new(tokio::sync::RwLock::new(axagent_trajectory::CrossSessionLearner::new())),
                Arc::new(tokio::sync::RwLock::new(axagent_trajectory::RLEngine::new(
                    axagent_trajectory::RLConfig::default(),
                    axagent_trajectory::RewardWeights::default(),
                ))),
                {
                    let storage =
                        axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(db.clone()));
                    Arc::new(axagent_trajectory::BatchProcessor::new(
                        std::sync::Arc::new(storage),
                        axagent_trajectory::BatchConfig::default(),
                    ))
                },
                Arc::new(tokio::sync::RwLock::new(axagent_trajectory::AutoMemoryExtractor::new(
                    Arc::new(axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(
                        db.clone(),
                    ))),
                    memory_service.clone(),
                    pattern_learner.clone(),
                ))),
                Arc::new(tokio::sync::RwLock::new(
                    axagent_trajectory::ParallelExecutionService::new(10),
                )),
                Arc::new(axagent_runtime_core::CronJobStore::new_ephemeral()),
                Arc::new(tokio::sync::RwLock::new(axagent_trajectory::UserProfile::new())),
                semantic_cache.clone(),
                Arc::new(PromptCache::new()),
                Arc::new(axagent_trajectory::DreamConsolidator::new()),
                Arc::new(axagent_trajectory::TrajectoryDreamDataProvider::new(
                    trajectory_storage.clone(),
                )),
                Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            ),
            skill: crate::state::SkillState::new(
                Arc::new(tokio::sync::Mutex::new(axagent_trajectory::SkillEvolutionEngine::new())),
                Arc::new(tokio::sync::RwLock::new(axagent_trajectory::SkillProposalService::new(
                    Arc::new(axagent_trajectory::TrajectoryStorage::new(std::sync::Arc::new(
                        db.clone(),
                    ))),
                ))),
                Arc::new(tokio::sync::RwLock::new(axagent_trajectory::SkillDecomposer::new())),
                crate::state::SandboxExecutorField::Real(Arc::new(
                    axagent_trajectory::SkillSandboxExecutor::with_default_policy(),
                )),
                None,
                None,
                Arc::new(tokio::sync::RwLock::new(axagent_plugins::PluginManager::new(
                    axagent_plugins::PluginManagerConfig::new(temp_dir.clone()),
                ))),
                None,
                Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                crate::state::BrowserClientField::Real(Arc::new(tokio::sync::Mutex::new(None))),
                Arc::new(tokio::sync::Mutex::new(axagent_trajectory::TextGradEngine::new(
                    axagent_trajectory::ComputationGraph::new(),
                    axagent_trajectory::TextGradConfig::default(),
                ))),
                Arc::new(tokio::sync::Mutex::new(axagent_trajectory::AutoToolCreator::new(
                    axagent_trajectory::AutoToolCreatorConfig::default(),
                    Box::new(axagent_trajectory::DefaultLlmToolProvider::new()),
                    Box::new(axagent_trajectory::DefaultSandboxToolTester),
                ))),
                Arc::new(tokio::sync::Mutex::new(
                    axagent_trajectory::IntrinsicMotivationEngine::new(
                        axagent_trajectory::IntrinsicMotivationConfig::default(),
                    ),
                )),
                Arc::new(tokio::sync::Mutex::new(axagent_trajectory::CoevolutionEnvironment::new(
                    axagent_trajectory::CoevolutionConfig::default(),
                ))),
                Arc::new(axagent_trajectory::ImmutableConstitution::new(
                    vec![
                        axagent_trajectory::ConstitutionalRule::NoSelfModificationOfReward,
                        axagent_trajectory::ConstitutionalRule::NoCodeExecutionWithoutSandbox,
                        axagent_trajectory::ConstitutionalRule::PreserveUserIntent,
                        axagent_trajectory::ConstitutionalRule::MaxModificationSize(0.5),
                    ],
                    axagent_trajectory::ConstitutionConfig::default(),
                )),
                Arc::new(
                    tokio::sync::Mutex::new(axagent_trajectory::ProcessRewardModel::default()),
                ),
                Arc::new(tokio::sync::RwLock::new(ProactiveService::new())),
            ),
        };

        let attachments = vec![AttachmentInput {
            file_name: "screen.png".to_string(),
            file_type: "image/png".to_string(),
            file_size: 3,
            data: base64::engine::general_purpose::STANDARD.encode(b"abc"),
        }];

        axagent_core::storage_paths::set_documents_root(temp_dir.clone());

        let persisted = persist_attachments(&state, &conversation.id, &attachments)
            .await
            .unwrap();
        assert_eq!(persisted.len(), 1);
        assert!(
            persisted[0].file_path.starts_with("images/"),
            "storage path should start with images/ bucket, got: {}",
            persisted[0].file_path
        );

        let stored_files = axagent_core::repo::stored_file::list_all_stored_files(&db)
            .await
            .unwrap();
        assert_eq!(
            stored_files.len(),
            1,
            "persisted chat attachments must be indexed for the files page"
        );
        assert_eq!(stored_files[0].original_name, "screen.png");
        assert_eq!(stored_files[0].mime_type, "image/png");

        // Cleanup: remove file written to documents root
        let _ = axagent_core::file_store::FileStore::new().delete_file(&persisted[0].file_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
