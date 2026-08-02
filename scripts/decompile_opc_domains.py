#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
反编译 seed_extended.rs 的 17 个领域函数（75 个工作流）→ 领域数据包。

用法:
    python scripts/decompile_opc_domains.py <seed_extended.rs 路径> <输出根目录>

输出:
    {out_root}/{domain}/manifest.yaml
    {out_root}/{domain}/workflows/{workflow_id}.yaml

与行业包（decompile_opc_industries.py）同 schema，独立目录 config/opc/domains/。
"""
import re
import sys
from pathlib import Path

# 领域函数名 → 领域 id（seed_extended.rs 的 17 个领域函数）
DOMAIN_FNS = [
    "seed_engineering", "seed_marketing", "seed_specialized", "seed_sales",
    "seed_design", "seed_testing", "seed_finance", "seed_security",
    "seed_support", "seed_product", "seed_pm", "seed_academic",
    "seed_gis", "seed_gamedev", "seed_paidmedia", "seed_spatial",
    "seed_strategy",
]

DOMAIN_NAMES = {
    "seed_engineering": "工程研发",
    "seed_marketing": "市场营销",
    "seed_specialized": "专业服务",
    "seed_sales": "销售增长",
    "seed_design": "设计",
    "seed_testing": "质量测试",
    "seed_finance": "财务金融",
    "seed_security": "安全",
    "seed_support": "客户支持",
    "seed_product": "产品",
    "seed_pm": "项目管理",
    "seed_academic": "学术研究",
    "seed_gis": "GIS 地理信息",
    "seed_gamedev": "游戏开发",
    "seed_paidmedia": "付费投放",
    "seed_spatial": "空间计算",
    "seed_strategy": "战略咨询",
}


def split_functions(src: str):
    """按函数名切分 17 个领域函数体。"""
    fn_pattern = re.compile(
        r'async fn (seed_\w+)\(db: &DatabaseConnection\) -> Result<\(\)\, String> \{(.*?)\n\}',
        re.S,
    )
    return {m.group(1): m.group(2) for m in fn_pattern.finditer(src)}


def extract_wf_calls(body: str):
    """提取函数体内全部 wf!(...) 调用（括号配平，深度从 wf!( 的 ( 起算）。"""
    out = []
    i = 0
    while True:
        j = body.find("wf!(", i)
        if j == -1:
            break
        depth = 1  # wf!( 的 ( 已计入
        k = j + 4
        while k < len(body):
            if body[k] == "(":
                depth += 1
            elif body[k] == ")":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        out.append(body[j:k + 1])
        i = k + 1
    return out


def parse_wf_call(call: str):
    """解析 wf! 调用 → dict(id, name, desc, icon, profile, steps)。"""
    # 提取 6 个顶层字符串参数 + steps 元组数组
    m = re.match(
        r'wf!\(\s*db,\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*\[(.*?)\]\)',
        call, re.S,
    )
    if not m:
        return None
    wf_id, name, desc, icon, profile, steps_raw = m.groups()
    steps = []
    for sm in re.finditer(
        r'\(\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*([\d.]+),\s*([\d.]+)\s*\)',
        steps_raw, re.S,
    ):
        steps.append({
            "id": sm.group(1),
            "title": sm.group(2),
            "prompt": sm.group(3),
            "x": float(sm.group(4)),
            "y": float(sm.group(5)),
        })
    return {
        "id": wf_id,
        "name": name,
        "description": desc,
        "icon": icon,
        "profile_id": profile,
        "steps": steps,
    }


def dump_manifest(domain_id: str, domain_name: str, wf_count: int) -> str:
    return (
        f"# Domain Pack: {domain_name}（{domain_id}）\n"
        "# 领域 = 数据资产包，非代码。与行业包同 schema，独立目录。\n"
        f"id: {domain_id}\n"
        f"name: {domain_name}\n"
        "icon: 🧩\n"
        f"description: {domain_name}领域通用工作流（{wf_count} 个）\n"
        "version: 1\n"
        "enabled: true\n"
    )


def yaml_str(s: str) -> str:
    """YAML 双引号转义（值含 : / 引号 / 特殊字符时必须加引号）。"""
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def dump_workflow(data: dict) -> str:
    lines = [
        f"# 工作流：{data['name']}",
        f"id: {data['id']}",
        f"name: {yaml_str(data['name'])}",
        f"description: {yaml_str(data['description'])}",
        f'icon: "{data["icon"]}"',
        "tags: [opc]",
        f"profile_id: {data['profile_id']}",
        "steps:",
    ]
    for s in data["steps"]:
        lines += [
            f"  - id: {s['id']}",
            f"    title: {yaml_str(s['title'])}",
            f"    prompt: {yaml_str(s['prompt'])}",
            "    inputs: {}",
        ]
    return "\n".join(lines)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    src_path = Path(sys.argv[1])
    out_root = Path(sys.argv[2])
    src = src_path.read_text(encoding="utf-8")

    fns = split_functions(src)
    total = 0
    for fn_name in DOMAIN_FNS:
        if fn_name not in fns:
            print(f"⚠️ 缺少函数 {fn_name}")
            continue
        body = fns[fn_name]
        wfs = [parse_wf_call(c) for c in extract_wf_calls(body)]
        wfs = [w for w in wfs if w]
        if not wfs:
            print(f"⚠️ {fn_name}: 0 个工作流")
            continue

        domain_id = fn_name.removeprefix("seed_")
        domain_name = DOMAIN_NAMES.get(fn_name, domain_id)
        domain_dir = out_root / domain_id
        wf_dir = domain_dir / "workflows"
        wf_dir.mkdir(parents=True, exist_ok=True)

        (domain_dir / "manifest.yaml").write_text(
            dump_manifest(domain_id, domain_name, len(wfs)), encoding="utf-8"
        )
        for w in wfs:
            (wf_dir / f"{w['id']}.yaml").write_text(
                dump_workflow(w), encoding="utf-8"
            )
        total += len(wfs)
        print(f"✅ {domain_id}: {domain_name}（{len(wfs)} 个工作流）")

    print(f"\n总计 {total} 个工作流 → {out_root}")


if __name__ == "__main__":
    main()
