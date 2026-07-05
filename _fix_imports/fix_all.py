#!/usr/bin/env python3
"""
批量修复 AxAgent Rust import 缺失。
读取 cargo_errors.txt，对每种缺失类型找到对应文件并添加缺失的 use 语句。
"""
import re
import os

BACKTICK = chr(96)

# === 配置：缺失类型 -> 需要添加的 use 语句 ===
FIX_MAP = {
    "ErrorResponse": "use crate::commands::error::ErrorResponse;",

    # Arc & HashMap & std 类型 - 只在确实缺失时添加
    "Arc": None,  # Special: add "use std::sync::Arc;"
    "HashMap": None,  # Special: add "use std::collections::HashMap;"
    "HashSet": None,  # Special: add "use std::collections::HashSet;"
    "Mutex": None,  # Special: add "use std::sync::Mutex;"
    "OnceLock": None,  # Special: add "use std::sync::OnceLock;"
    "DatabaseConnection": "use sea_orm::DatabaseConnection;",

    # axagent 核心类型
    "AppState": "use crate::app_state::AppState;",
    "State": "use crate::app_state::AppState as State;",

    # axagent_runtime_core
    "Conversation": "use axagent_runtime_core::Conversation;",
    "ConversationSearchResult": "use axagent_runtime_core::ConversationSearchResult;",
    "UpdateConversationInput": "use axagent_runtime_core::UpdateConversationInput;",
    "TokenUsage": "use axagent_runtime_core::TokenUsage;",

    # axagent_harness
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

    # info / warn (tracing macros)
    "info": None,  # Special: check if tracing is already imported
    "warn": None,  # Special: check if tracing is already imported
}

# Std types that need "use std::..." fix
STD_TYPES = {
    "Arc": ("sync", "Arc"),
    "HashMap": ("collections", "HashMap"),
    "HashSet": ("collections", "HashSet"),
    "Mutex": ("sync", "Mutex"),
    "OnceLock": ("sync", "OnceLock"),
}

# Types that need the crate::commands::error::ErrorResponse pattern
CRATE_COMMANDS_ERROR = {"ErrorResponse"}

# Types that potentially need crate::commands::agent:: prefix
# These are module lookups like agent_err, context_keys etc.
MODULE_CRATE_PREFIX = {
    "agent_err": "use crate::commands::agent::agent_err;",
    "skill_err": "use crate::commands::agent::skill_err;",
    "agent_status_err": "use crate::commands::agent::agent_status_err;",
    "context_keys": "use crate::commands::agent::context_keys;",
    "message": "use crate::commands::agent::message;",
    "search_provider": "use crate::commands::agent::search_provider;",
    "pricing": "use crate::commands::agent::pricing;",
}

# Other crate values
VALUE_MAP = {
    "UPSTREAM_EXTENSION_FOR_CHAT": "use crate::commands::agent::UPSTREAM_EXTENSION_FOR_CHAT;",
    "LAST_KNOWN_SETTINGS": "use crate::commands::agent::LAST_KNOWN_SETTINGS;",
}

# 其他 mapping
FUNCTION_REMAP = {
    "emit_status": "use crate::commands::agent::emit_status;",
    "steer_queue": "use crate::commands::agent::steer_queue;",
    "resolve_base_url_for_type": "use crate::commands::agent::resolve_base_url_for_type;",
}

# cargo_errors.txt in Windows-accessible path
ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"

# === 日志 ===
fixed_files = {}  # file -> list of added imports
skipped_files = {}  # file -> reason

def normalize_path(p):
    """标准化路径，/ 分隔符，相对于项目根"""
    p = p.replace("\\", "/")
    if p.startswith("src/"):
        return "src-tauri/" + p
    return p

def read_file(filepath):
    """读取文件内容"""
    full_path = r"d:\OneManager\AxAgent\src-tauri\src" + filepath.replace("src/", "/") if filepath.startswith("src/") else filepath
    # Normalize separators
    full_path = full_path.replace("/", os.sep)
    try:
        with open(full_path, "r", encoding="utf-8") as f:
            return f.read()
    except FileNotFoundError:
        return None

def write_file(filepath, content):
    full_path = r"d:\OneManager\AxAgent\src-tauri\src" + filepath.replace("src/", "/") if filepath.startswith("src/") else filepath
    full_path = full_path.replace("/", os.sep)
    with open(full_path, "w", encoding="utf-8") as f:
        f.write(content)

def has_import(content, import_stmt):
    """检查文件是否已有该 import 语句"""
    escaped = re.escape(import_stmt)
    return re.search(r"^\s*" + escaped, content, re.MULTILINE) is not None

def has_any_import_for_type(content, type_name):
    """检查文件是否已有该类型的任何引入（可能用不同路径）"""
    # Check if type name is used in any use statement
    return re.search(r"^\s*use\s+.*\b" + re.escape(type_name) + r"\b", content, re.MULTILINE) is not None

def add_import(content, import_stmt, group_key=None):
    """
    在文件顶部添加 import 语句。
    如果文件已有 use crate::commands::{...} 结构，添加 error::ErrorResponse 到花括号内。
    否则在第一个非空非注释行之前添加。
    """
    # Special: crate::commands::{...} 结构追加
    if import_stmt == "use crate::commands::error::ErrorResponse;":
        # Check for existing crate::commands:: block
        m = re.search(r"^(use crate::commands::)\{([^}]*)\}", content, re.MULTILINE | re.DOTALL)
        if m:
            prefix = m.group(1)
            inner = m.group(2)
            if "error::ErrorResponse" not in inner and "ErrorResponse" not in inner:
                # Remove trailing whitespace/spaces before closing brace
                inner_stripped = inner.rstrip()
                if inner_stripped.endswith(","):
                    new_inner = inner_stripped + " error::ErrorResponse,"
                elif inner_stripped:
                    new_inner = inner_stripped + ", error::ErrorResponse,"
                else:
                    new_inner = " error::ErrorResponse,"
                new_content = content[:m.start()] + prefix + "{" + new_inner + "}" + content[m.end():]
                return new_content, "appended_to_crate_commands_block"
            return content, "already_present_in_block"

    # Check if already present
    if has_import(content, import_stmt):
        return content, "already_exists"

    # Check for similar import (different path, same type)
    type_name_match = re.search(r"::(\w+);$", import_stmt)
    if type_name_match:
        type_name = type_name_match.group(1)
        if has_any_import_for_type(content, type_name):
            # Check if there's a use statement referencing this type
            return content, f"already_imported_as_different_path"

    # Find insertion point: after the last use statement or at top of file
    lines = content.split("\n")
    last_use_line = -1
    first_non_comment_line = -1
    first_use_block_end = -1  # For multi-line use blocks

    for idx, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("use ") and stripped.endswith(";"):
            last_use_line = idx
        # Check for multi-line use block start
        if stripped.startswith("use ") and "{" in stripped and "}" not in stripped:
            # Find the closing line
            for j in range(idx, len(lines)):
                if "}" in lines[j] and ";" in lines[j]:
                    last_use_line = j
                    break

    # For tracing info/warn, check if tracing is already imported
    if import_stmt.startswith("use tracing::{"):
        # Check if tracing already imported
        if has_import(content, "use tracing;"):
            return content, "tracing_already_imported"
        # Check if info/warn already imported
        if re.search(r"^\s*use\s+(?:tracing::)?info", content, re.MULTILINE):
            return content, "info_already_imported"
        if re.search(r"^\s*use\s+(?:tracing::)?warn", content, re.MULTILINE):
            return content, "warn_already_imported"

    # Handle std:: imports - add after existing std imports or other use statements
    # Insert after the last use statement
    if last_use_line >= 0:
        # Insert after last_use_line, maintaining blank line if exists
        insert_pos = last_use_line + 1
        indent = ""
        new_content = lines[:insert_pos] + [indent + import_stmt] + lines[insert_pos:]
        return "\n".join(new_content), "added_after_last_use"
    else:
        # No existing use statements, add after shebang or at top
        new_content = import_stmt + "\n" + content
        return new_content, "added_at_top"


def add_std_type(content, type_name):
    """添加 use std::... 类型的 import"""
    module_name, _ = STD_TYPES[type_name]
    import_stmt = f"use std::{module_name}::{type_name};"
    return add_import(content, import_stmt)


def add_tracing_macros(content, missing_macros):
    """添加 use tracing::{info, warn, ...};"""
    # Check if use tracing; already exists
    if re.search(r"^\s*use\s+(?:crate|.)*tracing;", content, re.MULTILINE):
        # Replace with use tracing::{info, warn};
        new_content = re.sub(
            r"^(.*use\s+tracing;)\s*$",
            rf"\1{{info, warn}}",
            content,
            count=1,
            flags=re.MULTILINE
        )
        return new_content, "upgraded_tracing_to_braces"

    # Check if use tracing::info; or use tracing::warn; already exists
    has_info = re.search(r"use\s+tracing::info;?", content)
    has_warn = re.search(r"use\s+tracing::warn;?", content)

    if has_info and has_warn:
        return content, "already_has_both"
    if has_info:
        # Add warn to existing info import
        content = re.sub(
            r"\b(use\s+tracing::info);?\s*$",
            r"\1, warn;",
            content,
            count=1,
            flags=re.MULTILINE
        )
        return content, "added_warn_to_existing_info"
    if has_warn:
        content = re.sub(
            r"\b(use\s+tracing::warn);?\s*$",
            r"\1, info;",
            content,
            count=1,
            flags=re.MULTILINE
        )
        return content, "added_info_to_existing_warn"

    # Neither exists - check for existing use tracing::{...}
    existing_tracing = re.search(r"use\s+tracing::\{(.*)\}", content)
    if existing_tracing:
        inner = existing_tracing.group(1)
        new_inner = inner
        if "info" not in inner:
            new_inner += ", info"
        if "warn" not in inner:
            new_inner += ", warn"
        new_content = content[:existing_tracing.start()] + f"use tracing::{{{new_inner}}}" + content[existing_tracing.end():]
        return new_content, "appended_to_tracing_braces"

    # Add new import
    return add_import(content, "use tracing::{info, warn};")


def fix_file(filepath, missing_items):
    """修复单个文件"""
    content = read_file(filepath)
    if content is None:
        # Try without src-tauri prefix
        # Already handled in read_file
        skipped_files[filepath] = "file_not_found"
        return

    original = content
    changes = []

    for item in missing_items:
        item_type = item["type"]  # "type", "macro", "value", "function", "module"
        item_name = item["name"]

        new_content = None
        change_desc = None

        if item_name == "ErrorResponse":
            new_content, change_desc = add_import(content, "use crate::commands::error::ErrorResponse;")
        elif item_name in STD_TYPES:
            new_content, change_desc = add_std_type(content, item_name)
        elif item_name == "info":
            # Will be handled together with warn
            continue
        elif item_name == "warn":
            new_content, change_desc = add_tracing_macros(content, ["info", "warn"])
        elif item_name in FIX_MAP:
            imp = FIX_MAP[item_name]
            if imp:
                new_content, change_desc = add_import(content, imp)
        elif item_name in MODULE_CRATE_PREFIX:
            new_content, change_desc = add_import(content, MODULE_CRATE_PREFIX[item_name])
        elif item_name in VALUE_MAP:
            new_content, change_desc = add_import(content, VALUE_MAP[item_name])
        elif item_name in FUNCTION_REMAP:
            new_content, change_desc = add_import(content, FUNCTION_REMAP[item_name])
        else:
            # Unknown - skip
            skipped_files.setdefault(filepath, []).append(f"unknown_type:{item_name}")
            continue

        if new_content:
            content = new_content
            if "skip" not in change_desc and "already" not in change_desc and "not_needed" not in change_desc:
                changes.append(f"{item_name} -> {change_desc}")
        elif change_desc:
            # Already present etc
            pass

    if content != original and changes:
        write_file(filepath, content)
        fixed_files[filepath] = changes
        print(f"  [FIXED] {filepath}: {', '.join(changes)}")


def extract_errors():
    """从 error 文件中提取所有错误并按文件分组"""
    with open(ERROR_FILE, "r", encoding="utf-8") as f:
        lines = f.readlines()

    file_errors = {}  # filepath -> list of {type, name}

    for i, line in enumerate(lines):
        m = re.search(r"cannot find (type|macro|value|function|module) .([^" + BACKTICK + r"]+).", line)
        if m:
            err_type = m.group(1)
            err_name = m.group(2)
            current_file = None
            for j in range(i-1, max(i-5, -1), -1):
                fm = re.search(r'-->\s+(.+):(\d+):(\d+)', lines[j])
                if fm:
                    current_file = fm.group(1).replace('\\', '/')
                    # Convert to src/ prefix
                    if "src-tauri/src/" in current_file:
                        current_file = current_file.split("src-tauri/src/", 1)[1]
                    break
            if current_file:
                if current_file not in file_errors:
                    file_errors[current_file] = []
                file_errors[current_file].append({"type": err_type, "name": err_name})

    return file_errors


def main():
    file_errors = extract_errors()
    print(f"Found errors in {len(file_errors)} files")

    # Process info/warn together per file
    # Group errors by file and fix
    for filepath, errors in sorted(file_errors.items()):
        # Deduplicate error names in this file
        seen = set()
        unique_errors = []
        for e in errors:
            key = e["name"]
            if key not in seen:
                seen.add(key)
                unique_errors.append(e)

        print(f"\nProcessing: {filepath} ({len(unique_errors)} missing items)")
        for e in unique_errors:
            print(f"  - {e['type']}: {e['name']}")

        fix_file(filepath, unique_errors)

    # Summary
    print(f"\n\n=== Summary ===")
    print(f"Fixed files: {len(fixed_files)}")
    print(f"Skipped files: {len(skipped_files)}")

    # Print files that were fixed
    print(f"\n=== Fixed Files ===")
    for f, changes in fixed_files.items():
        print(f"  {f}: {changes}")

    if skipped_files:
        print(f"\n=== Skipped Files ===")
        for f, reasons in skipped_files.items():
            if isinstance(reasons, list):
                print(f"  {f}: {', '.join(reasons)}")
            else:
                print(f"  {f}: {reasons}")


if __name__ == "__main__":
    main()
