#!/usr/bin/env python3
"""Insert `control: None,` into every `NodeOutput { ... }` constructor literal
in axagent-rt-workflow that is missing the `control` field.

Skips:
  - the struct definition line (`pub struct NodeOutput {`)
  - return-type lines (`-> NodeOutput {`)
  - literals that already contain a `control:` field

Field order does not matter in Rust struct literals, so inserting as the
first field is always valid.
"""
import os
import re

ROOT = r"d:\OneManager\AxAgent\src-tauri\crates\rt-workflow\src"
LIT_RE = re.compile(r"NodeOutput\s*\{$")

def process_file(path):
    with open(path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    out = []
    changed = 0
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]
        stripped = line.rstrip()
        # candidate: line ends with `NodeOutput {`
        if stripped.endswith("NodeOutput {") or stripped.endswith("NodeOutput {"):
            is_def = "struct NodeOutput" in line
            is_ret = "->" in line and "NodeOutput {" in line
            if not is_def and not is_ret:
                # scan next up-to-60 lines for an existing `control:` field
                has_control = False
                depth = 0
                for j in range(i, min(i + 60, n)):
                    depth += lines[j].count("{") - lines[j].count("}")
                    if re.search(r"^\s*control\s*:", lines[j]):
                        has_control = True
                        break
                    if depth <= 0 and j > i:
                        break
                if not has_control:
                    indent = re.match(r"(\s*)", line).group(1)
                    out.append(line)
                    out.append(f"{indent}    control: None,\n")
                    changed += 1
                    i += 1
                    continue
        out.append(line)
        i += 1

    if changed:
        with open(path, "w", encoding="utf-8") as f:
            f.writelines(out)
    return changed

def main():
    total = 0
    for dirpath, _, filenames in os.walk(ROOT):
        for fn in filenames:
            if fn.endswith(".rs"):
                p = os.path.join(dirpath, fn)
                c = process_file(p)
                if c:
                    total += c
                    print(f"  +{c}  {os.path.relpath(p, ROOT)}")
    print(f"TOTAL inserted: {total}")

if __name__ == "__main__":
    main()
