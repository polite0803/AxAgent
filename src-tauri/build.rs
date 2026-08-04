// SPDX-License-Identifier: AGPL-3.0-only

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Enable cfg(mobile) when building for Android or iOS
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("android")
        || target.contains("ios")
        || cfg!(target_os = "android")
        || cfg!(target_os = "ios")
    {
        println!("cargo:rustc-cfg=mobile");
        println!("cargo:warning=Building for mobile target: {}", target);
    }

    println!("cargo::rustc-check-cfg=cfg(mobile)");

    // ── Windows: 增大主线程栈到 8MB ──
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/STACK:8388608");
    }

    // ── Windows: Common Controls v6 manifest 统一处理 ──
    #[cfg(target_os = "windows")]
    {
        let is_tauri_workspace = std::env::var("__TAURI_WORKSPACE__").is_ok_and(|v| v == "true");
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if is_tauri_workspace && target_env == "msvc" {
            let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("common-controls.manifest");
            println!("cargo:rerun-if-changed={}", manifest.display());
            println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
            println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        }
    }

    // ── 生成命令元数据索引（简化版） ──
    generate_command_index();

    #[cfg(target_os = "windows")]
    {
        let is_tauri_workspace = std::env::var("__TAURI_WORKSPACE__").is_ok_and(|v| v == "true");
        if is_tauri_workspace {
            let attrs = tauri_build::Attributes::new()
                .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
            tauri_build::try_build(attrs).unwrap_or_else(|e| {
                println!("cargo:warning=tauri-build try_build failed: {e:#}");
                tauri_build::build();
            });
            return;
        }
    }

    tauri_build::build()
}

// ── 命令元数据生成（简化版） ─────────────────────────────────────

/// 命令元数据（仅用于索引生成）
struct CommandInfo {
    /// 短名称，如 "list_providers"
    name: String,
    /// 完整路径，如 "commands::providers::list_providers"
    full_path: String,
    /// 所属模块
    module: String,
}

/// 从模块路径推断命令域
fn infer_domain(module: &str) -> &'static str {
    match module {
        "providers" | "provider_balance" | "local_models" | "local_model" => "provider",
        "conversations" | "conversations_search" | "conversation_categories" | "branches" => {
            "conversation"
        }
        "messages" | "message_continuation" => "message",
        "knowledge" | "knowledge_graph" | "knowledge_source" | "paper" | "unified_knowledge" => {
            "knowledge"
        }
        "memory" => "memory",
        "settings" | "app_config" => "settings",
        "gateway" | "gateway_link" => "gateway",
        "workflows" | "workflow_template" | "workflow_ai" | "workflow_ai_apply"
        | "workflow_ai_diagnose" | "workflow_reflection" | "workflow_execution_stats"
        | "workflow_yaml" | "work_engine" => "workflow",
        "mcp" => "mcp",
        "skills" | "skills_hub" | "skill_decomposition" => "skill",
        "files" | "files_page" | "file_browser" => "files",
        "storage" => "storage",
        "backup" | "webdav" => "backup",
        "webhook" => "webhook",
        "terminal" | "pty" => "terminal",
        "theme" => "settings",
        "profile" => "settings",
        "desktop" | "computer_control" | "screen_vision" => "desktop",
        "browser" => "browser",
        "dashboard" => "dashboard",
        "prompt_templates" => "knowledge",
        "context_sources" | "search" | "sources" => "knowledge",
        "agent" | "sub_agent" | "multi_agent" => "core",
        "plan" => "workflow",
        "orchestrator" => "workflow",
        "agent_nudge" | "agent_insight" | "agent_analytics" | "agent_advanced" => "core",
        "smart_router" => "core",
        "evolution" | "evolution_engine" | "reflection" => "core",
        "proactive" => "core",
        "reminder" => "core",
        "scheduled_task" => "core",
        "user_profile" | "personality" => "settings",
        "fleet" => "core",
        "dreams" | "dream" => "memory",
        "background_tasks" => "core",
        "platform_integration" => "gateway",
        "dynamic_ui" => "core",
        "wiki" | "llm_wiki" => "knowledge",
        "hooks_config" => "settings",
        "artifacts" => "core",
        "index_jobs" => "knowledge",
        "evaluator" => "core",
        "research" => "core",
        "fine_tune" | "rl_training" | "trajectory" => "core",
        "tool_recommender" | "generated_tool" | "local_tool" => "skill",
        "marketplace" => "skill",
        "plugin" => "skill",
        "cloud_workspace" => "files",
        "distributed" => "workflow",
        "learning_graph" => "knowledge",
        "session_share" => "conversation",
        "crash_report" | "health" => "core",
        "migration" => "settings",
        "agency_expert" | "agent_role" | "business_role" => "core",
        "other" => "core",
        _ => "core",
    }
}

/// 从命令名推断调用类型
///
/// 返回值:
/// - "state_only": 只有 state 参数
/// - "state_input": 有 state + input 参数
/// - "unknown": 无法推断
fn infer_command_type(name: &str) -> &'static str {
    let lower = name.to_lowercase();

    // State-only 模式（只读查询）
    let state_only_patterns = [
        "list_", "get_", "fetch_", "check_", "is_", "has_", "count_",
        "total_", "stats", "status", "config", "settings",
        "query_", "find_", "search_", "select_", "show_", "display_",
        "can_", "should_", "would_", "could_", "may_",
    ];

    for pattern in &state_only_patterns {
        if lower.starts_with(pattern) {
            return "state_only";
        }
    }

    // State+Input 模式（写入操作）
    let state_input_patterns = [
        "create_", "update_", "delete_", "add_", "remove_", "save_",
        "send_", "execute_", "run_", "submit_", "publish_", "generate_",
        "import_", "install_", "apply_", "connect_", "disconnect_",
        "download_", "upload_", "backup_", "restore_", "sync_",
        "start_", "stop_", "restart_", "reload_", "refresh_",
        "resume_", "pause_", "approve_", "reject_", "confirm_",
        "schedule_", "spawn_", "dispatch_", "subscribe_", "emit_",
        "render_", "toggle_", "switch_", "reset_", "clear_",
        "batch_", "bulk_", "reorder_", "reindex_", "rebuild_",
        "train_", "learn_", "evolve_", "optimize_", "compile_",
        "set_", "put_", "post_", "push_", "write_", "edit_",
        "enable_", "disable_", "activate_", "deactivate_",
    ];

    for pattern in &state_input_patterns {
        if lower.starts_with(pattern) {
            return "state_input";
        }
    }

    // 默认：未知，需要手动处理
    "unknown"
}

/// 解析 register_commands.rs 获取所有命令
fn parse_commands() -> Vec<CommandInfo> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let register_path = PathBuf::from(&manifest_dir).join("src").join("register_commands.rs");

    let content = match fs::read_to_string(&register_path) {
        Ok(c) => c,
        Err(_) => {
            println!("cargo:warning=Cannot read register_commands.rs, using empty command index");
            return Vec::new();
        }
    };

    let mut commands = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // 跳过注释行、空行、cfg 属性
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("///")
            || trimmed.starts_with("#[cfg")
            || trimmed.starts_with("macro_rules!")
            || trimmed.starts_with("tauri::generate_handler!")
            || trimmed == "]"
            || trimmed == "};"
        {
            continue;
        }

        // 提取命令路径（移除末尾的逗号）
        let path = if let Some(stripped) = trimmed.strip_suffix(',') {
            stripped.trim().to_string()
        } else {
            trimmed.to_string()
        };

        // 只处理以 commands:: 或 crate:: 开头的路径
        if !path.starts_with("commands::") && !path.starts_with("crate::") {
            continue;
        }

        // 确保路径包含 ::
        if !path.contains("::") {
            continue;
        }

        let full_path = path.clone();

        // 提取模块和名称
        let parts: Vec<&str> = full_path.split("::").collect();
        let (module, name) = if parts.len() >= 3 {
            (parts[1].to_string(), parts[2].to_string())
        } else if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            continue;
        };

        commands.push(CommandInfo { full_path, name, module });
    }

    // 去重
    commands.sort_by(|a, b| a.full_path.cmp(&b.full_path));
    commands.dedup_by(|a, b| a.full_path == b.full_path);

    commands
}

/// 生成命令索引文件（简化版）
///
/// 只生成元数据表，不生成派发代码。
/// 派发逻辑在 command_bridge.rs 中通过 serde_json 动态调用。
fn generate_command_index() {
    let out_dir = env::var("OUT_DIR").unwrap_or_else(|_| ".".to_string());
    let output_path = PathBuf::from(&out_dir).join("generated_command_index.rs");

    let commands = parse_commands();
    let count = commands.len();

    // 打印构建信息
    println!("cargo:warning=Command index: {} commands from register_commands.rs", count);

    // 生成 Rust 代码
    let mut code = String::new();

    code.push_str("// Auto-generated by build.rs — do not edit manually.\n");
    code.push_str(&format!("// Total commands: {}\n", count));
    code.push('\n');

    // 生成命令元数据表
    code.push_str("/// 命令元数据: (名称, 完整路径, 域)\n");
    code.push_str("pub const COMMAND_METADATA: &[(&str, &str, &str)] = &[\n");

    for cmd in &commands {
        let domain = infer_domain(&cmd.module);
        code.push_str(&format!(
            "    (\"{}\", \"{}\", \"{}\"),\n",
            cmd.name, cmd.full_path, domain
        ));
    }

    code.push_str("];\n");
    code.push('\n');

    // 生成命令路径映射（短名称 → 完整路径）
    code.push_str("/// 命令短名称到完整路径的映射\n");
    code.push_str("pub const COMMAND_PATH_MAP: &[(&str, &str)] = &[\n");

    for cmd in &commands {
        code.push_str(&format!(
            "    (\"{}\", \"{}\"),\n",
            cmd.name, cmd.full_path
        ));
    }

    code.push_str("];\n");
    code.push('\n');

    // 生成命令调用列表（用于 serde_json 动态调用）
    code.push_str("/// 所有命令的完整路径列表\n");
    code.push_str("pub const ALL_COMMAND_PATHS: &[&str] = &[\n");

    for cmd in &commands {
        code.push_str(&format!("    \"{}\",\n", cmd.full_path));
    }

    code.push_str("];\n");
    code.push('\n');

    // 生成命令类型信息（用于运行时决定调用方式）
    code.push_str("/// 命令类型分类（基于命名推断）\n");
    code.push_str("///\n");
    code.push_str("/// 注意：这是基于命令名的启发式推断，可能不准确。\n");
    code.push_str("/// 对于推断错误的命令，需要在 command_bridge.rs 中手动处理。\n");
    code.push_str("pub const COMMAND_TYPES: &[(&str, &str)] = &[\n");

    for cmd in &commands {
        let cmd_type = infer_command_type(&cmd.name);
        code.push_str(&format!(
            "    (\"{}\", \"{}\"),\n",
            cmd.name, cmd_type
        ));
    }

    code.push_str("];\n");
    code.push('\n');

    // 写入文件
    if let Ok(mut file) = fs::File::create(&output_path) {
        let _ = file.write_all(code.as_bytes());
    } else {
        println!("cargo:warning=Cannot write generated_command_index.rs");
    }

    // 告诉 Cargo 何时重新运行
    println!("cargo:rerun-if-changed=src/register_commands.rs");
}
