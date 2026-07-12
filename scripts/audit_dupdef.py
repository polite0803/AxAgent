#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AxAgent 重复定义审计器
- Rust: src-tauri/**/*.rs (排除 target)
- TS:   src/**/*.{ts,tsx} (排除 node_modules/dist)
检测维度:
  1) 名称级重复: 同一符号名在多处定义
  2) 内容级重复: 字段签名相同但名称不同 (同内容不同名)
  3) 跨 crate / 跨文件违规定位 (对照 AGENTS.md 禁区12)
"""
import os, re, json, hashlib
from collections import defaultdict

ROOT = r"d:\OneManager\AxAgent"

# ---------- 路径扫描 ----------
rust_files = []
ts_files = []
for dirpath, dirnames, filenames in os.walk(os.path.join(ROOT, "src-tauri")):
    if "target" in dirpath.split(os.sep):
        continue
    for f in filenames:
        if f.endswith(".rs"):
            rust_files.append(os.path.join(dirpath, f))

for dirpath, dirnames, filenames in os.walk(os.path.join(ROOT, "src")):
    parts = dirpath.split(os.sep)
    if "node_modules" in parts or "dist" in parts:
        continue
    for f in filenames:
        if f.endswith(".ts") or f.endswith(".tsx"):
            ts_files.append(os.path.join(dirpath, f))

def rel(p):
    return os.path.relpath(p, ROOT).replace("\\", "/")

print(f"[scan] rust files: {len(rust_files)}, ts files: {len(ts_files)}")

# ---------- Rust 定义提取 ----------
# struct / enum / trait / type / const / fn (top-level pub 或 mod 级)
rust_defs = defaultdict(list)  # (kind, name) -> [(file, line, crate, span_fields)]
rust_field_sig = defaultdict(list)  # sig -> [(file,line,kind,name)]

# 仅匹配顶层定义（行首，可能带 pub / pub(...) / async）
re_struct = re.compile(r'^(?:pub(?:\([^)]*\))?\s+)?(?:#[^\n]*\n\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?struct\s+(\w+)')
re_enum   = re.compile(r'^(?:pub(?:\([^)]*\))?\s+)?(?:#[^\n]*\n\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?enum\s+(\w+)')
re_trait  = re.compile(r'^(?:pub(?:\([^)]*\))?\s+)?(?:#[^\n]*\n\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?trait\s+(\w+)')
re_type   = re.compile(r'^(?:pub(?:\([^)]*\))?\s+)?type\s+(\w+)')
re_const  = re.compile(r'^(?:pub(?:\([^)]*\))?\s+)?const\s+(\w+)')
re_fn     = re.compile(r'^(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)')

def crate_of(path):
    # src-tauri/crates/<name>/... 或 src-tauri/src/...
    p = rel(path)
    if p.startswith("src-tauri/crates/"):
        return p.split("/")[2]
    if p.startswith("src-tauri/src"):
        return "src-tauri(src)"
    if p.startswith("src-tauri/schema-gen"):
        return "schema-gen"
    return p

def extract_fields(text):
    """从 struct/enum 定义块中提取字段/变体名集合（内容签名用）。"""
    fields = set()
    # 取第一对花括号内容
    depth = 0
    started = False
    buf = []
    for ch in text:
        if ch == '{':
            started = True
            depth += 1
            continue
        if ch == '}':
            depth -= 1
            if started and depth == 0:
                break
            continue
        if started:
            buf.append(ch)
    body = "".join(buf)
    # 字段: 标识符 在行首（容忍缩进）后跟 : 或 { 或 ,
    for m in re.finditer(r'(?:^|\n)\s*([A-Za-z_]\w*)\s*[:{,]', body):
        fields.add(m.group(1))
    # enum variant: 行首标识符 后跟 ( 或 { 或 , 或 =
    for m in re.finditer(r'(?:^|\n)\s*([A-Za-z_]\w*)\s*(?:\(|\{|=|,|\n|$)', body):
        fields.add(m.group(1))
    # 去掉常见关键字噪音
    noise = {"pub", "fn", "let", "if", "else", "match", "return", "self", "Self",
             "where", "impl", "for", "loop", "while", "use", "true", "false", "async"}
    return tuple(sorted(fields - noise))

def block_after(text, idx):
    """截取定义所在行及其后续花括号块（用于字段签名）。"""
    return text[idx: idx+4000]

for path in rust_files:
    try:
        with open(path, encoding="utf-8", errors="ignore") as fh:
            lines = fh.readlines()
    except Exception:
        continue
    crate = crate_of(path)
    for i, line in enumerate(lines):
        s = line.strip()
        for rx, kind in [(re_struct,"struct"),(re_enum,"enum"),(re_trait,"trait"),
                          (re_type,"type"),(re_const,"const"),(re_fn,"fn")]:
            m = rx.match(line)
            if m:
                name = m.group(1)
                key = (kind, name)
                rust_defs[key].append({"file": rel(path), "line": i+1, "crate": crate})
                break
    # 字段签名（仅 struct/enum）单独扫
    text = "".join(lines)
    for rx, kind in [(re_struct,"struct"),(re_enum,"enum")]:
        for m in rx.finditer(text):
            name = m.group(1)
            sig = extract_fields(block_after(text, m.start()))
            if sig:
                rust_field_sig[sig].append({"file": rel(path), "line": text[:m.start()].count("\n")+1,
                                            "kind": kind, "name": name, "crate": crate_of(path)})

# ---------- TS 定义提取 ----------
ts_defs = defaultdict(list)  # (kind,name) -> [(file,line)]
ts_field_sig = defaultdict(list)

re_ts_interface = re.compile(r'\bexport\s+interface\s+([A-Za-z_]\w*)')
re_ts_type      = re.compile(r'\bexport\s+type\s+([A-Za-z_]\w*)\b')
re_ts_enum      = re.compile(r'\bexport\s+enum\s+([A-Za-z_]\w*)')
re_ts_class     = re.compile(r'\bexport\s+(?:default\s+)?(?:abstract\s+)?class\s+([A-Za-z_]\w*)')
re_ts_const     = re.compile(r'\bexport\s+const\s+([A-Za-z_]\w*)')
re_ts_fn        = re.compile(r'\bexport\s+(?:async\s+)?function\s+([A-Za-z_]\w*)')

def ts_extract_fields(text):
    fields = set()
    depth = 0; started=False; buf=[]
    for ch in text:
        if ch == '{':
            started=True; depth+=1; continue
        if ch == '}':
            depth-=1
            if started and depth==0: break
            continue
        if started: buf.append(ch)
    body="".join(buf)
    for m in re.finditer(r'(?:^|\n)\s*([A-Za-z_]\w*)\s*[:?]', body):
        fields.add(m.group(1))
    noise={"export","const","let","function","return","if","else","interface","type",
           "class","enum","extends","implements","import","from","async","await"}
    return tuple(sorted(fields-noise))

for path in ts_files:
    try:
        with open(path, encoding="utf-8", errors="ignore") as fh:
            text = fh.read()
    except Exception:
        continue
    lines = text.split("\n")
    for i, line in enumerate(lines):
        for rx, kind in [(re_ts_interface,"interface"),(re_ts_type,"type"),
                         (re_ts_enum,"enum"),(re_ts_class,"class"),
                         (re_ts_const,"const"),(re_ts_fn,"function")]:
            for m in rx.finditer(line):
                name = m.group(1)
                ts_defs[(kind,name)].append({"file": rel(path), "line": i+1})
    # 字段签名（interface/type object）
    for rx, kind in [(re_ts_interface,"interface"),(re_ts_type,"type")]:
        for m in rx.finditer(text):
            name=m.group(1)
            sig=ts_extract_fields(block_after(text,m.start()))
            if sig and len(sig)>=3:
                ts_field_sig[sig].append({"file":rel(path),"line":text[:m.start()].count("\n")+1,
                                          "kind":kind,"name":name})

# ---------- 汇总输出 ----------
def summarize(defs, lang):
    out=[]
    for (kind,name),locs in defs.items():
        if len(locs)>1:
            out.append({"kind":kind,"name":name,"count":len(locs),"locs":locs})
    out.sort(key=lambda x:(x["count"],x["name"]), reverse=True)
    return out

rust_dups = summarize(rust_defs, "rust")
ts_dups = summarize(ts_defs, "ts")

# 跨 crate 重复 (Rust)
rust_cross_crate=[]
for d in rust_dups:
    crates=set(l["crate"] for l in d["locs"])
    if len(crates)>1:
        d["cross_crate"]=True
        rust_cross_crate.append(d)
    else:
        d["cross_crate"]=False

# 跨文件重复 (TS)
ts_cross_file=[]
for d in ts_dups:
    files=set(l["file"] for l in d["locs"])
    if len(files)>1:
        d["cross_file"]=True
        ts_cross_file.append(d)
    else:
        d["cross_file"]=False

# 同内容不同名 (Rust struct/enum)
rust_samecontent=[]
for sig,items in rust_field_sig.items():
    names=set((it["name"],it["crate"]) for it in items)
    # 同一 crate 内同名不算；关注 "不同 name" 或 "跨 crate 同 name 同字段"
    by_name=defaultdict(list)
    for it in items: by_name[it["name"]].append(it)
    if len(by_name)>1:  # 不止一个名字拥有相同字段签名
        rust_samecontent.append({"sig":list(sig),"items":items})
rust_samecontent.sort(key=lambda x:len(x["items"]),reverse=True)

# 同内容不同名 (TS interface/type)
ts_samecontent=[]
for sig,items in ts_field_sig.items():
    by_name=defaultdict(list)
    for it in items: by_name[it["name"]].append(it)
    if len(by_name)>1:
        ts_samecontent.append({"sig":list(sig),"items":items})
ts_samecontent.sort(key=lambda x:len(x["items"]),reverse=True)

result={
    "stats":{
        "rust_files":len(rust_files),"ts_files":len(ts_files),
        "rust_dup_count":len(rust_dups),"rust_cross_crate":len(rust_cross_crate),
        "ts_dup_count":len(ts_dups),"ts_cross_file":len(ts_cross_file),
        "rust_samecontent":len(rust_samecontent),"ts_samecontent":len(ts_samecontent),
    },
    "rust_dups":rust_dups,
    "ts_dups":ts_dups,
    "rust_cross_crate":rust_cross_crate,
    "ts_cross_file":ts_cross_file,
    "rust_samecontent":rust_samecontent,
    "ts_samecontent":ts_samecontent,
}
with open(os.path.join(ROOT,"output","dupdef_result.json"),"w",encoding="utf-8") as f:
    json.dump(result,f,ensure_ascii=False,indent=2)

print("[done] stats:", json.dumps(result["stats"], ensure_ascii=False))
