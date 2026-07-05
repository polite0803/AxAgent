#!/usr/bin/env python3
"""
Fix missing Rust imports in AxAgent based on cargo_errors.txt analysis.
"""
import re
import os
import sys

BACKTICK = chr(96)
SRC_DIR = r"d:\OneManager\AxAgent\src-tauri"
ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"

# === Fix definitions ===

# 1. ErrorResponse -> crate::commands::error::ErrorResponse
# 2. Std types
STD_FIXES = {
    "Arc": ("sync", "Arc"),
    "HashMap": ("collections", "HashMap"),
    "HashSet": ("collections", "HashSet"),
    "Mutex": ("sync", "Mutex"),
    "OnceLock": ("sync", "OnceLock"),
}

# 3. AppState types
APP_STATE_FIXES = {
    "AppState": "use crate::app_state::AppState;",
    "State": "use crate::app_state::AppState as State;",
}

# 4. axagent_runtime_core
RUNTIME_FIXES = {
    "Conversation": "use axagent_runtime_core::Conversation;",
    "ConversationSearchResult": "use axagent_runtime_core::ConversationSearchResult;",
    "UpdateConversationInput": "use axagent_runtime_core::UpdateConversationInput;",
    "TokenUsage": "use axagent_runtime_core::TokenUsage;",
}

# 5. axagent_harness types
HARNESS_FIXES = {
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
}

# 6. Database
DB_FIXES = {
    "DatabaseConnection": "use sea_orm::DatabaseConnection;",
}

# 7. Module/crate refs
MODULE_FIXES = {
    "agent_err": "use crate::commands::agent::agent_err;",
    "skill_err": "use crate::commands::agent::skill_err;",
    "agent_status_err": "use crate::commands::agent::agent_status_err;",
    "context_keys": "use crate::commands::agent::context_keys;",
    "message": "use crate::commands::agent::message;",
    "search_provider": "use crate::commands::agent::search_provider;",
    "pricing": "use crate::commands::agent::pricing;",
    "conversation": "use crate::commands::conversations as conversation;",
    "provider": "use crate::commands::provider as provider;",
    "provider_err": "use crate::commands::providers::provider_err;",
}

# 8. Values
VALUE_FIXES = {
    "UPSTREAM_EXTENSION_FOR_CHAT": "use crate::commands::agent::UPSTREAM_EXTENSION_FOR_CHAT;",
    "LAST_KNOWN_SETTINGS": "use crate::commands::agent::LAST_KNOWN_SETTINGS;",
    "NODE_SCHEMAS_DOC": "use crate::commands::workflow_ai::NODE_SCHEMAS_DOC;",
    "SKILL_MCP_REGISTRY": "use crate::commands::agent::SKILL_MCP_REGISTRY;",
}

# 9. Functions
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
    "fs": "use std::fs;",
}

# Unknown types that need investigation - skip complex ones
SKIP_LIST = {"AgentQueryResponse", "AgentQueryRequest", "AppHandle", "Attachment"}

# Combine all fixes except those handled specially
ALL_FIXES = {}
for d in [APP_STATE_FIXES, RUNTIME_FIXES, HARNESS_FIXES, DB_FIXES, MODULE_FIXES, VALUE_FIXES, FUNC_FIXES]:
    ALL_FIXES.update(d)


def resolve_path(rel_path):
    """Resolve relative src/ path to absolute path"""
    rel_path = rel_path.replace("\\", "/")
    if "src-tauri/src/" in rel_path:
        rel_path = rel_path.split("src-tauri/src/", 1)[1]
    elif rel_path.startswith("src/"):
        rel_path = rel_path[4:]
    return os.path.join(SRC_DIR, "src", rel_path)


def read_file(rel_path):
    """Read file content"""
    full = resolve_path(rel_path)
    if not os.path.exists(full):
        # Try with src/ prefix
        alt = os.path.join(SRC_DIR, "src", rel_path)
        if not os.path.exists(alt):
            return None
        full = alt
    try:
        with open(full, "r", encoding="utf-8") as f:
            return f.read()
    except Exception:
        return None


def write_file(rel_path, content):
    """Write file content"""
    full = resolve_path(rel_path)
    if not os.path.exists(full):
        full = os.path.join(SRC_DIR, "src", rel_path)
    with open(full, "w", encoding="utf-8") as f:
        f.write(content)


def has_use_stmt(content, imp):
    """Check if content already has this exact use statement"""
    escaped = re.escape(imp)
    return bool(re.search(r"^\s*" + escaped, content, re.MULTILINE))


def has_type_in_uses(content, type_name):
    """Check if any use statement references this type"""
    return bool(re.search(r"^\s*use\s+[^;]*\b" + type_name + r"\b[^;]*;", content, re.MULTILINE) or
                re.search(r"^\s*use\s+\{[^}]*\b" + type_name + r"\b[^}]*\}", content, re.MULTILINE))


def add_import(content, import_stmt):
    """
    Add import statement to content.
    Returns (new_content, description)
    """
    if has_use_stmt(content, import_stmt):
        return content, "already_exists"

    # Extract type name from import
    type_m = re.search(r"::(\w+);$", import_stmt)
    if type_m:
        tn = type_m.group(1)
        if has_type_in_uses(content, tn):
            return content, f"already_imported({tn})"

    # Special: ErrorResponse -> append to crate::commands::block
    if import_stmt == "use crate::commands::error::ErrorResponse;":
        m = re.search(r"^(use crate::commands::)\{([^}]*)\}", content, re.MULTILINE)
        if m:
            inner = m.group(2)
            if "error::ErrorResponse" not in inner:
                stripped = inner.rstrip().rstrip(",")
                if stripped:
                    stripped += ", error::ErrorResponse"
                else:
                    stripped = "error::ErrorResponse"
                new_content = content[:m.start()] + "use crate::commands::{" + stripped + "}" + content[m.end():]
                return new_content, "appended_to_block"
            return content, "already_in_block"

    # Special: tracing::{} block - only for info/warn
    if "use tracing::" in import_stmt:
        # Check existing tracing use
        m1 = re.search(r"^\s*use\s+tracing;\s*$", content, re.MULTILINE)
        if m1:
            new_content = content[:m1.start()] + import_stmt + content[m1.end():]
            return new_content, "replaced_use_tracing"

        m_block = re.search(r"^\s*use\s+tracing::\{(.+)\};\s*$", content, re.MULTILINE)
        if m_block:
            inner = m_block.group(1)
            parts = [x.strip() for x in inner.split(",")]
            want = re.findall(r"\b(\w+)\b", import_stmt)
            new_parts = list(dict.fromkeys(parts))
            changed = False
            for w in want:
                if w not in new_parts and w != "use" and w != "tracing":
                    new_parts.append(w)
                    changed = True
            if changed:
                new_inner = ", ".join(new_parts)
                new_content = content[:m_block.start()] + "use tracing::{" + new_inner + "};" + content[m_block.end():]
                return new_content, "appended_to_tracing_block"
            return content, "already_has_tracing"

        m_info = re.search(r"^\s*use\s+tracing::(\w+);\s*$", content, re.MULTILINE)
        if m_info:
            existing = m_info.group(1)
            want_in_block = re.findall(r"\b(\w+)\b", import_stmt)[2:]  # skip "use" and "tracing"
            new_macros = [existing] + [w for w in want_in_block if w != existing]
            if len(new_macros) > 1:
                new_content = content[:m_info.start()] + "use tracing::{" + ", ".join(new_macros) + "};" + content[m_info.end():]
                return new_content, "merged_into_tracing_block"
            return content, "already_has_single"

        # Has use tracing::{existing, ...}
        if has_use_stmt(content, "use tracing;"):
            return content, "already_has_tracing"
        if has_use_stmt(content, import_stmt):
            return content, "already_exists"

    # Find insertion point
    lines = content.split("\n")
    last_use_line = -1
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("use ") and ";" in stripped:
            last_use_line = idx
        # Multi-line use {...}
        if stripped.startswith("use ") and "{" in stripped and "}" not in stripped:
            for j in range(idx, len(lines)):
                if ";" in lines[j]:
                    last_use_line = j
                    break

    if last_use_line >= 0:
        new_lines = lines[:last_use_line + 1] + [import_stmt] + lines[last_use_line + 1:]
        return "\n".join(new_lines), "added"

    return import_stmt + "\n" + content, "added_at_top"


def add_std_import(content, module, type_name):
    return add_import(content, f"use std::{module}::{type_name};")


def main():
    # Parse errors
    with open(ERROR_FILE, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    file_errors = {}

    for i, line in enumerate(lines):
        m = re.search("cannot find (type|macro|value|function) .([^" + BACKTICK + r"]+).", line)
        m2 = re.search("cannot find module or crate .([^" + BACKTICK + r"]+).", line)

        current_file = None
        err_name = None

        if m:
            err_name = m.group(2)
        elif m2:
            err_name = m2.group(1)

        if err_name:
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

    print(f"=== Found errors in {len(file_errors)} files ===")

    errors_by_file = {}
    for f, names in file_errors.items():
        errors_by_file[f] = sorted(names)
        print(f"\n{f}:")
        for n in names:
            print(f"  - {n}")

    print(f"\n{'='*60}")
    print(f"Starting fixes...")
    print(f"{'='*60}")

    fixed_files = 0
    total_added = 0
    for rel_path, err_names in sorted(file_errors.items()):
        content = read_file(rel_path)
        if content is None:
            print(f"\n[SKIP] File not found: {rel_path}")
            continue

        original = content
        file_changed = False
        file_changes = []

        # Handle info/warn specially
        has_info = "info" in err_names
        has_warn = "warn" in err_names
        tracing_needed = has_info or has_warn

        # Process non-tracing items
        for name in err_names:
            if name == "info" or name == "warn":
                continue
            if name in SKIP_LIST:
                file_changes.append(f"{name}:skipped(local_type)")
                continue

            if name in STD_FIXES:
                module, tn = STD_FIXES[name]
                content, desc = add_std_import(content, module, tn)
            elif name == "ErrorResponse":
                content, desc = add_import(content, "use crate::commands::error::ErrorResponse;")
            elif name in ALL_FIXES:
                content, desc = add_import(content, ALL_FIXES[name])
            else:
                file_changes.append(f"{name}:unknown")
                continue

            if "already" not in desc and "skip" not in desc:
                file_changed = True
                file_changes.append(f"{name}:{desc}")

        # Handle tracing
        if tracing_needed:
            content, desc = add_import(content, "use tracing::{info, warn};")
            if "already" not in desc and "has" not in desc:
                file_changed = True
                file_changes.append(f"tracing{{info,warn}}:{desc}")

        if file_changed:
            write_file(rel_path, content)
            fixed_files += 1
            total_added += len([c for c in file_changes if ":" in c])
            print(f"\n[FIXED] {rel_path} ({len(file_changes)} changes):")
            for c in file_changes:
                print(f"  + {c}")
        else:
            print(f"\n[OK] {rel_path} - no changes needed")

    print(f"\n{'='*60}")
    print(f"Fixed {fixed_files} files, added {total_added} imports")
    print(f"{'='*60}")


if __name__ == "__main__":
    main()
