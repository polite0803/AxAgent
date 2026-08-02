#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
反编译 seed_industries.rs 的 9 个行业函数 → Industry Pack yaml 数据资产包。

用法:
    python scripts/decompile_opc_industries.py <seed_industries.rs 路径> <输出根目录>

按 "// ═══" 分隔符切分 9 个行业函数块（文件内注释明确标注了
"1. AI科技" ~ "9. 教育培训"），逐块解析为:
    {out_root}/{industry_id}/manifest.yaml
    {out_root}/{industry_id}/workflows/{workflow_id}.yaml
"""
import re
import sys
from pathlib import Path

# 行业顺序（与 seed_industries.rs 中 ═══ 分隔块顺序一致）
INDUSTRY_ORDER = [
    ("ai_research", "AI 科技与研究"),
    ("software_dev", "软件开发"),
    ("finance_invest", "金融投资"),
    ("sales_growth", "销售增长"),
    ("content_media", "内容与媒体"),
    ("industry_consulting", "行业咨询"),
    ("accounting", "会计财务"),
    ("ecommerce", "品牌电商"),
    ("education", "教育培训"),
]


def split_industry_blocks(src: str):
    """按函数名直接定位全部 seed_xxx 函数体（非贪婪 + \n} 结尾，边界准确）。"""
    fn_pattern = re.compile(
        r'async fn (seed_\w+)\(db: &DatabaseConnection\) -> Result<\(\)\, String> \{(.*?)\n\}',
        re.S,
    )
    blocks = [(m.group(1), m.group(2)) for m in fn_pattern.finditer(src)]
    # 去掉入口函数 seed_industry_workflows（无行业内容）
    return [(n, b) for n, b in blocks if n != 'seed_industry_workflows']


def parse_block(body: str, industry_id: str):
    """解析单个行业函数体，返回 workflow dict。"""
    # id
    id_m = re.search(r'let id = "([^"]+)"', body)
    wf_id = id_m.group(1) if id_m else f"workflow-{industry_id}"

    # name / description / icon / tags
    name_m = re.search(r'name: "([^"]+)"\.into\(\)', body)
    desc_m = re.search(r'description: Some\("([^"]*)"\.into\(\)', body)
    icon_m = re.search(r'icon: "([^"]+)"\.into\(\)', body)
    tags_m = re.search(r'tags: vec!\[([^\]]+)\]', body)
    tags = []
    if tags_m:
        # 提取 "xxx".into() 或 "xxx" 形式的 tag
        tags = re.findall(r'"([^"]+)"', tags_m.group(1))

    # 所有节点（agent + approval），按源码位置排序
    all_nodes = []

    # agent_node("id", "title", "desc", "profile", "prompt", x, y, "output"[, input_mapping])
    agent_re = re.compile(
        r'agent_node(?:_with_input)?\(\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*'
        r'"([^"]+)",\s*(\d+\.?\d*),\s*(\d+\.?\d*),\s*"([^"]+)"'
        r'(?:,\s*\[\s*(.*?)\s*\]\s*\.into\(\))?\)',
        re.S,
    )
    for m in agent_re.finditer(body):
        g = m.groups()
        node_id, title, _desc, profile, prompt = g[0], g[1], g[2], g[3], g[4]
        inputs = {}
        if g[8]:
            for kv in re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', g[8]):
                inputs[kv[0]] = kv[1]
        all_nodes.append((m.start(), {
            "id": node_id, "title": title, "prompt": prompt, "inputs": inputs,
        }))

    approval_re = re.compile(
        r'WorkflowNode::Approval\(ApprovalNode \{\s*base: make_base\("([^"]+)", "([^"]+)"',
        re.S,
    )
    for m in approval_re.finditer(body):
        all_nodes.append((m.start(), {
            "id": m.group(1), "title": m.group(2),
            "node_type": "approval",
            "approval": {
                "message": "请审批。24小时超时自动拒绝。",
                "approver": "manager",
                "timeout_secs": 86400,
                "timeout_action": "auto_reject",
            },
        }))

    all_nodes.sort(key=lambda t: t[0])
    ordered = [n for _, n in all_nodes]

    # profile_id 取第一个 agent 的
    profile_id = None
    first_agent = agent_re.search(body)
    if first_agent:
        profile_id = first_agent.group(4)

    return {
        "id": wf_id,
        "name": name_m.group(1) if name_m else wf_id,
        "description": desc_m.group(1) if desc_m else "",
        "icon": icon_m.group(1) if icon_m else "📄",
        "tags": tags,
        "profile_id": profile_id or "opc-ceo-ceo-business-strategist",
        "steps": ordered,
    }


def dump_manifest(data: dict, industry_name: str, industry_id: str) -> str:
    return (
        f"# Industry Pack: {industry_name}（{industry_id}）\n"
        "# 行业 = 数据资产包，非代码。manifest 版本号驱动 seed/升级。\n"
        f"id: {industry_id}\n"
        f"name: {data['name']}\n"
        f"icon: {data['icon']}\n"
        f"description: {data['description']}\n"
        "version: 1\n"
        "enabled: true\n"
    )


def dump_workflow(data: dict) -> str:
    lines = [
        f"# 工作流：{data['name']}",
        f"id: {data['id']}",
        f"name: {data['name']}",
        f"description: {data['description']}",
        f"icon: {data['icon']}",
        f"tags: [{', '.join(data['tags'])}]",
        f"profile_id: {data['profile_id']}",
        "steps:",
    ]
    for s in data["steps"]:
        if s.get("node_type") == "approval":
            lines += [
                f"  - id: {s['id']}",
                f"    title: {s['title']}",
                "    node_type: approval",
                "    approval:",
                f"      message: {s['approval']['message']}",
                f"      approver: {s['approval']['approver']}",
                f"      timeout_secs: {s['approval']['timeout_secs']}",
                f"      timeout_action: {s['approval']['timeout_action']}",
                "    inputs: {}",
            ]
        else:
            lines += [
                f"  - id: {s['id']}",
                f"    title: {s['title']}",
                f"    prompt: {s['prompt']}",
            ]
            if s.get("inputs"):
                lines.append("    inputs:")
                for k, v in s["inputs"].items():
                    lines.append(f"      {k}: {v}")
            else:
                lines.append("    inputs: {}")
    return "\n".join(lines)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    src_path = Path(sys.argv[1])
    out_root = Path(sys.argv[2])
    src = src_path.read_text(encoding="utf-8")

    blocks = split_industry_blocks(src)
    if len(blocks) < len(INDUSTRY_ORDER):
        print(f"⚠️ 只切分出 {len(blocks)} 个函数块（期望 {len(INDUSTRY_ORDER)}），检查分隔符")
        for name, _ in blocks:
            print(f"  - {name}")

    for i, (industry_id, industry_name) in enumerate(INDUSTRY_ORDER):
        if i >= len(blocks):
            print(f"⚠️ {industry_id}: 无对应函数块")
            continue
        fn_name, body = blocks[i]
        data = parse_block(body, industry_id)
        if not data["steps"]:
            print(f"⚠️ {industry_id}（{fn_name}）: 0 steps，可能解析失败")

        industry_dir = out_root / industry_id
        wf_dir = industry_dir / "workflows"
        wf_dir.mkdir(parents=True, exist_ok=True)
        (industry_dir / "manifest.yaml").write_text(
            dump_manifest(data, industry_name, industry_id), encoding="utf-8"
        )
        (wf_dir / f"{industry_id}.yaml").write_text(
            dump_workflow(data), encoding="utf-8"
        )
        print(f"✅ {industry_id}: {data['name']}（{len(data['steps'])} steps, profile={data['profile_id']}）")


if __name__ == "__main__":
    main()
