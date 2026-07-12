#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""可靠核实：对每个 Tier1+Tier2 重复项，提取 harness 权威定义与各处重定义的
【完整块文本】，归一化后精确比对，分四类：
  EXACT   = 块文本逐字一致（最安全可合并）
  FIELDSET= 字段集合一致但 derive/属性/顺序微差（低风险合并候选）
  SUPERSET= 本地是 harness 超集（扩展了字段，需定夺，不能硬并）
  SUBSET  = 本地是 harness 子集（harness 更全，合并到 harness 安全）
  DIVERGENT=字段部分重叠/不相交（命名冲突，不合并）
  UNKNOWN = 提取失败
只读，不改动任何文件。
"""
import os, re, json
from collections import defaultdict

ROOT = r"d:\OneManager\AxAgent"
d = json.load(open(os.path.join(ROOT, "output", "dupdef_result.json"), encoding="utf-8"))
SEAORM = {"Model", "Relation", "ActiveModel", "Column", "Entity", "Linked"}
DATA_KINDS = {"struct", "enum", "type"}
HARNESS_AUTH = {"harness", "entities"}

def read_text(path):
    with open(os.path.join(ROOT, path), encoding="utf-8", errors="ignore") as f:
        return f.read()

def find_def_block(text, name, kind):
    """在 text 中找到 `pub (struct|enum|type) NAME ...` 的块，返回归一化块串或 None。"""
    # 定位定义起始行
    pat = re.compile(
        r'(?:pub\s+(?:async\s+)?)?' + kind + r'\s+' + re.escape(name) + r'\b[^\n{;(]*')
    m = pat.search(text)
    if not m:
        return None, None
    start = m.start()
    rest = text[start:]
    # type 别名：到 ; 结束
    if kind == "type":
        semi = rest.find(";")
        if semi < 0:
            return None, None
        block = rest[:semi+1]
        return normalize(block), start
    # 找第一个 { 或 (
    bi = rest.find("{")
    pi = rest.find("(")
    if bi < 0 and pi < 0:
        return None, start
    if pi >= 0 and (bi < 0 or pi < bi):
        # tuple struct: ( ... )
        depth = 0
        for i, ch in enumerate(rest[pi:], start=pi):
            if ch == "(": depth += 1
            elif ch == ")":
                depth -= 1
                if depth == 0:
                    return normalize(rest[:i+1]), start
        return None, start
    else:
        depth = 0
        for i, ch in enumerate(rest[bi:], start=bi):
            if ch == "{": depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    return normalize(rest[:i+1]), start
        return None, start

def normalize(s):
    """归一化：折叠所有空白（含换行）为单空格，去首尾。"""
    return re.sub(r"\s+", " ", s).strip()

def field_set(block):
    """从归一化块提取字段/变体名集合（仅在 { } 内）。"""
    if block is None:
        return None
    lb = block.find("{")
    rb = block.rfind("}")
    if lb < 0 or rb < 0 or rb <= lb:
        # tuple struct: 字段在 ( ) 内
        lp = block.find("(")
        rp = block.rfind(")")
        if lp >= 0 and rp > lp:
            body = block[lp+1:rp]
        else:
            return set()
    else:
        body = block[lb+1:rb]
    fields = set()
    for mm in re.finditer(r'(?:^|[,{]\s*|=\s*)([A-Za-z_]\w*)\s*[:{(]', body):
        fields.add(mm.group(1))
    for mm in re.finditer(r'(?:^|[,{]\s*|=\s*)([A-Za-z_]\w*)\s*(?:\(|\{|=|,)', body):
        fields.add(mm.group(1))
    noise = {"pub", "fn", "let", "if", "else", "match", "return", "self", "Self",
             "where", "impl", "for", "loop", "while", "use", "true", "false", "async",
             "Option", "Vec", "String", "Box", "Result", "HashMap", "serde", "derive",
             "pub", "crate", "super", "Self", "Option", "Vec"}
    return frozenset(f for f in fields if f not in noise)

# 建立 (kind,name) -> {crate: (file,line)}
by_key = defaultdict(dict)
for x in d["rust_cross_crate"]:
    if x["kind"] not in DATA_KINDS or x["name"] in SEAORM:
        continue
    for l in x["locs"]:
        by_key[(x["kind"], x["name"])][l["crate"]] = (l["file"], l["line"])

results = []
for (kind, name), crate_map in by_key.items():
    auth_crates = [c for c in crate_map if c in HARNESS_AUTH]
    if not auth_crates:
        continue
    auth_crate = "harness" if "harness" in auth_crates else auth_crates[0]
    af, al = crate_map[auth_crate]
    atext = read_text(af)
    ablock, _ = find_def_block(atext, name, kind)
    afields = field_set(ablock)

    others = []
    for c, (f, ln) in crate_map.items():
        if c in HARNESS_AUTH:
            continue
        ttext = read_text(f)
        block, _ = find_def_block(ttext, name, kind)
        if block is None:
            others.append({"crate": c, "file": f, "line": ln, "class": "UNKNOWN",
                          "auth_body": ablock is not None})
            continue
        flds = field_set(block)
        if ablock is not None and normalize(block) == normalize(ablock):
            cls = "EXACT"
        elif afields is not None and flds is not None and flds == afields:
            cls = "FIELDSET"
        elif afields is not None and flds is not None and flds > afields:
            cls = "SUPERSET"
        elif afields is not None and flds is not None and flds < afields:
            cls = "SUBSET"
        elif afields is not None and flds is not None and (flds & afields):
            cls = "DIVERGENT"
        elif afields is not None and flds is not None:
            cls = "NAMING_CONFLICT"   # 字段完全不相交
        else:
            cls = "UNKNOWN"
        others.append({"crate": c, "file": f, "line": ln, "class": cls,
                      "n_fields": len(flds) if flds is not None else -1})
    results.append({
        "kind": kind, "name": name, "auth": auth_crate,
        "auth_fields": len(afields) if afields is not None else -1,
        "auth_body_ok": ablock is not None,
        "others": others,
    })

# 汇总分类
def worst(classes):
    # classes: list[str] 直接是各 crate 的定级标签；取“最保守/最严重”的作为整体定级
    present = classes
    for c in ["NAMING_CONFLICT", "DIVERGENT", "SUPERSET", "SUBSET", "FIELDSET", "EXACT", "UNKNOWN"]:
        if c in present:
            return c
    return "UNKNOWN"

buckets = defaultdict(list)
for r in results:
    r["overall"] = worst([o["class"] for o in r["others"]])
    buckets[r["overall"]].append(r)

from collections import Counter
cnt = Counter(r["overall"] for r in results)

out = {
    "total": len(results),
    "counts": dict(cnt),
    "EXACT": buckets.get("EXACT", []),
    "FIELDSET": buckets.get("FIELDSET", []),
    "SUBSET": buckets.get("SUBSET", []),
    "SUPERSET": buckets.get("SUPERSET", []),
    "DIVERGENT": buckets.get("DIVERGENT", []),
    "NAMING_CONFLICT": buckets.get("NAMING_CONFLICT", []),
    "UNKNOWN": buckets.get("UNKNOWN", []),
}
with open(os.path.join(ROOT, "output", "dupdef_verified.json"), "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=2)

print(f"核实完成：Tier1+Tier2 共 {len(results)} 项\n")
for k in ["EXACT", "FIELDSET", "SUBSET", "SUPERSET", "DIVERGENT", "NAMING_CONFLICT", "UNKNOWN"]:
    print(f"  {k:16} {cnt.get(k,0)}")
print("\n--- EXACT 真·重复（块文本一致，最安全可合并）---")
for r in sorted(buckets.get("EXACT", []), key=lambda z: z["name"]):
    crates = ",".join(o["crate"] for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:26} <- {crates}')
print(f"\n--- FIELDSET 结构微差（字段集合一致，derive/属性/顺序微差）---")
for r in sorted(buckets.get("FIELDSET", []), key=lambda z: z["name"]):
    crates = ",".join(o["crate"] for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:26} <- {crates}')
print(f"\n--- SUBSET 本地是子集（harness 更全，合并到 harness 安全）---")
for r in sorted(buckets.get("SUBSET", []), key=lambda z: z["name"]):
    crates = ",".join(o["crate"] for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:26} <- {crates}')
print(f"\n--- SUPERSET 字段分歧（本地扩展了字段，需定夺，不能硬并）---")
for r in sorted(buckets.get("SUPERSET", []), key=lambda z: z["name"]):
    det = "; ".join(f'{o["crate"]}:+{o["n_fields"]}f' for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:26} | {det}')
print(f"\n--- DIVERGENT 部分重叠（命名冲突，不合并）---")
for r in sorted(buckets.get("DIVERGENT", []), key=lambda z: z["name"]):
    det = "; ".join(f'{o["crate"]}' for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:26} | {det}')
print(f"\n--- NAMING_CONFLICT 字段完全不相交（典型同名不同类型，不合并）---")
for r in sorted(buckets.get("NAMING_CONFLICT", []), key=lambda z: z["name"]):
    det = "; ".join(f'{o["crate"]}({o["n_fields"]}f)' for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:26} | {det}')
print(f"\n--- UNKNOWN 提取失败（需人工读源码）---")
for r in sorted(buckets.get("UNKNOWN", []), key=lambda z: z["name"]):
    det = "; ".join(f'{o["crate"]}:{o.get("file","?")}' for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:26} | {det}')
