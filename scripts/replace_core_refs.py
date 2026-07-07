#!/usr/bin/env python3
"""Replace axagent_core:: references with direct leaf crate paths across 9 crates.

Mapping from core re-exports (crates/core/src/lib.rs):
  error/*        → axagent_harness::core_error/*
  i18n/*         → axagent_harness::i18n/*
  constants/*    → axagent_harness::constants/*
  workflow_types/* → axagent_harness::workflow_types/* (except local types)
  platform_config/* → axagent_harness::platform_config/*
  validate_recursive → axagent_harness::schema_validator::validate_recursive
  ddl/*          → axagent_dao::ddl/*
  repo/*         → axagent_dao::repo/*
  db/*           → axagent_dao::db/*
  entity/*       → axagent_entities::*
  plan_compiler/*  → axagent_kit::plan_compiler/*
  secure_store/*   → axagent_kit::secure_store/*
  model_knowledge/* → axagent_kit::model_knowledge/*
  screen_vision/*  → axagent_kit::screen_vision/*
  browser_automation/* → axagent_kit::browser_automation/*
  computer_control/* → axagent_kit::computer_control/*
  git_tools/*      → axagent_kit::git_tools/*
  html_cleaner/*   → axagent_kit::html_cleaner/*
  skill_dirs/*     → axagent_kit::skill_dirs/*
  slash_command/*  → axagent_kit::slash_command/*
  crypto/*         → axagent_crypto::crypto/*
  search/*         → axagent_search::search/*
  validate_against_schema → axagent_kit::schema_validator::validate_against_schema
  extract_json_from_llm_response → axagent_kit::utils::extract_json_from_llm_response
"""

import os
import re
import subprocess

CRATES_DIR = os.path.join("src-tauri", "crates")
TARGET_CRATES = ["agent", "orchestrator", "migration", "providers",
                 "rt-messaging", "rt-workflow", "tools", "trajectory", "runtime"]

# Mapping: axagent_core::PREFIX → REPLACEMENT
PREFIX_MAP = {
    "axagent_core::error::":         "axagent_harness::core_error::",
    "axagent_core::i18n::":          "axagent_harness::i18n::",
    "axagent_core::constants::":     "axagent_harness::constants::",
    "axagent_core::workflow_types::":"axagent_harness::workflow_types::",
    "axagent_core::platform_config::":"axagent_harness::platform_config::",
    "axagent_core::ddl::":           "axagent_dao::ddl::",
    "axagent_core::repo::":          "axagent_dao::repo::",
    "axagent_core::db::":            "axagent_dao::db::",
    "axagent_core::entity::":        "axagent_entities::",
    "axagent_core::plan_compiler::": "axagent_kit::plan_compiler::",
    "axagent_core::secure_store::":  "axagent_kit::secure_store::",
    "axagent_core::model_knowledge::":"axagent_kit::model_knowledge::",
    "axagent_core::screen_vision::": "axagent_kit::screen_vision::",
    "axagent_core::browser_automation::":"axagent_kit::browser_automation::",
    "axagent_core::computer_control::":"axagent_kit::computer_control::",
    "axagent_core::git_tools::":     "axagent_kit::git_tools::",
    "axagent_core::html_cleaner::":  "axagent_kit::html_cleaner::",
    "axagent_core::skill_dirs::":    "axagent_kit::skill_dirs::",
    "axagent_core::slash_command::": "axagent_kit::slash_command::",
    "axagent_core::crypto::":        "axagent_crypto::crypto::",
    "axagent_core::search::":        "axagent_search::search::",
}

# Special function names (not module::function but just a bare fn name)
FUNCTION_MAP = {
    "axagent_core::validate_recursive": "axagent_harness::schema_validator::validate_recursive",
    "axagent_core::validate_against_schema": "axagent_kit::schema_validator::validate_against_schema",
    "axagent_core::extract_json_from_llm_response": "axagent_kit::utils::extract_json_from_llm_response",
}

# Cargo.toml dependencies to add based on which replacements were used
DEP_MAP = {
    "axagent_harness": [],
    "axagent_dao": [],
    "axagent_entities": [],
    "axagent_kit": [],
    "axagent_crypto": [],
    "axagent_search": [],
}

def apply_replacements(content):
    """Apply all prefix and function replacements to a file's content."""
    for old, new in FUNCTION_MAP.items():
        content = content.replace(old, new)
    for old, new in PREFIX_MAP.items():
        content = content.replace(old, new)
    return content

def get_used_replacement_modules(content):
    """Determine which leaf crate deps are needed based on replacements applied."""
    modules = set()
    if "axagent_harness::" in content:
        modules.add("axagent_harness")
    if "axagent_dao::" in content:
        modules.add("axagent_dao")
    if "axagent_entities::" in content:
        modules.add("axagent_entities")
    if "axagent_kit::" in content:
        modules.add("axagent_kit")
    if "axagent_crypto::" in content:
        modules.add("axagent_crypto")
    if "axagent_search::" in content:
        modules.add("axagent_search")
    return modules

def update_cargo_toml(crate_name, used_modules):
    """Add required leaf crate deps and remove axagent-core from Cargo.toml."""
    cargo_path = os.path.join(CRATES_DIR, crate_name, "Cargo.toml")
    if not os.path.exists(cargo_path):
        print(f"  Cargo.toml not found: {cargo_path}")
        return

    with open(cargo_path, "r") as f:
        lines = f.readlines()

    new_lines = []
    in_deps = False
    has_axagent_core = False
    existing_axagent_deps = set()

    for line in lines:
        if line.startswith("[dependencies]"):
            in_deps = True
            new_lines.append(line)
            continue
        elif line.startswith("[") and not line.startswith("[[") and in_deps:
            in_deps = False

        if in_deps:
            m = re.match(r'^\s*(axagent-\w+)', line)
            if m:
                existing_axagent_deps.add(m.group(1))
                if "axagent-core" in line:
                    has_axagent_core = True
                    continue  # skip axagent-core
            new_lines.append(line)
        else:
            new_lines.append(line)

    if not has_axagent_core:
        print(f"  (no axagent-core to remove)")
        # Still write back in case we need to add new deps
        pass

    # Check if new leaf crate deps are needed
    leaf_to_path = {
        "axagent-dao": '"../dao"',
        "axagent-entities": '"../entities"',
        "axagent-kit": '"../kit"',
        "axagent-crypto": '"../crypto"',
        "axagent-search": '"../search"',
    }

    needed_additions = []
    for module in sorted(used_modules):
        leaf_name = module.replace("_", "-")
        if leaf_name not in existing_axagent_deps and leaf_name != "axagent-harness":
            path = leaf_to_path.get(leaf_name, f'"../{leaf_name.replace("axagent-", "")}"')
            needed_additions.append(f"{leaf_name} = {{ path = {path} }}\n")

    if not has_axagent_core and not needed_additions:
        return  # no changes

    # Insert new deps after [dependencies] line
    insert_pos = None
    for i, line in enumerate(new_lines):
        if line.startswith("[dependencies]"):
            insert_pos = i + 1
            break

    if insert_pos is not None:
        for dep in needed_additions:
            new_lines.insert(insert_pos, dep)
            insert_pos += 1

    with open(cargo_path, "w") as f:
        f.writelines(new_lines)

    action = "Removed axagent-core"
    if needed_additions:
        action += f", added {needed_additions}"
    print(f"  {action}")

def main():
    for crate in TARGET_CRATES:
        src_dir = os.path.join(CRATES_DIR, crate, "src")
        if not os.path.exists(src_dir):
            print(f"=== {crate}: src/ not found ===")
            continue

        print(f"\n=== {crate} ===")
        replacements_found = False
        all_content = ""

        for root, dirs, files in os.walk(src_dir):
            for fname in files:
                if not fname.endswith(".rs"):
                    continue
                fpath = os.path.join(root, fname)
                with open(fpath, "r") as f:
                    content = f.read()

                if "axagent_core::" not in content:
                    continue

                new_content = apply_replacements(content)
                if new_content != content:
                    with open(fpath, "w") as f:
                        f.write(new_content)
                    replacements_found = True
                    # Count replacements
                    count_old = content.count("axagent_core::")
                    count_new = new_content.count("axagent_core::")
                    print(f"  {os.path.relpath(fpath)}: {count_old - count_new} replacements")
                    all_content += new_content

        if not replacements_found:
            print("  No axagent_core references found")

        # Update Cargo.toml
        with open(os.path.join(CRATES_DIR, crate, "src", "lib.rs")) if os.path.exists(os.path.join(CRATES_DIR, crate, "src", "lib.rs")) else open(os.devnull, "r"):
            pass

        # Determine needed deps from full source
        full_src = ""
        for root, dirs, files in os.walk(src_dir):
            for fname in files:
                if fname.endswith(".rs"):
                    with open(os.path.join(root, fname)) as f:
                        full_src += f.read()

        used_modules = get_used_replacement_modules(full_src)
        update_cargo_toml(crate, used_modules)

    print("\n=== Done ===")

if __name__ == "__main__":
    main()
