// SPDX-License-Identifier: AGPL-3.0-only

pub mod _shared_state;
pub(crate) mod _workflow_ai_protocol;
pub mod agency_expert;
pub mod agent;
pub mod agent_advanced;
pub mod agent_analytics;
pub mod agent_insight;
pub mod agent_nudge;
pub mod agent_profile;
pub mod agent_role;
pub mod analyst_evolution;
pub mod analyst_feedback;
pub mod app_config;
pub mod artifacts;
pub mod background_tasks;
pub mod backtest_validation;
pub mod backup;
pub mod branches;
#[cfg(not(mobile))]
pub mod browser;
pub mod chart_generator;
pub mod cloud_workspace;
#[cfg(not(mobile))]
pub mod computer_control;
pub mod context_breakdown;
pub mod context_sources;
pub mod conversation_categories;
pub mod conversations;
pub mod conversations_search;
pub mod crash_report;
pub mod dashboard;
pub mod db_config;
#[cfg(not(mobile))]
pub mod desktop;
pub mod device_sync;
pub mod dojo_sdk;
pub mod dream;
pub mod dynamic_ui;
pub mod error;
pub mod error_code;
pub mod evaluator;
pub mod evolution;
pub mod evolution_engine;
pub(crate) mod execution_bridge;
pub mod file_browser;
pub(crate) mod file_cleanup;
pub mod files;
pub mod files_page;
pub mod fine_tune;
pub mod fleet;
pub mod gateway;
pub mod gateway_link;
pub mod generated_tool;
pub mod health;
pub mod hooks_config;
pub mod image_gen;
pub mod image_gen_settings;
pub mod index_jobs;
pub mod knowledge;
pub mod knowledge_graph;
pub mod knowledge_source;
pub mod learning_graph;
pub mod llm_wiki;
pub mod local_model;
pub mod local_models;
pub mod local_tool;
pub mod market_mainline;
pub mod market_sim;
pub mod marketplace;
pub mod mcp;
pub mod memory;
pub mod message_continuation;
pub mod messages;
pub mod migration;
pub mod multi_agent;
pub mod multi_agent_setup;
pub mod nl_to_cron;
pub mod onboarding;
pub mod orchestrator;
pub mod paper;
pub mod paper_portfolio;
pub mod parallel_execution;
pub mod personality;
pub mod plan;
pub mod platform_integration;
pub mod plugin;
pub mod proactive;
pub mod profile;
pub mod prompt_templates;
pub mod provider_balance;
#[cfg(not(mobile))]
pub mod provider_ctx;
pub mod providers;
#[cfg(not(mobile))]
pub mod pty;
pub mod quant_backtest;
#[cfg(not(mobile))]
pub mod quickbar;
pub mod recommendation_cron;
pub mod reflection;
pub mod reminder;
pub mod research;
pub mod retrieval_feedback;
pub mod rl_training;
pub mod sandbox;
pub mod scheduled_task;
#[cfg(not(mobile))]
pub mod screen_vision;
pub mod screenshot_diagnosis;
pub mod search;
pub mod session_share;
// G18: 会话多格式导出（messages.jsonl / openai_dataset_jsonl / markdown / manifest_json）
pub mod sessions_export;
pub mod settings;
// simple_chat_completion 完整实现在 agent/mod.rs（stub 已删）
pub mod skill_decomposition;
pub mod skills;
pub mod skills_hub;
pub mod smart_router;
pub mod sources;
pub mod spawn_guard;
pub mod stock_analysis;
pub mod stock_analysis_bridge;
#[cfg(test)]
mod stock_analysis_bridge_tests;
pub mod stock_analysis_setup;
pub mod stock_pipeline;
pub mod stock_workflow;
pub mod storage;
pub mod strategy_pack;
pub mod sub_agent;
#[cfg(not(mobile))]
pub mod terminal;
pub mod theme;
pub mod tool_recommender;
pub mod tracer;
pub mod unified_knowledge;
pub mod user_profile;
pub mod webdav;
pub mod webhook;
// ── Walk-Forward + DES 对比 ──
pub mod wf_des;
pub mod wiki;
// 工作流执行引擎命令:含审批(HITL)命令 list_pending_approvals/resume_approval/cancel_approval、
// Loop 节点人工审查 resume_loop_after_review、节点执行白名单校验等。
// 注:历史规划中曾设想独立 approval.rs,实际审批命令统一位于此模块。
pub mod work_engine;
pub mod workflow_ai;
pub mod workflow_ai_apply;
pub mod workflow_ai_diagnose;
pub mod workflow_execution_stats;
pub mod workflow_reflection;
pub mod workflow_template;
pub mod workflow_yaml;
pub mod workflows;
// ── OPC（AI 一人公司）─ 行业数据资产包 + 台账 ──
pub mod opc;
pub mod opc_data;
pub mod opc_industry_actions;
pub mod opc_industry_bridge;
pub mod opc_industry_logic;
pub mod opc_industry_runtime;
pub mod opc_learning_hook;
pub(crate) mod opc_llm_bridge;
pub mod opc_setup;
pub mod opc_workflows;
