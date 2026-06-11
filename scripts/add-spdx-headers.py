#!/usr/bin/env python3
"""为 Rust 和 TS/TSX 源文件批量添加 AGPL-3.0 SPDX 头。"""

import os
import re

HEADER_RS = "// SPDX-License-Identifier: AGPL-3.0-only\n\n"
HEADER_TS = "// SPDX-License-Identifier: AGPL-3.0-only\n\n"

def already_has_spdx(content: str) -> bool:
    return "SPDX-License-Identifier" in content[:500]

def add_header(filepath: str, header: str) -> bool:
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()
    if already_has_spdx(content):
        return False
    # Insert header after shebang if present, otherwise at top
    if content.startswith("#!"):
        idx = content.index("\n") + 1
        content = content[:idx] + header + content[idx:]
    else:
        content = header + content
    with open(filepath, "w", encoding="utf-8") as f:
        f.write(content)
    return True

def main():
    root = "D:/OneManager/AxAgent"
    src_tauri = os.path.join(root, "src-tauri")
    frontend_src = os.path.join(root, "src")

    rust_files_modified = 0
    ts_files_modified = 0

    # Rust files in src-tauri/crates/ and src-tauri/src/
    for dirpath, _, filenames in os.walk(src_tauri):
        # Skip target/
        if "target" in dirpath.split(os.sep):
            continue
        for f in filenames:
            if f.endswith(".rs"):
                fp = os.path.join(dirpath, f)
                if add_header(fp, HEADER_RS):
                    rust_files_modified += 1

    # TS/TSX files in frontend src/
    for dirpath, _, filenames in os.walk(frontend_src):
        # Skip node_modules
        if "node_modules" in dirpath.split(os.sep):
            continue
        for f in filenames:
            if f.endswith((".ts", ".tsx")):
                fp = os.path.join(dirpath, f)
                if add_header(fp, HEADER_TS):
                    ts_files_modified += 1

    print(f"Modified {rust_files_modified} Rust files")
    print(f"Modified {ts_files_modified} TS/TSX files")
    print(f"Total: {rust_files_modified + ts_files_modified} files")

if __name__ == "__main__":
    main()
