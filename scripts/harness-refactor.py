#!/usr/bin/env python3
"""Harness 架构重构：将业务逻辑移出 harness crate。"""

import os
import re
import shutil

SRC_TAURI = "D:/OneManager/AxAgent/src-tauri"
HARNESS_SRC = os.path.join(SRC_TAURI, "crates", "harness", "src")
RUNTIME_CORE_SRC = os.path.join(SRC_TAURI, "crates", "runtime-core", "src")
RT_WORKFLOW_SRC = os.path.join(SRC_TAURI, "crates", "rt-workflow", "src")
PROVIDERS_SRC = os.path.join(SRC_TAURI, "crates", "providers", "src")

def read_file(path):
    with open(path, "r", encoding="utf-8") as f:
        return f.read()

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)

# ── Step 1: Copy execute_llm.rs to runtime-core ──
execute_llm_content = read_file(os.path.join(HARNESS_SRC, "execute_llm.rs"))
# Update import paths from `crate::` to `axagent_harness::`
execute_llm_content = re.sub(
    r'use crate::(audit_trail::\{AuditEntry, AuditRecorder\})',
    r'use axagent_harness::\1',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'use crate::prompt_guard::PromptGuard',
    r'use axagent_harness::prompt_guard::PromptGuard',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'use crate::provider::\{ProviderAdapter, ProviderRequestContext\}',
    r'use axagent_harness::provider::{ProviderAdapter, ProviderRequestContext}',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'use crate::types::\{ChatContent, ChatRequest, ChatResponse\}',
    r'use axagent_harness::types::{ChatContent, ChatRequest, ChatResponse}',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'use crate::retry_policy::RetryPolicy',
    r'use axagent_harness::retry_policy::RetryPolicy',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'crate::tool::InputSanitizer',
    r'axagent_harness::tool::InputSanitizer',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'crate::confidence::ConfidenceConfig',
    r'axagent_harness::confidence::ConfidenceConfig',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'crate::confidence::ConfidenceAction',
    r'axagent_harness::confidence::ConfidenceAction',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'crate::cache_interceptor::(HarnessCache|LlmCacheKey)',
    r'axagent_harness::cache_interceptor::\1',
    execute_llm_content,
)
execute_llm_content = re.sub(
    r'crate::audit_trail::AuditEntry',
    r'axagent_harness::audit_trail::AuditEntry',
    execute_llm_content,
)

write_file(os.path.join(RUNTIME_CORE_SRC, "llm_executor.rs"), execute_llm_content)
print("✅ Created runtime-core/src/llm_executor.rs")

# ── Step 2: Copy retry_policy.rs to runtime-core ──
retry_content = read_file(os.path.join(HARNESS_SRC, "retry_policy.rs"))
write_file(os.path.join(RUNTIME_CORE_SRC, "retry_policy.rs"), retry_content)
print("✅ Created runtime-core/src/retry_policy.rs")

# ── Step 3: Copy business_rules.rs to rt-workflow ──
rules_content = read_file(os.path.join(HARNESS_SRC, "business_rules.rs"))
write_file(os.path.join(RT_WORKFLOW_SRC, "business_rules.rs"), rules_content)
print("✅ Created rt-workflow/src/business_rules.rs")

# ── Step 4: Copy url_utils.rs to providers (update existing) ──
url_content = read_file(os.path.join(HARNESS_SRC, "url_utils.rs"))
# Update the providers url_utils to be the actual source, not a re-export
write_file(os.path.join(PROVIDERS_SRC, "url_utils.rs"), url_content)
print("✅ Updated providers/src/url_utils.rs")

# ── Step 5: Update harness lib.rs ──
lib_path = os.path.join(HARNESS_SRC, "lib.rs")
lib_content = read_file(lib_path)

# Remove the 4 business logic modules and their re-exports
# Remove business_rules
lib_content = re.sub(
    r'pub mod business_rules;\n.*?pub use business_rules::\{BusinessRule, BusinessRuleEngine, RuleResult\};\n',
    '',
    lib_content,
)
# Remove retry_policy
lib_content = re.sub(
    r'\n// ── 中心化重试/降级策略 ──\n.*?pub mod retry_policy;\n.*?pub use retry_policy::\{BackoffStrategy, FallbackStrategy, RetryPolicy\};\n',
    '',
    lib_content,
    flags=re.DOTALL,
)
# Remove execute_llm
lib_content = re.sub(
    r'\n// ── ExecuteLlm 中心化调用入口 ──\n.*?pub mod execute_llm;\n.*?pub use execute_llm::\{LlmCallConfig, LlmCallResult, LlmUsage, execute_llm\};\n',
    '',
    lib_content,
    flags=re.DOTALL,
)
# Remove url_utils
lib_content = re.sub(
    r'\n.*?pub mod url_utils;\n',
    '\n',
    lib_content,
)
lib_content = re.sub(
    r'\n// ── Provider 契约重导出 ──\n.*?pub use context_builder::build_provider_request_context;\n.*?pub use has_provider_registry::HasProviderRegistry;\n.*?pub use provider::\{ProviderAdapter, ProviderProxyConfig, ProviderRequestContext\};\n.*?pub use url_utils::\{\n.*?    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,\n.*?\};\n',
    '\n// ── Provider 契约重导出 ──\npub use context_builder::build_provider_request_context;\npub use has_provider_registry::HasProviderRegistry;\npub use provider::{ProviderAdapter, ProviderProxyConfig, ProviderRequestContext};\n',
    lib_content,
    flags=re.DOTALL,
)

write_file(lib_path, lib_content)
print("✅ Updated harness/src/lib.rs")

# ── Step 6: Update runtime-core lib.rs ──
rc_lib_path = os.path.join(RUNTIME_CORE_SRC, "lib.rs")
rc_lib = read_file(rc_lib_path)

# Add llm_executor and retry_policy modules (before the closing block)
rc_lib = rc_lib.rstrip()
# Insert before (or after) an existing module. Let's add after permission_enforcer
rc_lib = rc_lib.replace(
    'pub mod permission_enforcer;',
    'pub mod permission_enforcer;\npub mod retry_policy;\npub use retry_policy::{BackoffStrategy, FallbackStrategy, RetryPolicy};\npub mod llm_executor;\npub use llm_executor::{LlmCallConfig, LlmCallResult, LlmUsage, execute_llm};',
    1,
)
write_file(rc_lib_path, rc_lib)
print("✅ Updated runtime-core/src/lib.rs")

# ── Step 7: Update rt-workflow lib.rs ──
rw_lib_path = os.path.join(RT_WORKFLOW_SRC, "lib.rs")
rw_lib = read_file(rw_lib_path)
if "pub mod business_rules;" not in rw_lib:
    rw_lib = rw_lib.replace(
        'pub mod work_engine;',
        'pub mod business_rules;\npub mod work_engine;',
        1,
    )
write_file(rw_lib_path, rw_lib)
print("✅ Updated rt-workflow/src/lib.rs")

# ── Step 8: Delete old files from harness ──
os.remove(os.path.join(HARNESS_SRC, "execute_llm.rs"))
print("🗑️ Deleted harness/src/execute_llm.rs")
os.remove(os.path.join(HARNESS_SRC, "retry_policy.rs"))
print("🗑️ Deleted harness/src/retry_policy.rs")
os.remove(os.path.join(HARNESS_SRC, "business_rules.rs"))
print("🗑️ Deleted harness/src/business_rules.rs")
os.remove(os.path.join(HARNESS_SRC, "url_utils.rs"))
print("🗑️ Deleted harness/src/url_utils.rs")

# ── Step 9: Update caller import paths ──
# Callers that need updating:
callers = [
    # agent crate - execute_llm
    "D:/OneManager/AxAgent/src-tauri/crates/agent/src/wiki_compiler.rs",
    "D:/OneManager/AxAgent/src-tauri/crates/agent/src/react_engine.rs",
    "D:/OneManager/AxAgent/src-tauri/crates/agent/src/tree_of_thoughts.rs",
    # rt-workflow - execute_llm
    "D:/OneManager/AxAgent/src-tauri/crates/rt-workflow/src/work_engine/executors/llm_executor.rs",
    # rt-workflow - business_rules
    "D:/OneManager/AxAgent/src-tauri/crates/rt-workflow/src/work_engine/dispatcher.rs",
    # gateway - url_utils
    "D:/OneManager/AxAgent/src-tauri/crates/gateway/src/native.rs",
    "D:/OneManager/AxAgent/src-tauri/crates/gateway/src/handlers.rs",
    # providers - url_utils is already the source, just need to update
    # The providers re-export file will need updating
]

for caller in callers:
    if not os.path.exists(caller):
        print(f"⚠️  File not found: {caller}")
        continue
    c = read_file(caller)
    orig = c
    # Update execute_llm imports
    c = c.replace(
        "use axagent_harness::execute_llm::{LlmCallConfig, execute_llm};",
        "use axagent_runtime_core::{LlmCallConfig, execute_llm};",
    )
    # Update business_rules imports
    c = c.replace(
        "use axagent_harness::business_rules::RuleEvaluationOutcome;",
        "use crate::business_rules::RuleEvaluationOutcome;",
    )
    # Update url_utils imports
    c = c.replace(
        "use axagent_harness::url_utils::resolve_base_url_for_type;",
        "use axagent_providers::url_utils::resolve_base_url_for_type;",
    )
    if c != orig:
        write_file(caller, c)
        print(f"✅ Updated imports: {os.path.relpath(caller, SRC_TAURI)}")

# providers/src/url_utils.rs was a re-export file, now it's the source
# Update it to be a real module that provides the functions
prov_url_path = os.path.join(PROVIDERS_SRC, "url_utils.rs")
prov_url = read_file(prov_url_path)
# Remove the re-export line if it was `pub use axagent_harness::url_utils::{...}`
prov_url = prov_url.replace('pub use axagent_harness::url_utils::{\n    default_version_for_type, resolve_base_url, resolve_base_url_for_type, resolve_chat_url,\n};\n', '')
write_file(prov_url_path, prov_url)
print("✅ providers/src/url_utils.rs is now the source definition")

# Need to update providers lib.rs to ensure url_utils is re-exported
prov_lib = read_file(os.path.join(PROVIDERS_SRC, "lib.rs"))
if "pub mod url_utils;" not in prov_lib:
    prov_lib = prov_lib.replace(
        "pub use url_utils::{",
        "pub mod url_utils;\npub use url_utils::{",
    )
    write_file(os.path.join(PROVIDERS_SRC, "lib.rs"), prov_lib)
    print("✅ Added pub mod url_utils to providers/lib.rs")

print("\n=== 完成！===")
print("请手动检查并编译验证。")
