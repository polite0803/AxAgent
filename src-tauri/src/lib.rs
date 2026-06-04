#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::needless_borrow)]

mod android_utils;
mod commands;
mod context_manager;
mod indexing;
mod indexing_triggers;
mod init;
mod knowledge_integration;
mod memory_extract;
mod paths;
mod semantic_cache;
mod smart_router;

#[cfg(not(mobile))]
mod tray;
#[cfg(not(mobile))]
mod window_state;

#[cfg(mobile)]
mod tray {
    #[tauri::command]
    pub fn set_tray_labels(_app: tauri::AppHandle, _show_label: String, _quit_label: String) {}
}

#[cfg(target_os = "windows")]
mod windows_utils;

#[allow(clippy::disallowed_types)]
mod app_state;

use tauri::{Emitter, Manager};

pub use app_state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── 日志 / tracing（必须在 panic hook 之前初始化） ─
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("AxAgent"),
        );
        if let Err(e) = tracing_log::LogTracer::init() {
            // LogTracer 失败非致命：android_logger 仍可捕获直接 log:: 调用，
            // 只是 tracing 事件不会被转发到 logcat。
            log::error!("Failed to init LogTracer: {} — tracing->log bridge unavailable", e);
        }

        // ── 最早阶段的崩溃诊断标记 ──
        // 此标记在 `android_utils::mark_startup_phase` 可用之前写入，
        // 直接写入外部可访问路径（用户可通过文件管理器读取）。
        tracing::info!("=== AxAgent Android START ===");
        // 注意：使用 append 而非 overwrite，防止跨启动丢失日志
        let boot_msg = "[BOOT] run() entered\n";
        let boot_paths = [
            "/storage/emulated/0/Download/axagent-crash.log",
            "/storage/emulated/0/Android/data/top.axagent.desktop/files/axagent-crash.log",
        ];
        for bp in &boot_paths {
            // 追加而非覆盖
            let existing = std::fs::read_to_string(bp).unwrap_or_default();
            let _ = std::fs::write(bp, existing + &*boot_msg);
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }

    // ── 全局 panic hook ──
    std::panic::set_hook(Box::new(|info| {
        let msg = match (
            info.payload().downcast_ref::<&str>(),
            info.payload().downcast_ref::<String>(),
        ) {
            (Some(s), _) => s.to_string(),
            (_, Some(s)) => s.clone(),
            _ => "unknown panic".to_string(),
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        tracing::error!(
            panic.message = %msg,
            panic.location = %location,
            "FATAL: process panicked"
        );
        // 给日志一点时间刷新到 logcat/stderr
        std::thread::sleep(std::time::Duration::from_millis(100));
        android_utils::report_fatal_error(&format!("Panic: {} at {}", msg, location));
    }));

    #[cfg(target_os = "android")]
    {
        tracing::info!("AxAgent starting on Android (tracing -> log -> logcat)");
        android_utils::mark_startup_phase("run_start");
    }

    // ── TLS crypto provider ──
    if rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .is_err()
    {
        let ring_ok = rustls::crypto::ring::default_provider()
            .install_default()
            .is_ok();
        if !ring_ok {
            #[cfg(target_os = "android")]
            tracing::error!(
                "No TLS crypto provider available on Android (aws-lc-rs and ring both failed) — HTTPS will fail"
            );
            #[cfg(not(target_os = "android"))]
            tracing::warn!("No TLS crypto provider available, HTTPS connections may fail");
        } else {
            tracing::info!("TLS: aws-lc-rs unavailable, using ring fallback");
        }
    }

    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("register_plugins_start");
    let builder = tauri::Builder::default();
    let builder = init::register_plugins(builder);
    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("register_plugins_done");

    let build_result = builder
        .invoke_handler(tauri::generate_handler![
            commands::providers::list_providers,
            commands::providers::create_provider,
            commands::providers::update_provider,
            commands::providers::delete_provider,
            commands::providers::toggle_provider,
            commands::providers::add_provider_key,
            commands::providers::update_provider_key,
            commands::providers::delete_provider_key,
            commands::providers::toggle_provider_key,
            commands::providers::get_decrypted_provider_key,
            commands::providers::validate_provider_key,
            commands::providers::save_models,
            commands::providers::toggle_model,
            commands::providers::update_model_params,
            commands::providers::fetch_remote_models,
            commands::providers::test_model,
            commands::providers::reorder_providers,
            commands::provider_balance::fetch_provider_balance,
            commands::conversations::list_conversations,
            commands::conversations::create_conversation,
            commands::conversations::update_conversation,
            commands::conversations::delete_conversation,
            commands::conversations::batch_delete_conversations,
            commands::conversations::branch_conversation,
            commands::conversations::search_conversations,
            commands::conversations_search::session_search,
            commands::conversations::send_message,
            commands::conversations::toggle_pin_conversation,
            commands::conversations::toggle_archive_conversation,
            commands::conversations::archive_conversation_to_knowledge_base,
            commands::conversations::archive_workflow_session,
            commands::conversations::list_archived_conversations,
            commands::conversations::regenerate_message,
            commands::conversations::regenerate_with_model,
            commands::conversations::cancel_stream,
            commands::conversations::list_message_versions,
            commands::conversations::switch_message_version,
            commands::conversations::send_system_message,
            commands::context_breakdown::get_context_breakdown,
            commands::messages::list_messages,
            commands::messages::list_messages_page,
            commands::messages::delete_message,
            commands::messages::update_message_content,
            commands::messages::clear_conversation_messages,
            commands::messages::export_conversation,
            commands::messages::get_conversation_stats,
            commands::branches::list_branches,
            commands::conversation_categories::list_conversation_categories,
            commands::conversation_categories::create_conversation_category,
            commands::conversation_categories::update_conversation_category,
            commands::conversation_categories::delete_conversation_category,
            commands::conversation_categories::reorder_conversation_categories,
            commands::knowledge::list_knowledge_bases,
            commands::knowledge::list_knowledge_containers,
            commands::knowledge::create_knowledge_base,
            commands::knowledge::update_knowledge_base,
            commands::knowledge::delete_knowledge_base,
            commands::knowledge::reorder_knowledge_bases,
            commands::knowledge::list_knowledge_documents,
            commands::knowledge::add_knowledge_document,
            commands::knowledge::delete_knowledge_document,
            commands::knowledge::search_knowledge_base,
            commands::knowledge::rebuild_knowledge_index,
            commands::knowledge::clear_knowledge_index,
            commands::knowledge::list_knowledge_document_chunks,
            commands::knowledge::delete_knowledge_chunk,
            commands::knowledge::update_knowledge_chunk,
            commands::knowledge::add_knowledge_chunk,
            commands::knowledge::reindex_knowledge_chunk,
            commands::knowledge::rebuild_knowledge_document,
            commands::knowledge::list_knowledge_entities,
            commands::knowledge::create_knowledge_entity,
            commands::knowledge::list_knowledge_attributes,
            commands::knowledge::create_knowledge_attribute,
            commands::knowledge::list_knowledge_relations,
            commands::knowledge::create_knowledge_relation,
            commands::knowledge::list_knowledge_flows,
            commands::knowledge::create_knowledge_flow,
            commands::knowledge::list_knowledge_interfaces,
            commands::knowledge::create_knowledge_interface,
            commands::local_models::list_local_models,
            commands::local_models::download_model,
            commands::local_models::delete_model,
            commands::local_models::get_preset_models,
            knowledge_integration::analyze_knowledge_integration,
            commands::prompt_templates::list_prompt_templates,
            commands::prompt_templates::get_prompt_template,
            commands::prompt_templates::create_prompt_template,
            commands::prompt_templates::update_prompt_template,
            commands::prompt_templates::delete_prompt_template,
            commands::prompt_templates::get_prompt_template_versions,
            commands::prompt_templates::rollback_prompt_template,
            commands::prompt_templates::import_prompt_templates,
            commands::prompt_templates::export_prompt_templates,
            commands::prompt_templates::import_prompt_from_url,
            commands::prompt_templates::import_prompt_from_folder,
            commands::prompt_templates::increment_prompt_usage,
            commands::context_sources::list_context_sources,
            commands::context_sources::add_context_source,
            commands::context_sources::remove_context_source,
            commands::context_sources::toggle_context_source,
            commands::search::list_search_providers,
            commands::search::get_search_provider,
            commands::search::create_search_provider,
            commands::search::update_search_provider,
            commands::search::delete_search_provider,
            commands::search::test_search_provider,
            commands::search::execute_search,
            commands::mcp::list_mcp_servers,
            commands::mcp::create_mcp_server,
            commands::mcp::update_mcp_server,
            commands::mcp::delete_mcp_server,
            commands::mcp::test_mcp_server,
            commands::mcp::list_mcp_tools,
            commands::mcp::discover_mcp_tools,
            commands::mcp::list_tool_executions,
            commands::mcp::hot_reload_mcp_server,
            commands::mcp::discover_available_mcp_servers,

            commands::local_tool::get_tool_count,
            commands::local_tool::list_local_tools,
            commands::local_tool::toggle_local_tool_group,
            commands::local_tool::toggle_single_tool,
            commands::generated_tool::list_generated_tools,
            commands::generated_tool::delete_generated_tool,
            commands::memory::list_memory_namespaces,
            commands::memory::create_memory_namespace,
            commands::memory::delete_memory_namespace,
            commands::memory::update_memory_namespace,
            commands::memory::list_memory_items,
            commands::memory::add_memory_item,
            commands::memory::delete_memory_item,
            commands::memory::update_memory_item,
            commands::memory::search_memory,
            commands::memory::rebuild_memory_index,
            commands::memory::clear_memory_index,
            commands::memory::reindex_memory_item,
            commands::memory::reorder_memory_namespaces,
            commands::memory::extract_conversation_memories,
            commands::memory::sync_working_memory_to_namespace,
            commands::memory::promote_memory_entry,
            commands::memory::demote_memory_entry,
            commands::memory::add_memory_with_dedup,
            commands::memory::apply_memory_decay_tick,
            commands::memory::search_working_memories,
            commands::memory::update_memory_importance,
            commands::memory::get_memory_tier_stats,
            commands::memory::auto_extract_incremental_memories,
            commands::memory::extract_conversation_entities,
            commands::memory::graph_search_memories,
            commands::memory::disambiguate_memory_entities,
            commands::memory::list_knowledge_graph,
            commands::memory::search_memories_by_time,
            commands::memory::get_memories_time_grouped,
            commands::memory::search_memories_explained,
            commands::memory::get_memory_provenance,
            commands::memory::find_memory_clusters,
            commands::memory::consolidate_memory_cluster,
            commands::memory::submit_memory_feedback,
            commands::skills::list_skills,
            commands::skills::get_skill,
            commands::skills::toggle_skill,
            commands::skills::install_skill,
            commands::skills::uninstall_skill,
            commands::skills::uninstall_skill_group,
            commands::skills::open_skills_dir,
            commands::skills::open_skill_dir,
            commands::skills::search_marketplace,
            commands::skills::check_skill_updates,
            commands::skills::skill_create,
            commands::skills::skill_patch,
            commands::skills::skill_edit,
            commands::skills::skill_check_similar,
            commands::skills::skill_upgrade_or_create,
            commands::skills::get_skill_proposals,
            commands::skills::create_skill_from_proposal,
            commands::skills::skill_set_manifest,
            commands::skills::get_skill_versions,
            commands::skills::rollback_skill,
            commands::skills::get_marketplace_categories,
            commands::skills::skill_analyze_frontend,
            commands::skills::skill_read_asset,
            commands::skills_hub::skills_hub_search,
            commands::skills_hub::skills_hub_install,
            commands::skills_hub::skills_hub_review,
            commands::skills_hub::skills_hub_export,
            commands::skills_hub::skills_hub_import,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::backup::list_backups,
            commands::backup::create_backup,
            commands::backup::restore_backup,
            commands::backup::delete_backup,
            commands::backup::batch_delete_backups,
            commands::backup::get_backup_settings,
            commands::backup::update_backup_settings,
            commands::backup::upload_backup_to_cloud,
            commands::backup::list_cloud_backups,
            commands::backup::download_cloud_backup,
            commands::webdav::get_webdav_config,
            commands::webdav::save_webdav_config,
            commands::webdav::webdav_check_connection,
            commands::webdav::webdav_backup,
            commands::webdav::webdav_list_backups,
            commands::webdav::webdav_restore,
            commands::webdav::webdav_delete_backup,
            commands::webdav::get_webdav_sync_status,
            commands::webdav::restart_webdav_sync,
            commands::webhook::webhook_list_subscriptions,
            commands::webhook::webhook_create_subscription,
            commands::webhook::webhook_delete_subscription,
            commands::webhook::webhook_toggle_subscription,
            commands::webhook::webhook_test_subscription,
            commands::webhook::webhook_reload,
            #[cfg(not(mobile))]
            commands::terminal::git_get_branch,
            #[cfg(not(mobile))]
            commands::terminal::git_status,
            #[cfg(not(mobile))]
            commands::terminal::system_get_info,
            #[cfg(not(mobile))]
            commands::terminal::path_complete,
            #[cfg(not(mobile))]
            commands::terminal::session_get_status,
            commands::theme::list_themes,
            commands::theme::get_theme,
            commands::theme::get_xterm_theme,
            commands::theme::save_theme,
            commands::theme::delete_theme,
            commands::theme::load_user_themes,
            commands::profile::profile_list,
            commands::profile::profile_create,
            commands::profile::profile_delete,
            commands::profile::profile_switch,
            commands::profile::profile_active,
            #[cfg(not(mobile))]
            commands::desktop::get_desktop_capabilities,
            #[cfg(not(mobile))]
            commands::desktop::send_desktop_notification,
            #[cfg(not(mobile))]
            commands::desktop::get_window_state,
            #[cfg(not(mobile))]
            commands::desktop::set_always_on_top,
            #[cfg(not(mobile))]
            commands::desktop::set_close_to_tray,
            #[cfg(not(mobile))]
            commands::desktop::force_quit,
            #[cfg(not(mobile))]
            commands::desktop::apply_startup_settings,
            #[cfg(not(mobile))]
            commands::desktop::test_proxy,
            #[cfg(not(mobile))]
            commands::desktop::open_devtools,
            #[cfg(not(mobile))]
            commands::desktop::list_system_fonts,
            #[cfg(not(mobile))]
            commands::desktop::minimize_window,
            #[cfg(not(mobile))]
            commands::desktop::toggle_maximize_window,
            #[cfg(not(mobile))]
            commands::quickbar::show_quickbar,
            #[cfg(not(mobile))]
            commands::quickbar::hide_quickbar,
            commands::dashboard::dashboard_list_plugins,
            commands::dashboard::dashboard_register_plugin,
            commands::dashboard::dashboard_unregister_plugin,
            commands::dashboard::dashboard_enable_plugin,
            commands::dashboard::dashboard_disable_plugin,
            commands::dashboard::dashboard_render_panel,
            commands::dashboard::dashboard_reload_plugins,
            commands::dashboard::dashboard_open_plugins_folder,
            commands::dashboard::dashboard_install_plugin,
            #[cfg(not(mobile))]
            commands::computer_control::screen_capture,
            #[cfg(not(mobile))]
            commands::computer_control::find_ui_elements,
            #[cfg(not(mobile))]
            commands::computer_control::mouse_click,
            #[cfg(not(mobile))]
            commands::computer_control::type_text,
            #[cfg(not(mobile))]
            commands::computer_control::press_key,
            #[cfg(not(mobile))]
            commands::computer_control::mouse_scroll,
            #[cfg(not(mobile))]
            commands::computer_control::mouse_move,
            #[cfg(not(mobile))]
            commands::browser::browser_navigate,
            #[cfg(not(mobile))]
            commands::browser::browser_screenshot,
            #[cfg(not(mobile))]
            commands::browser::browser_click,
            #[cfg(not(mobile))]
            commands::browser::browser_fill,
            #[cfg(not(mobile))]
            commands::browser::browser_type,
            #[cfg(not(mobile))]
            commands::browser::browser_extract_text,
            #[cfg(not(mobile))]
            commands::browser::browser_extract_all,
            #[cfg(not(mobile))]
            commands::browser::browser_get_content,
            #[cfg(not(mobile))]
            commands::browser::browser_wait_for,
            #[cfg(not(mobile))]
            commands::browser::browser_select,
            #[cfg(not(mobile))]
            commands::browser::browser_close,
            commands::files::upload_file,
            commands::files::download_file,
            commands::files::list_files,
            commands::files::delete_file,
            commands::files_page::list_files_page_entries,
            commands::files_page::open_files_page_entry,
            commands::files_page::reveal_files_page_entry,
            commands::files_page::cleanup_missing_files_page_entry,
            commands::files_page::check_attachment_exists,
            commands::files_page::resolve_attachment_path,
            commands::files_page::read_attachment_preview,
            commands::files_page::reveal_attachment_file,
            commands::files_page::save_avatar_file,
            commands::files_page::open_attachment_file,
            commands::cloud_workspace::list_cloud_provider_presets,
            commands::cloud_workspace::list_cloud_directory,
            commands::cloud_workspace::sync_cloud_workspace,
            commands::cloud_workspace::push_cloud_workspace_changes,
            commands::cloud_workspace::get_cloud_conflicts,
            commands::cloud_workspace::resolve_cloud_conflict,
            commands::cloud_workspace::set_cloud_conflict_strategy,
            commands::cloud_workspace::check_cloud_connection,
            commands::storage::get_storage_inventory,
            commands::storage::open_storage_directory,
            commands::storage::validate_documents_root,
            commands::storage::change_documents_root,
            commands::storage::reset_documents_root,
            commands::agent::agent_query,
            commands::agent::agent_cancel,
            commands::agent::agent_is_running,
            commands::agent::agent_pause,
            commands::agent::agent_resume,
            commands::agent::agent_is_paused,
            commands::agent::agent_runtime_stats,
            commands::agent::agent_resolve_model,
            commands::agent::agent_update_session,
            commands::agent::agent_get_session,
            commands::agent::agent_ensure_workspace,
            commands::agent::classify_route,
            commands::agent::agent_steer,
            commands::agent::agent_approve,
            commands::agent::agent_respond_ask,
            commands::agent::agent_backup_and_clear_sdk_context,
            commands::agent::agent_restore_sdk_context_from_backup,
            commands::agent::workflow_create,
            commands::agent::workflow_execute,
            commands::agent::workflow_get_status,
            commands::agent::workflow_cancel,
            commands::agent::workflow_list,
            commands::agent::agent_estimate_complexity,
            commands::agent::sub_agent_list,
            commands::agent::sub_agent_get,
            commands::agent::sub_agent_get_children,
            commands::agent::sub_agent_get_messages,
            commands::agent::shared_memory_list,
            commands::agent::shared_memory_get,
            commands::agent::shared_memory_stats,
            commands::agent::get_conversation_workflow_preview,
            commands::agent::workflow_get_steps,
            commands::plan::plan_generate,
            commands::plan::plan_execute,
            commands::plan::plan_cancel,
            commands::plan::plan_activate,
            commands::plan::plan_get,
            commands::plan::plan_list,
            commands::plan::plan_modify_step,
            commands::agent_nudge::nudge_list,
            commands::agent_nudge::nudge_dismiss,
            commands::agent_nudge::nudge_snooze,
            commands::agent_nudge::nudge_execute,
            commands::agent_nudge::nudge_stats,
            commands::agent_nudge::nudge_closed_loop_list,
            commands::agent_nudge::nudge_closed_loop_acknowledge,
            commands::agent_nudge::skill_find_similar,
            commands::agent_nudge::skill_upgrade_propose,
            commands::agent_nudge::skill_upgrade_execute,
            commands::agent_insight::insight_list,
            commands::agent_insight::insight_get_by_category,
            commands::agent_insight::insight_report,
            commands::agent::memory_flush,
            commands::agent::record_feedback,
            // Proactive commands
            commands::proactive::proactive_list_suggestions,
            commands::proactive::proactive_refresh_suggestions,
            commands::proactive::proactive_predict,
            commands::proactive::proactive_list_reminders,
            commands::proactive::proactive_dismiss_suggestion,
            commands::proactive::proactive_accept_suggestion,
            commands::proactive::proactive_snooze_suggestion,
            commands::proactive::proactive_add_reminder,
            commands::proactive::proactive_delete_reminder,
            commands::proactive::proactive_complete_reminder,
            commands::proactive::proactive_set_enabled,
            commands::proactive::proactive_update_config,
            commands::proactive::proactive_prefetch,
            commands::proactive::list_insights,
            commands::message_continuation::continue_message,
            commands::message_continuation::list_continuable_messages,
            commands::onboarding::detect_ollama_availability,
            commands::onboarding::detect_api_keys,
            commands::onboarding::apply_quick_start_preset,
            commands::agent_analytics::trajectory_stats,
            commands::agent_analytics::trajectory_list,
            commands::agent_analytics::get_trajectory_detail,
            commands::agent_analytics::pattern_stats,
            commands::agent_analytics::closed_loop_status,
            commands::agent_analytics::rl_config,
            commands::agent_analytics::rl_export_training_data,
            commands::agent_analytics::rl_compute_rewards,
            // Advanced agent commands (ToT, replanning, semantic cache, error reports)
            commands::agent_advanced::tot_get_state,
            commands::agent_advanced::tot_backtrack,
            commands::agent_advanced::tot_explore,
            commands::agent_advanced::tot_score_node,
            commands::agent_advanced::tot_traverse,
            commands::agent_advanced::tot_prune,
            commands::agent_advanced::tot_get_best_path,
            commands::agent_advanced::planner_replan,
            commands::agent_advanced::planner_rollback,
            commands::agent_advanced::planner_diff_versions,
            commands::agent_advanced::planner_get_history,
            commands::agent_advanced::planner_get_versions,
            commands::agent_advanced::semantic_cache_stats,
            commands::agent_advanced::semantic_cache_clear,
            commands::agent_advanced::semantic_cache_set_enabled,
            commands::agent_advanced::semantic_cache_lookup,
            commands::agent_advanced::semantic_cache_store,
            commands::agent_advanced::semantic_cache_set_threshold,
            commands::agent_advanced::error_get_report,
            commands::agent_advanced::get_prompt_cache_state,
            commands::agent::skill_evolution_start,
            commands::agent::skill_evolution_status,
            commands::agent::user_profile_get,
            commands::agent::user_profile_set_preference,
            commands::agent::user_profile_set_expertise,
            commands::agent::user_profile_export_md,
            commands::agent::adaptation_status,
            commands::artifacts::list_artifacts,
            commands::artifacts::create_artifact,
            commands::artifacts::update_artifact,
            commands::artifacts::delete_artifact,
            commands::sandbox::execute_sandbox,
            commands::image_gen::generate_image,
            commands::image_gen_settings::get_image_gen_config,
            commands::image_gen_settings::save_image_gen_config,
            commands::chart_generator::generate_chart_config,
            commands::gateway::get_gateway_status,
            commands::gateway::start_gateway,
            commands::gateway::stop_gateway,
            // Gateway commands - additional
            commands::gateway::get_all_cli_tool_statuses,
            commands::gateway::connect_cli_tool,
            commands::gateway::disconnect_cli_tool,
            commands::gateway::list_gateway_keys,
            commands::gateway::create_gateway_key,
            commands::gateway::delete_gateway_key,
            commands::gateway::toggle_gateway_key,
            commands::gateway::decrypt_gateway_key,
            commands::gateway::get_gateway_metrics,
            commands::gateway::get_gateway_usage_by_key,
            commands::gateway::get_gateway_usage_by_provider,
            commands::gateway::get_gateway_usage_by_day,
            commands::gateway::get_connected_programs,
            commands::gateway::get_gateway_diagnostics,
            commands::gateway::get_program_policies,
            commands::gateway::save_program_policy,
            commands::gateway::delete_program_policy,
            commands::gateway::list_gateway_templates,
            commands::gateway::copy_gateway_template,
            commands::gateway::list_gateway_request_logs,
            commands::gateway::clear_gateway_request_logs,
            commands::gateway::generate_self_signed_cert,
            commands::gateway::get_active_gateway_platform,
            // Gateway Link commands
            commands::gateway_link::list_gateway_links,
            commands::gateway_link::create_gateway_link,
            commands::gateway_link::delete_gateway_link,
            commands::gateway_link::toggle_gateway_link,
            commands::gateway_link::connect_gateway_link,
            commands::gateway_link::disconnect_gateway_link,
            commands::gateway_link::update_gateway_link_status,
            commands::gateway_link::update_gateway_link_sync_settings,
            commands::gateway_link::get_gateway_link_model_syncs,
            commands::gateway_link::push_gateway_link_models,
            commands::gateway_link::sync_all_gateway_link_models,
            commands::gateway_link::get_gateway_link_skill_syncs,
            commands::gateway_link::push_gateway_link_skills,
            commands::gateway_link::sync_all_gateway_link_skills,
            commands::gateway_link::get_gateway_link_policy,
            commands::gateway_link::save_gateway_link_policy,
            commands::gateway_link::get_gateway_link_activities,
            commands::gateway_link::create_gateway_conversation,
            // Branches commands - additional
            commands::branches::fork_conversation,
            commands::branches::compare_branches,
            commands::branches::get_workspace_snapshot,
            commands::branches::update_workspace_snapshot,
            // Conversations commands - additional
            commands::conversations::regenerate_conversation_title,
            commands::conversations::delete_message_group,
            commands::conversations::compress_context,
            commands::conversations::get_compression_summary,
            commands::conversations::delete_compression,
            // Conversation categories - additional
            commands::conversation_categories::set_conversation_category_collapsed,
            // Agent commands - additional
            commands::agent::pattern_list,
            commands::agent::cross_session_insights,
            // Parallel execution commands
            commands::parallel_execution::create_parallel_execution,
            commands::parallel_execution::get_parallel_execution,
            commands::parallel_execution::list_parallel_executions,
            commands::parallel_execution::get_next_pending_task,
            commands::parallel_execution::update_task_result,
            commands::parallel_execution::update_task_error,
            commands::parallel_execution::cancel_parallel_execution,
            commands::parallel_execution::get_execution_result,
            commands::parallel_execution::delete_parallel_execution,
            commands::parallel_execution::start_parallel_execution,
            commands::parallel_execution::verify_parallel_execution,
            commands::parallel_execution::check_parallel_timeouts,
            // Scheduled task commands (基于 CronJobStore)
            commands::scheduled_task::list_scheduled_tasks,
            commands::scheduled_task::get_scheduled_task,
            commands::scheduled_task::create_scheduled_task,
            commands::scheduled_task::update_scheduled_task,
            commands::scheduled_task::delete_scheduled_task,
            commands::scheduled_task::pause_scheduled_task,
            commands::scheduled_task::resume_scheduled_task,
            commands::scheduled_task::execute_scheduled_task,
            commands::scheduled_task::get_scheduled_task_templates,
            commands::scheduled_task::create_daily_summary_task,
            commands::scheduled_task::create_backup_task,
            commands::scheduled_task::create_cleanup_task,
            commands::scheduled_task::load_scheduled_tasks_from_db,
            // Workflow template commands
            commands::workflow_template::list_workflow_templates,
            commands::workflow_template::get_workflow_template,
            commands::workflow_template::create_workflow_template,
            commands::workflow_template::update_workflow_template,
            commands::workflow_template::delete_workflow_template,
            commands::workflow_template::duplicate_workflow_template,
            commands::workflow_template::validate_workflow_template,
            commands::workflow_template::export_workflow_template,
            commands::workflow_template::import_workflow_template,
            commands::workflow_template::import_n8n_directory,
            commands::workflow_template::import_workflow_directory,
            commands::workflow_template::seed_preset_templates,
            commands::workflow_template::get_template_versions,
            commands::workflow_template::get_template_by_version,
            // Workflow AI commands
            commands::workflow_ai::generate_workflow_from_prompt,
            commands::workflow_ai::optimize_agent_prompt,
            commands::workflow_ai::recommend_nodes,
            commands::workflow_ai::workflow_ai_chat_stream,
            commands::workflow_ai::workflow_ai_chat_cancel,
            commands::workflow_ai_diagnose::llm_diagnose_workflow,
            // Platform integration commands
            commands::platform_integration::get_platform_config,
            commands::platform_integration::update_platform_config,
            commands::platform_integration::process_telegram_message,
            commands::platform_integration::process_discord_message,
            commands::platform_integration::create_platform_session,
            commands::platform_integration::get_active_sessions,
            commands::platform_integration::deactivate_platform_session,
            commands::platform_integration::send_telegram_message,
            commands::platform_integration::send_discord_message,
            commands::platform_integration::send_platform_message,
            commands::platform_integration::get_platform_statuses,
            commands::platform_integration::reconcile_platforms,
            commands::platform_integration::start_api_server,
            commands::platform_integration::stop_api_server,
            commands::platform_integration::process_platform_message,
            // Atomic Skill commands
            // Background Task commands
            commands::background_tasks::spawn_background_task,
            commands::background_tasks::list_background_tasks,
            commands::background_tasks::get_background_task_output,
            commands::background_tasks::stop_background_task,
            // Skill Decomposition commands
            commands::skill_decomposition::preview_decomposition,
            commands::skill_decomposition::confirm_decomposition,
            commands::skill_decomposition::generate_missing_tool,
            commands::skill_decomposition::check_tool_semantic_matches,
            commands::skill_decomposition::upgrade_tool_with_llm,
            commands::skill_decomposition::get_marketplace_skill_content,
            // Work Engine commands
            commands::work_engine::start_workflow_execution,
            commands::work_engine::pause_workflow_execution,
            commands::work_engine::resume_workflow_execution,
            commands::work_engine::cancel_workflow_execution,
            commands::work_engine::get_workflow_execution_status,
            commands::work_engine::list_workflow_executions,
            commands::work_engine::execute_workflow_node,
            commands::work_engine::list_node_executor_types,
            commands::work_engine::debug_run_workflow,
            commands::work_engine::set_workflow_breakpoints,
            commands::work_engine::resume_workflow_breakpoint,
            commands::work_engine::step_workflow_breakpoint,
            // User Profile & Style Migration commands
            commands::user_profile::get_user_profile,
            commands::user_profile::update_user_profile,
            commands::user_profile::clear_user_profile_data,
            commands::user_profile::style_get_profile,
            commands::user_profile::style_apply_code,
            commands::user_profile::style_apply_document,
            commands::user_profile::style_learn_code,
            commands::user_profile::style_learn_messages,
            commands::user_profile::style_export_profile,
            commands::user_profile::style_import_profile,
            commands::user_profile::style_get_stats,
            commands::tracer::tracer_start_span,
            commands::tracer::tracer_end_span,
            commands::tracer::tracer_record_error,
            commands::tracer::tracer_list_traces,
            commands::tracer::tracer_get_trace,
            commands::tracer::tracer_get_span,
            commands::tracer::tracer_get_metrics,
            commands::tracer::tracer_export_traces,
            commands::tracer::tracer_delete_trace,
            commands::tracer::tracer_delete_old_traces,
            commands::evaluator::evaluator_list_benchmarks,
            commands::evaluator::evaluator_get_benchmark,
            commands::evaluator::evaluator_run_benchmark,
            commands::evaluator::evaluator_generate_report,
            commands::evaluator::evaluator_list_datasets,
            commands::evaluator::evaluator_import_dataset,
            commands::evaluator::evaluator_export_report,
            commands::rl::rl_list_policies,
            commands::rl::rl_get_policy,
            commands::rl::rl_create_policy,
            commands::rl::rl_delete_policy,
            commands::rl::rl_get_stats,
            commands::rl::rl_record_experience,
            commands::rl::rl_train_policy,
            commands::rl::rl_export_model,
            commands::rl::rl_import_model,
            commands::research::generate_research_report,
            commands::reflection::reflect_on_task,
            commands::reflection::get_reflection_history,
            commands::reflection::clear_reflection_history,
            commands::reflection::get_reflection_insights,
            commands::reflection::search_reflection_insights,
            commands::reflection::get_reflection_insight_stats,
            commands::evolution::get_evolution_stats,
            commands::fine_tune::list_datasets,
            commands::fine_tune::get_dataset,
            commands::fine_tune::create_dataset,
            commands::fine_tune::add_sample,
            commands::fine_tune::delete_dataset,
            commands::fine_tune::list_training_jobs,
            commands::fine_tune::get_training_job,
            commands::fine_tune::create_training_job,
            commands::fine_tune::start_training_job,
            commands::fine_tune::cancel_training_job,
            commands::fine_tune::delete_training_job,
            commands::fine_tune::get_training_stats,
            commands::fine_tune::list_base_models,
            commands::fine_tune::list_lora_adapters,
            commands::fine_tune::set_active_model,
            commands::fine_tune::get_active_model,
            commands::tool_recommender::analyze_task,
            commands::tool_recommender::get_tool_recommendations,
            commands::tool_recommender::get_available_tools,
            commands::tool_recommender::get_tools_by_category,
            commands::tool_recommender::record_tool_usage,
            #[cfg(not(mobile))]
            commands::screen_vision::analyze_screen,
            #[cfg(not(mobile))]
            commands::screen_vision::analyze_image,
            #[cfg(not(mobile))]
            commands::screen_vision::find_element_on_screen,
            #[cfg(not(mobile))]
            commands::screen_vision::suggest_screen_action,
            #[cfg(not(mobile))]
            commands::screen_vision::click_element_at_position,
            #[cfg(not(mobile))]
            commands::screen_vision::execute_vision_action,
            // LLM Wiki commands
            commands::llm_wiki::llm_wiki_list,
            commands::llm_wiki::llm_wiki_create,
            commands::llm_wiki::llm_wiki_delete,
            commands::llm_wiki::llm_wiki_operations_list,
            commands::llm_wiki::llm_wiki_ingest,
            commands::llm_wiki::llm_wiki_compile,
            commands::llm_wiki::llm_wiki_query,
            commands::llm_wiki::llm_wiki_lint,
            commands::llm_wiki::llm_wiki_lint_update_score,
            commands::llm_wiki::llm_wiki_get_schema,
            commands::llm_wiki::llm_wiki_validate_frontmatter,
            commands::llm_wiki::llm_wiki_create_schema_version,
            commands::llm_wiki::llm_wiki_update_schema,
            commands::llm_wiki::llm_wiki_delete_schema,
            commands::llm_wiki::llm_wiki_lint_vault,
            commands::llm_wiki::llm_wiki_auto_fix,
            commands::llm_wiki::llm_wiki_ask,
            commands::llm_wiki::write_base64_to_file,
            commands::llm_wiki::wiki_sync_enqueue,
            commands::llm_wiki::wiki_sync_get_queue,
            commands::llm_wiki::wiki_sync_process,
            commands::llm_wiki::wiki_sync_process_pending,
            commands::llm_wiki::wiki_check_capacity,
            commands::llm_wiki::wiki_get_capacity_info,
            commands::llm_wiki::llm_wiki_get_purpose,
            commands::llm_wiki::llm_wiki_update_purpose,
            // Wiki notes commands
            commands::wiki::wiki_notes_list,
            commands::wiki::wiki_notes_get,
            commands::wiki::wiki_notes_get_by_path,
            commands::wiki::wiki_notes_create,
            commands::wiki::wiki_notes_update,
            commands::wiki::wiki_notes_delete,
            commands::wiki::rebuild_wiki_index,
            commands::wiki::wiki_notes_get_links,
            commands::wiki::wiki_notes_get_backlinks,
            commands::wiki::wiki_notes_sync_links,
            commands::wiki::wiki_notes_search,
            commands::wiki::get_wiki_graph,
            commands::wiki::wiki_graph_communities,
            commands::wiki::sync_note_to_knowledge_base,
            commands::wiki::sync_knowledge_document_to_wiki,
            commands::wiki::wiki_note_versions,
            commands::wiki::wiki_note_get_version,
            commands::wiki::wiki_note_restore_version,
            commands::wiki::wiki_template_list,
            commands::wiki::wiki_template_create,
            commands::wiki::wiki_template_delete,
            commands::wiki::wiki_note_create_from_template,
            commands::wiki::wiki_create_daily_note,
            commands::wiki::wiki_import_obsidian_vault,
            commands::wiki::wiki_export_markdown,
            commands::wiki::wiki_export_html,
            commands::wiki::wiki_note_export_pdf,
            // Unified source management
            commands::sources::list_all_sources,
            commands::sources::get_source_config,
            commands::sources::search_all_sources,
            commands::agency_expert::import_agency_experts,
            commands::agency_expert::list_agency_experts,
            commands::agency_expert::clear_agency_experts,
            commands::agency_expert::extract_expert_structure,
            commands::agency_expert::update_agency_expert,
            commands::agency_expert::delete_agency_expert,
            commands::agency_expert::export_agency_experts,
            commands::agent_profile::list_agent_profiles,
            commands::agent_profile::get_agent_profile,
            commands::agent_profile::create_agent_profile,
            commands::agent_profile::update_agent_profile,
            commands::agent_profile::delete_agent_profile,
            commands::agent_profile::import_agent_profiles_from_agency,
            commands::agent_profile::ensure_agent_profile,
            commands::agent_role::list_agent_roles,
            commands::agent_role::import_agent_roles,
            commands::agent_role::delete_agent_role,
            commands::agent_role::update_agent_role,
            // App config
            commands::app_config::get_app_config,
            commands::app_config::save_app_config,
            // Dream / consolidation
            commands::dream::dream_consolidate_now,
            commands::dream::dream_get_status,
            commands::dream::dream_set_config,
            // Plugin commands
            commands::plugin::plugin_list,
            commands::plugin::plugin_validate_source,
            commands::plugin::plugin_install,
            commands::plugin::plugin_enable,
            commands::plugin::plugin_disable,
            commands::plugin::plugin_uninstall,
            commands::plugin::plugin_update,
            // PTY
            commands::pty::pty_create_session,
            commands::pty::pty_kill_session,
            commands::pty::pty_remove_session,
            commands::pty::pty_write,
            commands::pty::pty_resize,
            commands::pty::pty_list_sessions,
            commands::pty::pty_analyze_output,
            commands::pty::pty_get_suggestions,
            // File authorizer
            commands::files::file_authorize,
            commands::files::file_check_authorization,
            commands::files::file_revoke_authorization,
            commands::files::request_file_permission,
            // Metrics
            commands::agent_nudge::get_invoke_metrics,
            commands::agent_nudge::proactive_convert_to_nudge,
            #[cfg(not(mobile))]
            crate::tray::set_tray_labels,
            // Session share
            commands::session_share::create_share_session,
            commands::session_share::join_share_session,
            commands::session_share::list_share_participants,
            // Crash diagnostics
            commands::crash_report::get_crash_log,
            // Service health check
            commands::health::get_service_health,
            // Personality commands
            commands::personality::personality_list,
            commands::personality::personality_get,
            commands::personality::personality_switch,
            commands::personality::personality_current,
            commands::personality::personality_create,
            commands::personality::personality_delete,
            // Migration commands
            commands::migration::migration_detect,
            commands::migration::migration_preview,
            commands::migration::migration_execute,
            commands::migration::migration_list_backups,
            commands::migration::migration_rollback,
        ])
        .setup(|app| {
            android_utils::mark_startup_phase("setup_start");

            #[cfg(target_os = "macos")]
            {
                use objc2::msg_send;
                use objc2::rc::Retained;
                use objc2::runtime::{AnyClass, AnyObject};
                // SAFETY:
                // 1. objc2 msg_send! 调用的都是 macOS Foundation 框架中文档完备的 API
                //    (NSUserDefaults、NSString)，其行为和线程安全性有明确保证。
                // 2. AnyClass::get() 使用 .expect() 进行检查，若类不存在会 panic，
                //    这在 #[cfg(target_os = "macos")] 限定下是可接受的——这些类在 macOS 上必然存在。
                // 3. c"" 语法的字符串常量是合法的 C 字符串，以 null 结尾，生命周期为 'static，
                //    传递给 stringWithUTF8String: 是安全的。
                // 4. Retained<AnyObject> 确保返回的 Objective-C 对象遵循正确的引用计数管理，
                //    不会提前释放或泄漏。
                unsafe {
                    let defaults_cls = AnyClass::get(c"NSUserDefaults").expect("NSUserDefaults class exists on macOS");
                    let defaults: Retained<AnyObject> = msg_send![defaults_cls, standardUserDefaults];
                    let str_cls = AnyClass::get(c"NSString").expect("NSString class exists on macOS");
                    let key: Retained<AnyObject> = msg_send![str_cls, stringWithUTF8String: c"AppleShowScrollBars".as_ptr()];
                    let value: Retained<AnyObject> = msg_send![str_cls, stringWithUTF8String: c"WhenScrolling".as_ptr()];
                    let _: () = msg_send![&*defaults, setObject: &*value, forKey: &*key];
                }
            }

            // ── 在主线程解析并创建 axagent_home ──
            // Android 子线程中 dirs::data_dir() 因缺少 JNI 上下文返回 None，
            // 回退到 / 导致 Permission denied。必须在主线程完成目录创建。
            let app_dir = {
                let dir = crate::paths::axagent_home();
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    tracing::error!("Failed to create AxAgent home dir: {}", e);
                    android_utils::report_fatal_error(&format!(
                        "Failed to create AxAgent home dir: {}",
                        e
                    ));
                    std::process::exit(1);
                }
                tracing::info!("axagent_home ready: {}", dir.display());
                dir
            };

            android_utils::mark_startup_phase("db_init_start");

            let db_result = match std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .or_else(|e| {
                        tracing::warn!("Failed to create multi-threaded runtime for DB init: {} — falling back to current-thread", e);
                        tokio::runtime::Builder::new_current_thread().enable_all().build()
                    })
                    .unwrap_or_else(|e| {
                        android_utils::report_fatal_error(&format!("Failed to create db init runtime: {}", e));
                        std::process::exit(1);
                    });
                rt.block_on(init::init_database_with_dir(app_dir))
            }).join() {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => {
                    tracing::error!("Database initialization failed: {}", e);
                    android_utils::report_fatal_error(&format!("Database init failed: {}", e));
                    #[cfg(target_os = "windows")]
                    {
                        windows_utils::show_error_dialog("AxAgent", &format!("数据库初始化失败: {}", e));
                    }
                    std::process::exit(1);
                }
                Err(e) => {
                    tracing::error!("DB init thread panicked: {:?}", e);
                    android_utils::report_fatal_error(&format!("DB init thread panicked: {:?}", e));
                    std::process::exit(1);
                }
            };

            android_utils::mark_startup_phase("db_init_done");

            // 在独立线程中运行初始化，避免在 Tauri 的 tokio runtime 内创建嵌套 Runtime
            android_utils::mark_startup_phase("state_init_start");
            let state = match std::thread::spawn(move || {
                init::state::create_app_state(db_result)
            }).join() {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    tracing::error!("App state init returned error: {}", e);
                    android_utils::report_fatal_error(&format!("App state init failed: {}", e));
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!("App state init thread panicked: {:?}", e);
                    android_utils::report_fatal_error(&format!("App state init thread panicked: {:?}", e));
                    std::process::exit(1);
                }
            };

            android_utils::mark_startup_phase("state_init_done");

            app.manage(state);

            let state = app.state::<AppState>();
            let sea_db = state.harness.db().clone();

            let sea_db2 = sea_db.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .or_else(|e| {
                        tracing::warn!("Failed to create multi-threaded runtime for session reset: {} — falling back to current-thread", e);
                        tokio::runtime::Builder::new_current_thread().enable_all().build()
                    })
                    .unwrap_or_else(|e| {
                        android_utils::report_fatal_error(&format!("Failed to create session reset runtime: {}", e));
                        std::process::exit(1);
                    });
                rt.block_on(async {
                    let _ = axagent_core::repo::agent_session::reset_running_sessions(&sea_db2).await;
                });
            }).join().unwrap_or_else(|e| {
                tracing::error!("Session reset thread panicked: {:?}", e);
            });

            // Initialize pricing configuration from pricing.toml
            commands::agent::init_pricing_config(app.handle());

            if let Some(home) = dirs::home_dir() {
                let user_md_path = home.join(".axagent").join("USER.md");
                if user_md_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&user_md_path) {
                        if let Some(profile) = axagent_trajectory::UserProfile::from_user_md(&content) {
                            let user_profile = state.user_profile.clone();
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new()
                                    .or_else(|e| {
                                        tracing::warn!("Failed to create multi-threaded runtime for user profile: {} — falling back to current-thread", e);
                                        tokio::runtime::Builder::new_current_thread().enable_all().build()
                                    })
                                    .unwrap_or_else(|e| {
                                        android_utils::report_fatal_error(&format!("Failed to create tokio runtime: {}", e));
                                        std::process::exit(1);
                                    });
                                rt.block_on(async {
                                    let mut p = user_profile.write().await;
                                    *p = profile;
                                    tracing::info!("[user-profile] Loaded profile from USER.md ({} preferences, {} expertise domains)",
                                        p.preferences.len(), p.expertise.len());
                                });
                            }).join().unwrap_or_else(|e| {
                                tracing::error!("User profile thread panicked: {:?}", e);
                            });
                        }
                    }
                }
            }

            if let Ok(persisted) = state.trajectory_storage.get_patterns() as Result<Vec<axagent_trajectory::TrajectoryPattern>, _> {
                if !persisted.is_empty() {
                    let pattern_count = persisted.len();
                    let pattern_learner = state.pattern_learner.clone();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new()
                            .or_else(|e| {
                                tracing::warn!("Failed to create multi-threaded runtime for pattern learner: {} — falling back to current-thread", e);
                                tokio::runtime::Builder::new_current_thread().enable_all().build()
                            })
                            .unwrap_or_else(|e| {
                                android_utils::report_fatal_error(&format!("Failed to create tokio runtime: {}", e));
                                std::process::exit(1);
                            });
                        rt.block_on(async {
                            let mut pl = pattern_learner.write().await;
                            for pattern in &persisted {
                                pl.learn_from_trajectory(&axagent_trajectory::Trajectory {
                                    id: pattern.id.clone(),
                                    session_id: String::new(),
                                    user_id: String::new(),
                                    topic: pattern.name.clone(),
                                    summary: pattern.description.clone(),
                                    outcome: if pattern.success_rate >= 0.5 {
                                        axagent_trajectory::TrajectoryOutcome::Success
                                    } else {
                                        axagent_trajectory::TrajectoryOutcome::Failure
                                    },
                                    duration_ms: 0,
                                    quality: axagent_trajectory::TrajectoryQuality {
                                        overall: pattern.average_quality,
                                        task_completion: pattern.average_quality,
                                        tool_efficiency: pattern.average_quality,
                                        reasoning_quality: pattern.average_quality,
                                        user_satisfaction: pattern.average_quality,
                                    },
                                    value_score: pattern.average_value_score,
                                    patterns: vec![],
                                    steps: vec![],
                                    rewards: vec![],
                                    created_at: pattern.created_at,
                                    replay_count: 0,
                                    last_replay_at: None,
                                });
                            }
                        });
                    }).join().unwrap_or_else(|e| {
                        tracing::error!("Pattern learner thread panicked: {:?}", e);
                    });
                    tracing::info!("[P5] Loaded {} persisted patterns into PatternLearner", pattern_count);
                }
            }

            let app_dir = state.app_data_dir.clone();

            #[cfg(not(mobile))]
            if let Some(main_window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                {
                    let _ = main_window.set_decorations(false);
                    let _ = main_window.set_minimizable(true);
                    let _ = main_window.set_maximizable(true);
                }

                if let Some(saved_state) = window_state::load_window_state(&app_dir) {
                    let restored_state = if let Ok(Some(monitor)) = main_window.current_monitor() {
                        let monitor_size = monitor.size().to_logical::<f64>(main_window.scale_factor().unwrap_or(1.0));
                        window_state::clamp_window_state_to_monitor(saved_state, monitor_size.width, monitor_size.height)
                    } else {
                        saved_state
                    };

                    let _ = main_window.set_size(tauri::LogicalSize::new(restored_state.width, restored_state.height));
                    if let (Some(x), Some(y)) = (restored_state.x, restored_state.y) {
                        let _ = main_window.set_position(tauri::LogicalPosition::new(x, y));
                    } else {
                        let _ = main_window.center();
                    }
                    if restored_state.fullscreen {
                        let _ = main_window.set_fullscreen(true);
                    } else if restored_state.maximized {
                        let _ = main_window.maximize();
                    }
                }
            }

            #[cfg(mobile)]
            if let Some(ref sync_engine) = state.sync_engine {
                tracing::info!("[mobile] Starting cloud sync engine...");
                let engine = sync_engine.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new()
                        .or_else(|e| {
                            tracing::warn!("Failed to create multi-threaded runtime for cloud sync: {} — falling back to current-thread", e);
                            tokio::runtime::Builder::new_current_thread().enable_all().build()
                        })
                        .unwrap_or_else(|e| {
                            android_utils::report_fatal_error(&format!("Failed to create cloud sync runtime: {}", e));
                            std::process::exit(1);
                        });
                    rt.block_on(async {
                        match engine.full_sync().await {
                            Ok(result) => {
                                let dl = result.pending_downloads.len();
                                let ul = result.pending_uploads.len();
                                tracing::info!("[mobile] Initial sync complete: {} pending downloads, {} pending uploads", dl, ul);
                            },
                            Err(e) => {
                                tracing::warn!("[mobile] Initial sync failed (non-critical): {}", e);
                            },
                        }
                    });
                }).join().unwrap_or_else(|e| {
                    tracing::error!("Mobile sync thread panicked: {:?}", e);
                });
            }

            let state = app.state::<AppState>();
            #[cfg(not(mobile))]
            let tray_language = {
                let db = state.harness.db().clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
                                android_utils::report_fatal_error(&format!("Failed to create tokio runtime: {}", e));
                                std::process::exit(1);
                            });
                    rt.block_on(axagent_core::repo::settings::get_settings(&db))
                        .map(|s| s.language)
                        .unwrap_or_else(|_| "en".to_string())
                }).join().unwrap_or_else(|e| {
                    tracing::error!("Tray language thread panicked: {:?}", e);
                    "en".to_string()
                })
            };
            #[cfg(mobile)]
            let tray_language = "en".to_string();
            init::services::start_background_services(app.handle(), &state, app_dir.clone(), tray_language);

            android_utils::mark_startup_phase("setup_complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                use std::sync::atomic::Ordering;
                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                        #[cfg(not(mobile))]
                        {
                            let app = window.app_handle();
                            let state = app.state::<AppState>();
                            let maximized = window.is_maximized().unwrap_or(false);
                            let fullscreen = window.is_fullscreen().unwrap_or(false);
                            let scale_factor = window.scale_factor().unwrap_or(1.0);
                            let prev = window_state::load_window_state(&state.app_data_dir);
                            if maximized || fullscreen {
                                if let Some(mut prev) = prev {
                                    prev.maximized = maximized;
                                    prev.fullscreen = fullscreen;
                                    let _ = window_state::save_window_state(&state.app_data_dir, prev);
                                }
                            } else if let (Ok(size), Ok(pos)) = (window.inner_size(), window.outer_position()) {
                                let logical_w = size.width as f64 / scale_factor;
                                let logical_h = size.height as f64 / scale_factor;
                                let logical_x = pos.x as f64 / scale_factor;
                                let logical_y = pos.y as f64 / scale_factor;
                                let _ = window_state::save_window_state(&state.app_data_dir, window_state::PersistedWindowState {
                                    width: logical_w, height: logical_h, maximized: false, fullscreen: false,
                                    x: Some(logical_x), y: Some(logical_y),
                                });
                            }
                        }
                    }
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        let app = window.app_handle();
                        let state = app.state::<AppState>();
                        if state.close_to_tray.load(Ordering::Acquire) {
                            let _ = window.hide();
                            api.prevent_close();
                        } else {
                            api.prevent_close();
                            let _ = app.emit("app-close-requested", ());
                        }
                    }
                    _ => {}
                }
            }
            if window.label() == "quickbar" {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!());

    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("build_done");

    let app = match build_result {
        Ok(app) => app,
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to build Tauri application: {}", error_msg);
            android_utils::report_fatal_error(&format!("Tauri build failed: {}", error_msg));
            #[cfg(target_os = "windows")]
            {
                let lower = error_msg.to_lowercase();
                if lower.contains("webview2") || lower.contains("webview") || lower.contains("edge")
                {
                    const WEBVIEW2_DOWNLOAD_URL: &str = "https://developer.microsoft.com/en-us/microsoft-edge/webview2/?form=MA13LH#download";
                    let user_ok = windows_utils::show_warning_ok_cancel(
                        "AxAgent",
                        "未检测到 Microsoft Edge WebView2 Runtime，AxAgent 无法启动。\n\n点击「确定」打开下载页面进行安装，安装完成后重新启动 AxAgent。",
                    );
                    if user_ok {
                        let _ = open::that(WEBVIEW2_DOWNLOAD_URL);
                    }
                } else {
                    windows_utils::show_error_dialog(
                        "AxAgent",
                        &format!("应用启动失败：{}", error_msg),
                    );
                }
            }
            std::process::exit(1);
        },
    };

    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("run_loop_start");

    app.run(|_app, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } = _event
        {
            if !has_visible_windows {
                if let Some(w) = _app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        }

        // 优雅关闭：通知后台任务停止并等待完成 (S-39)
        if let tauri::RunEvent::Exit = _event {
            let state = _app.state::<AppState>();
            state.shutdown_token.cancel();
            if let Some(flag) = state.skill_watcher_shutdown.get() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            tracing::info!("[shutdown] 正在停止后台任务...");

            let rt_handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
                tokio::runtime::Runtime::new()
                    .expect("Failed to create runtime for cleanup")
                    .handle()
                    .clone()
            });

            let timeout = std::time::Duration::from_secs(5);
            let await_handle = |handle: &std::sync::Arc<
                tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
            >,
                                name: &str| {
                let mut guard = rt_handle.block_on(handle.lock());
                if let Some(mut h) = guard.take() {
                    match rt_handle.block_on(async { tokio::time::timeout(timeout, &mut h).await })
                    {
                        Ok(Ok(())) => tracing::info!("[shutdown] {} 已优雅停止", name),
                        Ok(Err(e)) => tracing::warn!("[shutdown] {} join 错误: {}", name, e),
                        Err(_) => {
                            tracing::warn!("[shutdown] {} 超时 ({:?})，强制中止", name, timeout);
                            h.abort();
                        },
                    }
                }
            };

            await_handle(&state.auto_backup_handle, "auto_backup");
            await_handle(&state.webdav_sync_handle, "webdav_sync");
            await_handle(&state.api_server_handle, "api_server");
            await_handle(&state.trajectory_cleanup_handle, "trajectory_cleanup");
            // 集中式 TaskManager 兜底清理
            rt_handle.block_on(
                state
                    .task_manager
                    .shutdown(std::time::Duration::from_secs(5)),
            );
            tracing::info!("[shutdown] 退出完成");
        }
    });
}
