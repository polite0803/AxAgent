// SPDX-License-Identifier: AGPL-3.0-only

//! SeaORM entity definitions for AxAgent database tables.

// Smart Router 路由历史（v100 consolidated migration 创建）
pub mod route_history;

pub mod background_tasks;
pub mod conversation_categories;
pub mod conversation_summaries;
pub mod conversations;
pub mod desktop_state;
pub mod gateway_diagnostics;
pub mod gateway_keys;
pub mod gateway_link_activities;
pub mod gateway_link_policies;
pub mod gateway_links;
pub mod gateway_request_logs;
pub mod gateway_usage;
pub mod mcp_servers;
pub mod messages;
pub mod models;
pub mod program_policies;
pub mod provider_keys;
pub mod providers;
pub mod search_citations;
pub mod search_providers;
pub mod settings;
pub mod skill_states;
pub mod tool_descriptors;
pub mod tool_executions;

// Wave 2+ entities
pub mod artifacts;
pub mod backup_manifests;
pub mod backup_targets;
pub mod context_sources;
pub mod conversation_branches;
pub mod credentials;
pub mod import_jobs;
pub mod knowledge_attributes;
pub mod knowledge_bases;
pub mod knowledge_documents;
pub mod knowledge_entities;
pub mod knowledge_flows;
pub mod knowledge_interfaces;
pub mod knowledge_relations;
pub mod memory_items;
pub mod memory_namespaces;
pub mod retrieval_hits;

// Feedback data lake entities (v112)
pub mod memory_access_logs;
pub mod tool_call_logs;
pub mod wiki_edit_logs;

pub mod stored_files;

pub mod workflow_snapshots;

pub mod workflow_template;

pub mod workflow_template_version;

pub mod prompt_template;
pub mod prompt_template_version;

pub mod agent_profiles;
pub mod agent_roles;
pub mod agent_sessions;

// 业务岗位（与 agent_roles 抽象执行器类型区别：表达现实业务岗位如 CEO/CTO/产品经理）
pub mod business_roles;

// Wave 3: Atomic Skill & Work Engine entities
pub mod generated_tools;
pub mod workflow_approvals;
pub mod workflow_execution_stats;
pub mod workflow_executions;
pub mod workflow_marketplace;
pub mod workflow_marketplace_review;

// Wiki / LLM Wiki entities
pub mod agency_experts;
pub mod note_backlinks;
pub mod note_links;
pub mod notes;
pub mod plans;
pub mod wiki_operations;
pub mod wiki_page_versions;
pub mod wiki_pages;
pub mod wiki_sources;
pub mod wiki_sync_queue;
pub mod wiki_templates;
pub mod wikis;

pub mod trajectories;
// trajectory_entities/trajectory_relationships/trajectory_memories 已合并到 knowledge_entities/knowledge_relations/memory_items (v101)
pub mod trajectory_learned_patterns;
pub mod trajectory_messages;
pub mod trajectory_patterns;
pub mod trajectory_preferences;
pub mod trajectory_rewards;
pub mod trajectory_sessions;
pub mod trajectory_skill_executions;
pub mod trajectory_skills;
pub mod trajectory_steps;
pub mod trajectory_workflow_reflections;

// Dynamic UI entities
pub mod dynamic_ui_form_data;
pub mod dynamic_ui_pins;
pub mod dynamic_ui_schema_versions;
pub mod dynamic_ui_schemas;

// Index queue entities
pub mod index_jobs;

// Vector store entities
pub mod vec_collections;

// AxInvest entities (registered from orphaned files)
pub mod decision_validations;
pub mod divergence_logs;
pub mod earnings_events;
pub mod financial_snapshots;
pub mod fleet_members;
pub mod fleets;
pub mod fund_transfers;
pub mod lesson_applications;
pub mod market_mainlines;
pub mod news_archive;
pub mod opc_rl_experience;
pub mod opc_rl_training_stats;
pub mod paper_portfolios;
pub mod paper_positions;
pub mod portfolio_correlation_snapshot;
pub mod portfolio_holdings;
pub mod portfolio_metrics_daily;
pub mod price_alerts;
pub mod quant_paper_trades;
pub mod quant_runs;
pub mod quant_signals;
pub mod quant_strategies;
pub mod reco_picks;
pub mod reflection_lessons;
pub mod screenshot_diagnoses;
pub mod stock_analyses;
pub mod stock_pipeline_runs;
pub mod stock_reflections;
pub mod strategy_performance;
pub mod strategy_weight_history;
pub mod trades;
pub mod watchlist_items;

// Sync entities
pub mod sync_audit_log;
pub mod sync_change_log;
pub mod sync_device;
pub mod sync_history;
pub mod sync_permission;
pub mod sync_policy;

// Paper Overview Engine + Reading List & Queue
pub mod paper_overviews;
pub mod reading_list_items;
pub mod reading_lists;

pub use sea_orm;
