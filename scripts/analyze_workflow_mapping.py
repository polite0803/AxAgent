"""分析 Rust 工作流与行业页面的映射关系。"""
import re
from pathlib import Path

# 读取 generated.rs
GENERATED_RS = Path(__file__).resolve().parent.parent / "src-tauri" / "crates" / "analysis-engine" / "src" / "opc" / "domain" / "generated.rs"

with open(GENERATED_RS, "r", encoding="utf-8") as f:
    content = f.read()

# 提取所有 DomainWorkflowDef 的 ID 和 tags
# 使用更简单的方法：逐行解析
lines = content.split("\n")

workflows = []
current_wf = None
in_tags = False
tags_str = ""

for line in lines:
    # 开始一个新的 DomainWorkflowDef
    if "DomainWorkflowDef::new(" in line:
        # 保存之前的
        if current_wf:
            current_wf["tags"] = re.findall(r'"([^"]+)"', tags_str)
            workflows.append(current_wf)
        
        # 提取 ID 和名称
        match = re.search(r'DomainWorkflowDef::new\("([^"]+)",\s*"([^"]+)"\)', line)
        if match:
            wf_id = match.group(1)
            wf_name = match.group(2)
            current_wf = {"id": wf_id, "name": wf_name, "tags": []}
            tags_str = ""
            in_tags = False
    
    # 检查是否在 with_tags
    if ".with_tags(" in line and current_wf:
        in_tags = True
        tags_str = line
        if "]" in line:
            # 单行完成
            current_wf["tags"] = re.findall(r'"([^"]+)"', line)
            workflows.append(current_wf)
            current_wf = None
            in_tags = False
    
    # 多行 tags
    if in_tags and current_wf:
        tags_str += line
        if "]" in line:
            current_wf["tags"] = re.findall(r'"([^"]+)"', tags_str)
            workflows.append(current_wf)
            current_wf = None
            in_tags = False

# 保存最后一个
if current_wf:
    current_wf["tags"] = re.findall(r'"([^"]+)"', tags_str)
    workflows.append(current_wf)

# 从 ID 提取 domain
for wf in workflows:
    parts = wf["id"].split("-")
    wf["domain"] = parts[1] if len(parts) > 1 else "unknown"

# 按 domain 分组
domain_map = {}
for wf in workflows:
    domain = wf["domain"]
    if domain not in domain_map:
        domain_map[domain] = []
    domain_map[domain].append(wf)

# 打印统计
print("📊 领域工作流映射关系")
print("=" * 80)
for domain in sorted(domain_map.keys()):
    wfs = domain_map[domain]
    print(f"\n🎯 Domain: {domain} ({len(wfs)} 个工作流)")
    for wf in wfs[:3]:  # 只显示前 3 个
        print(f"   - {wf['id']}: {wf['name']}")
        print(f"     tags: {wf['tags']}")
    if len(wfs) > 3:
        print(f"   ... 还有 {len(wfs) - 3} 个")

print(f"\n\n📋 总计: {len(workflows)} 个工作流")
print(f"📂 分布在 {len(domain_map)} 个领域")

# 行业映射表
print("\n\n🔗 建议的行业-领域映射:")
print("=" * 80)
industry_domain_map = {
    "ai-research": ["acd"],  # academic
    "software-dev": ["eng"],  # engineering  
    "finance-invest": ["fin"],  # finance
    "sales-growth": ["sal", "mkt"],  # sales + marketing
    "content-media": ["spc"],  # specialized (包含 content 相关)
    "industry-consulting": ["spc", "strat"],  # specialized + strategy
    "accounting": ["fin"],  # finance
    "ecommerce": ["mkt", "spc"],  # marketing + specialized
    "education": ["acd", "sup"],  # academic + support
}

for industry, domains in industry_domain_map.items():
    print(f"\n🏢 {industry}")
    for d in domains:
        wfs = domain_map.get(d, [])
        print(f"   ↳ {d}: {len(wfs)} 个工作流")
        for wf in wfs[:2]:
            print(f"     • {wf['id']}: {wf['name']}")