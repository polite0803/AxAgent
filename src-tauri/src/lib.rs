#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]

mod commands;
mod context_manager;
mod indexing;
mod init;
mod paths;
mod tray;
mod window_state;

#[cfg(target_os = "windows")]
mod windows_utils;

#[allow(clippy::disallowed_types)]
mod app_state;

use tauri::{Emitter, Manager};

pub use app_state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    axagent_core::builtin_tools::init_builtin_handlers();

    if let Err(e) = axagent_core::builtin_tools_registry::validate_builtin_tools() {
        tracing::warn!("Builtin tools validation failed: {}", e);
    }

    let builder = tauri::Builder::default();
    let builder = init::register_plugins(builder);

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
            commands::conversations::list_conversations,
            commands::conversations::create_conversation,
            commands::conversations::update_conversation,
            commands::conversations::delete_conversation,
            commands::conversations::branch_conversation,
            commands::conversations::search_conversations,
            commands::conversations_search::session_search,
            commands::conversations::send_message,
            commands::conversations::toggle_pin_conversation,
            commands::conversations::toggle_archive_conversation,
            commands::conversations::archive_conversation_to_knowledge_base,
            commands::conversations::list_archived_conversations,
            commands::conversations::regenerate_message,
            commands::conversations::regenerate_with_model,
            commands::conversations::cancel_stream,
            commands::conversations::list_message_versions,
            commands::conversations::switch_message_version,
            commands::conversations::send_system_message,
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
            commands::context_sources::list_context_sources,
            commands::context_sources::add_context_source,
            commands::context_sources::remove_context_source,
            commands::context_sources::toggle_context_source,
            // commands::search::list_search_providers,        // TODO: implement
            // commands::search::create_search_provider,       // TODO: implement
            // commands::search::update_search_provider,       // TODO: implement
            // commands::search::delete_search_provider,       // TODO: implement
            // commands::search::test_search_provider,         // TODO: implement
            // commands::search::execute_search,               // TODO: implement
            commands::mcp::list_mcp_servers,
            commands::mcp::create_mcp_server,
            commands::mcp::update_mcp_server,
            commands::mcp::delete_mcp_server,
            commands::mcp::test_mcp_server,
            commands::mcp::list_mcp_tools,
            commands::mcp::discover_mcp_tools,
            commands::mcp::list_tool_executions,
            commands::mcp::hot_reload_mcp_server,
            commands::context_sources::list_context_sources,
            commands::context_sources::add_context_source,
            commands::context_sources::remove_context_source,
            commands::context_sources::toggle_context_source,

            commands::local_tool::list_local_tools,
            commands::local_tool::toggle_local_tool,
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
            commands::skills_hub::skills_hub_search,
            commands::skills_hub::skills_hub_install,
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
            commands::terminal::git_get_branch,
            commands::terminal::git_status,
            commands::terminal::system_get_info,
            commands::terminal::path_complete,
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
            commands::desktop::get_desktop_capabilities,
            commands::desktop::send_desktop_notification,
            commands::desktop::get_window_state,
            commands::desktop::set_always_on_top,
            commands::desktop::set_close_to_tray,
            commands::desktop::force_quit,
            commands::desktop::apply_startup_settings,
            commands::desktop::test_proxy,
            commands::desktop::open_devtools,
            commands::desktop::list_system_fonts,
            commands::desktop::minimize_window,
            commands::desktop::toggle_maximize_window,
            commands::dashboard::dashboard_list_plugins,
            commands::dashboard::dashboard_register_plugin,
            commands::dashboard::dashboard_unregister_plugin,
            commands::dashboard::dashboard_enable_plugin,
            commands::dashboard::dashboard_disable_plugin,
            commands::dashboard::dashboard_render_panel,
            commands::dashboard::dashboard_reload_plugins,
            commands::computer_control::screen_capture,
            commands::computer_control::find_ui_elements,
            commands::computer_control::mouse_click,
            commands::computer_control::type_text,
            commands::computer_control::press_key,
            commands::computer_control::mouse_scroll,
            commands::browser::browser_navigate,
            commands::browser::browser_screenshot,
            commands::browser::browser_click,
            commands::browser::browser_fill,
            commands::browser::browser_type,
            commands::browser::browser_extract_text,
            commands::browser::browser_extract_all,
            commands::browser::browser_get_content,
            commands::browser::browser_wait_for,
            commands::browser::browser_select,
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
            commands::agent::agent_approve,
            commands::agent::agent_respond_ask,
            commands::agent::agent_backup_and_clear_sdk_context,
            commands::agent::agent_restore_sdk_context_from_backup,
            commands::agent::workflow_create,
            commands::agent::workflow_execute,
            commands::agent::workflow_execute_with_session,
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
            commands::agent::save_skill_workflow_from_llm,
            commands::agent::force_save_skill_workflow,
            commands::agent::workflow_get_steps,
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
            commands::agent_analytics::trajectory_stats,
            commands::agent_analytics::trajectory_list,
            commands::agent_analytics::pattern_stats,
            commands::agent_analytics::closed_loop_status,
            commands::agent_analytics::rl_config,
            commands::agent_analytics::rl_export_training_data,
            commands::agent_analytics::rl_compute_rewards,
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
            // Scheduled task commands
            commands::scheduled_task::create_scheduled_task,
            commands::scheduled_task::create_daily_summary_task,
            commands::scheduled_task::create_backup_task,
            commands::scheduled_task::create_cleanup_task,
            commands::scheduled_task::get_scheduled_task,
            commands::scheduled_task::list_scheduled_tasks,
            commands::scheduled_task::get_scheduled_task_templates,
            commands::scheduled_task::list_due_tasks,
            commands::scheduled_task::update_scheduled_task,
            commands::scheduled_task::delete_scheduled_task,
            commands::scheduled_task::pause_scheduled_task,
            commands::scheduled_task::resume_scheduled_task,
            commands::scheduled_task::record_task_execution,
            commands::scheduled_task::get_task_execution_history,
            commands::scheduled_task::get_next_scheduled_time,
            commands::scheduled_task::register_task_definition,
            commands::scheduled_task::execute_scheduled_task,
            commands::scheduled_task::load_scheduled_tasks_from_db,
            // Workflow template commands - TODO: implement module
            // commands::workflow_template::list_workflow_templates,
            // commands::workflow_template::get_workflow_template,
            // commands::workflow_template::create_workflow_template,
            // commands::workflow_template::update_workflow_template,
            // commands::workflow_template::delete_workflow_template,
            // commands::workflow_template::duplicate_workflow_template,
            // commands::workflow_template::validate_workflow_template,
            // commands::workflow_template::export_workflow_template,
            // commands::workflow_template::import_workflow_template,
            // commands::workflow_template::seed_preset_templates,
            // commands::workflow_template::get_template_versions,
            // commands::workflow_template::get_template_by_version,
            // Workflow AI commands
            commands::workflow_ai::generate_workflow_from_prompt,
            commands::workflow_ai::optimize_agent_prompt,
            commands::workflow_ai::recommend_nodes,
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
            // Atomic Skill commands
            commands::atomic_skills::list_atomic_skills,
            commands::atomic_skills::get_atomic_skill,
            commands::atomic_skills::create_atomic_skill,
            commands::atomic_skills::update_atomic_skill,
            commands::atomic_skills::delete_atomic_skill,
            commands::atomic_skills::toggle_atomic_skill,
            commands::atomic_skills::check_semantic_uniqueness,
            commands::atomic_skills::get_skill_references,
            commands::atomic_skills::execute_atomic_skill,
            commands::atomic_skills::check_skill_semantic_matches,
            commands::atomic_skills::upgrade_skill_with_llm,
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
            commands::work_engine::migrate_workflow_nodes,
            commands::work_engine::migrate_all_workflows,
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
            commands::screen_vision::analyze_screen,
            commands::screen_vision::find_element_on_screen,
            commands::screen_vision::suggest_screen_action,
            commands::screen_vision::click_element_at_position,
            commands::screen_vision::execute_vision_action,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use objc2::msg_send;
                use objc2::rc::Retained;
                use objc2::runtime::{AnyClass, AnyObject};
                unsafe {
                    let defaults_cls = AnyClass::get(c"NSUserDefaults").unwrap();
                    let defaults: Retained<AnyObject> = msg_send![defaults_cls, standardUserDefaults];
                    let str_cls = AnyClass::get(c"NSString").unwrap();
                    let key: Retained<AnyObject> = msg_send![str_cls, stringWithUTF8String: c"AppleShowScrollBars".as_ptr()];
                    let value: Retained<AnyObject> = msg_send![str_cls, stringWithUTF8String: c"WhenScrolling".as_ptr()];
                    let _: () = msg_send![&*defaults, setObject: &*value, forKey: &*key];
                }
            }

            let db_result = match init::init_database() {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("Database initialization failed: {}", e);
                    #[cfg(target_os = "windows")]
                    {
                        windows_utils::show_error_dialog("AxAgent", &format!("æ•°æ®åº“åˆå§‹åŒ–å¤±è´¥: {}", e));
                    }
                    std::process::exit(1);
                }
            };

            let state = init::state::create_app_state(db_result);

            app.manage(state);

            let state = app.state::<AppState>();
            let sea_db = state.sea_db.clone();
            let rt = tokio::runtime::Runtime::new().unwrap();
            let _ = rt.block_on(axagent_core::repo::agent_session::reset_running_sessions(&sea_db));
            let _ = rt.block_on(commands::scheduled_task::load_tasks_from_db_internal(&sea_db, &state.scheduled_task_service));

            if let Some(home) = dirs::home_dir() {
                let user_md_path = home.join(".axagent").join("USER.md");
                if user_md_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&user_md_path) {
                        if let Some(profile) = axagent_trajectory::UserProfile::from_user_md(&content) {
                            let mut p = state.user_profile.write().unwrap();
                            *p = profile;
                            tracing::info!("[user-profile] Loaded profile from USER.md ({} preferences, {} expertise domains)",
                                p.preferences.len(), p.expertise.len());
                        }
                    }
                }
            }

            if let Ok(persisted) = state.trajectory_storage.get_patterns() as Result<Vec<axagent_trajectory::TrajectoryPattern>, _> {
                if !persisted.is_empty() {
                    let mut pl = state.pattern_learner.write().unwrap();
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
                    tracing::info!("[P5] Loaded {} persisted patterns into PatternLearner", persisted.len());
                }
            }

            let app_dir = state.app_data_dir.clone();

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

            let state = app.state::<AppState>();
            let tray_language = {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(axagent_core::repo::settings::get_settings(&state.sea_db))
                    .map(|s| s.language)
                    .unwrap_or_else(|_| "en".to_string())
            };
            init::services::start_background_services(app.handle(), &state, app_dir.clone(), tray_language);

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                use std::sync::atomic::Ordering;
                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
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
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        let app = window.app_handle();
                        let state = app.state::<AppState>();
                        if state.close_to_tray.load(Ordering::Relaxed) {
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
        })
        .build(tauri::generate_context!());

    let app = match build_result {
        Ok(app) => app,
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to build Tauri application: {}", error_msg);
            #[cfg(target_os = "windows")]
            {
                let lower = error_msg.to_lowercase();
                if lower.contains("webview2") || lower.contains("webview") || lower.contains("edge")
                {
                    let user_ok = windows_utils::show_warning_ok_cancel("AxAgent",
                        "æœªæ£€æµ‹åˆ° Microsoft Edge WebView2 Runtimeï¼ŒAxAgent æ— æ³•å¯åŠ¨ã€‚\n\nç‚¹å‡»ã€Œç¡®å®šã€æ‰“å¼€ä¸‹è½½é¡µé¢è¿›è¡Œå®‰è£…ï¼Œå®‰è£…å®ŒæˆåŽé‡æ–°å¯åŠ¨ AxAgentã€‚");
                    if user_ok {
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "https://developer.microsoft.com/en-us/microsoft-edge/webview2/?form=MA13LH#download"])
                            .spawn();
                    }
                } else {
                    windows_utils::show_error_dialog(
                        "AxAgent",
                        &format!("åº”ç”¨å¯åŠ¨å¤±è´¥ï¼š{}", error_msg),
                    );
                }
            }
            std::process::exit(1);
        }
    };

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
    });
}
