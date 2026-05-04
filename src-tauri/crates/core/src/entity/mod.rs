//! SeaORM entity definitions for AxAgent database tables.

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

pub mod stored_files;

pub mod scheduled_tasks;

pub mod workflow_template;

pub mod workflow_template_version;

pub mod prompt_template;
pub mod prompt_template_version;

pub mod agent_profiles;
pub mod agent_sessions;

// Wave 3: Atomic Skill & Work Engine entities
pub mod generated_tools;
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
pub mod wiki_pages;
pub mod wiki_sources;
pub mod wiki_sync_queue;
pub mod wikis;

pub mod trajectories;
pub mod trajectory_entities;
pub mod trajectory_learned_patterns;
pub mod trajectory_memories;
pub mod trajectory_messages;
pub mod trajectory_patterns;
pub mod trajectory_preferences;
pub mod trajectory_relationships;
pub mod trajectory_rewards;
pub mod trajectory_sessions;
pub mod trajectory_skill_executions;
pub mod trajectory_skills;
pub mod trajectory_steps;

pub use sea_orm;
