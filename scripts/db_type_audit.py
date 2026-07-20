#!/usr/bin/env python3
"""AxAgent 数据库三处一致性审计（DDL ↔ SeaORM 实体 ↔ 真实库）。

严格按宽度区分类型类别（见铁律），绝不把 REAL 当 DOUBLE、绝不把 i32 当 i64：

  类别        DDL 关键字                          Rust 类型
  ----------  ----------------------------------  ------------------------
  INT4        INTEGER / INT / INT4 / SERIAL       i32 / u32
  INT8        BIGINT / INT8 / BIGSERIAL           i64 / u64
  FLOAT4      REAL / FLOAT4                        f32
  FLOAT8      DOUBLE PRECISION / FLOAT8 / DOUBLE   f64
  BOOL        BOOLEAN / BOOL                       bool
  TEXT        TEXT / VARCHAR / CHAR                String
  BYTEA       BYTEA / BLOB                         Vec<u8>
  NUMERIC     NUMERIC / DECIMAL                    (f64 或 String，需业务判定)

跨宽度（INT4↔INT8 / FLOAT4↔FLOAT8 / BOOL↔INT* / INT↔FLOAT）一律 ERROR。

用法:
  python scripts/db_type_audit.py            # 静态审计
  python scripts/db_type_audit.py <sqlite.db># 附加真实库 introspect
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENTITY_DIR = ROOT / "src-tauri/crates/entities/src"
MIG_DIR = ROOT / "src-tauri/crates/dao/src/migrations"

# ── 类型归一化 ────────────────────────────────────────────────────────────

# DDL 关键字 → 类别。多词类型先匹配长的。
DDL_TO_CAT = [
    ("double precision", "FLOAT8"),
    ("float8", "FLOAT8"),
    ("float4", "FLOAT4"),
    ("real", "FLOAT4"),
    ("bigserial", "INT8"),
    ("bigint", "INT8"),
    ("int8", "INT8"),
    ("smallint", "INT4"),
    ("serial", "INT4"),
    ("integer", "INT4"),
    ("int4", "INT4"),
    ("int", "INT4"),
    ("boolean", "BOOL"),
    ("bool", "BOOL"),
    ("bytea", "BYTEA"),
    ("blob", "BYTEA"),
    ("numeric", "NUMERIC"),
    ("decimal", "NUMERIC"),
    ("tsvector", "TSVECTOR"),
    ("varchar", "TEXT"),
    ("char", "TEXT"),
    ("text", "TEXT"),
    ("jsonb", "TEXT"),
    ("json", "TEXT"),
    # PG 的 double 不带 precision 时按 DOUBLE 处理（不出现在本项目，保险）
    ("double", "FLOAT8"),
]

# Rust 类型 → 类别
RUST_TO_CAT = {
    "i32": "INT4", "u32": "INT4", "i16": "INT4", "u16": "INT4", "i8": "INT4", "u8": "INT4",
    "i64": "INT8", "u64": "INT8", "isize": "INT8", "usize": "INT8",
    "f32": "FLOAT4",
    "f64": "FLOAT8",
    "bool": "BOOL",
    "String": "TEXT",
    "Vec<u8>": "BYTEA",
}

# SeaORM column_type override 值 → 类别
SEAORM_CT_TO_CAT = {
    "text": "TEXT", "string": "TEXT",
    "integer": "INT4", "unsigned": "INT4",
    "biginteger": "INT8", "bigunsigned": "INT8",
    "float": "FLOAT4",
    "double": "FLOAT8",
    "boolean": "BOOL",
    "binary": "BYTEA", "varbinary": "BYTEA",
    "decimal": "NUMERIC", "money": "NUMERIC",
}


def ddl_type_to_cat(word: str) -> str | None:
    w = word.strip().lower()
    for kw, cat in DDL_TO_CAT:
        if w == kw:
            return cat
    # 兜底：包含匹配（处理 varchar(255) 之类）
    for kw, cat in DDL_TO_CAT:
        if kw in w:
            return cat
    return None


# ── 解析迁移 DDL ──────────────────────────────────────────────────────────

STOP_WORDS = {
    # 注意：不含 "key"/"value"——它们是 settings/trajectory_preferences 的真实列名；
    # "PRIMARY KEY"/"FOREIGN KEY" 的整段以 primary/foreign 开头，不会以 key 起始。
    "primary", "foreign", "unique", "constraint", "check", "fulltext",
    "using", "not", "default", "references", "on", "generated", "always",
    "stored", "create", "index", "table",
}

# (table, column) → (ddl_type_word, source)
ddl_cols: dict[tuple[str, str], tuple[str, str]] = {}


def load_ddl():
    for mig in sorted(MIG_DIR.glob("*.rs")):
        text = mig.read_text(encoding="utf-8")
        # 合并 Rust 字符串行续（反斜杠换行）
        joined = re.sub(r"\\\s*\n", "", text)
        src = mig.name

        # CREATE TABLE 主体
        for m in re.finditer(
            r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\w+)\s*\(", joined, re.IGNORECASE
        ):
            table = m.group(1).lower()
            depth = 1
            i = m.end()
            while i < len(joined) and depth > 0:
                if joined[i] == "(":
                    depth += 1
                elif joined[i] == ")":
                    depth -= 1
                i += 1
            body = joined[m.start() : i]
            # 去掉最外层括号内容里的嵌套括号（如 numeric(10,2)/FK 列表）以简化按逗号切列
            body_inner = joined[m.end() : i - 1]
            # 按顶层逗号切分列定义
            parts = split_top_level(body_inner)
            for part in parts:
                part = part.strip()
                if not part:
                    continue
                cm = re.match(r"(\w+)\s+([A-Za-z][\w ]*?)(?:\s*\(|\s|,|$)", part)
                if not cm:
                    continue
                col = cm.group(1).lower()
                if col in STOP_WORDS:
                    continue
                # 类型可能是 "double precision" 两词
                type_word = extract_type_word(part[cm.start(1) + len(cm.group(1)):])
                if type_word:
                    ddl_cols[(table, col)] = (type_word, f"{src}:CREATE")

        # 静态 ALTER TABLE ... ADD COLUMN "<lit>"
        for m in re.finditer(
            r'ALTER\s+TABLE\s+(\w+)\s+ADD\s+(?:COLUMN\s+)?(?:IF\s+NOT\s+EXISTS\s+)?(\w+)\s+([A-Za-z][\w ]*)',
            joined, re.IGNORECASE,
        ):
            table = m.group(1).lower()
            col = m.group(2).lower()
            type_word = extract_type_word(m.group(3))
            if type_word:
                ddl_cols[(table, col)] = (type_word, f"{src}:ALTER")

        # 动态数组常量：const NAME: &[(&str,&str,&str)] 或 &[(&str,&str)]
        # 形式1: MISSING_COLUMN_TARGETS &[("table","col","TYPE"), ...]
        for m in re.finditer(
            r'\(\s*"(\w+)"\s*,\s*"(\w+)"\s*,\s*"([A-Za-z][\w ]*)"\s*\)', joined
        ):
            table, col, ty = m.group(1).lower(), m.group(2).lower(), m.group(3)
            tw = extract_type_word(ty)
            if tw:
                ddl_cols[(table, col)] = (tw, f"{src}:DYN3")

    # 形式2: agency_experts_columns &[("col","TYPE"), ...] —— 表名在 format! 里硬编码
    # 需针对具体循环手动登记。这里扫描 "ALTER TABLE <table> ADD COLUMN {} {}" + 邻近数组。
    scan_two_tuple_loops()


def split_top_level(s: str) -> list[str]:
    """按顶层逗号切分（忽略括号内的逗号）。"""
    out, depth, cur = [], 0, []
    for ch in s:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            out.append("".join(cur))
            cur = []
        else:
            cur.append(ch)
    if cur:
        out.append("".join(cur))
    return out


def extract_type_word(rest: str) -> str | None:
    """从列定义剩余串里提取类型（支持 'double precision' 两词）。"""
    rest = rest.strip()
    m2 = re.match(r"(double\s+precision)", rest, re.IGNORECASE)
    if m2:
        return "double precision"
    m1 = re.match(r"([A-Za-z][A-Za-z0-9]*)", rest)
    if m1:
        return m1.group(1)
    return None


def scan_two_tuple_loops():
    """处理 `let X: &[(&str,&str)] = &[("col","TYPE"),...]` 后接
    `format!("ALTER TABLE <table> ADD COLUMN {} {}", col, ty)` 的循环。
    通过 format! 里的表名把 2 元组数组归位到具体表。"""
    for mig in sorted(MIG_DIR.glob("*.rs")):
        text = mig.read_text(encoding="utf-8")
        src = mig.name
        # 找 format! 里硬编码表名的 ADD COLUMN {} {}
        for fm in re.finditer(
            r'ALTER\s+TABLE\s+(\w+)\s+ADD\s+COLUMN(?:\s+IF\s+NOT\s+EXISTS)?\s+\{\}\s+\{\}',
            text, re.IGNORECASE,
        ):
            table = fm.group(1).lower()
            # 向上找最近的 2 元组数组
            head = text[: fm.start()]
            arr_m = None
            for arr_m in re.finditer(r'&\[\s*((?:\(\s*"\w+"\s*,\s*"[^"]+"\s*\)\s*,?\s*)+)\]', head):
                pass  # 取最后一个
            if arr_m:
                for tm in re.finditer(r'\(\s*"(\w+)"\s*,\s*"([^"]+)"\s*\)', arr_m.group(1)):
                    col, ty = tm.group(1).lower(), tm.group(2)
                    tw = extract_type_word(ty)
                    if tw:
                        ddl_cols[(table, col)] = (tw, f"{src}:LOOP2")


# ── 解析实体 ──────────────────────────────────────────────────────────────

# (table, column) → (rust_cat, rust_raw, override_cat_or_None, entity_file, is_opt)
entity_cols: dict[tuple[str, str], tuple] = {}


def load_entities():
    for f in sorted(ENTITY_DIR.glob("*.rs")):
        content = f.read_text(encoding="utf-8")
        m_tn = re.search(r'#\[sea_orm\(table_name\s*=\s*"([^"]+)"\)\]', content)
        if not m_tn:
            continue
        table = m_tn.group(1).lower()
        m_struct = re.search(r"pub struct Model \{(.+?)\n\}", content, re.DOTALL)
        if not m_struct:
            continue
        body = m_struct.group(1)
        # 逐字段：收集其上方的 #[sea_orm(...)] 属性
        pending_override = None
        pending_col_name = None
        for raw in body.split("\n"):
            line = raw.strip()
            if not line or line.startswith("//"):
                continue
            if line.startswith("#["):
                mo = re.search(r'column_type\s*=\s*"(\w+)"', line)
                if mo:
                    pending_override = mo.group(1).lower()
                mn = re.search(r'column_name\s*=\s*"(\w+)"', line)
                if mn:
                    pending_col_name = mn.group(1).lower()
                continue
            mf = re.match(
                r"pub\s+(\w+)\s*:\s*(Option\s*<\s*)?([A-Za-z_][A-Za-z0-9_]*(?:\s*<[^>]+>)?)",
                line,
            )
            if not mf:
                pending_override = None
                pending_col_name = None
                continue
            field = (pending_col_name or mf.group(1)).lower()
            is_opt = bool(mf.group(2))
            rust_raw = mf.group(3)
            rust_cat = RUST_TO_CAT.get(rust_raw)
            ov_cat = SEAORM_CT_TO_CAT.get(pending_override) if pending_override else None
            entity_cols[(table, field)] = (rust_cat, rust_raw, ov_cat, f.name, is_opt)
            pending_override = None
            pending_col_name = None


# ── 比对 ──────────────────────────────────────────────────────────────────

def audit():
    errors, warns, infos = [], [], []

    for (table, col), (rust_cat, rust_raw, ov_cat, ef, is_opt) in sorted(entity_cols.items()):
        if col == "id":
            continue
        # 实体侧期望类别：override 优先
        exp_cat = ov_cat or rust_cat
        if exp_cat is None:
            continue  # 非标量（枚举/自定义），跳过

        key = (table, col)
        if key not in ddl_cols:
            infos.append((table, col, f"实体 {rust_raw}({exp_cat}) ↔ DDL 无此列(CREATE) — 可能由动态ALTER/其它迁移补充,需手查", ef))
            continue

        ddl_word, ddl_src = ddl_cols[key]
        ddl_cat = ddl_type_to_cat(ddl_word)
        if ddl_cat is None:
            infos.append((table, col, f"实体 {rust_raw}({exp_cat}) ↔ DDL '{ddl_word}' 无法识别类别 [{ddl_src}]", ef))
            continue

        if exp_cat == ddl_cat:
            continue

        msg = f"{table}.{col} : 实体 {rust_raw}{'(override '+ov_cat+')' if ov_cat else ''}(期望{exp_cat}) ↔ DDL {ddl_word}({ddl_cat}) [{ddl_src}]"

        # 分级
        pair = {exp_cat, ddl_cat}
        if pair == {"INT4", "INT8"} or pair == {"FLOAT4", "FLOAT8"} or \
           "BOOL" in pair and pair & {"INT4", "INT8"} or \
           pair & {"INT4", "INT8"} and pair & {"FLOAT4", "FLOAT8"}:
            errors.append(msg)
        elif ddl_cat == "NUMERIC" or exp_cat == "NUMERIC" or ddl_cat == "TSVECTOR":
            warns.append(msg)
        elif pair & {"TEXT"} and pair & {"INT4", "INT8", "FLOAT4", "FLOAT8", "BOOL"}:
            errors.append(msg)
        else:
            warns.append(msg)

    return errors, warns, infos


# ── 真实库 introspect（SQLite）────────────────────────────────────────────

SQLITE_AFFINITY = {
    # PRAGMA type → 类别（SQLite 声明类型保留原样，可直接映射）
}


def introspect_sqlite(db_path: str):
    import sqlite3
    print(f"\n{'='*72}\n真实库 introspect (SQLite): {db_path}\n{'='*72}")
    con = sqlite3.connect(db_path)
    cur = con.cursor()
    cur.execute("SELECT name FROM sqlite_master WHERE type='table'")
    tables = {r[0].lower() for r in cur.fetchall()}
    drift = []
    for (table, col), (rust_cat, rust_raw, ov_cat, ef, is_opt) in sorted(entity_cols.items()):
        if col == "id" or table not in tables:
            continue
        exp_cat = ov_cat or rust_cat
        if exp_cat is None:
            continue
        cur.execute(f"PRAGMA table_info({table})")
        real = {r[1].lower(): r[2] for r in cur.fetchall()}
        if col not in real:
            continue
        real_cat = ddl_type_to_cat(real[col]) if real[col] else None
        if real_cat and real_cat != exp_cat:
            drift.append(f"{table}.{col} : 实体期望{exp_cat} ↔ 真实库 {real[col]}({real_cat}) — 库列漂移")
    con.close()
    if drift:
        print(f"发现 {len(drift)} 处真实库漂移:")
        for d in drift:
            print("  ERROR " + d)
    else:
        print("真实库列类型与实体期望一致，无漂移。")


# ── 真实库 introspect（PostgreSQL）────────────────────────────────────────

# 类别 → PG udt_name 归一化 CASE（供 SQL / Python 共用语义）
PG_UDT_TO_CAT = {
    "int2": "INT4", "int4": "INT4", "serial": "INT4",
    "int8": "INT8", "bigserial": "INT8",
    "float4": "FLOAT4",
    "float8": "FLOAT8",
    "bool": "BOOL",
    "text": "TEXT", "varchar": "TEXT", "bpchar": "TEXT", "json": "TEXT", "jsonb": "TEXT",
    "bytea": "BYTEA",
    "numeric": "NUMERIC",
    "tsvector": "TSVECTOR",
}


def _expected_pairs() -> list[tuple[str, str, str]]:
    """返回 (table, col, expected_cat) 列表（跳过 id 和非标量）。"""
    out = []
    for (table, col), (rust_cat, rust_raw, ov_cat, ef, is_opt) in sorted(entity_cols.items()):
        if col == "id":
            continue
        exp_cat = ov_cat or rust_cat
        if exp_cat is None:
            continue
        out.append((table, col, exp_cat))
    return out


def emit_pg_check_sql():
    """生成自包含 SQL：把实体期望类别做成 VALUES，与真实库 udt_name 归一化后比对。
    在远程 PG（psql / DBeaver）执行，直接列出漂移列。"""
    pairs = _expected_pairs()
    values = ",\n  ".join(
        f"('{t}','{c}','{cat}')" for (t, c, cat) in pairs
    )
    case_lines = "\n".join(
        f"      WHEN udt_name IN ({', '.join(repr(u) for u,cc in PG_UDT_TO_CAT.items() if cc==cat)}) THEN '{cat}'"
        for cat in ["INT4", "INT8", "FLOAT4", "FLOAT8", "BOOL", "TEXT", "BYTEA", "NUMERIC", "TSVECTOR"]
    )
    sql = f"""-- AxAgent DB 类型漂移检查（实体期望 vs 真实 PG 列类型）
-- 在生产 PG 上执行；返回行 = 存在漂移的列（应为空）。
WITH expected(tbl, col, cat) AS (VALUES
  {values}
),
actual AS (
  SELECT table_name AS tbl, column_name AS col, udt_name,
    CASE
{case_lines}
      ELSE udt_name
    END AS cat
  FROM information_schema.columns
  WHERE table_schema = 'public'
)
SELECT e.tbl, e.col, e.cat AS expected, a.cat AS actual, a.udt_name AS real_udt
FROM expected e
JOIN actual a ON e.tbl = a.tbl AND e.col = a.col
WHERE e.cat <> a.cat
ORDER BY e.tbl, e.col;
"""
    print(f"\n{'='*72}\nPG 漂移检查 SQL（复制到远程 psql/DBeaver 执行）\n{'='*72}")
    print(sql)


def introspect_pg(dsn: str):
    """直连远程 PG，跑与 emit_pg_check_sql 相同语义的比对。
    需 psycopg(v3) 或 psycopg2；缺驱动时提示改用 --emit-pg-sql。"""
    print(f"\n{'='*72}\n真实库 introspect (PostgreSQL)\n{'='*72}")
    conn = None
    try:
        try:
            import psycopg  # type: ignore
            conn = psycopg.connect(dsn)
        except ImportError:
            import psycopg2  # type: ignore
            conn = psycopg2.connect(dsn)
    except ImportError:
        print("未安装 psycopg/psycopg2，无法直连。请改用：")
        print("  python scripts/db_type_audit.py --emit-pg-sql")
        print("将生成的 SQL 复制到远程 psql/DBeaver 执行。")
        return
    except Exception as e:  # noqa: BLE001
        print(f"连接失败: {e}")
        print("可改用 --emit-pg-sql 手动在远程执行。")
        return

    cur = conn.cursor()
    cur.execute(
        "SELECT table_name, column_name, udt_name FROM information_schema.columns "
        "WHERE table_schema='public'"
    )
    real: dict[tuple[str, str], str] = {
        (r[0].lower(), r[1].lower()): r[2].lower() for r in cur.fetchall()
    }
    conn.close()

    drift = []
    for (table, col, exp_cat) in _expected_pairs():
        udt = real.get((table, col))
        if udt is None:
            continue
        real_cat = PG_UDT_TO_CAT.get(udt, udt)
        if real_cat != exp_cat:
            drift.append(
                f"{table}.{col} : 实体期望{exp_cat} ↔ 真实库 {udt}({real_cat}) — 库列漂移"
            )
    if drift:
        print(f"发现 {len(drift)} 处真实库漂移:")
        for d in drift:
            print("  ✗ ERROR " + d)
    else:
        print("真实库列类型与实体期望一致，无漂移。")


# ── main ──────────────────────────────────────────────────────────────────

def main():
    load_ddl()
    load_entities()
    errors, warns, infos = audit()

    print(f"{'='*72}")
    print(f"AxAgent DB 类型三处一致性审计")
    print(f"  DDL 列: {len(ddl_cols)}  |  实体列: {len(entity_cols)}")
    print(f"{'='*72}")

    print(f"\n### ERROR ({len(errors)}) — 解码/写入必失败，必修")
    for e in errors:
        print("  ✗ " + e)
    if not errors:
        print("  (无)")

    print(f"\n### WARN ({len(warns)}) — 潜在溢出/精度/需业务判定")
    for w in warns:
        print("  ! " + w)
    if not warns:
        print("  (无)")

    print(f"\n### INFO ({len(infos)}) — CREATE TABLE 无此列（需手查动态ALTER/其它迁移）")
    for i_ in infos:
        print(f"  · {i_[0]}.{i_[1]}  [{i_[3]}] {i_[2]}")
    if not infos:
        print("  (无)")

    for arg in sys.argv[1:]:
        if arg.startswith("postgres://") or arg.startswith("postgresql://"):
            introspect_pg(arg)
        elif arg == "--emit-pg-sql":
            emit_pg_check_sql()
        elif Path(arg).exists():
            introspect_sqlite(arg)

    print(f"\n{'='*72}")
    print(f"结论: ERROR={len(errors)} WARN={len(warns)} INFO={len(infos)}")
    print(f"{'='*72}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
