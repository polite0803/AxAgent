#!/usr/bin/env python3
"""
Batch fix missing Rust imports in AxAgent project.
"""
import re
import os
import sys

BACKTICK = chr(96)

ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"
SRC_DIR = r"d:\OneManager\AxAgent\src-tauri"

# === Fix mapping ===
CRATE_COMMANDS_ERROR = "use crate::commands::error::ErrorResponse;"

STD_IMPORTS = {
    "Arc": "use std::sync::Arc;",
    "HashMap": "use std::collections::HashMap;",
    "HashSet": "use std::collections::HashSet;",
    "Mutex": "use std::sync::Mutex;",
    "OnceLock": "use std::sync::OnceLock;",
}

CRATE_APP_STATE = {
    "AppState": "use crate::app_state::AppState;",
    "State": "use crate::app_state::AppState as State;",
}

RUNTIME_CORE = {
    "Conversation": "use axagent_runtime_core::Conversation;",
    "ConversationSearchResult": "use axagent_runtime_core::ConversationSearchResult;",
    "UpdateConversationInput": "use axagent_runtime_core::UpdateConversationInput;",
    "TokenUsage": "use axagent_runtime_core::TokenUsage;",
}

HARNESS_TYPES = {
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
}

DATABASE = {
    "DatabaseConnection": "use sea_orm::DatabaseConnection;",
}

MODULE_CRATE = {
    "agent_err": "use crate::commands::agent::agent_err;",
    "skill_err": "use crate::commands::agent::skill_err;",
    "agent_status_err": "use crate::commands::agent::agent_status_err;",
    "context_keys": "use crate::commands::agent::context_keys;",
    "message": "use crate::commands::agent::message;",
    "search_provider": "use crate::commands::agent::search_provider;",
    "pricing": "use crate::commands::agent::pricing;",
}

VALUES = {
    "UPSTREAM_EXTENSION_FOR_CHAT": "use crate::commands::agent::UPSTREAM_EXTENSION_FOR_CHAT;",
    "LAST_KNOWN_SETTINGS": "use crate::commands::agent::LAST_KNOWN_SETTINGS;",
}

FUNCTIONS = {
    "emit_status": "use crate::commands::agent::emit_status;",
    "steer_queue": "use crate::commands::agent::steer_queue;",
    "resolve_base_url_for_type": "use crate::commands::agent::resolve_base_url_for_type;",
}

# Build lookup: name -> import statement
ALL_FIXES = {}
ALL_FIXES.update(CRATE_APP_STATE)
ALL_FIXES.update(STD_IMPORTS)
ALL_FIXES.update(RUNTIME_CORE)
ALL_FIXES.update(HARNESS_TYPES)
ALL_FIXES.update(DATABASE)
ALL_FIXES.update(MODULE_CRATE)
ALL_FIXES.update(VALUES)
ALL_FIXES.update(FUNCTIONS)


def resolve_path(raw_path):
    """Convert error file path to absolute file path"""
    p = raw_path.replace("\\", "/")
    if "src-tauri/src/" in p:
        p = p.split("src-tauri/src/", 1)[1]
    elif p.startswith("src/"):
        p = p[4:]
    # Handle both absolute and relative paths
    if os.path.isabs(p) or p.startswith("d:"):
        return p
    return os.path.join(SRC_DIR, "src", p)


def read_file_content(rel_path):
    """Read file content using relative path from src/"""
    abs_path = resolve_path(rel_path)
    if not os.path.exists(abs_path):
        # Try with src-tauri/src/ prefix
        alt = os.path.join(SRC_DIR, "src", rel_path.replace("src/", ""))
        if os.path.exists(alt):
            abs_path = alt
        else:
            return None, abs_path
    try:
        with open(abs_path, "r", encoding="utf-8") as f:
            return f.read(), abs_path
    except Exception as e:
        return None, abs_path


def has_import(content, import_stmt):
    """Check if content already has this exact import"""
    # Escape for regex
    escaped = re.escape(import_stmt)
    return bool(re.search(r"^\s*" + escaped, content, re.MULTILINE))


def has_type_imported(content, type_name):
    """Check if a type name appears in any use statement"""
    pattern = r"^\s*use\s+[^;]*\b" + re.escape(type_name) + r"\b[^;]*;"
    return bool(re.search(pattern, content, re.MULTILINE))


def has_tracing(content):
    """Check if tracing is already imported (any form)"""
    patterns = [
        r"use\s+tracing;",
        r"use\s+tracing::\w+",
        r"use\s+tracing::\{[^}]*\}",
    ]
    return any(re.search(p, content) for p in patterns)


def add_import_to_content(content, import_stmt):
    """Add a use statement to content. Returns (new_content, description)"""
    # Check if already present
    if has_import(content, import_stmt):
        return content, "already_exists"

    # Special: for error::ErrorResponse, try to append to crate::commands::{...}
    if import_stmt == "use crate::commands::error::ErrorResponse;":
        m = re.search(r"^(use crate::commands::)\{([^}]*)\}", content, re.MULTILINE)
        if m:
            prefix = m.group(1)
            inner = m.group(2)
            if "error::ErrorResponse" not in inner:
                new_inner = inner.rstrip().rstrip(",")
                if new_inner:
                    new_inner += ", error::ErrorResponse"
                else:
                    new_inner = "error::ErrorResponse"
                new_content = content[:m.start()] + prefix + "{" + new_inner + "}" + content[m.end():]
                return new_content, "appended_to_block"
            return content, "already_in_block"

    # Special: for tracing::{info, warn}, handle existing tracing imports
    if import_stmt == "use tracing::{info, warn};":
        # Already has use tracing; -> upgrade
        m = re.search(r"^\s*use\s+tracing;\s*$", content, re.MULTILINE)
        if m:
            new_content = content[:m.start()] + "use tracing::{info, warn};" + content[m.end():]
            return new_content, "upgraded_from_use_tracing"

        # Already has use tracing::info; or use tracing::warn;
        m_info = re.search(r"^\s*use\s+tracing::info;\s*$", content, re.MULTILINE)
        m_warn = re.search(r"^\s*use\s+tracing::warn;\s*$", content, re.MULTILINE)
        if m_info and m_warn:
            return content, "already_has_both"
        if m_info:
            new_content = re.sub(r"^\s*use\s+tracing::info;\s*$", "use tracing::{info, warn};", content, count=1, flags=re.MULTILINE)
            return new_content, "added_warn_to_info"
        if m_warn:
            new_content = re.sub(r"^\s*use\s+tracing::warn;\s*$", "use tracing::{info, warn};", content, count=1, flags=re.MULTILINE)
            return new_content, "added_info_to_warn"

        # Already has use tracing::{...} block - append info,warn
        m_block = re.search(r"^\s*use\s+tracing::\{(.+)\};\s*$", content, re.MULTILINE)
        if m_block:
            inner = m_block.group(1)
            parts = [x.strip() for x in inner.split(",")]
            new_parts = list(dict.fromkeys(parts))  # dedup preserving order
            if "info" not in new_parts:
                new_parts.append("info")
            if "warn" not in new_parts:
                new_parts.append("warn")
            new_inner = ", ".join(new_parts)
            new_content = content[:m_block.start()] + "use tracing::{" + new_inner + "};" + content[m_block.end():]
            return new_content, "appended_to_tracing_block"

    # Check type already imported via different path
    type_match = re.search(r"::(\w+);\s*$", import_stmt)
    if type_match:
        type_name = type_match.group(1)
        if has_type_imported(content, type_name):
            return content, f"already_imported_via_different_path"

    # Find insertion point
    lines = content.split("\n")
    last_use_line = -1
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("use ") and ";" in stripped:
            last_use_line = idx
        # Handle multi-line use {...}
        if stripped.startswith("use ") and "{" in stripped and "}" not in stripped:
            for j in range(idx, len(lines)):
                if ";" in lines[j]:
                    last_use_line = j
                    break

    if last_use_line >= 0:
        insert_pos = last_use_line + 1
        # Maintain blank line if it exists
        new_lines = lines[:insert_pos] + [import_stmt] + lines[insert_pos:]
        return "\n".join(new_lines), "added"

    # No use statements - add at top
    return import_stmt + "\n" + content, "added_at_top"


def fix_file(rel_path, missing_imports):
    """Fix a single file with all its missing imports"""
    content, abs_path = read_file_content(rel_path)
    if content is None:
        print(f"  [SKIP] File not found: {rel_path} -> {abs_path}")
        return False, []

    # Check for info/warn - if both are missing, handle together
    has_info = "info" in missing_imports
    has_warn = "warn" in missing_imports
    tracing_missing = has_info or has_warn

    # Filter out info and warn as they'll be handled together
    items_to_fix = [x for x in missing_imports if x not in ("info", "warn")]

    fixed = False
    changes = []
    original = content

    for item in items_to_fix:
        if item in ALL_FIXES:
            import_stmt = ALL_FIXES[item]
        elif item == "ErrorResponse":
            import_stmt = CRATE_COMMANDS_ERROR
        else:
            print(f"  [SKIP] Unknown type: {item}")
            continue

        content, desc = add_import_to_content(content, import_stmt)
        if "already" not in desc and "imported" not in desc:
            fixed = True
            changes.append(f"{item}:{desc}")

    # Handle tracing (info/warn)
    if tracing_missing:
        content, desc = add_import_to_content(content, "use tracing::{info, warn};")
        if "already" not in desc and "imported" not in desc:
            fixed = True
            changes.append(f"tracing:info,warn:{desc}")

    if fixed:
        with open(abs_path, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"  [FIXED] {rel_path}: {', '.join(changes)}")
        return True, changes

    return False, []


def main():
    # Parse errors
    with open(ERROR_FILE, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    file_errors = {}
    for i, line in enumerate(lines):
        m = re.search("cannot find (type|macro|value|function|module) .([^" + BACKTICK + r"]+).", line)
        if m:
            err_type = m.group(1)
            err_name = m.group(2)
            current_file = None
            for j in range(i-1, max(i-5, -1), -1):
                fm = re.search(r'-->\s+(.+):(\d+):(\d+)', lines[j])
                if fm:
                    raw = fm.group(1)
                    raw = raw.replace("\\", "/")
                    # Normalize to relative path from src/
                    if "src-tauri/src/" in raw:
                        current_file = raw.split("src-tauri/src/", 1)[1]
                    elif raw.startswith("src/"):
                        current_file = raw
                    break
            if current_file:
                if current_file not in file_errors:
                    file_errors[current_file] = set()
                file_errors[current_file].add(err_name)

    print(f"Found errors in {len(file_errors)} files")

    # Fix files
    fixed_count = 0
    total_changes = 0
    summary = {}

    for rel_path, err_names in sorted(file_errors.items()):
        print(f"\n{rel_path}: missing {sorted(err_names)}")
        success, changes = fix_file(rel_path, list(err_names))
        if success:
            fixed_count += 1
            total_changes += len(changes)
            for c in changes:
                kind = c.split(":")[0]
                summary[kind] = summary.get(kind, 0) + 1

    print(f"\n{'='*50}")
    print(f"Total files fixed: {fixed_count}")
    print(f"Total changes: {total_changes}")
    if summary:
        print(f"\nChanges by type:")
        for kind, count in sorted(summary.items(), key=lambda x: -x[1]):
            print(f"  {kind}: {count}")
    print(f"{'='*50}")


if __name__ == "__main__":
    main()
