#![allow(clippy::result_large_err)]

// builtin_tools, builtin_tools_registry 已迁移至 axagent-tools crate
// ── Search/RAG 模块已迁至 axagent-search ──
pub use axagent_cache::cache;
#[allow(clippy::unwrap_used)]
pub use axagent_cache::cache_persister;
pub use axagent_cache::cache_snapshot;
pub use axagent_kit::billing;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::browser_automation;
pub use axagent_kit::command_validator;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::computer_control;
#[cfg(target_os = "android")]
pub use axagent_kit::computer_control;
#[allow(clippy::unwrap_used)]
pub use axagent_search::ast_index;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::cloud_storage;
pub use axagent_storage::cloud_workspace;
pub mod constants;
pub use axagent_crypto::crypto;
pub use axagent_dao::db;
pub use axagent_dao::ddl;
#[allow(clippy::unwrap_used)]
pub mod disk_cache;
pub mod document_parser;
pub use axagent_entities as entity;
#[allow(clippy::unwrap_used)]
pub mod error;
pub mod error_codes;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::git_tools;
#[cfg(target_os = "android")]
pub use axagent_kit::git_tools;
pub use axagent_kit::html_cleaner;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::markdown_parser;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::marketplace;
pub use axagent_kit::marketplace_service;
pub use axagent_kit::memory_forgetting;
pub use axagent_kit::model_knowledge;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::operation_audit;
pub use axagent_kit::output_processor;
#[allow(clippy::unwrap_used)]
pub use axagent_mcp::mcp_client;
pub use axagent_mcp::mcp_health;
pub use axagent_mcp::mcp_oauth;
#[allow(clippy::unwrap_used)]
pub use axagent_search::file_index;
pub use axagent_search::hybrid_search;
#[allow(clippy::unwrap_used)]
#[cfg(not(target_os = "android"))]
pub use axagent_search::incremental_indexer;
#[allow(clippy::unwrap_used)]
#[cfg(target_os = "android")]
pub use axagent_search::incremental_indexer;
#[allow(clippy::unwrap_used)]
pub use axagent_search::inference;
#[allow(clippy::unwrap_used)]
pub use axagent_search::model_downloader;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::file_authorizer;
pub use axagent_storage::file_store;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::path_vars;
pub mod persistence;
pub use axagent_kit::plan_compiler;
pub mod platform_config;
#[allow(clippy::unwrap_used)]
pub use axagent_dao::repo;
pub use axagent_kit::preset_templates;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::prompt_template;
pub use axagent_kit::prompts;
pub use axagent_kit::resource_limits;
pub use axagent_kit::sandbox_runner;
pub use axagent_kit::schema_validator;
pub use axagent_kit::screen_capture;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::screen_vision;
#[cfg(target_os = "android")]
pub use axagent_kit::screen_vision;
pub use axagent_kit::secure_store;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::service_container;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::shell_parser;
pub use axagent_kit::skill_dirs;
pub use axagent_kit::slash_command;
pub use axagent_kit::token_budget;
pub use axagent_kit::token_counter;
pub use axagent_search::query_enhancement;
#[allow(clippy::unwrap_used)]
pub use axagent_search::rag;
pub use axagent_search::rag_pipeline;
#[allow(clippy::unwrap_used)]
pub use axagent_search::recall_pipeline;
pub use axagent_search::reranker;
#[allow(clippy::unwrap_used)]
pub use axagent_search::search;
pub use axagent_search::self_rag;
#[allow(clippy::unwrap_used)]
pub use axagent_search::semantic_cache;
pub use axagent_search::text_chunker;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::storage_inventory;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::storage_migration;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::storage_paths;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::sync_conflict;
pub mod types;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::ui_automation;
#[cfg(target_os = "android")]
pub use axagent_kit::ui_automation;
pub use axagent_kit::unified_config;
pub use axagent_kit::utils;
#[allow(clippy::unwrap_used)]
pub use axagent_search::vector_cache;
pub use axagent_search::vector_store;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::webdav;
pub mod workflow_types;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::workflow_version;
#[cfg(target_os = "android")]
pub use axagent_kit::workflow_version;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::workspace_uri;

pub use memory_forgetting::{ForgettingConfig, MemoryEntry, MemoryForgettingEngine};
pub use resource_limits::ResourceLimits;
pub use schema_validator::{validate_against_schema, validate_recursive};
pub use service_container::ServiceContainer;
pub use utils::extract_json_from_llm_response;
