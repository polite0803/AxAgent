import re

BACKTICK = chr(96)
ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"

with open(ERROR_FILE, "r", encoding="utf-8", errors="replace") as f:
    lines = f.readlines()

# Test patterns
patterns = [
    r"cannot find (type|macro|value|function|module) .([^" + BACKTICK + r"]+).",
]

# Check lines with "cannot find type"
print("=== Testing 'cannot find type' lines ===")
for i, line in enumerate(lines):
    if "cannot find type" in line or "cannot find macro" in line:
        print(f"\nLine {i}: {repr(line[:120])}")
        for p in patterns:
            m = re.search(p, line)
            if m:
                print(f"  MATCHED: type={m.group(1)}, name={m.group(2)}")
            else:
                print(f"  NO MATCH with pattern")
        break

# Count how many have type/macro keyword vs bare
type_count = 0
macro_count = 0
bare_count = 0
for line in lines:
    if "cannot find type " in line:
        type_count += 1
    elif "cannot find macro " in line:
        macro_count += 1
    elif "cannot find " + BACKTICK in line:
        bare_count += 1

print(f"\ntype: {type_count}, macro: {macro_count}, bare: {bare_count}")

# Now extract files for each error type
# Simple approach: just find all "cannot find type `X`" patterns
err_map = {}
backtick_re = re.compile("cannot find (type|macro|value|function|module) .([^" + BACKTICK + r"]+).")

for i, line in enumerate(lines):
    m = backtick_re.search(line)
    if m:
        err_name = m.group(2)
        current_file = None
        # Look FORWARD for the --> line (it comes after the error line)
        for j in range(i+1, min(i+5, len(lines))):
            stripped = lines[j].strip()
            if stripped.startswith("--> "):
                raw = stripped[4:].strip()
                raw = raw.split(":")[0]  # Remove line:col
                current_file = raw.replace("\\", "/")
                if "src-tauri/src/" in current_file:
                    current_file = current_file.split("src-tauri/src/", 1)[1]
                break
        if current_file:
            if err_name not in err_map:
                err_map[err_name] = set()
            err_map[err_name].add(current_file)

print(f"\n=== Errors by type ({len(err_map)} types) ===")
for err_name, files in sorted(err_map.items(), key=lambda x: -len(x[1])):
    if len(files) >= 2:
        print(f"\n{err_name} ({len(files)} files):")
        for f in sorted(files)[:5]:
            print(f"  {f}")
        if len(files) > 5:
            print(f"  ... and {len(files)-5} more")
