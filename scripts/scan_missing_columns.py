#!/usr/bin/env python3
"""Robust scan: check each entity field against v100 DDL by simple text matching."""

import os, re, sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent

ENTITY_DIR = _PROJECT_ROOT / "src-tauri/crates/entities/src"
V100_FILE  = _PROJECT_ROOT / "src-tauri/crates/dao/src/migrations/v100_consolidated.rs"
V101_FILE  = _PROJECT_ROOT / "src-tauri/crates/dao/src/migrations/v101_route_history.rs"
V102_FILE  = _PROJECT_ROOT / "src-tauri/crates/dao/src/migrations/v102_skill_failure_fields.rs"

TYPE_MAP = {
    "String": "TEXT",
    "i32": "INTEGER",
    "i64": "BIGINT",
    "bool": "INTEGER",
    "f32": "REAL",
    "f64": "REAL",
    "Vec<u8>": "BLOB",
}

# ── Load all DDL content ──
v100_text = V100_FILE.read_text(encoding="utf-8")
v101_text = V101_FILE.read_text(encoding="utf-8")
v102_text = V102_FILE.read_text(encoding="utf-8")

# ── Build lookup: for each table, which columns are mentioned in the DDL ──
def find_ddl_columns(text: str) -> dict[str, set[str]]:
    """Check all CREATE TABLE IF NOT EXISTS <table> (...) and ALTER TABLE <table> ADD COLUMN <col>."""
    result: dict[str, set[str]] = {}

    # ALTER TABLE ADD COLUMN
    for m in re.finditer(r'ALTER\s+TABLE\s+(\w+)\s+ADD\s+(?:COLUMN\s+)?(\w+)', text, re.IGNORECASE):
        result.setdefault(m.group(1).lower(), set()).add(m.group(2).lower())

    # CREATE TABLE — find each table's block
    for m in re.finditer(r'CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\w+)', text):
        table = m.group(1)
        name = table.lower()
        # Simple: all words (column-like identifiers) between this table name and the next table name
        rest = text[m.end():]
        # Find the closing paren of this CREATE TABLE
        paren_start = rest.find('(')
        if paren_start < 0:
            continue
        depth = 1
        i = paren_start + 1
        while i < len(rest) and depth > 0:
            if rest[i] == '(':
                depth += 1
            elif rest[i] == ')':
                depth -= 1
            i += 1
        block = rest[paren_start+1:i-1]
        # Extract all potential column names (word followed by word)
        for col_m in re.finditer(r'(\w+)\s+(\w+)', block):
            col_name = col_m.group(1).lower()
            type_word = col_m.group(2).upper()
            if col_name not in ('primary', 'foreign', 'unique', 'constraint',
                                 'check', 'key', 'fulltext', 'using', 'not',
                                 'default', 'references', 'on', 'generated',
                                 'always', 'stored'):
                result.setdefault(name, set()).add(col_name)
        # Also check for additional ALTER TABLE on this table after the DDL block
        for am in re.finditer(r'ALTER\s+TABLE\s+' + re.escape(table) + r'\s+ADD\s+(?:COLUMN\s+)?(\w+)', text, re.IGNORECASE):
            result.setdefault(name, set()).add(am.group(1).lower())

    return result

ddl_cols = find_ddl_columns(v100_text)
# Merge v101
v101_cols = find_ddl_columns(v101_text)
for k, v in v101_cols.items():
    ddl_cols.setdefault(k, set()).update(v)
# Merge v102
v102_cols = find_ddl_columns(v102_text)
for k, v in v102_cols.items():
    ddl_cols.setdefault(k, set()).update(v)

print(f"Found {len(ddl_cols)} tables in DDL", file=sys.stderr)
# Debug: check agency_experts
ae = ddl_cols.get('agency_experts', set())
print(f"agency_experts DDL cols ({len(ae)}): {sorted(ae)}", file=sys.stderr)

# ── Scan entities ──
missing = []
for f in sorted(ENTITY_DIR.glob("*.rs")):
    content = f.read_text(encoding="utf-8")

    m_tn = re.search(r'#\[sea_orm\(table_name\s*=\s*"([^"]+)"\)\]', content)
    if not m_tn:
        continue
    table_name = m_tn.group(1)

    m_struct = re.search(r'pub struct Model \{(.+?)\}', content, re.DOTALL)
    if not m_struct:
        continue

    existing = ddl_cols.get(table_name.lower(), set())
    if not existing:
        missing.append((table_name, '(whole table)', '', False))
        continue

    for line in m_struct.group(1).split("\n"):
        line = line.strip()
        if not line or line.startswith("//") or line.startswith("#"):
            continue
        m_f = re.match(r'pub\s+(\w+)\s*:\s*(Option<)?(\w+)', line)
        if not m_f:
            continue
        field_name = m_f.group(1).lower()
        is_opt = bool(m_f.group(2))
        rust_type = m_f.group(3)
        if field_name not in existing:
            missing.append((table_name, field_name, rust_type, is_opt))

# ── Report ──
missing.sort(key=lambda x: (x[0], x[1]))
print(f"\n{'='*70}")
print(f"FOUND {len(missing)} MISSING COLUMN(S)")
print(f"{'='*70}")

# Group by table
from collections import OrderedDict
by_table: dict[str, list] = OrderedDict()
for table, col, rtype, is_opt in missing:
    by_table.setdefault(table, []).append((col, rtype, is_opt))

for table, cols in by_table.items():
    print(f"\n  {table}:")
    for col, rtype, is_opt in cols:
        ddl_type = TYPE_MAP.get(rtype, "TEXT")
        nullable = "" if is_opt else " NOT NULL"
        print(f"    {col}: {ddl_type}{nullable}")

    # Generate SQL
    print(f"\n    -- SQL:")
    for col, rtype, is_opt in cols:
        ddl_type = TYPE_MAP.get(rtype, "TEXT")
        nullable = "" if is_opt else " NOT NULL"
        print(f"    ALTER TABLE {table} ADD COLUMN {col} {ddl_type}{nullable};")
