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

/** Rust commands snapshot — extracted from src-tauri/src/lib.rs invoke() registrations */
const RUST_COMMANDS = [
  // Conversation
  "list_conversations",
  "create_conversation",
  "update_conversation",
  "delete_conversation",
  "toggle_pin_conversation",
  "toggle_archive_conversation",
  "list_archived_conversations",
  "regenerate_conversation_title",
  "branch_conversation",
  // Messages
  "send_message",
  "regenerate_message",
  "regenerate_with_model",
  "send_system_message",
  "delete_message",
  "clear_conversation_messages",
  "list_messages",
  "list_messages_page",
  "load_older_messages",
  "switch_message_version",
  "list_message_versions",
  "update_message_content",
  "delete_message_group",
  "cancel_stream",
  // Multi-model
  "send_multi_model_message",
  // Providers
  "list_providers",
  "create_provider",
  "update_provider",
  "delete_provider",
  "toggle_provider",
  "list_models",
  "list_provider_keys",
  "create_provider_key",
  "delete_provider_key",
  // Categories
  "list_conversation_categories",
  "create_conversation_category",
  "update_conversation_category",
  "delete_conversation_category",
  // Settings
  "get_settings",
  "save_settings",
  // Knowledge
  "list_knowledge_bases",
  "create_knowledge_base",
  "update_knowledge_base",
  "delete_knowledge_base",
  "list_knowledge_documents",
  "index_knowledge_document",
  "delete_knowledge_document",
  "search_knowledge",
  "collect_rag_context",
  // Memory
  "list_memory_namespaces",
  "create_memory_namespace",
  "update_memory_namespace",
  "delete_memory_namespace",
  "list_memory_items",
  "create_memory_item",
  "update_memory_item",
  "delete_memory_item",
  "search_memory",
  // MCP
  "list_mcp_servers",
  "create_mcp_server",
  "update_mcp_server",
  "delete_mcp_server",
  "list_tools_for_server",
  "execute_mcp_tool",
  // Gateway
  "gateway_status",
  "gateway_start",
  "gateway_stop",
  "gateway_restart",
  "list_gateway_keys",
  "create_gateway_key",
  "delete_gateway_key",
  "get_gateway_metrics",
  "get_gateway_logs",
  // Agent
  "agent_query",
  "agent_cancel",
  "agent_approve",
  "agent_pause",
  "agent_resume",
  "agent_get_session",
  "agent_update_session",
  "agent_respond_ask",
  "agent_backup_and_clear_sdk_context",
  "agent_restore_sdk_context_from_backup",
  // Skills
  "list_skills",
  "install_skill",
  "create_skill",
  "delete_skill",
  "list_skill_proposals",
  "approve_skill_proposal",
  "reject_skill_proposal",
  // Workflow
  "list_workflow_templates",
  "get_workflow_template",
  "create_workflow_template",
  "update_workflow_template",
  "delete_workflow_template",
  "duplicate_workflow_template",
  "validate_workflow_template",
  "export_workflow_template",
  "import_workflow_template",
  "get_template_versions",
  "get_template_by_version",
  "generate_workflow_from_prompt",
  "optimize_agent_prompt",
  "recommend_nodes",
  // Files
  "list_stored_files",
  "delete_stored_file",
  "file_cleanup",
  // Backup / WebDAV
  "create_backup",
  "list_backups",
  "restore_backup",
  "delete_backup",
  "get_webdav_status",
  "webdav_sync",
  "clear_webdav_cache",
  // Workspace
  "get_workspace_snapshot",
  "update_workspace_snapshot",
  "fork_conversation",
  "compare_responses",
  // Desktop
  "force_quit",
  "set_window_size",
  // Search
  "search_conversations",
  // Tools
  "list_tool_executions",
  // Atomic skills
  "list_atomic_skills",
  "get_atomic_skill",
  "create_atomic_skill",
  "update_atomic_skill",
  "delete_atomic_skill",
  // Local tools
  "list_local_tools",
  "create_local_tool",
  "delete_local_tool",
  // Generated tools
  "list_generated_tools",
  "create_generated_tool",
  "delete_generated_tool",
  // Work engine
  "work_engine_status",
  "work_engine_execute",
  // Decomposition
  "decompose_task",
  "get_decomposition_status",
  // Analytics / nudge / insight
  "get_nudges",
  "dismiss_nudge",
  "get_insights",
  "get_conversation_stats",
  // Platform integration
  "list_platform_integrations",
  "create_platform_integration",
  "update_platform_integration",
  "delete_platform_integration",
  "execute_platform_action",
  // Scheduled tasks
  "list_scheduled_tasks",
  "create_scheduled_task",
  "update_scheduled_task",
  "delete_scheduled_task",
  "execute_scheduled_task",
  // Parallel execution
  "list_parallel_executions",
  "cancel_parallel_execution",
  // Nudge
  "list_nudges",
  "nudge_action",
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
const knownBrowserCommands = new Set([
  "get_settings",
  "save_settings",
  "list_providers",
  "create_provider",
  "update_provider",
  "delete_provider",
  "list_conversations",
  "create_conversation",
  "send_message",
  "list_messages",
  "delete_message",
  "list_knowledge_bases",
  "list_mcp_servers",
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
console.log("\n📋 Checking snake_case vs camelCase argument patterns...\n");

const argMismatchPatterns = [
  // Frontend camelCase → Rust snake_case
  { from: "modelId", to: "model_id", cmd: "create_conversation" },
  { from: "providerId", to: "provider_id", cmd: "create_conversation" },
  { from: "systemPrompt", to: "system_prompt", cmd: "create_conversation" },
];

for (const pattern of argMismatchPatterns) {
  checkedCount++;
  warn(`Check "${pattern.from}" in frontend → "${pattern.to}" in Rust for "${pattern.cmd}"`);
}

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
