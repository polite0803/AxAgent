#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""AxAgent harness 架构违规扫描器。
读取 src-tauri 下所有 Cargo.toml，按 AGENTS.md 的角色分类校验依赖方向。
"""
import tomllib
import os
import re
import json
from collections import defaultdict

ROOT = os.path.dirname(os.path.abspath(__file__))  # src-tauri/

# ---- 角色分类（AGENTS.md 权威定义）----
ROLE = {
    "axagent-harness": "foundation",
    "axagent-entities": "foundation",
    "axagent-disk-cache": "foundation",
    "axagent-rt-dashboard": "foundation",
    "axagent-rt-theme": "foundation",
    "axagent-schema-gen": "foundation",
    "axagent-agent": "consumer",
    "axagent-orchestrator": "consumer",
    "axagent-runtime-core": "consumer",
    "axagent-gateway": "consumer",
    "axagent-dao": "implementor",
    "axagent-storage": "implementor",
    "axagent-migration": "implementor",
    "axagent-kit": "implementor",
    "axagent-cache": "implementor",
    "axagent-crypto": "implementor",
    "axagent-credential": "implementor",
    "axagent-mcp": "implementor",
    "axagent-search": "implementor",
    "axagent-providers": "implementor",
    "axagent-prompt-guard": "implementor",
    "axagent-telemetry": "implementor",
    "axagent-trajectory": "implementor",
    "axagent-plugins": "implementor",
    "axagent-npm": "implementor",
    "axagent-document-parser": "implementor",
    "axagent-rt-webhook": "implementor",
    "axagent-scanner": "implementor",
    "axagent-tools": "hybrid",
    "axagent-rt-messaging": "hybrid",
    "axagent-rt-workflow": "hybrid",
    "axagent-runtime": "wiring",
    # 根 workspace 包 / 代码生成器
    "axagent": "wiring",
    "schema-gen": "foundation",
}

FOUNDATION = {"harness", "entities", "disk-cache", "rt-dashboard", "rt-theme", "schema-gen"}
CONSUMER = {"agent", "orchestrator", "runtime-core", "gateway"}
IMPLEMENTOR = {"dao", "storage", "migration", "kit", "cache", "crypto", "credential",
               "mcp", "search", "providers", "prompt-guard", "telemetry", "trajectory",
               "plugins", "npm", "document-parser", "rt-webhook", "scanner"}
HYBRID = {"tools", "rt-messaging", "rt-workflow"}
WIRING = {"runtime"}

SHORT = {k: k.replace("axagent-", "") for k in ROLE}


def short(name: str) -> str:
    return name.replace("axagent-", "")


def read_cargo(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def axagent_deps(cargo):
    """返回 (normal_deps, dev_deps) 的短名集合，仅含 axagent-*。"""
    normal = set()
    dev = set()

    def collect(section):
        out = set()
        if not isinstance(section, dict):
            return out
        for k, v in section.items():
            if k.startswith("axagent-"):
                out.add(short(k))
        return out

    normal |= collect(cargo.get("dependencies"))
    dev |= collect(cargo.get("dev-dependencies"))
    # target.*.dev-dependencies
    for k, v in cargo.items():
        if k.startswith("target.") and isinstance(v, dict):
            dev |= collect(v.get("dev-dependencies"))
    return normal, dev


def find_tomls():
    results = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        if "target" in dirpath.split(os.sep):
            continue
        for fn in filenames:
            if fn == "Cargo.toml":
                results.append(os.path.join(dirpath, fn))
    return results


def main():
    tomls = find_tomls()
    crates = {}  # short -> info
    for path in tomls:
        cargo = read_cargo(path)
        pkg = cargo.get("package", {})
        name = pkg.get("name")
        if not name or not name.startswith("axagent-"):
            # 非 axagent crate（如 schema-gen 可能就叫 axagent-schema-gen；跳过非 axagent）
            if name and name.startswith("axagent-"):
                pass
            else:
                # 仍记录以便发现未知
                crates[short(name) if name else os.path.basename(os.path.dirname(path))] = {
                    "path": path, "pkg": name, "cargo": cargo,
                    "normal": set(), "dev": set(), "role": "unknown",
                }
                continue
        s = short(name)
        normal, dev = axagent_deps(cargo)
        crates[s] = {
            "path": path, "pkg": name, "cargo": cargo,
            "normal": normal, "dev": dev,
            "role": ROLE.get(name, "unknown"),
        }

    violations = []   # 硬违规
    warnings = []     # 需注意

    # ---- 依赖方向校验 ----
    for s, info in crates.items():
        role = info["role"]
        for dep in info["normal"]:
            if dep == s:
                continue
            dep_role = ROLE.get("axagent-" + dep, "unknown")
            # foundation harness: 零依赖
            if role == "foundation" and s == "harness":
                if dep_role != "unknown":
                    violations.append((s, dep, "foundation/harness 不应依赖任何 axagent-* crate"))
            elif role == "foundation" and s == "entities":
                if dep != "harness":
                    violations.append((s, dep, "foundation/entities 仅可依赖 harness"))
            elif role == "foundation":
                # disk-cache/rt-dashboard/rt-theme/schema-gen
                if dep != "harness":
                    violations.append((s, dep, f"foundation/{s} 仅可依赖 harness（当前依赖 implementor/foundation 其它层）"))
            elif role == "consumer":
                if dep != "harness":
                    violations.append((s, dep, "consumer 仅可依赖 harness，越过 harness 直接依赖实现层"))
            elif role == "implementor":
                if dep_role in ("consumer", "hybrid", "wiring"):
                    violations.append((s, dep, f"implementor 禁止依赖 {dep_role}/{dep}"))
                elif dep_role == "foundation" and dep not in ("harness", "entities"):
                    warnings.append((s, dep, f"implementor 依赖 foundation/{dep}（非常规，请确认）"))
            elif role == "hybrid":
                if dep_role in ("consumer", "entities", "wiring"):
                    violations.append((s, dep, f"hybrid 禁止依赖 {dep_role}/{dep}（entities/consumer 越界）"))
                elif dep_role == "foundation" and dep not in ("harness", "entities"):
                    warnings.append((s, dep, f"hybrid 依赖 foundation/{dep}（非常规，请确认）"))
            elif role == "wiring":
                pass  # wiring 允许全部
            elif role == "unknown":
                warnings.append((s, dep, "crate 角色未知，无法校验"))

    # ---- dev-dependencies 测试分层校验（铁律 5）----
    for s, info in crates.items():
        role = info["role"]
        for dep in info["dev"]:
            if dep == s:
                continue
            dep_role = ROLE.get("axagent-" + dep, "unknown")
            if role == "consumer":
                if dep != "harness":
                    violations.append((s, dep, f"[dev-dep] consumer 测试仅可用 harness::test_support，禁止 dev 依赖 {dep_role}/{dep}"))
            elif role in ("implementor", "hybrid", "wiring"):
                # 仅允许 dao 用于 create_test_pool
                if dep != "dao" and dep_role in ("implementor", "consumer", "hybrid", "wiring", "foundation"):
                    if dep == "dao":
                        continue
                    violations.append((s, dep, f"[dev-dep] {role} 测试仅允许 dev 依赖 dao（create_test_pool），当前依赖 {dep_role}/{dep}"))

    # ---- 循环依赖检测 ----
    graph = {s: set() for s in crates}
    for s, info in crates.items():
        for dep in info["normal"]:
            if dep in crates and dep != s:
                graph[s].add(dep)
    # Tarjan SCC
    index = {}
    low = {}
    onstack = {}
    stack = []
    sccs = []
    counter = [0]

    import sys
    sys.setrecursionlimit(10000)

    def strongconnect(v):
        index[v] = counter[0]
        low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        onstack[v] = True
        for w in graph[v]:
            if w not in index:
                strongconnect(w)
                low[v] = min(low[v], low[w])
            elif onstack.get(w):
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop()
                onstack[w] = False
                comp.append(w)
                if w == v:
                    break
            sccs.append(comp)

    for v in list(graph.keys()):
        if v not in index:
            strongconnect(v)
    for comp in sccs:
        if len(comp) > 1:
            violations.append(("CYCLE", " <-> ".join(sorted(comp)),
                               "循环依赖（非法的双向/环依赖）"))

    # ---- 角色声明检查（铁律 6）----
    missing_decl = []
    for s, info in crates.items():
        crate_dir = os.path.dirname(info["path"])
        declared = False
        for fn in ("README.md", "AGENTS.md"):
            p = os.path.join(crate_dir, fn)
            if os.path.exists(p):
                try:
                    txt = open(p, encoding="utf-8", errors="ignore").read().lower()
                    if any(k in txt for k in ("foundation", "consumer", "implementor", "hybrid", "wiring")):
                        declared = True
                except Exception:
                    pass
        if not declared:
            missing_decl.append(s)

    # ---- 已知 DTO 重复定义扫描（铁律 4）----
    known_dtos = ["ConversationMessage", "TokenUsage", "Session",
                  "PermissionMode", "HookEvent"]
    dto_defs = defaultdict(list)  # name -> [crate]
    all_type_defs = defaultdict(list)  # name -> [(crate, path)]
    for s, info in crates.items():
        crate_dir = os.path.dirname(info["path"])
        # 根 workspace 包目录即 src-tauri 全体，跳过以免把子 crate 文件误归到主 crate
        if crate_dir == ROOT:
            continue
        for root, dirs, files in os.walk(crate_dir):
            if "target" in root.split(os.sep):
                continue
            for fn in files:
                if fn.endswith(".rs"):
                    fp = os.path.join(root, fn)
                    try:
                        with open(fp, encoding="utf-8", errors="ignore") as f:
                            for line in f:
                                for dt in known_dtos:
                                    if re.search(rf"\b(pub\s+struct|pub\s+enum)\s+{dt}\b", line):
                                        dto_defs[dt].append(s)
                                m = re.search(r"\b(pub\s+struct|pub\s+enum)\s+([A-Z][A-Za-z0-9_]*)\b", line)
                                if m:
                                    all_type_defs[m.group(2)].append((s, fp))
                    except Exception:
                        pass

    dto_violations = []
    for dt, locs in dto_defs.items():
        locs = sorted(set(locs))
        if "harness" in locs and len(locs) > 1:
            others = [x for x in locs if x != "harness"]
            dto_violations.append((dt, others, "权威定义在 harness，但以下 crate 重复定义"))
        elif "harness" not in locs and len(locs) > 1:
            dto_violations.append((dt, sorted(set(locs)), "在多个 crate 重复定义（均非 harness 权威源）"))

    # ---- 全量跨 crate 重复类型扫描（启发式）----
    # 仅关注在 >=2 个 crate 出现、且非 harness 内不同模块的场景
    broad_dup = []
    for tname, occ in all_type_defs.items():
        crates_with = sorted({c for c, _ in occ})
        if len(crates_with) >= 2:
            # 排除仅在 harness 内部多模块出现的情况（单独报告）
            if not (set(crates_with) == {"harness"}):
                broad_dup.append((tname, crates_with, [p for _, p in occ]))

    return {
        "crates": {s: {"role": crates[s]["role"], "normal": sorted(crates[s]["normal"]),
                       "dev": sorted(crates[s]["dev"])} for s in crates if crates[s]["role"] != "unknown" or s == "axagent" or s == "schema-gen"},
        "violations": violations,
        "warnings": warnings,
        "cycles": [c for c in sccs if len(c) > 1],
        "missing_decl": sorted(missing_decl),
        "dto_violations": dto_violations,
        "broad_dup": broad_dup,
    }

    return {
        "crates": {s: {"role": crates[s]["role"], "normal": sorted(crates[s]["normal"]),
                       "dev": sorted(crates[s]["dev"])} for s in crates},
        "violations": violations,
        "warnings": warnings,
        "cycles": [c for c in sccs if len(c) > 1],
        "missing_decl": sorted(missing_decl),
        "dto_violations": dto_violations,
    }


if __name__ == "__main__":
    res = main()
    print(json.dumps(res, ensure_ascii=False, indent=2))
