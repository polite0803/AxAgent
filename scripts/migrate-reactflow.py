#!/usr/bin/env python3
"""Batch replace 'reactflow' imports with '@xyflow/react' across the codebase."""

import os
import re

SRC = "D:/OneManager/AxAgent/src"
FILES_TO_CHECK = []

# Collect all TS/TSX files
for dirpath, _, filenames in os.walk(SRC):
    if "node_modules" in dirpath:
        continue
    for f in filenames:
        if f.endswith((".ts", ".tsx")):
            FILES_TO_CHECK.append(os.path.join(dirpath, f))

changed_files = 0
css_changed = 0

for fp in FILES_TO_CHECK:
    with open(fp, "r", encoding="utf-8") as f:
        content = f.read()
    
    original = content
    
    # Replace JS/TS imports
    content = re.sub(
        r'''from\s+["']reactflow["']''',
        'from "@xyflow/react"',
        content,
    )
    
    # Replace CSS imports
    content = content.replace(
        'reactflow/dist/style.css',
        '@xyflow/react/dist/style.css',
    )
    
    if content != original:
        changed_files += 1
        if 'reactflow/dist/style.css' in original:
            css_changed += 1
        with open(fp, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"  {os.path.relpath(fp, SRC)}")

print(f"\nChanged {changed_files} files ({css_changed} with CSS import)")
