#!/usr/bin/env python3
"""
Fix missing Rust imports in AxAgent.
Only inserts imports at TOP-LEVEL of the file (column 0), never inside functions.
"""
import re
import os

BACKTICK = chr(96)
SRC_DIR = r"d:\OneManager\AxAgent\src-tauri"
ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"

# ===== Fix definitions =====

STD_FIXES = {
    "Arc": ("sync", "Arc"),
    "HashMap": ("collections", "HashMap"),
    "HashSet": ("collections", "HashSet"),
    "Mutex": ("sync", "Mutex"),
    "OnceLock": ("sync", "OnceLock"),
}

TYPE_FIXES = {
    "AppState": "use crate::app_state::AppState;",
    "State": "use crate::app_state::AppState as State;",
    "Conversation": "use axagent_runtime_core::Conversation;",
    "ConversationSearchResult": "use axagent_runtime_core::ConversationSearchResult;",
    "UpdateConversationInput": "use axagent_runtime_core::UpdateConversationInput;",
    "TokenUsage": "use axagent_runtime_core::TokenUsage;",
    "MessageRole": "use axagent_harness::types::conversation::MessageRole;",
    "ProviderType": "use axagent_harness::types::provider_model::ProviderType;",
    "ChatContent": "use axagent_harness::types::settings_chat::ChatContent;",
    "ChatTool": "use axagent_harness::types::function_call::ChatTool;",
    "ChatMessage": "use axagent_harness::types::conversation::ChatMessage;",
    "ChatStreamChunk": "use axagent_harness::types::conversation::ChatStreamChunk;",
    "ChatStreamEvent": "use axagent_harness::types::conversation::ChatStreamEvent;",
    "ChatStreamErrorEvent": "use axagent_harness::types::conversation::ChatStreamErrorEvent;",
    "ChatRequest": "use axagent_harness::types::function_call::ChatRequest;",
    "AgentContentBlock": "use axagent_harness::types::agent::AgentContentBlock;",
    "AgentErrorPayload": "use axagent_harness::types::agent::AgentErrorPayload;",
    "ToolCall": "use axagent_harness::types::function_call::ToolCall;",
    "Value": "use axagent_harness::types::function_call::Value;",
    "StreamConsumptionParams": "use axagent_harness::types::streaming::StreamConsumptionParams;",
    "ProviderRequestContext": "use axagent_harness::types::provider::ProviderRequestContext;",
    "ProviderConfig": "use axagent_harness::types::provider::ProviderConfig;",
    "ProviderProxyConfig": "use axagent_harness::types::provider::ProviderProxyConfig;",
    "ProviderAdapter": "use axagent_harness::traits::provider_adapter::ProviderAdapter;",
    "AppSettings": "use axagent_harness::types::settings::AppSettings;",
    "Settings": "use axagent_core::repo::settings::Settings;",
    "McpServer": "use axagent_harness::types::mcp::McpServer;",
    "SkillExecutionContext": "use axagent_harness::types::skill::SkillExecutionContext;",
    "TitleFallbackModel": "use axagent_harness::types::settings::TitleFallbackModel;",
    "DisabledThinkingStripState": "use axagent_harness::types::provider::DisabledThinkingStripState;",
    "UnifiedToolRegistry": "use axagent_harness::traits::tool_registry::UnifiedToolRegistry;",
    "AiChatMessage": "use axagent_harness::types::agent::AiChatMessage;",
    "DatabaseConnection": "use sea_orm::DatabaseConnection;",
}

MODULE_FIXES = {
    "agent_err": "use crate::commands::agent::agent_err;",
    "skill_err": "use crate::commands::agent::skill_err;",
    "agent_status_err": "use crate::commands::agent::agent_status_err;",
    "context_keys": "use crate::commands::agent::context_keys;",
    "message": "use crate::commands::agent::message;",
    "search_provider": "use crate::commands::agent::search_provider;",
    "pricing": "use crate::commands::agent::pricing;",
    "conversation": "use crate::commands::conversations as conversation;",
    "provider": "use crate::commands::providers as provider;",
    "provider_err": "use crate::commands::providers::provider_err;",
    "fs": "use std::fs;",
}

VALUE_FIXES = {
    "UPSTREAM_EXTENSION_FOR_CHAT": "use crate::commands::agent::UPSTREAM_EXTENSION_FOR_CHAT;",
    "LAST_KNOWN_SETTINGS": "use crate::commands::agent::LAST_KNOWN_SETTINGS;",
    "NODE_SCHEMAS_DOC": "use crate::commands::workflow_ai::NODE_SCHEMAS_DOC;",
    "SKILL_MCP_REGISTRY": "use crate::commands::agent::SKILL_MCP_REGISTRY;",
}

FUNC_FIXES = {
    "emit_status": "use crate::commands::agent::emit_status;",
    "steer_queue": "use crate::commands::agent::steer_queue;",
    "resolve_base_url_for_type": "use crate::commands::agent::resolve_base_url_for_type;",
    "load_enabled_skill_contents": "use crate::commands::agent::load_enabled_skill_contents;",
    "load_skill_tools": "use crate::commands::agent::load_skill_tools;",
    "execute_skill_sync": "use crate::commands::agent::execute_skill_sync;",
    "build_streaming_api_client": "use crate::commands::agent::build_streaming_api_client;",
    "build_agent_system_prompt": "use crate::commands::agent::build_agent_system_prompt;",
    "check_and_suggest_workflow_match": "use crate::commands::agent::check_and_suggest_workflow_match;",
    "consume_stream": "use crate::commands::conversations::messages::streaming::consume_stream;",
    "execute_tool_call": "use crate::commands::conversations::messages::streaming::execute_tool_call;",
    "generate_ai_title": "use crate::commands::conversations::messages::streaming::generate_ai_title;",
    "resolve_rag_ids": "use crate::commands::conversations::messages::streaming::resolve_rag_ids;",
    "build_memory_retrieval_tag": "use crate::commands::conversations::messages::streaming::build_memory_retrieval_tag;",
    "dedup_rag_against_working_memory": "use crate::commands::conversations::messages::streaming::dedup_rag_against_working_memory;",
    "apply_rag_token_budget": "use crate::commands::conversations::messages::streaming::apply_rag_token_budget;",
    "build_rag_chat_message": "use crate::commands::conversations::messages::streaming::build_rag_chat_message;",
    "build_working_memory_chat_message": "use crate::commands::conversations::messages::streaming::build_working_memory_chat_message;",
    "sync_context_sources": "use crate::commands::conversations::sync_context_sources;",
    "extract_reasoning_from_text": "use crate::commands::conversations::extract_reasoning_from_text;",
    "collect_skill_content": "use crate::commands::skills::collect_skill_content;",
    "decrypt_key": "use crate::commands::skills::decrypt_key;",
    "delete_conversation_with_attachments_using": "use crate::commands::conversations::delete_conversation_with_attachments_using;",
}

ALL_FIXES = {}
for d in [TYPE_FIXES, MODULE_FIXES, VALUE_FIXES, FUNC_FIXES]:
    ALL_FIXES.update(d)

SKIP_LIST = {"AgentQueryResponse", "AgentQueryRequest", "AppHandle", "Attachment"}


def resolve_path(rel_path):
    rel_path = rel_path.replace("\\", "/")
    if "src-tauri/src/" in rel_path:
        rel_path = rel_path.split("src-tauri/src/", 1)[1]
    elif rel_path.startswith("src/"):
        rel_path = rel_path[4:]
    return os.path.join(SRC_DIR, "src", rel_path)


def read_file(rel_path):
    full = resolve_path(rel_path)
    if not os.path.exists(full):
        alt = os.path.join(SRC_DIR, "src", rel_path)
        if not os.path.exists(alt):
            return None, None
        full = alt
    try:
        with open(full, "r", encoding="utf-8") as f:
            return f.read(), full
    except Exception:
        return None, None


def write_file(path, content):
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)


def find_top_level_uses(content):
    """
    Find all top-level (column 0) use statements and their positions.
    Returns list of (line_idx, line_text, is_block_start) tuples.
    """
    lines = content.split("\n")
    top_uses = []
    in_block = False
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("//") or stripped.startswith("#!"):
            continue
        if stripped.startswith("use ") and not line.startswith(" ") and not line.startswith("\t"):
            top_uses.append(idx)
            if "{" in stripped and "}" not in stripped:
                in_block = True
            elif not in_block:
                pass
        elif in_block:
            # Continuation of multi-line use block
            if "}" in line:
                in_block = False
                top_uses.append(idx)

    return top_uses


def find_top_level_insert_point(content):
    """
    Find the best line to insert new imports.
    Returns the line index to insert AFTER.
    """
    top_use_lines = find_top_level_uses(content)
    if top_use_lines:
        return max(top_use_lines)
    return -1


def has_import_at_top(content, import_stmt):
    """Check if import already exists as a top-level use statement"""
    escaped = re.escape(import_stmt)
    return bool(re.search(r"^" + escaped, content, re.MULTILINE))


def has_type_in_top_uses(content, type_name):
    """Check if a type is referenced in any top-level use statement"""
    pattern = r"^use\s+[^;]*\b" + re.escape(type_name) + r"\b"
    return bool(re.search(pattern, content, re.MULTILINE))


def add_import_top_level(content, import_stmt):
    """
    Add import at top level of file only.
    Never inserts inside functions.
    """
    # Already exists check
    if has_import_at_top(content, import_stmt):
        return content, "already_exists"

    # Check type already imported via different path
    type_m = re.search(r"::(\w+);\s*$", import_stmt)
    if type_m:
        tn = type_m.group(1)
        if has_type_in_top_uses(content, tn):
            return content, f"already_imported({tn})"

    # Special: ErrorResponse - try to append to crate::commands::{...}
    if import_stmt == "use crate::commands::error::ErrorResponse;":
        m = re.search(r"^use crate::commands::\{([^}]*)\}", content, re.MULTILINE)
        if m:
            inner = m.group(1)
            if "error::ErrorResponse" not in inner:
                stripped = inner.rstrip().rstrip(",")
                if stripped:
                    stripped += ", error::ErrorResponse"
                else:
                    stripped = "error::ErrorResponse"
                new_content = content[:m.start()] + "use crate::commands::{" + stripped + "}" + content[m.end():]
                return new_content, "appended_to_commands_block"
            return content, "already_in_block"

    # Special: info/warn tracing
    if import_stmt == "use tracing::{info, warn};":
        # Check use tracing;
        m = re.search(r"^use tracing;\s*$", content, re.MULTILINE)
        if m:
            new_content = content[:m.start()] + "use tracing::{info, warn};" + content[m.end():]
            return new_content, "upgraded_tracing"

        # Check use tracing::X;
        m_single = re.search(r"^use tracing::(\w+);\s*$", content, re.MULTILINE)
        if m_single:
            existing = m_single.group(1)
            new_macros = list(dict.fromkeys([existing, "info", "warn"]))
            new_content = content[:m_single.start()] + "use tracing::{" + ", ".join(new_macros) + "};" + content[m_single.end():]
            return new_content, "expanded_tracing"

        # Check use tracing::{X, Y};
        m_block = re.search(r"^use tracing::\{([^}]+)\};\s*$", content, re.MULTILINE)
        if m_block:
            inner = m_block.group(1)
            parts = [x.strip() for x in inner.split(",")]
            new_parts = list(dict.fromkeys(parts))
            changed = False
            for w in ["info", "warn"]:
                if w not in new_parts:
                    new_parts.append(w)
                    changed = True
            if changed:
                new_content = content[:m_block.start()] + "use tracing::{" + ", ".join(new_parts) + "};" + content[m_block.end():]
                return new_content, "appended_to_tracing_block"
            return content, "already_has_tracing"

    # Find insert point at top-level
    insert_after = find_top_level_insert_point(content)
    lines = content.split("\n")

    if insert_after >= 0:
        new_lines = lines[:insert_after + 1] + [import_stmt] + lines[insert_after + 1:]
        return "\n".join(new_lines), "added"
    else:
        # No top-level use statements - add before first non-comment, non-attr line
        for idx, line in enumerate(lines):
            stripped = line.strip()
            if stripped and not stripped.startswith("//") and not stripped.startswith("#"):
                new_lines = lines[:idx] + [import_stmt, ""] + lines[idx:]
                return "\n".join(new_lines), "added_at_top"
        return import_stmt + "\n" + content, "added_at_top"


def main():
    with open(ERROR_FILE, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    file_errors = {}
    for i, line in enumerate(lines):
        m = re.search("cannot find (type|macro|value|function) .([^" + BACKTICK + r"]+).", line)
        m2 = re.search("cannot find module or crate .([^" + BACKTICK + r"]+).", line)
        err_name = None
        if m:
            err_name = m.group(2)
        elif m2:
            err_name = m2.group(1)
        if err_name:
            current_file = None
            for j in range(i+1, min(i+5, len(lines))):
                stripped = lines[j].strip()
                if stripped.startswith("--> "):
                    raw = stripped[4:].strip().split(":")[0].replace("\\", "/")
                    if "src-tauri/src/" in raw:
                        current_file = raw.split("src-tauri/src/", 1)[1]
                    elif raw.startswith("src/"):
                        current_file = raw
                    break
            if current_file:
                if current_file not in file_errors:
                    file_errors[current_file] = set()
                file_errors[current_file].add(err_name)

    print(f"Found errors in {len(file_errors)} files\n")

    fixed_count = 0
    import_count = 0

    for rel_path, err_names in sorted(file_errors.items()):
        content, abs_path = read_file(rel_path)
        if content is None:
            print(f"[SKIP] {rel_path}: file not found")
            continue

        original = content
        changed = False
        changes = []

        has_info = "info" in err_names
        has_warn = "warn" in err_names

        for name in err_names:
            if name in ("info", "warn"):
                continue
            if name in SKIP_LIST:
                continue

            if name in STD_FIXES:
                mod, tn = STD_FIXES[name]
                imp = f"use std::{mod}::{tn};"
            elif name == "ErrorResponse":
                imp = "use crate::commands::error::ErrorResponse;"
            elif name in ALL_FIXES:
                imp = ALL_FIXES[name]
            else:
                changes.append(f"{name}:UNKNOWN")
                continue

            content, desc = add_import_top_level(content, imp)
            if "already" not in desc and "imported" not in desc:
                changed = True
                changes.append(f"{name}:{desc}")

        # Handle info/warn together
        if has_info or has_warn:
            content, desc = add_import_top_level(content, "use tracing::{info, warn};")
            if "already" not in desc and "has" not in desc:
                changed = True
                changes.append(f"tracing{{info,warn}}:{desc}")

        if changed:
            write_file(abs_path, content)
            fixed_count += 1
            import_count += len(changes)
            print(f"[FIXED] {rel_path}:")
            for c in changes:
                print(f"  + {c}")
        else:
            print(f"[OK] {rel_path}: no changes needed")

    print(f"\n{'='*50}")
    print(f"Fixed: {fixed_count} files, {import_count} imports")
    print(f"{'='*50}")


if __name__ == "__main__":
    main()
