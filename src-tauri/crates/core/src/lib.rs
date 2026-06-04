#![allow(clippy::result_large_err)]

// builtin_tools, builtin_tools_registry 已迁移至 axagent-tools crate
// ── Search/RAG 模块已迁至 axagent-search ──
#[allow(clippy::unwrap_used)]
pub use axagent_search::ast_index as ast_index;
pub use axagent_kit::billing as billing;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::browser_automation as browser_automation;
pub use axagent_cache::cache as cache;
#[allow(clippy::unwrap_used)]
pub use axagent_cache::cache_persister as cache_persister;
pub use axagent_cache::cache_snapshot as cache_snapshot;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::cloud_storage as cloud_storage;
pub use axagent_storage::cloud_workspace as cloud_workspace;
pub use axagent_kit::command_validator as command_validator;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::computer_control as computer_control;
#[cfg(target_os = "android")]
pub use axagent_kit::computer_control as computer_control;
pub mod constants;
pub use axagent_crypto::crypto as crypto;
pub use axagent_dao::db as db;
pub use axagent_dao::ddl as ddl;
#[allow(clippy::unwrap_used)]
pub mod disk_cache;
pub mod document_parser;
pub use axagent_entities as entity;
#[allow(clippy::unwrap_used)]
pub mod error;
pub mod error_codes;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::file_authorizer as file_authorizer;
#[allow(clippy::unwrap_used)]
pub use axagent_search::file_index as file_index;
pub use axagent_storage::file_store as file_store;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::git_tools as git_tools;
#[cfg(target_os = "android")]
pub use axagent_kit::git_tools as git_tools;
pub use axagent_kit::html_cleaner as html_cleaner;
pub use axagent_search::hybrid_search as hybrid_search;
#[allow(clippy::unwrap_used)]
#[cfg(not(target_os = "android"))]
pub use axagent_search::incremental_indexer as incremental_indexer;
#[allow(clippy::unwrap_used)]
#[cfg(target_os = "android")]
pub use axagent_search::incremental_indexer as incremental_indexer;
#[allow(clippy::unwrap_used)]
pub use axagent_search::inference as inference;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::markdown_parser as markdown_parser;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::marketplace as marketplace;
pub use axagent_kit::marketplace_service as marketplace_service;
#[allow(clippy::unwrap_used)]
pub use axagent_mcp::mcp_client as mcp_client;
pub use axagent_mcp::mcp_health as mcp_health;
pub use axagent_mcp::mcp_oauth as mcp_oauth;
pub use axagent_kit::memory_forgetting as memory_forgetting;
#[allow(clippy::unwrap_used)]
pub use axagent_search::model_downloader as model_downloader;
pub use axagent_kit::model_knowledge as model_knowledge;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::operation_audit as operation_audit;
pub use axagent_kit::output_processor as output_processor;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::path_vars as path_vars;
pub mod persistence;
pub use axagent_kit::plan_compiler as plan_compiler;
pub mod platform_config;
pub use axagent_kit::preset_templates as preset_templates;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::prompt_template as prompt_template;
pub use axagent_kit::prompts as prompts;
pub use axagent_search::query_enhancement as query_enhancement;
#[allow(clippy::unwrap_used)]
pub use axagent_search::rag as rag;
pub use axagent_search::rag_pipeline as rag_pipeline;
#[allow(clippy::unwrap_used)]
pub use axagent_search::recall_pipeline as recall_pipeline;
#[allow(clippy::unwrap_used)]
pub use axagent_dao::repo as repo;
pub use axagent_search::reranker as reranker;
pub use axagent_kit::resource_limits as resource_limits;
pub use axagent_kit::sandbox_runner as sandbox_runner;
pub use axagent_kit::schema_validator as schema_validator;
pub use axagent_kit::screen_capture as screen_capture;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::screen_vision as screen_vision;
#[cfg(target_os = "android")]
pub use axagent_kit::screen_vision as screen_vision;
#[allow(clippy::unwrap_used)]
pub use axagent_search::search as search;
pub use axagent_kit::secure_store as secure_store;
pub use axagent_search::self_rag as self_rag;
#[allow(clippy::unwrap_used)]
pub use axagent_search::semantic_cache as semantic_cache;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::service_container as service_container;
#[allow(clippy::unwrap_used)]
pub use axagent_kit::shell_parser as shell_parser;
pub use axagent_kit::skill_dirs as skill_dirs;
pub use axagent_kit::slash_command as slash_command;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::storage_inventory as storage_inventory;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::storage_migration as storage_migration;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::storage_paths as storage_paths;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::sync_conflict as sync_conflict;
pub use axagent_search::text_chunker as text_chunker;
pub use axagent_kit::token_budget as token_budget;
pub use axagent_kit::token_counter as token_counter;
pub mod types;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::ui_automation as ui_automation;
#[cfg(target_os = "android")]
pub use axagent_kit::ui_automation as ui_automation;
pub use axagent_kit::unified_config as unified_config;
pub use axagent_kit::utils as utils;
#[allow(clippy::unwrap_used)]
pub use axagent_search::vector_cache as vector_cache;
pub use axagent_search::vector_store as vector_store;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::webdav as webdav;
pub mod workflow_types;
#[cfg(not(target_os = "android"))]
pub use axagent_kit::workflow_version as workflow_version;
#[cfg(target_os = "android")]
pub use axagent_kit::workflow_version as workflow_version;
#[allow(clippy::unwrap_used)]
pub use axagent_storage::workspace_uri as workspace_uri;

pub use memory_forgetting::{ForgettingConfig, MemoryEntry, MemoryForgettingEngine};
pub use resource_limits::ResourceLimits;
pub use schema_validator::{validate_against_schema, validate_recursive};
pub use service_container::ServiceContainer;
pub use utils::extract_json_from_llm_response;
