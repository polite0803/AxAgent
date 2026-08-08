#!/usr/bin/env python3
"""将 config/opc/domains/ 下的所有 YAML 工作流转换为 Rust 代码。

从 Git 历史中恢复 YAML 文件，解析后生成 domain/mod.rs 的完整实现。
保留所有关键字段：tools, inputs, agent, condition, user_input 等。
"""

import subprocess
import sys
import os
import re
from pathlib import Path

try:
    import yaml
except ImportError:
    print("需要 pyyaml：pip install pyyaml")
    sys.exit(1)

# ── 配置 ──────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).resolve().parent.parent
GIT_BASE = "HEAD"
YAML_PREFIX = "config/opc/domains"
OUTPUT_FILE = REPO_ROOT / "src-tauri" / "crates" / "analysis-engine" / "src" / "opc" / "domain" / "generated.rs"

# 17 个领域及其目录名
DOMAINS = [
    ("academic", "学术研究"),
    ("design", "设计与创意"),
    ("engineering", "工程与开发"),
    ("finance", "财务与会计"),
    ("gamedev", "游戏开发"),
    ("gis", "地理信息系统"),
    ("marketing", "市场营销"),
    ("paidmedia", "付费媒体"),
    ("pm", "项目管理"),
    ("product", "产品管理"),
    ("sales", "销售与商务"),
    ("security", "安全与合规"),
    ("spatial", "空间计算"),
    ("specialized", "专业服务"),
    ("strategy", "战略规划"),
    ("support", "客户支持"),
    ("testing", "测试与质量"),
]


def git_show(path: str) -> str:
    """从 Git 历史中读取文件内容。"""
    result = subprocess.run(
        ["git", "show", f"{GIT_BASE}:{path}"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode != 0:
        raise FileNotFoundError(f"Git 中找不到文件: {path}\n{result.stderr}")
    return result.stdout


def rust_escape_str(s: str) -> str:
    """将字符串转为 Rust 转义字符串。"""
    escaped = s.replace("\\", "\\\\")
    escaped = escaped.replace('"', '\\"')
    escaped = escaped.replace("\n", "\\n")
    escaped = escaped.replace("\r", "\\r")
    escaped = escaped.replace("\t", "\\t")
    return f'"{escaped}"'


def escape_for_rust(s: str) -> str:
    """转义字符串以用于 Rust 字符串字面量。"""
    return s.replace("\\", "\\\\").replace('"', '\\"')


def format_prompt_rust(s: str, indent: int = 20) -> str:
    """格式化 prompt 字符串为 Rust 代码中的多行字符串（使用 concat! 宏连接）。"""
    prefix = " " * indent
    lines = s.split("\n")
    # 过滤末尾空行
    while lines and not lines[-1].strip():
        lines.pop()
    # 过滤开头空行
    while lines and not lines[0].strip():
        lines.pop(0)

    if not lines:
        return f'{prefix}""'

    # 使用 concat! 宏连接多行字符串
    parts = []
    for line in lines:
        stripped = line.rstrip()
        escaped = escape_for_rust(stripped)
        parts.append(f'"{escaped}\\n"')

    # 最后一行不加 \n
    if parts:
        # 修改最后一行，去掉 \n
        last = parts[-1]
        if last.endswith('\\n"'):
            parts[-1] = last[:-3] + '"'
    
    parts_str = ", ".join(parts)
    return f'{prefix}concat!({parts_str})'


def step_to_rust(step: dict, indent: int = 20) -> str:
    """将 YAML 步骤定义转换为 Rust DomainStepDef 构造代码。"""
    prefix = " " * indent
    step_id = step.get("id", "unknown")
    title = step.get("title", step_id)
    node_type = step.get("type", step.get("node_type", "agent"))

    lines = []

    # 步骤类型
    if node_type == "approval":
        lines.append(f'{prefix}DomainStepDef::approval("{step_id}", "{title}")')
    else:
        lines.append(f'{prefix}DomainStepDef::agent("{step_id}", "{title}")')

    # 1. Prompt（必填）
    prompt = step.get("prompt", "")
    if prompt:
        # 检查 prompt 是否包含 \n 字面量（需要展开为多行）
        if "\\n" in prompt or len(prompt) > 80:
            # 展开 \n 为实际换行
            prompt_expanded = prompt.replace("\\n", "\n")
            prompt_code = format_prompt_rust(prompt_expanded, indent + 8)
            lines.append(f"{prefix}    .with_prompt(")
            lines.append(prompt_code)
            lines.append(f"{prefix}    )")
        else:
            escaped = escape_for_rust(prompt)
            lines.append(f'{prefix}    .with_prompt("{escaped}")')

    # 2. Tools（可选）
    tools = step.get("tools", [])
    if tools:
        tools_str = ", ".join(f'"{t}".to_string()' for t in tools)
        lines.append(f"{prefix}    .with_tools(vec![{tools_str}])")

    # 3. Inputs（可选）
    inputs = step.get("inputs", {})
    if inputs and isinstance(inputs, dict) and len(inputs) > 0:
        lines.append(f"{prefix}    .with_inputs({{")
        lines.append(f"{prefix}        let mut m = HashMap::new();")
        for k, v in inputs.items():
            lines.append(f'{prefix}        m.insert("{k}".to_string(), "{v}".to_string());')
        lines.append(f"{prefix}        m")
        lines.append(f"{prefix}    }})")

    # 4. Condition（可选）
    condition = step.get("condition", "")
    if condition:
        escaped = escape_for_rust(condition)
        lines.append(f'{prefix}    .with_condition("{escaped}")')

    # 5. Agent（可选）
    agent = step.get("agent", {})
    if agent and isinstance(agent, dict) and agent.get("id"):
        agent_id = escape_for_rust(agent.get("id", ""))
        agent_role = escape_for_rust(agent.get("role", ""))
        lines.append(f'{prefix}    .with_agent(DomainAgentDef::new("{agent_id}", "{agent_role}"))')

    # 6. User Input（可选）
    user_input = step.get("user_input", {})
    if user_input and isinstance(user_input, dict) and user_input.get("enabled", False):
        lines.append(f"{prefix}    .with_user_input({{")
        lines.append(f"{prefix}        let mut ui = DomainUserInput::new();")
        ui_mode = user_input.get("mode", "approval_gate")
        lines.append(f'{prefix}        ui = ui.with_mode("{ui_mode}");')
        ui_prompt = user_input.get("prompt", "")
        if ui_prompt:
            escaped_ui_prompt = escape_for_rust(ui_prompt)
            lines.append(f'{prefix}        ui = ui.with_prompt("{escaped_ui_prompt}");')
        fields = user_input.get("fields", [])
        if fields and isinstance(fields, list) and len(fields) > 0:
            field_strs = []
            for field in fields:
                if isinstance(field, dict):
                    fname = escape_for_rust(field.get("name", ""))
                    ftype = escape_for_rust(field.get("type", "text"))
                    flabel = escape_for_rust(field.get("label", fname))
                    field_lines = [f'DomainUserInputField::new("{fname}", "{ftype}", "{flabel}")']
                    if field.get("options"):
                        opts = field["options"]
                        opts_str = ", ".join(f'"{escape_for_rust(o)}".to_string()' for o in opts)
                        field_lines.append(f'    .with_options(vec![{opts_str}])')
                    if field.get("required", False):
                        field_lines.append(f'    .with_required(true)')
                    if field.get("placeholder"):
                        ph = escape_for_rust(field["placeholder"])
                        field_lines.append(f'    .with_placeholder("{ph}")')
                    field_strs.append("".join(field_lines))
            fields_code = ", ".join(field_strs)
            lines.append(f"{prefix}        ui = ui.with_fields(vec![{fields_code}]);")
        lines.append(f"{prefix}        ui")
        lines.append(f"{prefix}    }})")

    # 7. Continue on fail（可选）
    if step.get("continue_on_fail", False):
        lines.append(f"{prefix}    .with_continue_on_fail(true)")

    # 8. On error（可选）
    on_error = step.get("on_error", "")
    if on_error:
        escaped = escape_for_rust(on_error)
        lines.append(f'{prefix}    .with_on_error("{escaped}")')

    return "\n".join(lines)


def workflow_to_rust(wf: dict, domain_id: str) -> str:
    """将 YAML 工作流定义转换为 Rust DomainWorkflowDef 构造代码。"""
    prefix = "            "  # 12 spaces for workflow body inside vec![]

    wf_id = wf.get("id", "unknown")
    name = wf.get("name", wf_id)
    description = wf.get("description", "")
    icon = wf.get("icon", "📄")
    tags = wf.get("tags", [])
    profile_id = wf.get("profile_id", "")
    steps = wf.get("steps", [])

    # 确保 tags 包含领域 ID
    if domain_id not in tags:
        tags.append(domain_id)
    if "opc" not in tags:
        tags.insert(0, "opc")

    lines = []
    lines.append(f'{prefix}DomainWorkflowDef::new("{wf_id}", "{name}")')

    if description:
        desc_escaped = escape_for_rust(description)
        lines.append(f'{prefix}    .with_description("{desc_escaped}")')

    if icon:
        icon_escaped = escape_for_rust(icon)
        lines.append(f'{prefix}    .with_icon("{icon_escaped}")')

    if tags:
        tags_str = ", ".join(f'"{t}".to_string()' for t in tags)
        lines.append(f'{prefix}    .with_tags(vec![{tags_str}])')

    if profile_id:
        lines.append(f'{prefix}    .with_profile_id("{profile_id}")')

    if steps:
        lines.append(f'{prefix}    .with_steps(vec![')
        for step in steps:
            step_code = step_to_rust(step, indent=20)
            lines.append(step_code)
            lines.append(f"{prefix}        ,")
        lines.append(f'{prefix}    ])')

    return "\n".join(lines)


def process_domain(domain_id: str, domain_name: str, all_files: list[str]) -> tuple[str, list[str]]:
    """处理单个领域，返回 (domain_id, [workflow Rust 代码列表])。"""
    # 从全量文件列表中过滤
    prefix = f"{YAML_PREFIX}/{domain_id}/workflows/"
    yaml_files = sorted([f for f in all_files if f.startswith(prefix) and f.endswith((".yaml", ".yml"))])

    if not yaml_files:
        print(f"  ⚠️  领域 {domain_id} 无工作流文件")
        return (domain_id, [])

    print(f"  📂 领域 {domain_id} ({domain_name}): {len(yaml_files)} 个工作流")

    workflows = []
    for yaml_path in yaml_files:
        try:
            yaml_content = git_show(yaml_path)
            wf_data = yaml.safe_load(yaml_content)
            if not wf_data:
                print(f"    ⚠️  空文件: {yaml_path}")
                continue

            wf_id = wf_data.get("id", yaml_path.split("/")[-1].replace(".yaml", ""))
            steps = wf_data.get("steps", [])

            # 统计步骤中的特性
            has_tools = sum(1 for s in steps if s.get("tools"))
            has_inputs = sum(1 for s in steps if s.get("inputs"))
            has_approval = sum(1 for s in steps if s.get("type") == "approval" or s.get("node_type") == "approval")
            has_condition = sum(1 for s in steps if s.get("condition"))
            has_agent = sum(1 for s in steps if s.get("agent"))

            print(f"    ✅ {wf_id}: {len(steps)} 步骤 (tools={has_tools}, inputs={has_inputs}, approvals={has_approval}, conditions={has_condition}, agents={has_agent})")

            rust_code = workflow_to_rust(wf_data, domain_id)
            workflows.append(rust_code)

        except Exception as e:
            print(f"    ❌ 错误处理 {yaml_path}: {e}")
            import traceback
            traceback.print_exc()
            continue

    return (domain_id, workflows)


def generate_rust_file(domains_data: dict[str, list[str]]) -> str:
    """生成完整的 Rust 源文件。"""
    lines = []

    lines.append("// Auto-generated from YAML via convert_yaml_to_rust.py")
    lines.append("// DO NOT EDIT MANUALLY — edit the YAML source and re-run the converter")
    lines.append("//")
    lines.append("// 注意：本文件通过 include! 引入到 mod.rs，")
    lines.append("// 所有类型导入已在 mod.rs 中完成，请勿在此添加 use 语句。")
    lines.append("")
    lines.append("impl DomainAdapterFactory {")

    # 为每个领域生成方法
    for domain_id, domain_name in DOMAINS:
        wfs = domains_data.get(domain_id, [])
        lines.append("")
        lines.append(f"    /// {domain_name} ({domain_id}) — {len(wfs)} 个工作流")
        lines.append(f"    pub fn {domain_id}() -> Vec<DomainWorkflowDef> {{")
        lines.append(f"        vec![")
        for i, wf_code in enumerate(wfs):
            if i < len(wfs) - 1:
                # 非最后一个工作流，末尾加逗号
                wf_code_with_comma = wf_code.rstrip() + ","
                lines.append(wf_code_with_comma)
            else:
                lines.append(wf_code)
        lines.append(f"        ]")
        lines.append(f"    }}")

    # create() 方法
    lines.append("")
    lines.append("    pub fn create(id: &str) -> Option<DomainWorkflowDef> {")
    lines.append("        match id.trim() {")
    for domain_id, _ in DOMAINS:
        lines.append(f'            "{domain_id}" => Self::{domain_id}().into_iter().next(),')
    lines.append("            _ => None,")
    lines.append("        }")
    lines.append("    }")

    # list_all() 方法
    lines.append("")
    lines.append("    pub fn list_all() -> Vec<(&'static str, &'static str)> {")
    lines.append("        vec![")
    for domain_id, domain_name in DOMAINS:
        lines.append(f'            ("{domain_id}", "{domain_name}"),')
    lines.append("        ]")
    lines.append("    }")

    # create_all() 方法
    lines.append("")
    lines.append("    pub fn create_all() -> Vec<DomainWorkflowDef> {")
    lines.append("        let mut all = Vec::new();")
    for domain_id, _ in DOMAINS:
        lines.append(f"        all.extend(Self::{domain_id}());")
    lines.append("        all")
    lines.append("    }")

    lines.append("}")

    return "\n".join(lines)


def main():
    print("=" * 60)
    print("OPC Domain YAML → Rust 转换器")
    print("=" * 60)

    # 先获取所有 YAML 文件列表
    print("\n📋 获取全量文件列表...")
    result = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", GIT_BASE, YAML_PREFIX],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    all_yaml_files = [
        f for f in result.stdout.strip().split("\n")
        if f.endswith((".yaml", ".yml")) and "/workflows/" in f
    ]
    print(f"   Git 中共有 {len(all_yaml_files)} 个工作流 YAML 文件")

    domains_data = {}
    total_workflows = 0

    for domain_id, domain_name in DOMAINS:
        did, wfs = process_domain(domain_id, domain_name, all_yaml_files)
        domains_data[did] = wfs
        total_workflows += len(wfs)

    print(f"\n{'=' * 60}")
    print(f"汇总：")
    print(f"  领域数: {len(domains_data)}")
    print(f"  工作流数: {total_workflows}")
    print(f"{'=' * 60}")

    # 生成 Rust 文件
    rust_code = generate_rust_file(domains_data)

    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_FILE.write_text(rust_code, encoding="utf-8")

    print(f"\n📝 已生成: {OUTPUT_FILE}")
    print(f"   大小: {len(rust_code)} 字符, {rust_code.count(chr(10))} 行")

    # 验证
    print(f"\n🔍 验证步骤...")

    # 检查是否所有领域都有数据
    missing = [did for did, wfs in domains_data.items() if not wfs]
    if missing:
        print(f"  ⚠️  无工作流的领域: {missing}")
    else:
        print(f"  ✅ 所有 {len(domains_data)} 个领域均有工作流定义")

    # 检查关键特性
    tool_matches = rust_code.count(".with_tools(")
    input_matches = rust_code.count(".with_inputs(")
    cond_matches = rust_code.count(".with_condition(")
    approval_matches = rust_code.count("DomainStepDef::approval")

    print(f"  ✅ 工具绑定: {tool_matches} 处")
    print(f"  ✅ 输入映射: {input_matches} 处")
    print(f"  ✅ 条件分支: {cond_matches} 处")
    print(f"  ✅ 审批节点: {approval_matches} 处")

    if tool_matches == 0 or input_matches == 0:
        print(f"\n  ⚠️ 警告: 部分关键字段缺失，请检查 YAML 源文件")

    print("\n完成！现在需要将生成的 generated.rs 内容合并到 domain/mod.rs 中。")


if __name__ == "__main__":
    main()