// SPDX-License-Identifier: AGPL-3.0-only

//! Rhai AST 全局缓存（单例）。
//!
//! 避免重复编译静态脚本（如 portfolio-mgr.rhai / data-quality.rhai 等）。
//! 在批量股票分析场景下，N 只股票会触发 N 次工作流执行，
//! 若每次都重新编译 1373 行的 portfolio-mgr.rhai，会造成显著的 CPU 浪费。
//!
//! # 设计
//!
//! - 全局 `OnceLock<RwLock<HashMap<String, Arc<AST>>>>`，进程级单例
//! - key = SHA256(code) 前 16 hex 字符（code 变化时自动失效，旧条目保留但不再命中）
//! - 编译时需要一个 `&Engine` 引用（调用 `engine.compile(code)`）
//! - 缓存的 AST 可被任何 Engine 执行（`engine.eval_ast_with_scope(&ast)`），
//!   函数在 eval_ast 时按 Engine 查找，与 AST 本身无关
//! - AST 实现 Send + Sync（rhai 开启 sync feature），可安全跨线程共享
//!
//! # 线程安全
//!
//! 使用 `std::sync::RwLock`（非 `tokio::sync::RwLock`），因为：
//! 1. 读写操作不跨 await（编译是同步操作）
//! 2. 主要在 `spawn_blocking` 上下文中调用
//! 3. 锁持有时间极短（HashMap 查找或单次插入）
//!
//! AGENTS.md 禁区第 8 条针对"跨 await 持有锁"的场景，本模块不属此列。

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use rhai::AST;

/// 全局 AST 缓存（进程级单例）。
///
/// key = code 的 SHA256 短 hash，value = 编译后的 AST（Arc 共享）。
fn ast_cache() -> &'static RwLock<HashMap<String, Arc<AST>>> {
    static CACHE: OnceLock<RwLock<HashMap<String, Arc<AST>>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 计算 code 的短 hash（SHA256 前 8 字节 = 16 hex 字符），用作缓存 key。
///
/// 用 SHA256 而非 DefaultHasher：DefaultHasher 每次进程启动结果不同
/// （RandomState 种子随机），无法跨进程复用；SHA256 确定性更高，便于诊断。
fn code_hash(code: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8])
}

/// 获取或编译 AST。
///
/// - `cache_key`：脚本逻辑名（如 "portfolio-mgr"、"data-quality"），仅用于日志诊断
/// - `code`：Rhai 脚本源码
/// - `engine`：用于编译的 Engine（需注册脚本依赖的函数）
///
/// 返回 `Arc<AST>`，可被任何 Engine 执行（函数在 eval_ast 时按 Engine 查找）。
///
/// # 线程安全
///
/// 内部用 RwLock，读多写少。编译只在首次或 code 变化时发生。
/// 锁不跨 await，使用 std::sync::RwLock 即可。
///
/// # 性能
///
/// - 缓存命中：1 次 RwLock 读锁 + 1 次 HashMap 查找 + 1 次 Arc clone
/// - 缓存未命中：1 次 RwLock 读锁（未命中）+ 1 次 engine.compile + 1 次 RwLock 写锁 + 1 次 HashMap insert
///
/// 在批量分析 100 只股票的场景下，portfolio-mgr.rhai（1373 行）只编译 1 次，
/// 后续 99 次命中缓存，节省约 99 × ~5ms = ~500ms 的编译开销。
pub fn get_or_compile_ast(
    cache_key: &str,
    code: &str,
    engine: &rhai::Engine,
) -> Result<Arc<AST>, String> {
    let hash = code_hash(code);

    // 先尝试读缓存（读锁，允许多线程并发读）
    {
        let cache = ast_cache().read().unwrap_or_else(|e| e.into_inner());
        if let Some(ast) = cache.get(&hash) {
            tracing::debug!(
                cache_key = cache_key,
                hash = %hash,
                "[harness::rhai_ast_cache] 缓存命中"
            );
            return Ok(ast.clone());
        }
    }

    // 缓存未命中，编译 AST（编译是 CPU 密集操作，但不持锁）
    tracing::info!(
        cache_key = cache_key,
        hash = %hash,
        code_len = code.len(),
        "[harness::rhai_ast_cache] 缓存未命中，编译 AST"
    );
    let ast = engine
        .compile(code)
        .map_err(|e| format!("Rhai 编译失败 (cache_key={cache_key}, hash={hash}): {e}"))?;
    let ast = Arc::new(ast);

    // 写入缓存（写锁，短暂持有）
    {
        let mut cache = ast_cache().write().unwrap_or_else(|e| e.into_inner());
        // Double-check：其他线程可能已在此期间编译并写入
        if let Some(existing) = cache.get(&hash) {
            return Ok(existing.clone());
        }
        cache.insert(hash, ast.clone());
    }

    Ok(ast)
}

/// 清除所有缓存的 AST。
///
/// 仅供测试使用。生产环境中 AST 缓存是进程级单例，无需清除
/// （code 变化时会自动产生新 key，旧条目不再命中）。
#[cfg(test)]
pub fn clear_cache() {
    let mut cache = ast_cache().write().unwrap_or_else(|e| e.into_inner());
    cache.clear();
}

/// 返回当前缓存条目数（用于诊断/测试）。
pub fn cache_size() -> usize {
    let cache = ast_cache().read().unwrap_or_else(|e| e.into_inner());
    cache.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 测试串行锁：全局 AST 缓存是进程级单例，并行测试会互相干扰。
    // 用 Mutex 保证同一时间只有一个测试操作缓存。
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn cache_hit_avoids_recompile() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_cache();
        let mut engine = rhai::Engine::new();
        crate::register_common_functions(&mut engine);
        let code = "let x = 1 + 2; x";

        // 首次编译
        let ast1 = get_or_compile_ast("test", code, &engine).unwrap();
        assert_eq!(cache_size(), 1);

        // 第二次应命中缓存（Arc 指针相等）
        let ast2 = get_or_compile_ast("test", code, &engine).unwrap();
        assert_eq!(cache_size(), 1);
        assert!(Arc::ptr_eq(&ast1, &ast2));
    }

    #[test]
    fn code_change_invalidates_cache() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_cache();
        let mut engine = rhai::Engine::new();
        crate::register_common_functions(&mut engine);

        let code1 = "let x = 1; x";
        let code2 = "let x = 2; x";

        let ast1 = get_or_compile_ast("test", code1, &engine).unwrap();
        let ast2 = get_or_compile_ast("test", code2, &engine).unwrap();

        // code 不同 → 两个缓存条目
        assert_eq!(cache_size(), 2);
        // AST 指针不同
        assert!(!Arc::ptr_eq(&ast1, &ast2));
    }

    #[test]
    fn same_code_different_cache_key_shares_ast() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // 相同 code 用不同 cache_key 调用，应共享同一 AST（key 只用 code hash）
        clear_cache();
        let mut engine = rhai::Engine::new();
        crate::register_common_functions(&mut engine);

        let code = "let x = 42; x";
        let ast1 = get_or_compile_ast("script-a", code, &engine).unwrap();
        let ast2 = get_or_compile_ast("script-b", code, &engine).unwrap();

        // 相同 code → 只有一个缓存条目
        assert_eq!(cache_size(), 1);
        // Arc 指针相等（共享 AST）
        assert!(Arc::ptr_eq(&ast1, &ast2));
    }

    #[test]
    fn cached_ast_executes_correctly() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // 验证缓存的 AST 能被任意 Engine 正确执行
        clear_cache();
        let mut engine = rhai::Engine::new();
        crate::register_common_functions(&mut engine);

        let code = "clamp(15.0, 0.0, 10.0)";
        let ast = get_or_compile_ast("clamp-test", code, &engine).unwrap();

        // 用另一个 Engine 执行缓存的 AST
        let mut engine2 = rhai::Engine::new();
        crate::register_common_functions(&mut engine2);
        let mut scope = rhai::Scope::new();
        let result: f64 = engine2.eval_ast_with_scope(&mut scope, &ast).expect("AST 执行失败");
        assert_eq!(result, 10.0);
    }
}
