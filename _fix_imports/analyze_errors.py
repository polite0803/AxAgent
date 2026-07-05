#!/usr/bin/env python3
import re

with open('/tmp/cargo_errors.txt', 'r') as f:
    lines = f.readlines()

errors = {}
backtick = chr(96)  # backtick character

for i, line in enumerate(lines):
    pattern = "cannot find (type|macro|value|function|module) [" + backtick + "]([^" + backtick + "]+)[" + backtick + "]"
    m = re.search(pattern, line)
    if m:
        err_type = m.group(1)
        err_name = m.group(2)
        current_file = None
        for j in range(i-1, max(i-5, -1), -1):
            fm = re.search(r'-->\s+(.+):(\d+):(\d+)', lines[j])
            if fm:
                current_file = fm.group(1).replace('\\', '/')
                break
        if current_file:
            key = err_name + " (" + err_type + ")"
            if key not in errors:
                errors[key] = set()
            errors[key].add(current_file)

for err, files in sorted(errors.items(), key=lambda x: -len(x[1])):
    print("\n=== " + err + " (" + str(len(files)) + " files) ===")
    for f in sorted(files):
        print("  " + f)
