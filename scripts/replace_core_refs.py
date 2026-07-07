#!/usr/bin/env python3
"""Replace all axagent_core:: references with direct leaf crate paths.

Usage: python3 scripts/replace_core_refs.py
"""

import os
import re
import glob

ROOT = os.path.join(os.path.dirname(__file__), "..", "src-tauri")

def find_rs_files():
    """Find all .rs files that might reference axagent_core::"""
    files = []
    for root, dirs, _ in os.walk(os.path.join(ROOT, "crates")):
        # Skip crates/core itself
        if "crates/core" in root:
            continue
        for f in os.listdir(root):
            if f.endswith(".rs"):
                files.append(os.path.join(root, f))
    # Also include src/ (app binary)
    src_dir = os.path.join(ROOT, "src")
    for root, dirs, _ in os.walk(src_dir):
        for f in os.listdir(root):
            if f.endswith(".rs"):
                files.append(os.path.join(root, f))
    # schema-gen too
    schema_dir = os.path.join(ROOT, "schema-gen")
    for root, dirs, _ in os.walk(schema_dir):
        for f in os.listdir(root):
            if f.endswith(".rs"):
                files.append(os.path.join(root, f))
    return sorted(set(files))

# Replacement mapping: axagent_core::MODULE -> LEAF_CRATE_PATH
# Ordered from most specific to least specific to avoid partial matches
REPLACEMENTS = [
    # Special cases with different crate/name mappings
    ("axagent_core::crypto", "axagent_crypto"),
    ("axagent_core::computer_control", "axagent_kit::computer_control"),
    ("axagent_core::document_parser", "axagent_document_parser"),
    ("axagent_core::disk_cache", "axagent_disk_cache"),

    # Kit re-exports
    ("axagent_core::billing", "axagent_kit::billing"),
    ("axagent_core::browser_automation", "axagent_kit::browser_automation"),
    ("axagent_core::command_validator", "axagent_kit::command_validator"),
    ("axagent_core::git_tools", "axagent_kit::git_tools"),
    ("axagent_core::html_cleaner", "axagent_kit::html_cleaner"),
    ("axagent_core::markdown_parser", "axagent_kit::markdown_parser"),
    ("axagent_core::marketplace_service", "axagent_kit::marketplace_service"),
    ("axagent_core::marketplace", "axagent_kit::marketplace"),
    ("axagent_core::model_knowledge", "axagent_kit::model_knowledge"),
    ("axagent_core::operation_audit", "axagent_kit::operation_audit"),
    ("axagent_core::output_processor", "axagent_kit::output_processor"),
    ("axagent_core::plan_compiler", "axagent_kit::plan_compiler"),
    ("axagent_core::preset_templates", "axagent_kit::preset_templates"),
    ("axagent_core::prompt_template", "axagent_kit::prompt_template"),
    ("axagent_core::prompts", "axagent_kit::prompts"),
    ("axagent_core::resource_limits", "axagent_kit::resource_limits"),
    ("axagent_core::sandbox_runner", "axagent_kit::sandbox_runner"),
    ("axagent_core::schema_validator", "axagent_kit::schema_validator"),
    ("axagent_core::screen_capture", "axagent_kit::screen_capture"),
    ("axagent_core::screen_vision", "axagent_kit::screen_vision"),
    ("axagent_core::secure_store", "axagent_kit::secure_store"),
    ("axagent_core::service_container", "axagent_kit::service_container"),
    ("axagent_core::shell_parser", "axagent_kit::shell_parser"),
    ("axagent_core::skill_dirs", "axagent_kit::skill_dirs"),
    ("axagent_core::slash_command", "axagent_kit::slash_command"),
    ("axagent_core::token_budget", "axagent_kit::token_budget"),
    ("axagent_core::token_counter", "axagent_kit::token_counter"),
    ("axagent_core::ui_automation", "axagent_kit::ui_automation"),
    ("axagent_core::unified_config", "axagent_kit::unified_config"),
    ("axagent_core::utils", "axagent_kit::utils"),
    ("axagent_core::workflow_version", "axagent_kit::workflow_version"),

    # Search re-exports
    ("axagent_core::ast_index", "axagent_search::ast_index"),
    ("axagent_core::file_index", "axagent_search::file_index"),
    ("axagent_core::hybrid_search", "axagent_search::hybrid_search"),
    ("axagent_core::incremental_indexer", "axagent_search::incremental_indexer"),
    ("axagent_core::inference", "axagent_search::inference"),
    ("axagent_core::model_downloader", "axagent_search::model_downloader"),
    ("axagent_core::query_enhancement", "axagent_search::query_enhancement"),
    ("axagent_core::rag_pipeline", "axagent_search::rag_pipeline"),
    ("axagent_core::rag", "axagent_search::rag"),
    ("axagent_core::recall_pipeline", "axagent_search::recall_pipeline"),
    ("axagent_core::reranker", "axagent_search::reranker"),
    ("axagent_core::search", "axagent_search::search"),
    ("axagent_core::self_rag", "axagent_search::self_rag"),
    ("axagent_core::semantic_cache", "axagent_search::semantic_cache"),
    ("axagent_core::text_chunker", "axagent_search::text_chunker"),
    ("axagent_core::vector_cache", "axagent_search::vector_cache"),
    ("axagent_core::vector_store", "axagent_search::vector_store"),

    # Storage re-exports
    ("axagent_core::cloud_storage", "axagent_storage::cloud_storage"),
    ("axagent_core::cloud_workspace", "axagent_storage::cloud_workspace"),
    ("axagent_core::file_authorizer", "axagent_storage::file_authorizer"),
    ("axagent_core::file_store", "axagent_storage::file_store"),
    ("axagent_core::path_vars", "axagent_storage::path_vars"),
    ("axagent_core::storage_inventory", "axagent_storage::storage_inventory"),
    ("axagent_core::storage_migration", "axagent_storage::storage_migration"),
    ("axagent_core::storage_paths", "axagent_storage::storage_paths"),
    ("axagent_core::sync_conflict", "axagent_storage::sync_conflict"),
    ("axagent_core::webdav", "axagent_storage::webdav"),
    ("axagent_core::workspace_uri", "axagent_storage::workspace_uri"),

    # Cache re-exports
    ("axagent_core::cache_persister", "axagent_cache::cache_persister"),
    ("axagent_core::cache_snapshot", "axagent_cache::cache_snapshot"),
    ("axagent_core::cache", "axagent_cache::cache"),

    # MCP re-exports
    ("axagent_core::mcp_client", "axagent_mcp::mcp_client"),
    ("axagent_core::mcp_health", "axagent_mcp::mcp_health"),
    ("axagent_core::mcp_oauth", "axagent_mcp::mcp_oauth"),

    # DAO re-exports
    ("axagent_core::repo", "axagent_dao::repo"),
    ("axagent_core::db", "axagent_dao::db"),
    ("axagent_core::ddl", "axagent_dao::ddl"),

    # Entities re-export
    ("axagent_core::entity", "axagent_entities"),

    # Harness re-exports
    ("axagent_core::workflow_types", "axagent_harness::workflow_types"),
    ("axagent_core::platform_config", "axagent_harness::platform_config"),
    ("axagent_core::constants", "axagent_harness::constants"),
    ("axagent_core::error_codes", "axagent_harness::error_codes"),
    ("axagent_core::error", "axagent_harness::core_error"),
    ("axagent_core::persistence", "axagent_harness"),
    ("axagent_core::i18n", "axagent_harness::i18n"),
]

def replace_in_file(filepath):
    """Replace all axagent_core:: references in a single .rs file."""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    # Skip files that don't contain axagent_core::
    if "axagent_core::" not in content:
        return False

    old_content = content
    for old, new in REPLACEMENTS:
        content = content.replace(old, new)

    if content != old_content:
        print(f"  Modified: {os.path.relpath(filepath, ROOT)}")
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(content)
        return True
    return False

def main():
    files = find_rs_files()
    print(f"Found {len(files)} .rs files to scan")

    modified = 0
    for f in files:
        if replace_in_file(f):
            modified += 1

    print(f"\nModified {modified} files")
    print("Done!")

if __name__ == "__main__":
    main()
