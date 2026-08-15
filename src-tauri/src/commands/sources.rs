// SPDX-License-Identifier: AGPL-3.0-only

use agent_macro::agent_command;
use axagent_harness::types::CreateSourceInput;
use axagent_search::rag::{ContainerType, KnowledgeContainer};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSource {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub container_type: String,
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub sort_order: i32,
    pub enabled: bool,
}

impl From<KnowledgeContainer> for UnifiedSource {
    fn from(c: KnowledgeContainer) -> Self {
        let container_type = match c.container_type {
            ContainerType::KnowledgeBase => "knowledge".to_string(),
            ContainerType::Memory => "memory".to_string(),
            ContainerType::WikiVault => "wiki".to_string(),
        };
        UnifiedSource {
            id: c.id,
            name: c.name,
            description: c.description,
            container_type,
            embedding_provider: c.embedding_provider,
            embedding_dimensions: c.embedding_dimensions,
            retrieval_threshold: c.retrieval_threshold,
            retrieval_top_k: c.retrieval_top_k,
            icon_type: c.icon_type,
            icon_value: c.icon_value,
            sort_order: c.sort_order,
            enabled: c.enabled,
        }
    }
}

async fn fetch_all_sources(
    db: &sea_orm::DatabaseConnection,
    container_types: Option<&Vec<String>>,
) -> Vec<UnifiedSource> {
    let mut sources = Vec::new();

    let include_kb = container_types.is_none_or(|t| t.contains(&"knowledge".to_string()));
    let include_mem = container_types.is_none_or(|t| t.contains(&"memory".to_string()));
    let include_wiki = container_types.is_none_or(|t| t.contains(&"wiki".to_string()));

    if include_kb {
        match axagent_dao::repo::knowledge::list_knowledge_bases(db).await {
            Ok(kbs) => {
                for kb in kbs {
                    sources.push(UnifiedSource::from(KnowledgeContainer::from_knowledge_base(&kb)));
                }
            },
            Err(e) => tracing::warn!("Failed to list knowledge bases: {}", e),
        }
    }

    if include_mem {
        match axagent_dao::repo::memory::list_namespaces(db).await {
            Ok(nss) => {
                for ns in nss {
                    sources.push(UnifiedSource::from(KnowledgeContainer::from_memory_ns(&ns)));
                }
            },
            Err(e) => tracing::warn!("Failed to list memory namespaces: {}", e),
        }
    }

    if include_wiki {
        match axagent_dao::repo::wiki::list_wikis(db).await {
            Ok(wikis) => {
                for w in wikis {
                    sources.push(UnifiedSource::from(KnowledgeContainer::from_wiki(&w)));
                }
            },
            Err(e) => tracing::warn!("Failed to list wikis: {}", e),
        }
    }

    sources.sort_by_key(|a| a.sort_order);
    sources
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "列出所有知识源")]
#[tauri::command]
pub async fn list_all_sources(
    state: State<'_, AppState>,
    container_types: Option<Vec<String>>,
) -> Result<Vec<UnifiedSource>, String> {
    Ok(fetch_all_sources(state.harness.db(), container_types.as_ref()).await)
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "创建知识源")]
#[tauri::command]
pub async fn create_source(
    state: State<'_, AppState>,
    input: CreateSourceInput,
) -> Result<UnifiedSource, String> {
    let db = state.harness.db();

    // 如果未提供 embedding_provider，回退到系统默认 provider
    let mut embedding_provider = if input.embedding_provider.is_some() {
        input.embedding_provider
    } else {
        axagent_dao::repo::settings::get_settings(db).await.ok().and_then(|s| s.default_provider_id)
    };

    // 从源头杜绝旧格式：仅 provider_id（或回退得到的 default_provider_id）时
    // 自动补全为完整 "providerId::model_id"，避免容器存入不完整的嵌入配置
    // （运行时 indexing::resolve_embedding_provider 仍会兜底，但每次补全都有 WARN 提示）。
    // 补全失败时保留原值不阻断创建流程，由运行时兜底。
    if let Some(p) = embedding_provider.clone() {
        if !p.contains("::") {
            match crate::indexing::resolve_embedding_provider(db, &p).await {
                Ok((resolved, _)) => embedding_provider = Some(resolved),
                Err(e) => {
                    tracing::warn!("[sources] 嵌入配置自动补全失败，保留原值 '{p}'：{e}");
                },
            }
        }
    }

    match input.source_type.as_str() {
        "knowledge" => {
            let kb = axagent_dao::repo::knowledge::create_knowledge_base(
                db,
                axagent_harness::types::CreateKnowledgeBaseInput {
                    name: input.name,
                    description: input.description,
                    embedding_provider,
                    enabled: Some(true),
                    kind: axagent_harness::KbKind::Indexed,
                    vault_path: None,
                },
            )
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            Ok(UnifiedSource::from(KnowledgeContainer::from_knowledge_base(&kb)))
        },
        "memory" => {
            let ns = axagent_dao::repo::memory::create_namespace(
                db,
                axagent_harness::types::CreateMemoryNamespaceInput {
                    name: input.name,
                    scope: input.scope.unwrap_or_else(|| "global".to_string()),
                    embedding_provider,
                    embedding_dimensions: None,
                    retrieval_threshold: None,
                    retrieval_top_k: None,
                    icon_type: None,
                    icon_value: None,
                },
            )
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            Ok(UnifiedSource::from(KnowledgeContainer::from_memory_ns(&ns)))
        },
        "wiki" => {
            let wiki = axagent_dao::repo::wiki::create_wiki(
                db,
                axagent_dao::repo::wiki::CreateWikiInput {
                    name: input.name,
                    description: input.description,
                    root_path: input
                        .root_path
                        .clone()
                        .ok_or_else(|| "wiki requires root_path".to_string())?,
                    embedding_provider,
                    knowledge_base_id: None,
                },
            )
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            Ok(UnifiedSource::from(KnowledgeContainer::from_wiki(&wiki)))
        },
        "obsidian_vault" => {
            // ConnectedVault 类型 KB：指针指向外部 Obsidian vault，不走 RAG 索引
            let vault_path = input
                .vault_path
                .clone()
                .ok_or_else(|| "obsidian_vault requires vault_path".to_string())?;

            // 验证 vault 路径存在且是目录（必须包含 .obsidian 子目录或为合法 vault 根）
            let path = std::path::Path::new(&vault_path);
            if !path.is_absolute() {
                return Err("vault_path must be an absolute path".to_string());
            }
            if !path.is_dir() {
                return Err(format!("vault_path is not a directory: {}", vault_path));
            }

            let kb = axagent_dao::repo::knowledge::create_knowledge_base(
                db,
                axagent_harness::types::CreateKnowledgeBaseInput {
                    name: input.name,
                    description: input.description,
                    embedding_provider: None, // ConnectedVault 不需要 embedding
                    enabled: Some(true),
                    kind: axagent_harness::KbKind::ConnectedVault,
                    vault_path: Some(vault_path),
                },
            )
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

            // 立即注册到全局 VaultRegistry，供 obsidian_* 工具使用
            if let Some(root) = kb.vault_path.clone() {
                if let Err(e) = axagent_tools::tools::obsidian::register_vault(
                    &kb.id,
                    std::path::PathBuf::from(root),
                ) {
                    tracing::warn!(kb_id = %kb.id, error = %e, "Failed to register Obsidian vault");
                }
            }

            Ok(UnifiedSource::from(KnowledgeContainer::from_knowledge_base(&kb)))
        },
        _ => Err(format!("unknown source_type: {}", input.source_type)),
    }
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "获取知识源配置")]
#[tauri::command]
pub async fn get_source_config(
    state: State<'_, AppState>,
    container_type: String,
    container_id: String,
) -> Result<axagent_harness::types::SourceConfig, String> {
    let db = state.harness.db();

    let config = match container_type.as_str() {
        "knowledge" => axagent_dao::repo::knowledge::get_knowledge_base(db, &container_id)
            .await
            .map(|kb| kb.source_config())
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?,
        "memory" => axagent_dao::repo::memory::get_namespace(db, &container_id)
            .await
            .map(|ns| ns.source_config())
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?,
        "wiki" => {
            let w = axagent_dao::repo::wiki::get_wiki(db, &container_id).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            w.source_config()
        },
        _ => return Err(format!("Unknown container type: {}", container_type)),
    };

    Ok(config)
}

#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "搜索所有知识源")]
#[tauri::command]
pub async fn search_all_sources(
    state: State<'_, AppState>,
    query: String,
    top_k: Option<usize>,
) -> Result<axagent_harness::types::RagContextResult, String> {
    let db = state.harness.db();
    let master_key = state.harness.master_key();
    let vector_store = &state.vector_store;
    let top_k = top_k.unwrap_or(5);

    let all_sources = fetch_all_sources(db, None).await;

    let kb_ids: Vec<String> = all_sources
        .iter()
        .filter(|s| s.container_type == "knowledge" && s.embedding_provider.is_some())
        .map(|s| s.id.clone())
        .collect();
    let mem_ids: Vec<String> = all_sources
        .iter()
        .filter(|s| s.container_type == "memory" && s.embedding_provider.is_some())
        .map(|s| s.id.clone())
        .collect();
    let wiki_ids: Vec<String> = all_sources
        .iter()
        .filter(|s| s.container_type == "wiki" && s.embedding_provider.is_some())
        .map(|s| s.id.clone())
        .collect();

    let result = crate::indexing::collect_rag_context(
        db,
        master_key,
        vector_store,
        &kb_ids,
        &mem_ids,
        &wiki_ids,
        &query,
        top_k,
        &state.credential_manager,
    )
    .await;

    Ok(result)
}
