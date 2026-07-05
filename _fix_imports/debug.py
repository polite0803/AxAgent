import re
import os

BACKTICK = chr(96)
ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"

with open(ERROR_FILE, "r", encoding="utf-8", errors="replace") as f:
    lines = f.readlines()

print(f"Total lines: {len(lines)}")

count = 0
for i, line in enumerate(lines[:20]):
    line_repr = repr(line[:120])
    print(f"  {i}: {line_repr}")

print("\n--- Searching for cannot find ---")
for i, line in enumerate(lines):
    if "cannot find" in line:
        count += 1
        if count <= 5:
            print(f"\nLine {i}: {repr(line[:120])}")
            for j in range(i-3, i):
                print(f"  prev {j}: {repr(lines[j][:120])}")
            # Try the regex
            m = re.search("cannot find (type|macro|value|function|module) .([^" + BACKTICK + r"]+).", line)
            if m:
                print(f"  REGEX MATCH: {m.groups()}")
            else:
                print(f"  REGEX NO MATCH")

print(f"\nTotal 'cannot find' occurrences: {count}")
