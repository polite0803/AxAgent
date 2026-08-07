//! seed 工作流模板工具声明 ↔ 运行时解析空间 一致性校验。
//!
//! 检测面：seed_stock_analysis.rs / seed_serenity.rs 里 ToolDef.name 声明的每个工具，
//! 必须能命中 ToolResolver 的解析空间：
//!
//! ```text
//! 全局 registry（tools::tools::register_all） ∪ stock_mcp_tools schema 清单
//!   ∪ industry_chain（stock-analysis） ∪ RhaiToolDef（workflow 内部 rhai 工具）
//! ```
//!
//! 若命中不了 → 运行时 ToolResolver 返回 None → 工具调用被 core.rs Failed 分支
//! `emit degraded: true` **静默吞掉**（节点标记 completed 但结果为空，无报错）。
//! 这是最隐蔽的故障形态，本测试用源码级提取把失联工具暴露出来。
//!
//! 提取规则说明（源码级，经实锤）：
//! - `name: "..."` 且下方 3 行内出现 `var_type:` → 是 Variable.name（模板变量），非工具，跳过；
//! - `name: "..."` 其余 → ToolDef.name（工具声明）；
//! - `tool_name: "..."` 字面量 → RhaiToolDef（workflow 内部 rhai 工具，走 code_executor，
//!   不经过 ToolResolver，计入可解析集合）。

use std::collections::HashSet;

/// 从 seed 源文件提取 (工具声明集, rhai 工具名集)。
fn seed_tool_def_names(seed_rs: &str) -> (Vec<String>, Vec<String>) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/commands/stock_analysis_setup")
        .join(seed_rs);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {seed_rs} 失败: {e}"));
    let lines: Vec<&str> = src.lines().collect();
    let mut declared: Vec<String> = Vec::new();
    let mut rhai: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(start) = line.find("name: \"") {
            let rest = &line[start + 7..];
            if let Some(end) = rest.find('"') {
                let name = rest[..end].to_string();
                // 下方 3 行内出现 var_type: → 模板变量，跳过
                let is_var = lines.iter().skip(i + 1).take(3).any(|l| l.contains("var_type:"));
                if !is_var {
                    declared.push(name);
                }
            }
        }
        // RhaiToolDef：`tool_name: "..."` 字面量（"tool_name: \"" 共 12 字符）
        if let Some(start) = line.find("tool_name: \"") {
            let rest = &line[start + 12..];
            if let Some(end) = rest.find('"') {
                let name = rest[..end].to_string();
                if !rhai.contains(&name) {
                    rhai.push(name);
                }
            }
        }
    }
    (declared, rhai)
}

/// ToolResolver 解析空间 = 全局 registry ∪ stock schema 清单 ∪ G3 产业链 ∪ rhai 工具。
fn resolvable_tool_names(rhai: &[String]) -> HashSet<String> {
    let mut set = HashSet::new();
    // 全局 registry（tools crate 全部内置工具，与 init/services.rs ToolResolver 一致）
    let mut registry = axagent_tools::registry::ToolRegistry::new();
    axagent_tools::tools::register_all(&mut registry);
    set.extend(registry.list_all().into_iter().map(|t| t.name));
    // astock-data 股票工具 schema 清单（STOCK_TOOL_NAMES 来源之一）
    set.extend(
        axagent_astock_data::mcp_tools::stock_mcp_tools()
            .into_iter()
            .filter_map(|t| t.get("name").and_then(|v| v.as_str()).map(String::from)),
    );
    // G3 产业链工具（STOCK_TOOL_NAMES 来源之二）
    set.extend(
        axagent_analysis_engine::mcp_tools::industry_chain_mcp_tools()
            .into_iter()
            .filter_map(|t| t.get("name").and_then(|v| v.as_str()).map(String::from)),
    );
    // workflow 内部 rhai 工具（code_executor 执行，不经 ToolResolver）
    set.extend(rhai.iter().cloned());
    set
}

fn assert_all_resolvable(seed_rs: &str) {
    let (declared, rhai) = seed_tool_def_names(seed_rs);
    let resolvable = resolvable_tool_names(&rhai);
    let missing: Vec<String> =
        declared.iter().filter(|n| !resolvable.contains(*n)).cloned().collect();
    assert!(
        missing.is_empty(),
        "[{seed_rs}] 声明 {} 个工具，其中 {} 个不在 ToolResolver 解析空间:\n  {:?}\n\
         这些工具运行时调用会解析为 None，被 degraded 机制静默吞掉（节点 completed 但结果为空）。\n\
         rhai 工具（可解析）: {:?}",
        declared.len(),
        missing.len(),
        missing,
        rhai
    );
}

#[test]
fn seed_stock_analysis_tools_all_resolvable() {
    assert_all_resolvable("seed_stock_analysis.rs");
}

#[test]
fn seed_serenity_tools_all_resolvable() {
    assert_all_resolvable("seed_serenity.rs");
}
