// SPDX-License-Identifier: AGPL-3.0-only

pub mod agent_turn_adapter;
pub mod axinvest_decorators;
pub mod browser_fetcher;
pub mod cognitive_router_init;
pub mod cron_delivery_sink;
pub mod database;
pub mod llm_providers;
pub mod monitor_emitter;
pub mod news_archive_sink;
pub mod notification_adapters;
pub mod opc_knowledge;
pub mod plugins;
pub mod services;
pub mod state;
pub mod trigger_recovery;
pub mod workflow_injections;

pub use cognitive_router_init::{
    COGNITIVE_L1_DOMAIN_ROUTER_ID, COGNITIVE_L2_CLUSTER_ROUTER_ID,
    COGNITIVE_L3_CAPABILITY_ROUTER_ID, COGNITIVE_ROUTER_MAIN_ID, COGNITIVE_ROUTER_TAG,
    ensure_cognitive_router_templates,
};
pub use database::init_database_with_dir;
pub use plugins::register_plugins;
pub use state::{create_app_state, run_deferred_init};
