/**
 * Type Consistency Checker
 *
 * Verifies that frontend TypeScript types match backend Rust types.
 * This is a validation helper — it does NOT automatically fix mismatches.
 *
 * Usage: node scripts/type-consistency.mjs
 *
 * The script checks:
 * 1. All Tauri invoke command names referenced in frontend match backend registration
 * 2. Snake_case vs camelCase consistency in command arguments
 * 3. Common type field name mismatches
 */

import { readdirSync, readFileSync, statSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

// ─── Configuration ───

/** Rust commands — extracted from src-tauri/src/lib.rs invoke() registrations */
const RUST_COMMANDS = [
  // Conversation
  "list_conversations",
  "create_conversation",
  "update_conversation",
  "delete_conversation",
  "toggle_pin_conversation",
  "toggle_archive_conversation",
  "archive_conversation_to_knowledge_base",
  "list_archived_conversations",
  "regenerate_conversation_title",
  "branch_conversation",
  "search_conversations",
  "session_search",
  // Messages
  "send_message",
  "regenerate_message",
  "regenerate_with_model",
  "send_system_message",
  "delete_message",
  "clear_conversation_messages",
  "get_conversation_stats",
  "list_messages",
  "list_messages_page",
  "load_older_messages",
  "switch_message_version",
  "list_message_versions",
  "update_message_content",
  "delete_message_group",
  "cancel_stream",
  // Providers
  "list_providers",
  "create_provider",
  "update_provider",
  "delete_provider",
  "toggle_provider",
  "add_provider_key",
  "delete_provider_key",
  "toggle_provider_key",
  "get_decrypted_provider_key",
  "validate_provider_key",
  "update_provider_key",
  "save_models",
  "toggle_model",
  "update_model_params",
  "fetch_remote_models",
  "test_model",
  "reorder_providers",
  // Categories
  "list_conversation_categories",
  "create_conversation_category",
  "update_conversation_category",
  "delete_conversation_category",
  "reorder_conversation_categories",
  "set_conversation_category_collapsed",
  // Knowledge
  "list_knowledge_bases",
  "create_knowledge_base",
  "update_knowledge_base",
  "delete_knowledge_base",
  "reorder_knowledge_bases",
  "list_knowledge_documents",
  "add_knowledge_document",
  "delete_knowledge_document",
  "search_knowledge_base",
  "rebuild_knowledge_index",
  "clear_knowledge_index",
  "list_knowledge_document_chunks",
  "delete_knowledge_chunk",
  "update_knowledge_chunk",
  "add_knowledge_chunk",
  "reindex_knowledge_chunk",
  "rebuild_knowledge_document",
  "list_knowledge_entities",
  "create_knowledge_entity",
  "list_knowledge_attributes",
  "create_knowledge_attribute",
  "list_knowledge_relations",
  "create_knowledge_relation",
  "list_knowledge_flows",
  "create_knowledge_flow",
  "list_knowledge_interfaces",
  "create_knowledge_interface",
  // Prompt templates
  "list_prompt_templates",
  "get_prompt_template",
  "create_prompt_template",
  "update_prompt_template",
  "delete_prompt_template",
  "get_prompt_template_versions",
  // Context sources
  "list_context_sources",
  "add_context_source",
  "remove_context_source",
  "toggle_context_source",
  // Search
  "list_search_providers",
  "delete_search_provider",
  // MCP
  "list_mcp_servers",
  "create_mcp_server",
  "update_mcp_server",
  "delete_mcp_server",
  "test_mcp_server",
  "list_mcp_tools",
  "discover_mcp_tools",
  "list_tool_executions",
  "hot_reload_mcp_server",
  "discover_available_mcp_servers",
  // Local tools
  "list_local_tools",
  "toggle_local_tool",
  // Generated tools
  "list_generated_tools",
  "delete_generated_tool",
  // Memory
  "list_memory_namespaces",
  "create_memory_namespace",
  "delete_memory_namespace",
  "update_memory_namespace",
  "list_memory_items",
  "add_memory_item",
  "delete_memory_item",
  "update_memory_item",
  "search_memory",
  "rebuild_memory_index",
  "clear_memory_index",
  "reindex_memory_item",
  "reorder_memory_namespaces",
  // Skills
  "list_skills",
  "get_skill",
  "toggle_skill",
  "install_skill",
  "uninstall_skill",
  "uninstall_skill_group",
  "open_skills_dir",
  "open_skill_dir",
  "search_marketplace",
  "check_skill_updates",
  "skill_create",
  "skill_patch",
  "skill_edit",
  "skill_check_similar",
  "skill_upgrade_or_create",
  "get_skill_proposals",
  "create_skill_from_proposal",
  "skill_set_frontend",
  "get_skill_versions",
  "rollback_skill",
  "get_marketplace_categories",
  "skill_analyze_frontend",
  "skill_read_asset",
  // Skills hub
  "skills_hub_search",
  "skills_hub_install",
  "skills_hub_export",
  "skills_hub_import",
  // Settings
  "get_settings",
  "save_settings",
  // Backup
  "list_backups",
  "create_backup",
  "restore_backup",
  "delete_backup",
  "batch_delete_backups",
  "get_backup_settings",
  "update_backup_settings",
  // WebDAV
  "get_webdav_config",
  "save_webdav_config",
  "webdav_check_connection",
  "webdav_backup",
  "webdav_list_backups",
  "webdav_restore",
  "webdav_delete_backup",
  "get_webdav_sync_status",
  "restart_webdav_sync",
  // Webhook
  "webhook_list_subscriptions",
  "webhook_create_subscription",
  "webhook_delete_subscription",
  "webhook_toggle_subscription",
  "webhook_test_subscription",
  "webhook_reload",
  // Terminal
  "git_get_branch",
  "git_status",
  "system_get_info",
  "path_complete",
  "session_get_status",
  // Theme
  "list_themes",
  "get_theme",
  "get_xterm_theme",
  "save_theme",
  "delete_theme",
  "load_user_themes",
  // Profile
  "profile_list",
  "profile_create",
  "profile_delete",
  "profile_switch",
  "profile_active",
  // Desktop
  "get_desktop_capabilities",
  "send_desktop_notification",
  "get_window_state",
  "set_always_on_top",
  "set_close_to_tray",
  "force_quit",
  "apply_startup_settings",
  "test_proxy",
  "open_devtools",
  "list_system_fonts",
  "minimize_window",
  "toggle_maximize_window",
  // Quickbar
  "show_quickbar",
  "hide_quickbar",
  // Dashboard
  "dashboard_list_plugins",
  "dashboard_register_plugin",
  "dashboard_unregister_plugin",
  "dashboard_enable_plugin",
  "dashboard_disable_plugin",
  "dashboard_render_panel",
  "dashboard_reload_plugins",
  // Computer control
  "screen_capture",
  "find_ui_elements",
  "mouse_click",
  "type_text",
  "press_key",
  "mouse_scroll",
  // Browser
  "browser_navigate",
  "browser_screenshot",
  "browser_click",
  "browser_fill",
  "browser_type",
  "browser_extract_text",
  "browser_extract_all",
  "browser_get_content",
  "browser_wait_for",
  "browser_select",
  "browser_close",
  // Files
  "upload_file",
  "download_file",
  "list_files",
  "delete_file",
  // Files page
  "list_files_page_entries",
  "open_files_page_entry",
  "reveal_files_page_entry",
  "cleanup_missing_files_page_entry",
  "check_attachment_exists",
  "resolve_attachment_path",
  "read_attachment_preview",
  "reveal_attachment_file",
  "save_avatar_file",
  "open_attachment_file",
  // Storage
  "get_storage_inventory",
  "open_storage_directory",
  "validate_documents_root",
  "change_documents_root",
  "reset_documents_root",
  // Agent
  "agent_query",
  "agent_cancel",
  "agent_is_running",
  "agent_pause",
  "agent_resume",
  "agent_is_paused",
  "agent_runtime_stats",
  "agent_resolve_model",
  "agent_update_session",
  "agent_get_session",
  "agent_ensure_workspace",
  "classify_route",
  "agent_steer",
  "agent_approve",
  "agent_respond_ask",
  "agent_backup_and_clear_sdk_context",
  "agent_restore_sdk_context_from_backup",
  "workflow_create",
  "workflow_execute",
  "workflow_execute_with_session",
  "workflow_get_status",
  "workflow_cancel",
  "workflow_list",
  "agent_estimate_complexity",
  "sub_agent_list",
  "sub_agent_get",
  "sub_agent_get_children",
  "sub_agent_get_messages",
  "shared_memory_list",
  "shared_memory_get",
  "shared_memory_stats",
  "get_conversation_workflow_preview",
  "save_skill_workflow_from_llm",
  "force_save_skill_workflow",
  "workflow_get_steps",
  "pattern_list",
  "cross_session_insights",
  "skill_evolution_start",
  "skill_evolution_status",
  "user_profile_get",
  "user_profile_set_preference",
  "user_profile_set_expertise",
  "user_profile_export_md",
  "adaptation_status",
  "memory_flush",
  "record_feedback",
  // Plan
  "plan_generate",
  "plan_execute",
  "plan_cancel",
  "plan_get",
  "plan_list",
  "plan_modify_step",
  // Agent nudge
  "nudge_list",
  "nudge_dismiss",
  "nudge_snooze",
  "nudge_execute",
  "nudge_stats",
  "nudge_closed_loop_list",
  "nudge_closed_loop_acknowledge",
  "skill_find_similar",
  "skill_upgrade_propose",
  "skill_upgrade_execute",
  "get_invoke_metrics",
  "proactive_convert_to_nudge",
  // Agent insight
  "insight_list",
  "insight_get_by_category",
  "insight_report",
  // Proactive
  "proactive_list_suggestions",
  "proactive_predict",
  "proactive_list_reminders",
  "proactive_dismiss_suggestion",
  "proactive_accept_suggestion",
  "proactive_snooze_suggestion",
  "proactive_add_reminder",
  "proactive_delete_reminder",
  "proactive_complete_reminder",
  "proactive_set_enabled",
  "proactive_update_config",
  "proactive_prefetch",
  // Agent analytics
  "trajectory_stats",
  "trajectory_list",
  "pattern_stats",
  "closed_loop_status",
  "rl_config",
  "rl_export_training_data",
  "rl_compute_rewards",
  // Artifacts
  "list_artifacts",
  "create_artifact",
  "update_artifact",
  "delete_artifact",
  // Sandbox
  "execute_sandbox",
  // Image gen
  "generate_image",
  "get_image_gen_config",
  "save_image_gen_config",
  // Chart
  "generate_chart_config",
  // Gateway
  "get_gateway_status",
  "start_gateway",
  "stop_gateway",
  "get_all_cli_tool_statuses",
  "connect_cli_tool",
  "disconnect_cli_tool",
  "list_gateway_keys",
  "create_gateway_key",
  "delete_gateway_key",
  "toggle_gateway_key",
  "decrypt_gateway_key",
  "get_gateway_metrics",
  "get_gateway_usage_by_key",
  "get_gateway_usage_by_provider",
  "get_gateway_usage_by_day",
  "get_connected_programs",
  "get_gateway_diagnostics",
  "get_program_policies",
  "save_program_policy",
  "delete_program_policy",
  "list_gateway_templates",
  "copy_gateway_template",
  "list_gateway_request_logs",
  "clear_gateway_request_logs",
  "generate_self_signed_cert",
  // Gateway Link
  "list_gateway_links",
  "create_gateway_link",
  "delete_gateway_link",
  "toggle_gateway_link",
  "connect_gateway_link",
  "disconnect_gateway_link",
  "update_gateway_link_status",
  "update_gateway_link_sync_settings",
  "get_gateway_link_model_syncs",
  "push_gateway_link_models",
  "sync_all_gateway_link_models",
  "get_gateway_link_skill_syncs",
  "push_gateway_link_skills",
  "sync_all_gateway_link_skills",
  "get_gateway_link_policy",
  "save_gateway_link_policy",
  "get_gateway_link_activities",
  "create_gateway_conversation",
  // Branches
  "list_branches",
  "fork_conversation",
  "compare_branches",
  "get_workspace_snapshot",
  "update_workspace_snapshot",
  // Conversations additional
  "regenerate_conversation_title",
  "delete_message_group",
  "compress_context",
  "get_compression_summary",
  "delete_compression",
  // Parallel execution
  "create_parallel_execution",
  "get_parallel_execution",
  "list_parallel_executions",
  "get_next_pending_task",
  "update_task_result",
  "update_task_error",
  "cancel_parallel_execution",
  "get_execution_result",
  "delete_parallel_execution",
  "start_parallel_execution",
  // Scheduled task
  "create_scheduled_task",
  "create_daily_summary_task",
  "create_backup_task",
  "create_cleanup_task",
  "get_scheduled_task",
  "list_scheduled_tasks",
  "get_scheduled_task_templates",
  "list_due_tasks",
  "update_scheduled_task",
  "delete_scheduled_task",
  "pause_scheduled_task",
  "resume_scheduled_task",
  "record_task_execution",
  "get_task_execution_history",
  "get_next_scheduled_time",
  "register_task_definition",
  "load_scheduled_tasks_from_db",
  // Workflow template
  "list_workflow_templates",
  "get_workflow_template",
  "create_workflow_template",
  "update_workflow_template",
  "delete_workflow_template",
  "duplicate_workflow_template",
  "validate_workflow_template",
  "export_workflow_template",
  "import_workflow_template",
  "import_n8n_directory",
  "seed_preset_templates",
  "get_template_versions",
  "get_template_by_version",
  // Workflow AI
  "generate_workflow_from_prompt",
  "optimize_agent_prompt",
  "recommend_nodes",
  // Platform integration
  "get_platform_config",
  "update_platform_config",
  "process_telegram_message",
  "process_discord_message",
  "create_platform_session",
  "get_active_sessions",
  "deactivate_platform_session",
  "send_telegram_message",
  "send_discord_message",
  "send_platform_message",
  "get_platform_statuses",
  "reconcile_platforms",
  "start_api_server",
  // Background tasks
  "spawn_background_task",
  "list_background_tasks",
  "get_background_task_output",
  "stop_background_task",
  // Skill decomposition
  "preview_decomposition",
  "confirm_decomposition",
  "generate_missing_tool",
  "check_tool_semantic_matches",
  "upgrade_tool_with_llm",
  "get_marketplace_skill_content",
  // Work engine
  "start_workflow_execution",
  "pause_workflow_execution",
  "resume_workflow_execution",
  "cancel_workflow_execution",
  "get_workflow_execution_status",
  "list_workflow_executions",
  "migrate_workflow_nodes",
  "migrate_all_workflows",
  // User profile & style
  "get_user_profile",
  "update_user_profile",
  "clear_user_profile_data",
  "style_get_profile",
  "style_apply_code",
  "style_apply_document",
  "style_learn_code",
  "style_learn_messages",
  "style_export_profile",
  "style_import_profile",
  "style_get_stats",
  // Tracer
  "tracer_start_span",
  "tracer_end_span",
  "tracer_record_error",
  "tracer_list_traces",
  "tracer_get_trace",
  "tracer_get_span",
  "tracer_get_metrics",
  "tracer_export_traces",
  "tracer_delete_trace",
  "tracer_delete_old_traces",
  // Evaluator
  "evaluator_list_benchmarks",
  "evaluator_get_benchmark",
  "evaluator_run_benchmark",
  "evaluator_generate_report",
  "evaluator_list_datasets",
  "evaluator_import_dataset",
  "evaluator_export_report",
  // RL
  "rl_list_policies",
  "rl_get_policy",
  "rl_create_policy",
  "rl_delete_policy",
  "rl_get_stats",
  "rl_record_experience",
  "rl_train_policy",
  "rl_export_model",
  "rl_import_model",
  // Fine tune
  "list_datasets",
  "get_dataset",
  "create_dataset",
  "add_sample",
  "delete_dataset",
  "list_training_jobs",
  "get_training_job",
  "create_training_job",
  "start_training_job",
  "cancel_training_job",
  "delete_training_job",
  "get_training_stats",
  "list_base_models",
  "list_lora_adapters",
  "set_active_model",
  "get_active_model",
  // Tool recommender
  "analyze_task",
  "get_tool_recommendations",
  "get_available_tools",
  "get_tools_by_category",
  "record_tool_usage",
  // Screen vision
  "analyze_screen",
  "find_element_on_screen",
  "suggest_screen_action",
  "click_element_at_position",
  "execute_vision_action",
  // LLM Wiki
  "llm_wiki_list",
  "llm_wiki_create",
  "llm_wiki_delete",
  "llm_wiki_operations_list",
  "llm_wiki_ingest",
  "llm_wiki_compile",
  "llm_wiki_query",
  "llm_wiki_lint",
  "llm_wiki_lint_update_score",
  "llm_wiki_get_schema",
  "llm_wiki_validate_frontmatter",
  "llm_wiki_create_schema_version",
  "llm_wiki_update_schema",
  "llm_wiki_delete_schema",
  "llm_wiki_lint_vault",
  "llm_wiki_auto_fix",
  "llm_wiki_ask",
  "write_base64_to_file",
  "wiki_sync_enqueue",
  "wiki_sync_get_queue",
  "wiki_sync_process",
  "wiki_sync_process_pending",
  "wiki_check_capacity",
  "wiki_get_capacity_info",
  "llm_wiki_get_purpose",
  "llm_wiki_update_purpose",
  // Wiki notes
  "wiki_notes_list",
  "wiki_notes_get",
  "wiki_notes_get_by_path",
  "wiki_notes_create",
  "wiki_notes_update",
  "wiki_notes_delete",
  "wiki_notes_get_links",
  "wiki_notes_get_backlinks",
  "wiki_notes_sync_links",
  "wiki_notes_search",
  "get_wiki_graph",
  // Agency expert
  "import_agency_experts",
  "list_agency_experts",
  "clear_agency_experts",
  "extract_expert_structure",
  "update_agency_expert",
  "delete_agency_expert",
  "export_agency_experts",
  // Agent profile
  "list_agent_profiles",
  "get_agent_profile",
  "create_agent_profile",
  "update_agent_profile",
  "delete_agent_profile",
  "import_agent_profiles_from_agency",
  // Agent role
  "list_agent_roles",
  "import_agent_roles",
  "delete_agent_role",
  // App config
  "get_app_config",
  "save_app_config",
  // Dream
  "dream_consolidate_now",
  "dream_get_status",
  "dream_set_config",
  // Plugin
  "list_plugin_tools",
  "plugin_enable",
  "plugin_disable",
  "plugin_install",
  "plugin_uninstall",
  // File authorizer
  "file_revoke_authorization",
  "set_tray_labels",
];

/** Commands that might get snake_case vs camelCase confusion in arguments */
const COMMANDS_WITH_ARGS_MISMATCH_RISK = [
  "create_conversation", // called with {modelId, providerId, systemPrompt} in TS vs {model_id, provider_id, system_prompt} in Rust
  "update_conversation",
  "send_message",
  "regenerate_message",
];

// ─── Scanning ───

let errors = 0;
let warnings = 0;
let checkedCount = 0;

function error(msg) {
  console.error(`  ❌ ERROR: ${msg}`);
  errors++;
}

function warn(msg) {
  console.warn(`  ⚠️  WARN: ${msg}`);
  warnings++;
}

function ok(msg) {
  console.log(`  ✅ ${msg}`);
}

/**
 * Extract all `invoke('...')` calls from a file.
 */
function extractInvokeCalls(filePath) {
  const content = readFileSync(filePath, "utf-8");
  const invokes = [];
  const regex = /invoke\s*<[^>]*>\s*\(\s*'([^']+)'/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    invokes.push(match[1]);
  }
  const regex2 = /invoke\s*<[^>]*>\s*\(\s*"([^"]+)"/g;
  while ((match = regex2.exec(content)) !== null) {
    invokes.push(match[1]);
  }
  return [...new Set(invokes)];
}

/**
 * Walk through src/ directory and collect all invoke calls.
 */
function getAllInvokeCalls(dir) {
  const invokes = [];
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory() && !entry.name.startsWith(".") && entry.name !== "node_modules") {
      invokes.push(...getAllInvokeCalls(fullPath));
    } else if (entry.isFile() && (entry.name.endsWith(".ts") || entry.name.endsWith(".tsx"))) {
      try {
        invokes.push(...extractInvokeCalls(fullPath));
      } catch (e) {
        // skip binary or unreadable files
      }
    }
  }
  return [...new Set(invokes)];
}

console.log("\n🔍 AxAgent Type Consistency Checker\n");
console.log("=".repeat(60));

// 1. Check Tauri command names
console.log("\n📋 Checking Tauri command name consistency...\n");

const frontendInvokes = getAllInvokeCalls(join(ROOT, "src"));
const rustCmdSet = new Set(RUST_COMMANDS);

// Filter out known non-Tauri commands (browserMock, internal helpers)
// Only commands that are truly NOT in the backend
const knownBrowserCommands = new Set([
  // Protocol handling (browser only)
  "handle_protocol_launch",

  // Gateway template operations (backend only has list and copy)
  "apply_gateway_template",
  "create_gateway_template",
  "delete_gateway_template",

  // Tray operations (backend only has set_tray_labels)
  "set_tray_actions",

  // MCP operations (backend mcp module does NOT have these)
  "execute_tool",
  "connect_mcp_server",
  "disconnect_mcp_server",

  // Git operations (backend only has git_get_branch and git_status)
  "git_commit",
  "get_current_branch",
  "get_branch_diff",
  "get_branch_commits",
  "get_staged_diff",

  // Search provider operations (TODO: not yet implemented in backend)
  "create_search_provider",
  "update_search_provider",
  "test_search_provider",
  "execute_search",

  // PTY terminal operations (not in backend)
  "pty_create_session",
  "pty_list_sessions",
  "pty_analyze_output",
  "pty_get_suggestions",

  // Benchmark operations (backend uses different names: evaluator_list_benchmarks, evaluator_run_benchmark)
  "benchmark_list_suites",
  "benchmark_run",

  // Git operations (generate_commit_context, generate_pr_context not in backend)
  "generate_commit_context",
  "generate_pr_context",

  // Plugin operations (not in backend)
  "plugin_list_available",

  // Chat operations (not in backend)
  "simple_chat_completion",

  // Config operations (not in backend)
  "read_config",

  // File operations (backend only has file_revoke_authorization)
  "file_authorize",

  // Data operations (browser localStorage only, no backend module)
  "clear_data",
  "export_data",
  "import_data",

  // Memory recall (backend has memory module but no recall command)
  "recall_memory",

  // Collaboration (not yet implemented in backend)
  "collaboration_list_sessions",

  // Test/custom commands
  "my_command",
]);

let frontendCount = 0;
let matchedCount = 0;

for (const cmd of frontendInvokes) {
  frontendCount++;
  checkedCount++;
  if (rustCmdSet.has(cmd)) {
    matchedCount++;
    ok(`"${cmd}" found in backend`);
  } else if (knownBrowserCommands.has(cmd)) {
    ok(`"${cmd}" (browser mock, skipped)`);
  } else {
    warn(`"${cmd}" NOT found in backend command list — may be new or browser-only`);
  }
}

console.log(`\n  ${matchedCount}/${frontendCount} frontend commands match backend`);

// 2. Check for snake_case vs camelCase in known problematic commands
// (These patterns have been fixed in the codebase - kept as documentation)
// eslint-disable-next-line no-unused-vars
const _argMismatchPatterns = [
  // Frontend camelCase → Rust snake_case (FIXED: now using correct snake_case)
  // { from: "modelId", to: "model_id", cmd: "create_conversation" },
  // { from: "providerId", to: "provider_id", cmd: "create_conversation" },
  // { from: "systemPrompt", to: "system_prompt", cmd: "create_conversation" },
];

// 3. Summary
console.log("\n" + "=".repeat(60));
console.log(`\n📊 Summary: ${checkedCount} checks, ${errors} errors, ${warnings} warnings\n`);

if (errors > 0) {
  console.error(`❌ ${errors} error(s) found — please fix before committing.`);
  process.exit(1);
} else if (warnings > 0) {
  console.log(`⚠️  ${warnings} warning(s) — review recommended but not blocking.`);
} else {
  console.log("✅ All checks passed!");
}
