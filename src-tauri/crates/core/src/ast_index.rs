//! AST-based structured code index for semantic code search.
//!
//! Extracts function signatures, class definitions, interface declarations,
//! variable declarations, and call relationships from source code using
//! pattern-based parsing (upgradeable to tree-sitter).
//!
//! Stores extracted definitions in SQLite for fast semantic matching during
//! the L2 phase of the three-level recall pipeline.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub id: String,
    pub file_path: String,
    pub name: String,
    pub signature: String,
    pub line_start: usize,
    pub line_end: usize,
    pub visibility: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDef {
    pub id: String,
    pub file_path: String,
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub language: String,
    pub parent_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDef {
    pub id: String,
    pub file_path: String,
    pub name: String,
    pub line_start: usize,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDecl {
    pub id: String,
    pub file_path: String,
    pub name: String,
    pub type_annotation: Option<String>,
    pub line: usize,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller_file: String,
    pub caller_function: String,
    pub callee_name: String,
    pub line: usize,
}

pub struct AstIndex {
    pub(crate) conn: Connection,
}

impl std::fmt::Debug for AstIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstIndex").finish_non_exhaustive()
    }
}

impl AstIndex {
    pub fn new(conn: Connection) -> Result<Self, String> {
        let index = Self { conn };
        index.ensure_tables()?;
        Ok(index)
    }

    fn ensure_tables(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS ast_functions (
                    id TEXT PRIMARY KEY,
                    file_path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    signature TEXT NOT NULL DEFAULT '',
                    line_start INTEGER NOT NULL DEFAULT 0,
                    line_end INTEGER NOT NULL DEFAULT 0,
                    visibility TEXT NOT NULL DEFAULT '',
                    language TEXT NOT NULL DEFAULT ''
                );
                CREATE INDEX IF NOT EXISTS idx_ast_fn_name ON ast_functions(name);
                CREATE INDEX IF NOT EXISTS idx_ast_fn_file ON ast_functions(file_path);

                CREATE TABLE IF NOT EXISTS ast_classes (
                    id TEXT PRIMARY KEY,
                    file_path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    line_start INTEGER NOT NULL DEFAULT 0,
                    line_end INTEGER NOT NULL DEFAULT 0,
                    language TEXT NOT NULL DEFAULT '',
                    parent_class TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_ast_cls_name ON ast_classes(name);

                CREATE TABLE IF NOT EXISTS ast_interfaces (
                    id TEXT PRIMARY KEY,
                    file_path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    line_start INTEGER NOT NULL DEFAULT 0,
                    language TEXT NOT NULL DEFAULT ''
                );

                CREATE TABLE IF NOT EXISTS ast_variables (
                    id TEXT PRIMARY KEY,
                    file_path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    type_annotation TEXT,
                    line INTEGER NOT NULL DEFAULT 0,
                    language TEXT NOT NULL DEFAULT ''
                );

                CREATE TABLE IF NOT EXISTS ast_call_edges (
                    caller_file TEXT NOT NULL,
                    caller_function TEXT NOT NULL,
                    callee_name TEXT NOT NULL,
                    line INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_ast_edge_callee ON ast_call_edges(callee_name);
                CREATE INDEX IF NOT EXISTS idx_ast_edge_file ON ast_call_edges(caller_file);",
            )
            .map_err(|e| format!("Failed to create AST tables: {e}"))
    }

    /// Index a file's AST, replacing previous entries for this file.
    pub fn index_file(&self, file_path: &str, content: &str) -> Result<usize, String> {
        let lang = detect_language(file_path);
        let functions = extract_functions(content, file_path, lang);
        let classes = extract_classes(content, file_path, lang);
        let interfaces = extract_interfaces(content, file_path, lang);
        let variables = extract_variables(content, file_path, lang);
        let call_edges = extract_call_edges(content, file_path, &functions);

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| format!("tx: {e}"))?;

        tx.execute(
            "DELETE FROM ast_functions WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(|e| format!("delete fn: {e}"))?;
        tx.execute(
            "DELETE FROM ast_classes WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(|e| format!("delete cls: {e}"))?;
        tx.execute(
            "DELETE FROM ast_interfaces WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(|e| format!("delete iface: {e}"))?;
        tx.execute(
            "DELETE FROM ast_variables WHERE file_path = ?1",
            params![file_path],
        )
        .map_err(|e| format!("delete var: {e}"))?;
        tx.execute(
            "DELETE FROM ast_call_edges WHERE caller_file = ?1",
            params![file_path],
        )
        .map_err(|e| format!("delete edge: {e}"))?;

        let mut total = 0;

        for f in &functions {
            tx.execute(
                "INSERT INTO ast_functions (id, file_path, name, signature, line_start, line_end, visibility, language) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![f.id, f.file_path, f.name, f.signature, f.line_start, f.line_end, f.visibility, f.language],
            ).map_err(|e| format!("insert fn: {e}"))?;
            total += 1;
        }
        for c in &classes {
            tx.execute(
                "INSERT INTO ast_classes (id, file_path, name, line_start, line_end, language, parent_class) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![c.id, c.file_path, c.name, c.line_start, c.line_end, c.language, c.parent_class],
            ).map_err(|e| format!("insert cls: {e}"))?;
            total += 1;
        }
        for i in &interfaces {
            tx.execute(
                "INSERT INTO ast_interfaces (id, file_path, name, line_start, language) VALUES (?1,?2,?3,?4,?5)",
                params![i.id, i.file_path, i.name, i.line_start, i.language],
            ).map_err(|e| format!("insert iface: {e}"))?;
            total += 1;
        }
        for v in &variables {
            tx.execute(
                "INSERT INTO ast_variables (id, file_path, name, type_annotation, line, language) VALUES (?1,?2,?3,?4,?5,?6)",
                params![v.id, v.file_path, v.name, v.type_annotation, v.line, v.language],
            ).map_err(|e| format!("insert var: {e}"))?;
            total += 1;
        }
        for e in &call_edges {
            tx.execute(
                "INSERT INTO ast_call_edges (caller_file, caller_function, callee_name, line) VALUES (?1,?2,?3,?4)",
                params![e.caller_file, e.caller_function, e.callee_name, e.line],
            ).map_err(|e| format!("insert edge: {e}"))?;
            total += 1;
        }

        tx.commit().map_err(|e| format!("commit: {e}"))?;
        Ok(total)
    }

    /// Remove all AST entries for a given file.
    pub fn remove_file(&self, file_path: &str) -> Result<(), String> {
        self.conn
            .execute_batch(&format!(
                "DELETE FROM ast_functions WHERE file_path = '{0}';
                 DELETE FROM ast_classes WHERE file_path = '{0}';
                 DELETE FROM ast_interfaces WHERE file_path = '{0}';
                 DELETE FROM ast_variables WHERE file_path = '{0}';
                 DELETE FROM ast_call_edges WHERE caller_file = '{0}';",
                file_path.replace('\'', "''")
            ))
            .map_err(|e| format!("remove_file: {e}"))?;
        Ok(())
    }

    /// Search functions by name (partial match).
    pub fn search_functions(&self, query: &str, limit: usize) -> Result<Vec<FunctionDef>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, file_path, name, signature, line_start, line_end, visibility, language FROM ast_functions WHERE name LIKE ?1 OR signature LIKE ?1 LIMIT ?2")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map(params![format!("%{query}%"), limit], |row| {
                Ok(FunctionDef {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    name: row.get(2)?,
                    signature: row.get(3)?,
                    line_start: row.get(4)?,
                    line_end: row.get(5)?,
                    visibility: row.get(6)?,
                    language: row.get(7)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    /// Search classes by name.
    pub fn search_classes(&self, query: &str, limit: usize) -> Result<Vec<ClassDef>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, file_path, name, line_start, line_end, language, parent_class FROM ast_classes WHERE name LIKE ?1 LIMIT ?2")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map(params![format!("%{query}%"), limit], |row| {
                Ok(ClassDef {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    name: row.get(2)?,
                    line_start: row.get(3)?,
                    line_end: row.get(4)?,
                    language: row.get(5)?,
                    parent_class: row.get(6)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    /// Find callers of a function.
    pub fn find_callers(&self, function_name: &str) -> Result<Vec<CallEdge>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT caller_file, caller_function, callee_name, line FROM ast_call_edges WHERE callee_name = ?1")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map(params![function_name], |row| {
                Ok(CallEdge {
                    caller_file: row.get(0)?,
                    caller_function: row.get(1)?,
                    callee_name: row.get(2)?,
                    line: row.get(3)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(results)
    }

    /// Search for matching definitions across all types.
    ///
    /// Returns file paths that contain definitions matching the query.
    pub fn search_all(&self, query: &str, limit: usize) -> Result<Vec<String>, String> {
        let pattern = format!("%{query}%");
        let mut results = std::collections::HashSet::new();

        for table in &[
            "ast_functions",
            "ast_classes",
            "ast_interfaces",
            "ast_variables",
        ] {
            let sql = format!("SELECT DISTINCT file_path FROM {table} WHERE name LIKE ?1 LIMIT ?2");
            let mut stmt = self
                .conn
                .prepare(&sql)
                .map_err(|e| format!("prepare {table}: {e}"))?;
            let rows = stmt
                .query_map(params![&pattern, limit], |row| row.get::<_, String>(0))
                .map_err(|e| format!("query {table}: {e}"))?;
            for row in rows {
                if let Ok(path) = row {
                    results.insert(path);
                }
            }
        }

        let mut sorted: Vec<String> = results.into_iter().collect();
        sorted.truncate(limit);
        Ok(sorted)
    }

    /// Get the total count of indexed definitions.
    pub fn total_definitions(&self) -> Result<usize, String> {
        let fn_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM ast_functions", [], |r| r.get(0))
            .unwrap_or(0);
        let cls_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM ast_classes", [], |r| r.get(0))
            .unwrap_or(0);
        Ok((fn_count + cls_count) as usize)
    }
}

fn detect_language(file_path: &str) -> &str {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".rs") {
        "rust"
    } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        "typescript"
    } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
        "javascript"
    } else if lower.ends_with(".py") {
        "python"
    } else if lower.ends_with(".go") {
        "go"
    } else if lower.ends_with(".java") {
        "java"
    } else if lower.ends_with(".cpp") || lower.ends_with(".cc") || lower.ends_with(".cxx") {
        "cpp"
    } else if lower.ends_with(".c") || lower.ends_with(".h") {
        "c"
    } else {
        "unknown"
    }
}

fn extract_functions(content: &str, file_path: &str, lang: &str) -> Vec<FunctionDef> {
    let mut functions = Vec::new();
    let mut func_id = 0;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let vis = detect_visibility(trimmed);

        let fn_name = match lang {
            "rust" => extract_rust_fn_name(trimmed),
            "python" => extract_python_fn_name(trimmed),
            "typescript" | "javascript" => extract_ts_fn_name(trimmed),
            "go" => extract_go_fn_name(trimmed),
            _ => None,
        };

        if let Some(name) = fn_name {
            if is_common_keyword(&name) {
                continue;
            }
            func_id += 1;
            let sig = trimmed.chars().take(200).collect::<String>();
            let end_line = find_block_end(content, line_num);
            functions.push(FunctionDef {
                id: format!("{}_{}", file_path.replace(['/', '\\'], "_"), func_id),
                file_path: file_path.to_string(),
                name,
                signature: sig,
                line_start: line_num + 1,
                line_end: end_line + 1,
                visibility: vis.to_string(),
                language: lang.to_string(),
            });
        }
    }
    functions
}

fn extract_classes(content: &str, file_path: &str, lang: &str) -> Vec<ClassDef> {
    let mut classes = Vec::new();
    let mut cls_id = 0;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let cls_name = match lang {
            "rust" => extract_rust_struct_enum_name(trimmed),
            "python" => extract_python_class_name(trimmed),
            "typescript" | "javascript" => extract_ts_class_name(trimmed),
            "go" => extract_go_type_name(trimmed),
            _ => None,
        };

        if let Some(name) = cls_name {
            cls_id += 1;
            let end_line = find_block_end(content, line_num);
            classes.push(ClassDef {
                id: format!("{}_cls_{}", file_path.replace(['/', '\\'], "_"), cls_id),
                file_path: file_path.to_string(),
                name,
                line_start: line_num + 1,
                line_end: end_line + 1,
                language: lang.to_string(),
                parent_class: None,
            });
        }
    }
    classes
}

fn extract_interfaces(_content: &str, _file_path: &str, _lang: &str) -> Vec<InterfaceDef> {
    Vec::new()
}

fn extract_variables(content: &str, file_path: &str, lang: &str) -> Vec<VariableDecl> {
    let mut vars = Vec::new();
    let mut var_id = 0;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let var_info = extract_variable_declaration(trimmed, lang);
        if let Some((name, type_ann)) = var_info {
            if is_common_keyword(&name) || name.len() < 2 {
                continue;
            }
            var_id += 1;
            vars.push(VariableDecl {
                id: format!("{}_var_{}", file_path.replace(['/', '\\'], "_"), var_id),
                file_path: file_path.to_string(),
                name,
                type_annotation: type_ann,
                line: line_num + 1,
                language: lang.to_string(),
            });
        }
    }
    vars
}

fn extract_call_edges(content: &str, file_path: &str, functions: &[FunctionDef]) -> Vec<CallEdge> {
    let mut edges = Vec::new();
    let fn_names: std::collections::HashSet<&str> =
        functions.iter().map(|f| f.name.as_str()).collect();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("--") {
            continue;
        }

        for fn_name in &fn_names {
            if trimmed.contains(*fn_name)
                && !trimmed.contains(&format!("fn {}", fn_name))
                && !trimmed.contains(&format!("def {}", fn_name))
                && !trimmed.contains(&format!("function {}", fn_name))
            {
                let caller = functions
                    .iter()
                    .find(|f| f.line_start <= line_num + 1 && f.line_end >= line_num + 1);
                if let Some(caller_fn) = caller {
                    edges.push(CallEdge {
                        caller_file: file_path.to_string(),
                        caller_function: caller_fn.name.clone(),
                        callee_name: fn_name.to_string(),
                        line: line_num + 1,
                    });
                }
            }
        }
    }
    edges
}

// ── Language-specific extractors ──────────────────────────────────────────

fn detect_visibility(line: &str) -> &str {
    if line.starts_with("pub ") || line.starts_with("pub(") {
        "public"
    } else if line.starts_with("pub(crate) ") {
        "crate"
    } else {
        "private"
    }
}

fn extract_rust_fn_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("fn ")
        && !line.starts_with("pub fn ")
        && !line.starts_with("pub(crate) fn ")
        && !line.starts_with("async fn ")
        && !line.starts_with("pub async fn ")
    {
        return None;
    }
    let after_fn = line.split("fn ").nth(1)?;
    let name = after_fn.split(['(', '<']).next()?.trim();
    if name.is_empty() || name == "fn" {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_rust_struct_enum_name(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("struct ") || line.starts_with("pub struct ") {
        let after = line.split("struct ").nth(1)?;
        let name = after.split(['<', '{', '(', ';']).next()?.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else if line.starts_with("enum ") || line.starts_with("pub enum ") {
        let after = line.split("enum ").nth(1)?;
        let name = after.split(['<', '{']).next()?.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else if line.starts_with("trait ") || line.starts_with("pub trait ") {
        let after = line.split("trait ").nth(1)?;
        let name = after.split(['<', '{']).next()?.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else if line.starts_with("impl ") {
        let after = line.trim_start_matches("impl ");
        let name = after.split(['<', ' ', '{']).next()?.trim();
        if name == "for" {
            after
                .split("for ")
                .nth(1)?
                .split(['<', '{'])
                .next()
                .map(|n| n.trim().to_string())
        } else if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else {
        None
    }
}

fn extract_python_fn_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("def ") && !line.starts_with("async def ") {
        return None;
    }
    let after = if line.starts_with("async def ") {
        line.split("async def ").nth(1)?
    } else {
        line.split("def ").nth(1)?
    };
    let name = after.split(['(', ':']).next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_python_class_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("class ") {
        return None;
    }
    let after = line.strip_prefix("class ")?;
    let name = after.split(['(', ':']).next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_ts_fn_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.contains("function ") && !line.contains("=>") {
        return None;
    }
    if line.starts_with("function ") {
        let after = line.strip_prefix("function ")?;
        let name = after.split(['(', '<']).next()?.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else if line.starts_with("export function ") {
        let after = line.strip_prefix("export function ")?;
        let name = after.split(['(', '<']).next()?.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else if line.starts_with("export const ") || line.starts_with("const ") {
        let after = if line.starts_with("export const ") {
            line.strip_prefix("export const ")?
        } else {
            line.strip_prefix("const ")?
        };
        let name = after.split(['=', ':', '(']).next()?.trim();
        if name.contains("=>") || name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else {
        None
    }
}

fn extract_ts_class_name(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("class ") || line.starts_with("export class ") {
        let after = if line.starts_with("export class ") {
            line.strip_prefix("export class ")?
        } else {
            line.strip_prefix("class ")?
        };
        let name = after.split(['<', '{', ' ', ':']).next()?.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    } else {
        None
    }
}

fn extract_go_fn_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("func ") {
        return None;
    }
    let after = line.strip_prefix("func ")?;
    let rest = if after.starts_with('(') {
        after.split(')').nth(1)?.trim_start()
    } else {
        after
    };
    let name = rest.split(['(', '<']).next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_go_type_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("type ") {
        return None;
    }
    let after = line.strip_prefix("type ")?;
    let name = after.split([' ', '[', '{']).next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_variable_declaration(line: &str, lang: &str) -> Option<(String, Option<String>)> {
    let line = line.trim();
    match lang {
        "rust" => {
            if line.starts_with("let ") && !line.contains('=') {
                return None;
            }
            if line.starts_with("let mut ") || line.starts_with("let ") {
                let after = line
                    .strip_prefix("let mut ")
                    .or_else(|| line.strip_prefix("let "))?;
                let name = after.split(['=', ':', ' ']).next()?.trim();
                let type_ann = after
                    .split(':')
                    .nth(1)
                    .and_then(|t| t.split('=').next())
                    .map(|t| t.trim().to_string());
                Some((name.to_string(), type_ann))
            } else {
                None
            }
        },
        "typescript" | "javascript" => {
            if line.starts_with("let ") || line.starts_with("var ") || line.starts_with("const ") {
                let after = line
                    .trim_start_matches("export ")
                    .trim_start_matches("let ")
                    .trim_start_matches("var ")
                    .trim_start_matches("const ");
                let name = after.split(['=', ':', ' ']).next()?.trim();
                if name.is_empty() || name == "{" {
                    return None;
                }
                let type_ann = after
                    .split(':')
                    .nth(1)
                    .and_then(|t| t.split('=').next())
                    .map(|t| t.trim().to_string());
                Some((name.to_string(), type_ann))
            } else {
                None
            }
        },
        "python" => {
            if line.contains('=')
                && !line.starts_with("if ")
                && !line.starts_with("for ")
                && !line.starts_with("while ")
                && !line.starts_with("def ")
                && !line.starts_with("class ")
            {
                let name = line.split('=').next()?.trim();
                if name.is_empty() || name.contains(' ') {
                    return None;
                }
                Some((name.to_string(), None))
            } else {
                None
            }
        },
        _ => None,
    }
}

fn is_common_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "for"
            | "while"
            | "match"
            | "switch"
            | "case"
            | "return"
            | "break"
            | "continue"
            | "true"
            | "false"
            | "None"
            | "Some"
            | "Ok"
            | "Err"
            | "self"
            | "Self"
            | "super"
            | "this"
            | "new"
            | "use"
            | "mod"
            | "crate"
            | "pub"
            | "async"
            | "await"
            | "let"
            | "const"
            | "var"
            | "import"
            | "export"
            | "from"
            | "try"
            | "catch"
            | "finally"
            | "throw"
            | "yield"
            | "with"
            | "type"
            | "interface"
            | "enum"
            | "struct"
            | "impl"
            | "trait"
            | "fn"
            | "def"
            | "class"
            | "function"
            | "static"
            | "public"
            | "private"
            | "protected"
            | "final"
            | "abstract"
            | "override"
    )
}

fn find_block_end(content: &str, start: usize) -> usize {
    let mut depth: i32 = 0;
    let mut started = false;
    for (i, line) in content.lines().enumerate() {
        if i < start {
            continue;
        }
        let trimmed = line.trim();
        let opens = trimmed.matches('{').count() as i32 + trimmed.matches("do").count() as i32;
        let closes = trimmed.matches('}').count() as i32;

        if i == start {
            if trimmed.ends_with('{') || trimmed.ends_with(':') {
                started = true;
            } else if !trimmed.contains('{') && !trimmed.ends_with(':') {
                return i; // Single-line definition
            }
        }

        if started {
            depth += opens - closes;
            if depth <= 0 && i > start {
                return i;
            }
        }
    }
    content.lines().count().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_index() -> AstIndex {
        let conn = Connection::open_in_memory().unwrap();
        AstIndex::new(conn).unwrap()
    }

    #[test]
    fn test_extract_rust_functions() {
        let code = "fn main() {\n    println!(\"hello\");\n}\n\npub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        let idx = test_index();
        idx.index_file("/test.rs", code).unwrap();
        let results = idx.search_functions("add", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "add");
        assert!(results[0].signature.contains("pub fn add"));
    }

    #[test]
    fn test_extract_rust_structs() {
        let code = "pub struct User {\n    name: String,\n}\n\nenum Color { Red, Blue }\n";
        let idx = test_index();
        idx.index_file("/test.rs", code).unwrap();
        let results = idx.search_classes("User", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "User");

        let all = idx.search_classes("Color", 10).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_search_all() {
        let code = "fn calculate() -> u32 { 42 }\nfn render() {}\nstruct Widget {}\n";
        let idx = test_index();
        idx.index_file("/test.rs", code).unwrap();
        let results = idx.search_all("calc", 10).unwrap();
        assert!(results.contains(&"/test.rs".to_string()));
    }

    #[test]
    fn test_extract_python() {
        let code = "def hello():\n    print('hi')\n\nclass MyClass:\n    pass\n";
        let idx = test_index();
        idx.index_file("/test.py", code).unwrap();
        let fns = idx.search_functions("hello", 10).unwrap();
        assert_eq!(fns[0].name, "hello");
        let cls = idx.search_classes("MyClass", 10).unwrap();
        assert_eq!(cls[0].name, "MyClass");
    }
}
