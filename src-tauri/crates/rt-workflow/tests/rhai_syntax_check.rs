//! 语法编译测试：用 rhai v1.25.0 编译 src/commands 下所有 .rhai 文件。
//!
//! 目的：CI 阶段就一次性捕获所有 Rhai 语法错误（如 `as f64`/`let mut` 等
//! Rust 残留语法），避免运行时才报错导致反复修复。
//!
//! 注意：本测试只编译（engine.compile），不执行。输入变量全部注入空值 (),
//! 因为 compile 阶段不需要变量实际有值——只需要语法合法。
use rhai::Engine;
use std::path::PathBuf;

/// 注册所有脚本中用到的全局辅助函数（与运行时注册的函数保持一致）。
fn register_globals(engine: &mut Engine) {
    engine.register_fn("clamp", |v: f64, min: f64, max: f64| -> f64 {
        if v < min {
            min
        } else if v > max {
            max
        } else {
            v
        }
    });
    engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
        arr.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(sep)
    });
    // json_parse 返回 unit 即可，编译阶段不解析 JSON
    engine.register_fn("json_parse", |_s: &str| -> rhai::Dynamic { rhai::Dynamic::UNIT });
    // print 在测试中无操作
    engine.register_fn("print", |_s: &str| {});
}

/// 列出 src/commands 下所有 .rhai 文件路径。
fn collect_rhai_files() -> Vec<PathBuf> {
    // CARGO_MANIFEST_DIR = src-tauri/crates/rt-workflow
    // 需要到 src-tauri/src/commands，即向上两级再进入 src/commands
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("src")
        .join("commands");
    std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("读取 rhai 目录失败 {:?}: {e}", dir))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|ext| ext == "rhai").unwrap_or(false))
        .collect()
}

/// 对单个 .rhai 文件做编译检查。返回 Ok(()) 或 Err(错误信息)。
fn compile_one(path: &PathBuf) -> Result<(), String> {
    let code = std::fs::read_to_string(path).map_err(|e| format!("读取失败: {e}"))?;
    let mut engine = Engine::new();
    engine.set_max_expr_depths(1024, 1024);
    register_globals(&mut engine);
    // 编译阶段不需要注入变量值——未定义变量在 compile 时不会报错
    // （Rhai 的变量解析在运行时）。这里只测语法合法性。
    engine.compile(&code).map(|_| ()).map_err(|e| format!("{e}"))
}

#[test]
fn all_rhai_scripts_compile() {
    let files = collect_rhai_files();
    assert!(!files.is_empty(), "未找到任何 .rhai 文件，测试目录可能配置错误");

    let mut failures = Vec::new();
    for f in &files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        match compile_one(f) {
            Ok(()) => eprintln!("=== PARSE OK: {name} ==="),
            Err(e) => failures.push(format!("[{name}] {e}")),
        }
    }

    if !failures.is_empty() {
        panic!(
            "以下 .rhai 文件编译失败 (共 {}/{}):\n\n{}",
            failures.len(),
            files.len(),
            failures.join("\n\n")
        );
    }
}

/// 兼容旧测试名：单独验证 bottleneck-calc.rhai 仍可编译。
#[test]
fn bottleneck_calc_v9_compiles() {
    let code = include_str!("../../../src/commands/bottleneck-calc.rhai");
    let mut engine = Engine::new();
    engine.set_max_expr_depths(1024, 1024);
    register_globals(&mut engine);
    match engine.compile(code) {
        Ok(_) => eprintln!("=== PARSE OK ==="),
        Err(e) => panic!("编译失败: {e}"),
    }
}
