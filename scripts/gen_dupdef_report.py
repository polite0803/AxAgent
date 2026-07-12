#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""依据 dupdef_result.json 生成重复定义审计报告 (HTML + Markdown)。"""
import json, os, html

ROOT = r"d:\OneManager\AxAgent"
d = json.load(open(os.path.join(ROOT, "output", "dupdef_result.json"), encoding="utf-8"))

# --- 排除项：sea-orm 每实体自动生成的规范名（非业务重复）---
SEAORM = {"Model", "Relation", "ActiveModel", "Column", "Entity", "Linked"}
# --- 架构角色 ---
HARNESS_AUTH = {"harness", "entities"}          # 共享类型权威源
CONSUMERS   = {"runtime-core", "agent", "gateway", "orchestrator"}
WIRING      = {"src-tauri(src)"}               # 二进制/wiring 层
# Rust 数据模型种类
DATA_KINDS = {"struct", "enum", "type"}

def crate_role(c):
    if c in HARNESS_AUTH: return "authority"
    if c in CONSUMERS: return "consumer"
    if c in WIRING: return "wiring"
    return "implementor"

# ---------- Rust 分级 ----------
rust_tier1, rust_tier2, rust_tier3 = [], [], []
rust_excluded = []
for x in d["rust_cross_crate"]:
    if x["kind"] not in DATA_KINDS:
        continue
    if x["name"] in SEAORM:
        rust_excluded.append(x)
        continue
    crates = set(l["crate"] for l in x["locs"])
    roles = {crate_role(c) for c in crates}
    entry = x
    if (HARNESS_AUTH & crates) and ((CONSUMERS | WIRING) & crates):
        entry["tier"] = 1
        rust_tier1.append(entry)
    elif HARNESS_AUTH & crates:
        entry["tier"] = 2
        rust_tier2.append(entry)
    else:
        entry["tier"] = 3
        rust_tier3.append(entry)

# 同 crate 内重复（非跨 crate，但仍可能是同文件多定义/误放）
rust_samecrate = [x for x in d["rust_dups"]
                  if x["kind"] in DATA_KINDS and x["name"] not in SEAORM
                  and not x["cross_crate"]]

# ---------- TS 分级 ----------
# 权威源目录
TS_AUTH_DIRS = ("src/types/",)
ts_findings = []
for x in d["ts_cross_file"]:
    if x["kind"] not in ("interface", "type"):
        continue
    files = sorted(set(l["file"] for l in x["locs"]))
    auth_files = [f for f in files if any(f.startswith(p) for p in TS_AUTH_DIRS)]
    # 若在 src/types 有定义，且另有其他文件也定义 -> 违规（其他应从 @/types import）
    violation = len(files) > 1 and len(auth_files) >= 1
    ts_findings.append({
        "kind": x["kind"], "name": x["name"], "count": x["count"],
        "files": files, "auth": auth_files, "violation": violation,
    })
ts_violations = [t for t in ts_findings if t["violation"]]
ts_nonviolation = [t for t in ts_findings if not t["violation"]]

# ---------- 生成 HTML ----------
def fmt_locs(locs):
    return "<br>".join(html.escape(f'{l["file"]}:{l["line"]} ({l["crate"]})') for l in locs)

def rust_table(items):
    if not items:
        return '<p class="muted">（无）</p>'
    rows = []
    for x in sorted(items, key=lambda z: z["name"]):
        crates = sorted(set(l["crate"] for l in x["locs"]))
        rows.append(
            f"<tr><td><code>{html.escape(x['kind'])}</code></td>"
            f"<td><code>{html.escape(x['name'])}</code></td>"
            f"<td>{x['count']}</td>"
            f"<td>{html.escape(', '.join(crates))}</td>"
            f"<td class='loc'>{fmt_locs(x['locs'])}</td></tr>")
    return ("<table><thead><tr><th>类型</th><th>名称</th><th>重复数</th>"
            "<th>涉及 crate</th><th>定义位置</th></tr></thead><tbody>"
            + "".join(rows) + "</tbody></table>")

def ts_table(items):
    if not items:
        return '<p class="muted">（无）</p>'
    rows = []
    for t in sorted(items, key=lambda z: z["name"]):
        auth = "✅ 是" if t["auth"] else "❌ 否"
        files = "<br>".join(html.escape(f) for f in t["files"])
        rows.append(
            f"<tr><td><code>{html.escape(t['kind'])}</code></td>"
            f"<td><code>{html.escape(t['name'])}</code></td>"
            f"<td>{t['count']}</td>"
            f"<td>{auth}</td>"
            f"<td class='loc'>{files}</td></tr>")
    return ("<table><thead><tr><th>类型</th><th>名称</th><th>重复数</th>"
            "<th>src/types 权威源?</th><th>定义文件</th></tr></thead><tbody>"
            + "".join(rows) + "</tbody></table>")

html_doc = f"""<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="utf-8">
<title>AxAgent 重复定义审计报告</title>
<style>
:root{{--bg:#0d1117;--card:#161b22;--border:#30363d;--fg:#e6edf3;--muted:#8b949e;
--red:#f85149;--orange:#d29922;--yellow:#e3b341;--green:#3fb950;--blue:#58a6ff;}}
body{{background:var(--bg);color:var(--fg);font-family:-apple-system,'Segoe UI',sans-serif;
margin:0;padding:24px;line-height:1.5;}}
h1{{font-size:24px;margin:0 0 4px;}} h2{{font-size:19px;margin:28px 0 10px;
border-bottom:1px solid var(--border);padding-bottom:6px;}}
.sub{{color:var(--muted);margin-bottom:20px;}}
.card{{background:var(--card);border:1px solid var(--border);border-radius:8px;
padding:16px;margin:12px 0;}}
table{{border-collapse:collapse;width:100%;font-size:13px;margin-top:6px;}}
th,td{{border:1px solid var(--border);padding:6px 9px;text-align:left;vertical-align:top;}}
th{{background:#1c2330;position:sticky;top:0;}}
td.loc{{font-family:ui-monospace,monospace;font-size:12px;color:var(--muted);}}
code{{background:#1c2330;padding:1px 5px;border-radius:4px;color:var(--blue);}}
.badge{{display:inline-block;padding:2px 9px;border-radius:10px;font-size:12px;font-weight:600;}}
.b1{{background:rgba(248,81,73,.15);color:var(--red);border:1px solid var(--red);}}
.b2{{background:rgba(210,153,34,.15);color:var(--orange);border:1px solid var(--orange);}}
.b3{{background:rgba(88,166,255,.13);color:var(--blue);border:1px solid var(--blue);}}
.muted{{color:var(--muted);}}
.kpi{{display:flex;gap:14px;flex-wrap:wrap;}}
.kpi .box{{background:var(--card);border:1px solid var(--border);border-radius:8px;
padding:14px 18px;min-width:120px;}}
.kpi .n{{font-size:26px;font-weight:700;}}
.kpi .l{{color:var(--muted);font-size:12px;margin-top:2px;}}
.note{{border-left:3px solid var(--yellow);background:rgba(227,179,65,.08);
padding:10px 14px;margin:12px 0;border-radius:4px;}}
ul{{margin:6px 0;padding-left:20px;}} li{{margin:3px 0;}}
</style></head><body>
<h1>AxAgent 重复定义 / 多次定义审计报告</h1>
<div class="sub">生成于 2026-07-11 · 对照 AGENTS.md 禁区 12（禁止重复定义）· 扫描范围：src-tauri/*.rs（排除 target）、src/**/*.ts(x)（排除 node_modules/dist）</div>

<div class="kpi">
<div class="box"><div class="n" style="color:var(--red)">{len(rust_tier1)}</div><div class="l">Rust Tier1 严重违规<br>(harness 定义被消费者/ wiring 重定义)</div></div>
<div class="box"><div class="n" style="color:var(--orange)">{len(rust_tier2)}</div><div class="l">Rust Tier2 高<br>(harness 类型被 implementor 重定义)</div></div>
<div class="box"><div class="n" style="color:var(--blue)">{len(rust_tier3)}</div><div class="l">Rust Tier3 中<br>(跨 crate 同名，无 harness 参与)</div></div>
<div class="box"><div class="n" style="color:var(--red)">{len(ts_violations)}</div><div class="l">TS 违规<br>(src/types 权威源被他处重定义)</div></div>
<div class="box"><div class="n">{len(rust_excluded)}</div><div class="l">已排除<br>(sea-orm 生成名)</div></div>
</div>

<div class="note">
<b>方法学与判定口径</b><br>
• <b>名称级</b>：提取 <code>struct/enum/type/const/fn</code>（Rust）与 <code>interface/type/enum/class/const/function</code>（TS）定义，按名称聚合。<br>
• <b>违规分级</b>：依据 AGENTS.md 禁区 12 —— 后端共享类型权威源为 <code>axagent-harness</code>（runtime-core 等消费者须 <code>pub use</code> re-export，不得重定义）；前端类型权威源为 <code>src/types/</code>（store/component 须 <code>import</code>，不得重定义）。<br>
• <b>已排除</b>：sea-orm 每实体自动生成的 <code>Model/Relation/ActiveModel/Column/Entity/Linked</code>（<code>Model</code>×87、<code>Relation</code>×86 属此类，非业务重复）。<br>
• <b>误报可能</b>：同名但不同语义的类型（如不同域的 <code>Position</code>）需人工复核；本报告侧重「权威源已定义却被他处重定义」这一明确违例。
</div>

<h2><span class="badge b1">Tier 1</span> Rust 严重违规：harness 权威类型被消费者 / wiring 重定义</h2>
<div class="card">{rust_table(rust_tier1)}</div>

<h2><span class="badge b2">Tier 2</span> Rust 高：harness 类型被 implementor crate 重定义（应改为 pub use re-export）</h2>
<div class="card">{rust_table(rust_tier2)}</div>

<h2><span class="badge b3">Tier 3</span> Rust 中：跨 crate 同名数据模型（无 harness 参与，建议复核是否可合并）</h2>
<div class="card">{rust_table(rust_tier3)}</div>

<h2><span class="badge b1">TS 违规</span> 前端：<code>src/types/</code> 权威类型被 store / component / sdk 重定义</h2>
<div class="card">{ts_table(ts_violations)}</div>

<h2><span class="badge b3">TS 观察</span> 跨文件同名类型（src/types 非唯一权威，建议复核）</h2>
<div class="card">{ts_table(ts_nonviolation)}</div>

<h2>整改 Playbook（对照禁区 12）</h2>
<div class="card">
<b>Rust（后端）</b>
<ul>
<li><b>权威源唯一化</b>：所有共享 DTO / 事件 / 配置类型只保留在 <code>axagent-harness</code>（或 <code>axagent-entities</code> 数据定义）。</li>
<li><b>消费者 crate</b>（runtime-core / agent / gateway / orchestrator）：删除本地 <code>struct/enum/type</code> 定义，改为 <code>pub use axagent_harness::X;</code> 或 <code>use axagent_harness::X;</code>。</li>
<li><b>wiring 层</b>（src-tauri/src）：同上，通过 harness trait 传递，不得本地重定义。</li>
<li><b>implementor crate</b>：若需使用 harness 已有类型，必须 <code>pub use</code> re-export，不得重新声明相同字段的结构体。</li>
<li><b>删除前确认零引用</b>：用 <code>grep -rn "Type"</code> 确认所有引用已切换到 harness 路径，再删本地定义。</li>
</ul>
<b>TypeScript（前端）</b>
<ul>
<li><b>权威源唯一化</b>：所有共享类型只保留在 <code>src/types/</code>（含 <code>src/types/index.ts</code> barrel）。</li>
<li><b>store / component / sdk</b>：删除本地 <code>interface/type</code> 重定义，改为 <code>import type {{ X }} from '@/types';</code>。</li>
<li><b>特别注意</b>：<code>src/sdk/types.ts</code> 整族 <code>Skill*Capability</code> 与 <code>src/types/index.ts</code> 重复，应统一到 <code>src/types</code> 并由 sdk re-export。</li>
</ul>
</div>
<p class="muted">说明：本报告为静态扫描结果，Tier 仅表示违规可能性与架构影响面，具体每个类型是否完全同构需结合字段逐一确认后再做合并。建议按 Tier1 → Tier2 → Tier3 顺序治理。</p>
</body></html>"""

with open(os.path.join(ROOT, "output", "dupdef_report.html"), "w", encoding="utf-8") as f:
    f.write(html_doc)

# ---------- Markdown 摘要 ----------
md = f"""# AxAgent 重复定义审计报告（摘要）

> 生成于 2026-07-11 · 对照 AGENTS.md 禁区 12（禁止重复定义）
> 扫描：src-tauri/*.rs（排除 target）、src/**/*.ts(x)（排除 node_modules/dist）

## 核心数字
- Rust Tier1（harness 权威类型被消费者/wiring 重定义）：**{len(rust_tier1)}**
- Rust Tier2（harness 类型被 implementor 重定义）：**{len(rust_tier2)}**
- Rust Tier3（跨 crate 同名，无 harness）：**{len(rust_tier3)}**
- TS 违规（src/types 权威被重定义）：**{len(ts_violations)}**
- 已排除 sea-orm 生成名：**{len(rust_excluded)}**

## Tier1 严重违规清单（Rust，harness 权威源被消费者/wiring 重定义）
"""
for x in sorted(rust_tier1, key=lambda z: z["name"]):
    crates = sorted(set(l["crate"] for l in x["locs"]))
    md += f"- `{x['kind']} {x['name']}` ×{x['count']} @ {', '.join(crates)}\n"
md += "\n## TS 违规清单（src/types 权威源被他处重定义）\n"
for t in sorted(ts_violations, key=lambda z: z["name"]):
    md += f"- `{t['kind']} {t['name']}` ×{t['count']} :: " + " | ".join(t["files"]) + "\n"

with open(os.path.join(ROOT, "output", "dupdef_report.md"), "w", encoding="utf-8") as f:
    f.write(md)

print("报告已生成: output/dupdef_report.html / output/dupdef_report.md")
print(f"Tier1={len(rust_tier1)} Tier2={len(rust_tier2)} Tier3={len(rust_tier3)} TS违规={len(ts_violations)} 排除={len(rust_excluded)}")
