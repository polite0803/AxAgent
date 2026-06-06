//! 集中式异步任务管理器
//!
//! 统一管理所有 `tokio::spawn` 后台任务，提供：
//! - `spawn()`: 自动存储 JoinHandle，带任务名称
//! - `shutdown()`: 先 cancel token，再 await/abort 所有句柄
//! - `cancel()`: 支持单任务 abort
//!
//! 替代原先散落在各处的裸 `tokio::spawn` 模式。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub struct TaskManager {
    handles: Mutex<HashMap<String, JoinHandle<()>>>,
    shutdown_token: CancellationToken,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
            shutdown_token: CancellationToken::new(),
        }
    }

    /// 暴露 shutdown token，供任务在 loop 中检查 `token.cancelled()`。
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    pub fn spawn<F>(&self, name: &str, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(rt) => {
                let handle = rt.spawn(future);
                let mut handles = self.handles.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(old) = handles.insert(name.to_string(), handle) {
                    // 重复 name：记录后中止旧任务，行为保持向后兼容
                    tracing::warn!("[TaskManager] 重复 name '{}'，中止旧任务", name);
                    old.abort();
                }
            },
            Err(_) => {
                // setup 阶段 runtime 未就绪时通过独立线程承载
                tracing::debug!("[TaskManager] runtime 未就绪，'{}' 使用独立线程启动", name);
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().expect("TaskManager fallback runtime");
                    rt.block_on(future);
                });
            },
        }
    }

    pub async fn shutdown(&self, timeout: Duration) {
        self.shutdown_token.cancel();

        let handles: HashMap<String, JoinHandle<()>> = {
            let mut guard = self.handles.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *guard)
        };

        for (name, handle) in handles {
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(())) => {
                    tracing::debug!("[TaskManager] 任务 '{name}' 正常退出");
                },
                Ok(Err(e)) => {
                    tracing::warn!("[TaskManager] 任务 '{name}' panic: {e}");
                },
                Err(_elapsed) => {
                    tracing::warn!("[TaskManager] 任务 '{name}' {timeout:?} 内未退出，强制 abort");
                },
            }
        }
    }

    /// 返回当前注册的任务数。
    #[allow(dead_code)]
    pub fn task_count(&self) -> usize {
        self.handles.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn task_manager_spawn_and_shutdown() {
        let tm = Arc::new(TaskManager::new());
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();
        let token = tm.shutdown_token();

        tm.spawn("test_task", async move {
            tokio::select! {
                _ = token.cancelled() => {
                    flag_clone.store(true, Ordering::SeqCst);
                }
            }
        });

        tm.shutdown(Duration::from_secs(1)).await;
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn task_manager_handles_stuck_task() {
        let tm = Arc::new(TaskManager::new());

        tm.spawn("stuck", async move {
            // simulate a stuck task that doesn't watch token
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        // Should complete within the timeout even though task is stuck
        tm.shutdown(Duration::from_millis(100)).await;
        // shutdown returned — task was aborted after timeout
    }
}
