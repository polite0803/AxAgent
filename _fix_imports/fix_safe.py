#!/usr/bin/env python3
"""
SAFE fixer: only adds missing imports to the top-level use block.
Preserves multi-line use statements. Does NOT modify file structure.
"""
import re
import os
import sys

BACKTICK = chr(96)
SRC_DIR = r"d:\OneManager\AxAgent\src-tauri"
ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"

# ===== Fix definitions =====
STD_IMPORTS = {
    "Arc": "use std::sync::Arc;",
    "HashMap": "use std::collections::HashMap;",
    "HashSet": "use std::collections::HashSet;",
    "Mutex": "use std::sync::Mutex;",
    "OnceLock": "use std::sync::OnceLock;",
}

TYPE_IMPORTS = {
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
    "Value": "use serde_json::Value;",
}

MODULE_IMPORTS = {
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

VALUE_IMPORTS = {
    "UPSTREAM_EXTENSION_FOR_CHAT": "use crate::commands::agent::UPSTREAM_EXTENSION_FOR_CHAT;",
    "LAST_KNOWN_SETTINGS": "use crate::commands::agent::LAST_KNOWN_SETTINGS;",
    "NODE_SCHEMAS_DOC": "use crate::commands::workflow_ai::NODE_SCHEMAS_DOC;",
    "SKILL_MCP_REGISTRY": "use crate::commands::agent::SKILL_MCP_REGISTRY;",
}

FUNC_IMPORTS = {
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

ALL_IMPORTS = {}
for d in [STD_IMPORTS, TYPE_IMPORTS, MODULE_IMPORTS, VALUE_IMPORTS, FUNC_IMPORTS]:
    ALL_IMPORTS.update(d)

SKIP_LIST = {"AgentQueryResponse", "AgentQueryRequest", "AppHandle", "Attachment"}


def resolve_path(rel_path):
    rel_path = rel_path.replace("\\", "/")
    if "src-tauri/src/" in rel_path:
        rel_path = rel_path.split("src-tauri/src/", 1)[1]
    elif rel_path.startswith("src/"):
        rel_path = rel_path[4:]
    return os.path.join(SRC_DIR, "src", rel_path)


def find_use_block(lines):
    """
    Find the top-level use block (before any code).
    Returns: (start_idx, end_idx) or None
    start_idx: first line of use block (could be comment, whitespace, or use)
    end_idx: last line of use block (inclusive)
    """
    in_use_block = False
    in_multi_line = False
    block_start = None
    block_end = None

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Skip shebangs, copyright comments at top
        if not in_use_block:
            if not stripped or stripped.startswith("//") or stripped.startswith("#!"):
                continue
            if stripped.startswith("use "):
                in_use_block = True
                block_start = i
                if "{" in stripped and "}" not in stripped:
                    in_multi_line = True
                block_end = i
            else:
                # First non-use, non-comment, non-empty line -> code starts
                break
        else:
            if stripped.startswith("use "):
                block_end = i
                if "{" in stripped and "}" not in stripped:
                    in_multi_line = True
            elif in_multi_line:
                # Continuation of multi-line use block
                if "}" in stripped and ";" in stripped:
                    in_multi_line = False
                    block_end = i
                else:
                    block_end = i
            else:
                # Non-use line after use block -> code
                break

    if block_start is not None:
        return (block_start, block_end)
    return None


def has_import(lines, import_stmt, start, end):
    """Check if import exists in the given line range"""
    imp_clean = import_stmt.strip()
    for i in range(start, end + 1):
        # Check if this import (or part of multi-line block) contains the type
        if lines[i].strip() == imp_clean:
            return True
        # Check if the type name appears in a multi-line block
        type_m = re.search(r"::(\w+);\s*$", imp_clean)
        if type_m:
            type_name = type_m.group(1)
            if re.search(r"\b" + re.escape(type_name) + r"\b", lines[i]):
                # Make sure it's in a real use statement
                if lines[i].strip().startswith("use ") or (i > 0 and "use " in lines[i-1]):
                    # Check if it could be from a multi-line block
                    pass  # Too complex, rely on exact match
    return False


def add_import(lines, import_stmt, block_end):
    """Add import after block_end"""
    result = list(lines)
    result.insert(block_end + 1, import_stmt)
    return result


def main():
    with open(ERROR_FILE, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    # Build file -> error names map from errors
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

    print(f"=== Found errors in {len(file_errors)} files ===\n")

    fixed_count = 0
    total_additions = 0

    for rel_path, err_names in sorted(file_errors.items()):
        abs_path = resolve_path(rel_path)
        if not os.path.exists(abs_path):
            print(f"[SKIP] {rel_path}: file not found")
            continue

        with open(abs_path, "r", encoding="utf-8") as f:
            content = f.read()

        file_lines = content.split("\n")
        block = find_use_block(file_lines)

        if block is None:
            print(f"[SKIP] {rel_path}: no use block found")
            continue

        block_start, block_end = block
        additions = []

        has_info = "info" in err_names
        has_warn = "warn" in err_names
        tracing_needed = has_info or has_warn

        # Process each error name
        for name in sorted(err_names):
            if name in ("info", "warn"):
                continue
            if name in SKIP_LIST:
                continue

            if name in ALL_IMPORTS:
                imp = ALL_IMPORTS[name]
            elif name == "ErrorResponse":
                imp = "use crate::commands::error::ErrorResponse;"
            elif name in STD_IMPORTS:
                imp = STD_IMPORTS[name]
            else:
                print(f"  [SKIP] {name}: unknown import for {rel_path}")
                continue

            if has_import(file_lines, imp, block_start, block_end):
                continue

            additions.append(imp)

        if tracing_needed:
            imp = "use tracing::{info, warn};"
            # Check if already present in various forms
            found = False
            for i in range(block_start, block_end + 1):
                l = file_lines[i].strip()
                if l.startswith("use tracing") and ("info" in l or "warn" in l):
                    # Already have tracing imports
                    if ("info" in l and "warn" in l) or l == "use tracing;":
                        found = True
                    break
            if not found:
                additions.append(imp)

        if additions:
            # Insert all additions at block_end
            # Sort them alphabetically
            additions.sort()
            new_lines = list(file_lines)
            insert_at = block_end + 1
            for a in reversed(additions):
                new_lines.insert(insert_at, a)

            with open(abs_path, "w", encoding="utf-8") as f:
                f.write("\n".join(new_lines))

            fixed_count += 1
            total_additions += len(additions)
            print(f"[FIXED] {rel_path} (+{len(additions)} imports):")
            for a in additions:
                print(f"  + {a}")
        else:
            print(f"[OK] {rel_path}: no changes needed")

    print(f"\n{'='*50}")
    print(f"Fixed {fixed_count} files, added {total_additions} imports")
    print(f"{'='*50}")


if __name__ == "__main__":
    main()
