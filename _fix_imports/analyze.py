import re

BACKTICK = chr(96)
ERROR_FILE = r"d:\OneManager\AxAgent\_fix_imports\cargo_errors.txt"

with open(ERROR_FILE, "r", encoding="utf-8", errors="replace") as f:
    lines = f.readlines()

err_map = {}

for i, line in enumerate(lines):
    # Match "cannot find type|macro|value|function `Name`" or "cannot find module or crate `Name`"
    m = re.search("cannot find (type|macro|value|function) .([^" + BACKTICK + r"]+).", line)
    m2 = re.search("cannot find module or crate .([^" + BACKTICK + r"]+).", line)

    current_file = None
    err_name = None
    err_type = None

    if m:
        err_type = m.group(1)
        err_name = m.group(2)
    elif m2:
        err_type = "module_or_crate"
        err_name = m2.group(1)

    if err_name:
        # Look FORWARD for the file path
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
            key = err_name
            if key not in err_map:
                err_map[key] = {}
                err_map[key]["files"] = set()
                err_map[key]["types"] = set()
            err_map[key]["files"].add(current_file)
            err_map[key]["types"].add(err_type)

print(f"=== All missing items ({len(err_map)} types) sorted by frequency ===\n")
for err_name, data in sorted(err_map.items(), key=lambda x: -len(x[1]["files"])):
    files = sorted(data["files"])
    types = data["types"]
    print(f"{err_name} ({len(files)} files, types: {', '.join(sorted(types))}):")
    for f in files:
        print(f"  {f}")
    print()
