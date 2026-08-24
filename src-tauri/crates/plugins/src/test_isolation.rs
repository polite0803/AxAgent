// SPDX-License-Identifier: AGPL-3.0-only

// Test isolation utilities for plugin tests
// ROADMAP #41: Stop ambient plugin state from skewing CLI regression checks

#![allow(clippy::disallowed_types)]

use std::env;
use std::path::PathBuf;
// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，仅在同步 lock() 内操作。
#[allow(clippy::disallowed_types)]
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，测试环境锁为同步临界区。
#[allow(clippy::disallowed_types)]
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Lock for test environment isolation
pub struct EnvLock {
    // SAFETY: 此处 parking_lot::MutexGuard 不跨 await 使用，仅在持有锁期间操作。
    #[allow(clippy::disallowed_types)]
    _guard: parking_lot::MutexGuard<'static, ()>,
    temp_home: PathBuf,
}

impl EnvLock {
    /// Acquire environment lock for test isolation
    pub fn lock() -> Self {
        let guard = ENV_LOCK.lock();
        let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_home = std::env::temp_dir().join(format!("plugin-test-{count}"));

        // Set up isolated environment
        std::fs::create_dir_all(&temp_home).expect("failed to create temp_home");
        std::fs::create_dir_all(temp_home.join(".claude/plugins/installed"))
            .expect("failed to create plugins dir");
        std::fs::create_dir_all(temp_home.join(".config")).expect("failed to create .config dir");

        // Redirect HOME and XDG_CONFIG_HOME to temp directory
        // SAFETY: Test-only code; EnvLock ensures exclusive access and restores
        // original values on drop; HOME/XDG_CONFIG_HOME/XDG_DATA_HOME are set to
        // isolated temp directories to prevent test pollution.
        unsafe { env::set_var("HOME", &temp_home) };
        unsafe { env::set_var("XDG_CONFIG_HOME", temp_home.join(".config")) };
        unsafe { env::set_var("XDG_DATA_HOME", temp_home.join(".local/share")) };

        EnvLock { _guard: guard, temp_home }
    }

    /// Get the temporary home directory for this test
    #[must_use]
    pub fn temp_home(&self) -> &PathBuf {
        &self.temp_home
    }
}

impl Drop for EnvLock {
    fn drop(&mut self) {
        // Cleanup temp directory
        std::fs::remove_dir_all(&self.temp_home).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_lock_creates_isolated_home_and_directories() {
        let lock = EnvLock::lock();
        let home = env::var("HOME").expect("测试：var 应成功");
        assert!(home.contains("plugin-test-"));
        assert_eq!(home, lock.temp_home().to_str().expect("测试：路径转字符串应成功"));

        let plugins_dir = lock.temp_home().join(".claude/plugins/installed");
        assert!(plugins_dir.exists(), "plugins dir should exist at {:?}", plugins_dir,);
    }
}
