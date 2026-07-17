#!/usr/bin/env python3
"""Comprehensive entity → DDL type mismatch scanner.

For each entity field, determines the correct PostgreSQL type that SeaORM
expects, then checks whether the DDL (after pg_ddl conversion) matches.

Assumes the entity is the source of truth.
"""

import os, re, sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent

ENTITY_DIR = _PROJECT_ROOT / "src-tauri/crates/entities/src"
V100_FILE  = _PROJECT_ROOT / "src-tauri/crates/dao/src/migrations/v100_consolidated.rs"

# SeaORM Rust type → expected PostgreSQL native type after pg_ddl
RUST_TO_PG = {
    "String":     "text",          # → TEXT
    "i32":        "integer",       # → INTEGER (INT4)
    "i64":        "bigint",        # → BIGINT (INT8)
    "bool":       "boolean",       # → BOOLEAN
    "f32":        "real",          # → REAL (FLOAT4)
    "f64":        "double precision",  # → DOUBLE PRECISION (FLOAT8)
    "Vec<u8>":    "bytea",         # → BYTEA on PG, BLOB on SQLite
}

# ── Load v100 DDL, normalize line continuations ──
v100_text = V100_FILE.read_text(encoding="utf-8")
v100_text = re.sub(r'\\\n', '', v100_text)  # Rust string continuations

# ── Extract CREATE TABLE columns ──
# Build dict: table_name → {column_name → ddl_type}
ddl_columns: dict[str, dict[str, str]] = {}

# Collect ALTER TABLE ADD COLUMN
for m in re.finditer(r'ALTER\s+TABLE\s+(\w+)\s+ADD\s+(?:COLUMN\s+)?(\w+)\s+(\w+)', v100_text, re.IGNORECASE):
    table = m.group(1).lower()
    col = m.group(2).lower()
    ddl_type = m.group(3).lower()
    ddl_columns.setdefault(table, {})[col] = ddl_type

# Collect CREATE TABLE
for m in re.finditer(r'CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\w+)\s*\(', v100_text):
    table = m.group(1).lower()
    start = m.end()
    depth = 1
    i = start
    while i < len(v100_text) and depth > 0:
        if v100_text[i] == '(':
            depth += 1
        elif v100_text[i] == ')':
            depth -= 1
        i += 1
    body = v100_text[start:i-1]
    
    for line in body.split('\n'):
        line = line.strip().rstrip(',').rstrip('\\')
        if not line:
            continue
        cm = re.match(r'(\w+)\s+(\w+)', line)
        if cm:
            col_name = cm.group(1).lower()
            type_word = cm.group(2).lower()
            if col_name not in ('primary', 'foreign', 'unique', 'constraint',
                                'check', 'key', 'fulltext', 'using', 'not',
                                'default', 'references', 'on', 'generated',
                                'always', 'stored'):
                ddl_columns.setdefault(table, {})[col_name] = type_word

# ── Apply pg_ddl transformations to DDL types ──
def pg_ddl_transform(type_word: str) -> str:
    """Simulate pg_ddl's type transformations."""
    t = type_word
    # REAL → DOUBLE PRECISION
    if t == 'real':
        return 'double precision'
    return t

# ── Scan entities ──
issues = []

for f in sorted(ENTITY_DIR.glob("*.rs")):
    content = f.read_text(encoding="utf-8")
    name = f.stem

    # Extract table_name
    m_tn = re.search(r'#\[sea_orm\(table_name\s*=\s*"([^"]+)"\)\]', content)
    if not m_tn:
        continue
    table_name = m_tn.group(1).lower()

    # Extract Model struct
    m_struct = re.search(r'pub struct Model \{(.+?)\}', content, re.DOTALL)
    if not m_struct:
        continue

    for line in m_struct.group(1).split("\n"):
        line = line.strip()
        if not line or line.startswith("//") or line.startswith("#["):
            continue
        m_f = re.match(r'pub\s+(\w+)\s*:\s*(Option<)?(\w+)', line)
        if not m_f:
            continue
        field_name = m_f.group(1).lower()
        is_opt = bool(m_f.group(2))
        rust_type = m_f.group(3)

        if field_name == 'id':
            continue  # PK is special

        expected_pg = RUST_TO_PG.get(rust_type)
        if expected_pg is None:
            continue

        # Check if table exists in DDL
        if table_name not in ddl_columns:
            issues.append((table_name, field_name, rust_type, expected_pg, 'TABLE_MISSING', ''))
            continue

        # Check if column exists in DDL
        table_cols = ddl_columns[table_name]
        if field_name not in table_cols:
            issues.append((table_name, field_name, rust_type, expected_pg, 'COLUMN_MISSING', ''))
            continue

        # Check if DDL type (after pg_ddl transform) matches expected PG type
        ddl_raw = table_cols[field_name]
        ddl_pg = pg_ddl_transform(ddl_raw)
        if ddl_pg != expected_pg:
            issues.append((table_name, field_name, rust_type, expected_pg, f'TYPE_MISMATCH', f'DDL raw={ddl_raw} → PG={ddl_pg}'))

# ── Report ──
issues.sort(key=lambda x: (x[0], x[1]))

print(f"\n{'='*70}")
print(f"TYPE MISMATCHES: {len(issues)} found")
print(f"{'='*70}")

types_to_fix = set()
for table, field, rtype, expected, issue, detail in issues:
    marker = "⚠️" if issue in ('TYPE_MISMATCH', 'COLUMN_MISSING') else "ℹ️"
    print(f"  {marker} {table}.{field}  entity:{rtype} → PG:{expected}  [{issue}] {detail}")
    if issue in ('TYPE_MISMATCH', 'COLUMN_MISSING'):
        types_to_fix.add((table, field, rtype, expected))

if types_to_fix:
    print(f"\n{'='*70}")
    print(f"SUMMARY: DDL changes needed for {len(types_to_fix)} columns")
    print(f"{'='*70}")
    for table, field, rtype, expected in sorted(types_to_fix):
        print(f"  {table}.{field}: entity={rtype} needs DDL type={expected.upper()}")
