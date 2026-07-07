// SPDX-License-Identifier: AGPL-3.0-only
//
//! 通用 tokio::spawn 任务保护工具
//!
//! 推广自 `commands/conversations/messages/streaming/mod.rs::spawn_stream_task`
//! 的 Drop guard + catch_unwind + 错误事件模式，适用于所有"长生命周期 / 持有外部
//! 资源 / 需要状态机收敛"的 spawn 任务。
//!
//! ## 三个原语
//!
//! - [`SpawnGuard`]：RAII guard。drop 时若未 [`SpawnGuard::finish`]，执行 on_drop 兜底。
//!   用于"panic / 早退 / return"路径上的资源清理、状态收敛、事件上报。
//!
//! - [`catch_unwind_logged`]：仅 catch_unwind + panic 日志。适用于"无外部资源，
//!   panic 只丢部分输出"的 fire-and-forget 任务（如 stdout 读取循环）。
//!
//! - [`panic_message`]：把 catch_unwind 捕获的 `Box<dyn Any + Send>` 负载转成可读字符串。
//!
//! ## 典型用法
//!
//! ```ignore
//! tokio::spawn(async move {
//!     let guard = SpawnGuard::new("my_task", || {
//!         // 兜底：兜底写库 / emit 失败事件 / 清理 cancel_flag
//!     });
//!     let result = AssertUnwindSafe(async {
//!         // 业务
//!         guard.finish();
//!     }).catch_unwind().await;
//!     if let Err(p) = result {
//!         tracing::error!("[my_task] PANIC: {}", panic_message(&p));
//!     }
//! });
//! ```

use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::FutureExt;

/// catch_unwind 捕获的 panic 负载转可读字符串。
/// 优先匹配 `String` / `&'static str`，其它类型降级为占位文本。
pub fn panic_message(panic: &Box<dyn Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = panic.downcast_ref::<&'static str>() {
        (*s).to_owned()
    } else {
        "Unknown panic".to_string()
    }
}

/// tokio::spawn 任务的 RAII 保护。
///
/// 业务正常完成时调用 [`SpawnGuard::finish`]；任何其它路径退出（panic / 早退 /
/// 早 return）drop 时执行 `on_drop` 兜底回调一次。
pub struct SpawnGuard {
    name: &'static str,
    finished: AtomicBool,
    on_drop: Option<Box<dyn FnOnce() + Send>>,
}

impl SpawnGuard {
    /// 创建 guard。`on_drop` 在 drop 时执行一次（前提是未 [`Self::finish`]）。
    pub fn new(name: &'static str, on_drop: impl FnOnce() + Send + 'static) -> Self {
        Self { name, finished: AtomicBool::new(false), on_drop: Some(Box::new(on_drop)) }
    }

    /// 业务正常完成，阻止 drop 兜底回调。
    /// 注意：调用后即使后续代码 panic，兜底也不再触发。
    pub fn finish(&self) {
        self.finished.store(true, Ordering::Release);
    }
}

impl Drop for SpawnGuard {
    fn drop(&mut self) {
        if !self.finished.swap(true, Ordering::AcqRel) {
            tracing::error!("[{}] panic guard fired (no business finish)", self.name);
            if let Some(cb) = self.on_drop.take() {
                cb();
            }
        }
    }
}

/// catch_unwind 包裹 future，panic 时仅记录错误日志（不调用任何业务兜底）。
///
/// 适用于"无外部资源、panic 只丢部分输出"的纯计算/IO 读取循环，例如：
/// - stdout/stderr 实时打印循环
/// - 周期心跳 / 指标采集
pub async fn catch_unwind_logged<F>(name: &'static str, fut: F)
where
    F: Future<Output = ()>,
{
    let result = AssertUnwindSafe(fut).catch_unwind().await;
    if let Err(p) = result {
        tracing::error!("[{}] PANIC: {}", name, panic_message(&p));
    }
}
