// SPDX-License-Identifier: AGPL-3.0-only

//! 命令文档生成器 — 基于 #[agent_command] 宏元数据自动生成 API 文档
//!
//! 使用方法：在项目根目录执行 `cargo test -- generate_command_docs`
//! 这将在 docs/commands 目录下生成 Markdown 格式的命令文档。

use std::fs;
use std::io::Write;
use std::path::Path;

/// 生成所有命令的文档
#[test]
fn generate_command_docs() {
    let output_dir = Path::new("docs/commands");
    fs::create_dir_all(output_dir).expect("无法创建文档输出目录");

    let commands = agent_command_types::registry::get_all();

    if commands.is_empty() {
        println!("⚠ 警告: 没有找到任何注册的命令。请确保已添加 #[agent_command] 宏。");
        return;
    }

    // 按域分组
    let mut by_domain: Vec<(&str, Vec<&agent_command_types::CommandMetadata>)> = Vec::new();
    let mut domains = std::collections::HashSet::new();

    for cmd in &commands {
        if !domains.contains(cmd.domain) {
            domains.insert(cmd.domain.to_string());
            by_domain.push((cmd.domain, Vec::new()));
        }
        if let Some((_, list)) = by_domain.iter_mut().find(|(d, _)| *d == cmd.domain) {
            list.push(cmd);
        }
    }

    // 为每个域生成文档
    for (domain, commands_in_domain) in &mut by_domain {
        commands_in_domain.sort_by_key(|c| c.name);

        let doc_content = generate_domain_document(domain, commands_in_domain);
        let filename = format!("{}.md", domain.replace('_', "-"));
        let filepath = output_dir.join(&filename);

        let mut file = fs::File::create(&filepath).expect("无法创建文档文件");
        file.write_all(doc_content.as_bytes()).expect("写入文档失败");

        println!("✅ 已生成: {}", filepath.display());
    }

    // 生成索引文档
    let index_content = generate_index_document(&by_domain, commands.len());
    let index_path = output_dir.join("README.md");
    let mut index_file = fs::File::create(&index_path).expect("无法创建索引文件");
    index_file.write_all(index_content.as_bytes()).expect("写入索引失败");

    println!("✅ 已生成: {}", index_path.display());
    println!("📊 共生成 {} 个命令的文档，分布在 {} 个域中", commands.len(), by_domain.len());
}

/// 生成单个域的文档
fn generate_domain_document(
    domain: &str,
    commands: &[&agent_command_types::CommandMetadata],
) -> String {
    let mut doc = String::new();

    // 标题
    doc.push_str(&format!("# {} 命令\n\n", get_domain_display_name(domain)));
    doc.push_str(&format!("> 域标识符: `{}`\n\n", domain));
    doc.push_str(&format!("> 命令数量: {}\n\n", commands.len()));

    // 安全级别统计
    let safe_count =
        commands.iter().filter(|c| c.safety == agent_command_types::CommandSafety::Safe).count();
    let caution_count =
        commands.iter().filter(|c| c.safety == agent_command_types::CommandSafety::Caution).count();
    let dangerous_count = commands
        .iter()
        .filter(|c| c.safety == agent_command_types::CommandSafety::Dangerous)
        .count();

    doc.push_str("## 安全级别分布\n\n");
    doc.push_str("| 级别 | 数量 | 说明 |\n");
    doc.push_str("|------|------|------|\n");
    doc.push_str(&format!("| ✓ Safe | {} | 只读操作，无需确认 |\n", safe_count));
    doc.push_str(&format!("| ⚠ Caution | {} | 写入操作，需用户确认 |\n", caution_count));
    doc.push_str(&format!("| ✗ Dangerous | {} | 危险操作，需显式授权 |\n\n", dangerous_count));

    // 命令列表
    doc.push_str("## 命令列表\n\n");
    doc.push_str("| 命令名 | 描述 | 安全级别 | 调用模式 |\n");
    doc.push_str("|--------|------|----------|----------|\n");

    for cmd in commands {
        let safety_icon = match cmd.safety {
            agent_command_types::CommandSafety::Safe => "✓",
            agent_command_types::CommandSafety::Caution => "⚠",
            agent_command_types::CommandSafety::Dangerous => "✗",
        };

        let call_mode_display = match cmd.call_mode {
            agent_command_types::CallMode::StateOnly => "StateOnly (仅状态)",
            agent_command_types::CallMode::StateInput => "StateInput (状态+输入)",
            agent_command_types::CallMode::Manual => "Manual (手动)",
        };

        doc.push_str(&format!(
            "| `{}` | {} | {} {} | {} |\n",
            cmd.name,
            escape_markdown(cmd.description),
            safety_icon,
            cmd.safety.as_str(),
            call_mode_display
        ));
    }

    doc.push_str("\n## 详细说明\n\n");

    // 每个命令的详细说明
    for cmd in commands {
        doc.push_str(&format!("### `{}`\n\n", cmd.name));
        doc.push_str(&format!("- **描述**: {}\n", cmd.description));
        doc.push_str(&format!("- **源模块**: `{}`\n", cmd.source_module));
        doc.push_str(&format!("- **完整路径**: `{}`\n", cmd.full_path()));
        doc.push_str(&format!(
            "- **安全级别**: {} (`{}`)\n",
            get_safety_display_name(cmd.safety),
            cmd.safety.as_str()
        ));
        doc.push_str(&format!("- **调用模式**: {}\n\n", get_call_mode_display_name(cmd.call_mode)));
    }

    doc
}

/// 生成索引文档
fn generate_index_document(
    by_domain: &[(&str, Vec<&agent_command_types::CommandMetadata>)],
    total_count: usize,
) -> String {
    let mut doc = String::new();

    doc.push_str("# AxAgent 命令文档\n\n");
    doc.push_str(&format!("> 基于 `#[agent_command]` 宏自动生成\n\n"));
    doc.push_str(&format!("> **总命令数**: {}\n\n", total_count));
    doc.push_str(&format!("> **命令域数**: {}\n\n", by_domain.len()));

    doc.push_str("## 命令域索引\n\n");
    doc.push_str("| 域 | 显示名 | 命令数 | 文档链接 |\n");
    doc.push_str("|----|--------|--------|----------|\n");

    for (domain, commands) in by_domain {
        let display_name = get_domain_display_name(domain);
        let filename = domain.replace('_', "-");
        doc.push_str(&format!(
            "| `{}` | {} | {} | [查看]({}.md) |\n",
            domain,
            display_name,
            commands.len(),
            filename
        ));
    }

    doc.push_str("\n## 使用说明\n\n");
    doc.push_str("### Agent 调用方式\n\n");
    doc.push_str("所有命令通过 `execute_tauri_command` 工具调用：\n\n");
    doc.push_str("```json\n{\n");
    doc.push_str("  \"command\": \"命令名称\",\n");
    doc.push_str("  \"args\": {\n");
    doc.push_str("    // 命令参数\n");
    doc.push_str("  }\n");
    doc.push_str("}\n");
    doc.push_str("```\n\n");

    doc.push_str("### 安全级别说明\n\n");
    doc.push_str("- **✓ Safe**: 只读操作，Agent 可直接调用，无需用户确认\n");
    doc.push_str("- **⚠ Caution**: 写入操作，系统会返回确认请求，需要用户明确确认后执行\n");
    doc.push_str("- **✗ Dangerous**: 危险操作，始终需要显式授权，默认情况下被阻止执行\n\n");

    doc.push_str("### 域映射说明\n\n");
    doc.push_str("命令域与工具域的映射关系由 `DomainMappingConfig` 控制。\n");
    doc.push_str("当 Agent 激活某个工具域时，该工具域映射的所有命令域的命令将对 Agent 可见。\n\n");

    doc
}

/// 获取域的显示名称
fn get_domain_display_name(domain: &str) -> &'static str {
    match domain {
        "core" => "核心命令",
        "knowledge" => "知识库",
        "workflow" => "工作流",
        "provider" => "LLM 提供商",
        "gateway" => "API 网关",
        "mcp" => "MCP 服务器",
        "skill" => "技能系统",
        "conversation" => "会话管理",
        "message" => "消息管理",
        "memory" => "记忆系统",
        "settings" => "应用设置",
        "invest" => "投资分析",
        "opc" => "一人公司运营",
        "quant" => "量化回测",
        "market_sim" => "市场模拟",
        "portfolio" => "投资组合",
        _ => "其他",
    }
}

/// 获取安全级别的显示名称
fn get_safety_display_name(safety: agent_command_types::CommandSafety) -> &'static str {
    match safety {
        agent_command_types::CommandSafety::Safe => "✓ Safe (安全)",
        agent_command_types::CommandSafety::Caution => "⚠ Caution (需确认)",
        agent_command_types::CommandSafety::Dangerous => "✗ Dangerous (危险)",
    }
}

/// 获取调用模式的显示名称
fn get_call_mode_display_name(call_mode: agent_command_types::CallMode) -> &'static str {
    match call_mode {
        agent_command_types::CallMode::StateOnly => "StateOnly — 仅使用应用状态，无需额外输入",
        agent_command_types::CallMode::StateInput => "StateInput — 使用应用状态和用户输入参数",
        agent_command_types::CallMode::Manual => "Manual — 需要专用 Handler 手动处理",
    }
}

/// 转义 Markdown 特殊字符
fn escape_markdown(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ").replace('\r', "")
}
