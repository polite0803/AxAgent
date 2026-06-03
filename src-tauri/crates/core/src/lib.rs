#![allow(clippy::result_large_err)]

// builtin_tools, builtin_tools_registry 已迁移至 axagent-tools crate
#[allow(clippy::unwrap_used)]
pub mod ast_index;
pub mod billing;
#[cfg(not(target_os = "android"))]
pub mod browser_automation;
pub mod cache;
#[allow(clippy::unwrap_used)]
pub mod cache_persister;
pub mod cache_snapshot;
#[allow(clippy::unwrap_used)]
pub mod cloud_storage;
pub mod cloud_workspace;
pub mod command_validator;
#[cfg(not(target_os = "android"))]
pub mod computer_control;
#[cfg(target_os = "android")]
pub mod computer_control;
pub mod constants;
pub mod crypto;
pub mod db;
pub mod ddl;
#[allow(clippy::unwrap_used)]
pub mod disk_cache;
pub mod document_parser;
pub mod entity;
#[allow(clippy::unwrap_used)]
pub mod error;
pub mod error_codes;
#[allow(clippy::unwrap_used)]
pub mod file_authorizer;
#[allow(clippy::unwrap_used)]
pub mod file_index;
pub mod file_store;
#[cfg(not(target_os = "android"))]
pub mod git_tools;
#[cfg(target_os = "android")]
pub mod git_tools;
pub mod html_cleaner;
pub mod hybrid_search;
#[allow(clippy::unwrap_used)]
#[cfg(not(target_os = "android"))]
pub mod incremental_indexer;
#[allow(clippy::unwrap_used)]
#[cfg(target_os = "android")]
pub mod incremental_indexer;
#[allow(clippy::unwrap_used)]
pub mod inference;
#[allow(clippy::unwrap_used)]
pub mod markdown_parser;
#[allow(clippy::unwrap_used)]
pub mod marketplace;
pub mod marketplace_service;
#[allow(clippy::unwrap_used)]
pub mod mcp_client;
pub mod mcp_health;
pub mod mcp_oauth;
pub mod memory_forgetting;
#[allow(clippy::unwrap_used)]
pub mod model_downloader;
pub mod model_knowledge;
#[allow(clippy::unwrap_used)]
pub mod operation_audit;
pub mod output_processor;
#[allow(clippy::unwrap_used)]
pub mod path_vars;
pub mod persistence;
pub mod platform_config;
pub mod preset_templates;
#[allow(clippy::unwrap_used)]
pub mod prompt_template;
pub mod prompts;
pub mod query_enhancement;
#[allow(clippy::unwrap_used)]
pub mod rag;
pub mod rag_pipeline;
#[allow(clippy::unwrap_used)]
pub mod recall_pipeline;
#[allow(clippy::unwrap_used)]
pub mod repo;
pub mod reranker;
pub mod resource_limits;
pub mod sandbox_runner;
pub mod schema_validator;
pub mod screen_capture;
#[cfg(not(target_os = "android"))]
pub mod screen_vision;
#[cfg(target_os = "android")]
pub mod screen_vision;
#[allow(clippy::unwrap_used)]
pub mod search;
pub mod secure_store;
pub mod self_rag;
#[allow(clippy::unwrap_used)]
pub mod semantic_cache;
#[allow(clippy::unwrap_used)]
pub mod service_container;
#[allow(clippy::unwrap_used)]
pub mod shell_parser;
pub mod skill_dirs;
pub mod slash_command;
#[allow(clippy::unwrap_used)]
pub mod storage_inventory;
#[allow(clippy::unwrap_used)]
pub mod storage_migration;
#[allow(clippy::unwrap_used)]
pub mod storage_paths;
#[allow(clippy::unwrap_used)]
pub mod sync_conflict;
pub mod text_chunker;
pub mod token_budget;
pub mod token_counter;
pub mod types;
pub mod ui_automation;
pub mod unified_config;
pub mod utils;
#[allow(clippy::unwrap_used)]
pub mod vector_cache;
pub mod vector_store;
#[allow(clippy::unwrap_used)]
pub mod webdav;
pub mod workflow_types;
pub mod workflow_version;
#[allow(clippy::unwrap_used)]
pub mod workspace_uri;

pub use memory_forgetting::{ForgettingConfig, MemoryEntry, MemoryForgettingEngine};
pub use resource_limits::ResourceLimits;
pub use schema_validator::{validate_against_schema, validate_recursive};
pub use service_container::ServiceContainer;
pub use utils::extract_json_from_llm_response;
