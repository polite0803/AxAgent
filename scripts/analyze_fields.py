#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""字段级比对：对每个 Rust 跨crate 数据模型重复项，
提取 harness(权威) 字段集 与 各重定义 crate 字段集 比对，
分出 SAFE（一致/子集，可合并到 harness）与 MANUAL（分歧，需人工）。
只读，不改动任何文件。
"""
import os, re, json
from collections import defaultdict

ROOT = r"d:\OneManager\AxAgent"
d = json.load(open(os.path.join(ROOT, "output", "dupdef_result.json"), encoding="utf-8"))
SEAORM = {"Model", "Relation", "ActiveModel", "Column", "Entity", "Linked"}
DATA_KINDS = {"struct", "enum", "type"}
HARNESS_AUTH = {"harness", "entities"}

def extract_block(text, idx):
    """从 idx（行首）开始，截取到匹配的第一个 {} 块（含内部）。"""
    # 找到 struct/enum 关键字后的 {
    rest = text[idx:]
    bi = rest.find("{")
    if bi < 0:
        return None
    depth = 0
    out = []
    for i, ch in enumerate(rest[bi:], start=bi):
        out.append(ch)
        if ch == "{": depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                break
    return "".join(out)

def rust_fields(block):
    """从 struct/enum 块体提取字段/变体名集合。"""
    if block is None:
        return None
    body = block[block.find("{")+1: block.rfind("}")]
    fields = set()
    # 字段: 行首标识符 后跟 : { , (=只在enum)
    for m in re.finditer(r'(?:^|\n)\s*([A-Za-z_]\w*)\s*[:{,]', body):
        fields.add(m.group(1))
    for m in re.finditer(r'(?:^|\n)\s*([A-Za-z_]\w*)\s*(?:\(|\{|=|\n|$)', body):
        fields.add(m.group(1))
    noise = {"pub", "fn", "let", "if", "else", "match", "return", "self", "Self",
             "where", "impl", "for", "loop", "while", "use", "true", "false", "async",
             "Option", "Vec", "String", "Box", "Result", "HashMap", "serde", "derive"}
    return frozenset(fields - noise)

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
        continue  # Tier3，跳过（非 harness 权威）
    # 取权威字段集：优先 harness，其次 entities
    auth_crate = "harness" if "harness" in auth_crates else auth_crates[0]
    af, al = crate_map[auth_crate]
    apath = os.path.join(ROOT, af)
    with open(apath, encoding="utf-8", errors="ignore") as fh:
        alines = fh.readlines()
    atext = "".join(alines)
    # 定位权威定义行
    m = re.search(r'(?:pub\s+)?(?:async\s+)?'+kind+r'\s+'+re.escape(name)+r'\b', alines[al-1])
    # 从文件全文搜该 struct/enum 块
    am = re.search(re.escape(name)+r'\b[^{]*\{', atext)
    auth_block = extract_block(atext, am.start()) if am else None
    auth_fields = rust_fields(auth_block)

    others = []
    for c, (f, ln) in crate_map.items():
        if c in HARNESS_AUTH:
            continue
        fpath = os.path.join(ROOT, f)
        with open(fpath, encoding="utf-8", errors="ignore") as fh:
            tlines = fh.readlines()
        ttext = "".join(tlines)
        tm = re.search(re.escape(name)+r'\b[^{]*\{', ttext)
        block = extract_block(ttext, tm.start()) if tm else None
        flds = rust_fields(block)
        if auth_fields is None or flds is None:
            rel = "unknown"
        elif flds == auth_fields:
            rel = "identical"
        elif flds < auth_fields:
            rel = "subset"
        elif flds > auth_fields:
            rel = "superset"
        elif flds & auth_fields:
            rel = "overlap"
        else:
            rel = "disjoint"
        others.append({"crate": c, "file": f, "line": ln,
                       "n_fields": len(flds) if flds else -1,
                       "rel": rel})
    results.append({
        "kind": kind, "name": name,
        "auth": auth_crate, "auth_fields": len(auth_fields) if auth_fields else -1,
        "others": others,
    })

# 汇总
def rel_class(others):
    rels = {o["rel"] for o in others}
    if rels <= {"identical", "subset"}:
        return "SAFE"
    return "MANUAL"

safe, manual = [], []
for r in results:
    cls = rel_class(r["others"])
    r["class"] = cls
    (safe if cls == "SAFE" else manual).append(r)

# 统计各 rel
from collections import Counter
relc = Counter()
for r in results:
    for o in r["others"]:
        relc[o["rel"]] += 1

out = {
    "total": len(results),
    "safe": len(safe), "manual": len(manual),
    "rel_counts": dict(relc),
    "safe": safe, "manual": manual,
}
with open(os.path.join(ROOT, "output", "dupdef_fieldcheck.json"), "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=2)

print(f"Tier1+Tier2 字段比对完成: 总 {len(results)} 项")
print(f"  SAFE(可安全合并): {len(safe)}")
print(f"  MANUAL(字段分歧/需人工): {len(manual)}")
print(f"  关系分布: {dict(relc)}")
print("\n--- SAFE 列表（字段一致/子集，可合并到 harness）---")
for r in sorted(safe, key=lambda z: z["name"]):
    crates = ",".join(o["crate"] for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:28} 权威({r["auth"]},{r["auth_fields"]}f) <- {crates}')
print(f"\n--- MANUAL 列表（{len(manual)} 项，字段分歧，需逐一定夺）---")
for r in sorted(manual, key=lambda z: z["name"]):
    det = "; ".join(f'{o["crate"]}:{o["rel"]}({o["n_fields"]}f)' for o in r["others"])
    print(f'  {r["kind"]:7} {r["name"]:28} 权威({r["auth"]},{r["auth_fields"]}f) | {det}')
