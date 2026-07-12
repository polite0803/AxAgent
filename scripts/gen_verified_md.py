#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import json, os
ROOT = r"d:\OneManager\AxAgent"
v = json.load(open(os.path.join(ROOT, "output", "dupdef_verified.json"), encoding="utf-8"))

lines = []
lines.append("# 重复定义 · 逐对核实清单（暂不改动）\n")
lines.append("> 生成 2026-07-11 · 对照 AGENTS.md 禁区 12\n")
lines.append("> 方法：提取 harness 权威定义与各处重定义的 **完整块文本** 做归一化比对。\n")
lines.append("> ⚠️ 仅 `EXACT`（块文本逐字一致）为机械可信；其余分类依赖「字段名集合」比对，\n")
lines.append("> 而字段**类型**差异抓不到，且已证明字段名解析器对同名不同类型会误判（见下），故不可直接采信。\n")

lines.append("\n## 一、真·重复（EXACT，块文本完全一致，可安全合并到 harness）\n")
lines.append(f"共 **{len(v['EXACT'])}** 项。合并方式：删本地定义，改 `pub use axagent_harness::X;` 再导出。\n")
for r in sorted(v["EXACT"], key=lambda z: z["name"]):
    crates = ",".join(o["crate"] for o in r["others"])
    lines.append(f"- `{r['kind']} {r['name']}` — 权威 `{r['auth']}`，重定义于 `{crates}`")

lines.append("\n## 二、已读源码证伪（同名不同类型，⛔ 不可合并）\n")
lines.append("- `struct GraphNode` — harness `graph_dtos.rs`（知识图谱节点：id/title/node_type/tags/link_count/backlink_count/path）vs trajectory `learning_graph.rs`（学习图节点：id/label/kind/category/timestamp_ms/use_count/state/detail）\n")
lines.append("- `struct GraphEdge` — harness `graph_dtos.rs`（source/target/edge_type）vs trajectory `learning_graph.rs`（source/target/weight/relation）\n")
lines.append("- `struct CompressedTrajectory` — harness `trajectory_types.rs`（id/topic/outcome/quality_score/value_score/step_summaries/tool_sequence/final_reward）vs trajectory `trajectory_compressor.rs`（id/session_id/topic/outcome/steps/decision_points/compression_ratio）。**且该文件已于 2026-07-05 标记 ABANDONED（`#[cfg(feature=\"abandoned\")]` 隔离），不参与常规编译**\n")

lines.append("\n## 三、其余 81 项（UNVERIFIED，需逐对读源码核对字段【类型】）\n")
lines.append("> 自动字段名比对不可靠（GraphNode 误判即证），以下仅按「字段名集合」粗分，\n")
lines.append("> **未经验证**，每一项都必须读 harness 与本地两端定义、比对字段名+类型+语义后才能定夺。\n")

def dump(title, items):
    if not items:
        return
    lines.append(f"\n### {title}\n")
    for r in sorted(items, key=lambda z: z["name"]):
        crates = ",".join(o["crate"] for o in r["others"])
        lines.append(f"- `{r['kind']} {r['name']}` — 权威 `{r['auth']}` <- `{crates}`")

dump(f"FIELDSET 字段名一致但块文本微差（{len(v['FIELDSET'])}，可能仅 derive/顺序差，也可能藏类型差 → 需读）", v["FIELDSET"])
dump(f"SUBSET 本地是 harness 子集（{len(v['SUBSET'])}，若字段类型也一致则合并安全，需读确认）", v["SUBSET"])
dump(f"SUPERSET 本地扩展了字段（{len(v['SUPERSET'])}，⛔ 不能硬并，需你定夺）", v["SUPERSET"])
dump(f"DIVERGENT 字段部分重叠（{len(v['DIVERGENT'])}，命名冲突，不合并）", v["DIVERGENT"])
dump(f"NAMING_CONFLICT 字段完全不相交（{len(v['NAMING_CONFLICT'])}，典型同名不同类型，不合并）", v["NAMING_CONFLICT"])

lines.append("\n## 四、结论与建议\n")
lines.append("- **可立即安全合并**：仅上面「一」的 10 项（块文本逐字一致）。\n")
lines.append("- **不可合并（已证伪）**：「二」的 3 组同名不同类型，合并会语义崩坏。\n")
lines.append("- **需逐对核实**：「三」的 81 项，自动比对不可信，每项要读两端源码比对字段【类型】。\n")
lines.append("- **TS 侧 27 项**：`src/types/` 权威被 store/component/sdk 重定义，属前端「同名重定义」高风险区，建议同样逐文件核对后改为 `import type`。\n")
lines.append("- **Tier3 的 79 个同名**：大概率同名不同义，已排除在合并范围外。\n")

with open(os.path.join(ROOT, "output", "dupdef_verified.md"), "w", encoding="utf-8") as f:
    f.write("\n".join(lines))
print("已生成 output/dupdef_verified.md")
print(f"EXACT={len(v['EXACT'])} FIELDSET={len(v['FIELDSET'])} SUBSET={len(v['SUBSET'])} SUPERSET={len(v['SUPERSET'])} DIVERGENT={len(v['DIVERGENT'])} NAMING={len(v['NAMING_CONFLICT'])}")
