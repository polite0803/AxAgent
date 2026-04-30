//! Resource governor — dynamic CPU/memory monitoring and adaptive throttling.
//!
//! Integrates with `sysinfo` to monitor system resource usage in real-time.
//! Implements the following behaviors:
//!
//! - **Idle state**: CPU < 50%, memory < 60% → background tasks (index updates,
//!   cache maintenance) run silently without impacting the foreground.
//! - **Moderate load**: CPU 50-80% → index updates continue at lower priority.
//! - **High load**: CPU > 80% or memory > 85% → pause all background tasks,
//!   trim memory caches, prioritize only the current user-facing operation.
//! - **Critical**: Memory > 95% → emergency cache flush, release all non-essential
//!   resources immediately.
//!
//! # Integration
//!
//! The governor tracks global system metrics via `sysinfo::System`. Integrators
//! should call `tick()` periodically (every 1-5 seconds) in the main event loop
//! and react to the returned `ResourceState` by pausing or resuming background tasks.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::System;

/// The current system resource state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceState {
    /// System is idle — run background tasks freely.
    Idle,
    /// Moderate load — run background tasks at reduced priority.
    Moderate,
    /// High load — pause background tasks, trim caches.
    High,
    /// Critical — emergency flush, release all non-essential memory.
    Critical,
}

/// Governor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorConfig {
    pub cpu_idle_threshold: f32,
    pub cpu_high_threshold: f32,
    pub memory_moderate_threshold: f32,
    pub memory_high_threshold: f32,
    pub memory_critical_threshold: f32,
    pub tick_interval_ms: u64,
    pub enable_adaptive_throttling: bool,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            cpu_idle_threshold: 50.0,
            cpu_high_threshold: 80.0,
            memory_moderate_threshold: 60.0,
            memory_high_threshold: 85.0,
            memory_critical_threshold: 95.0,
            tick_interval_ms: 2000,
            enable_adaptive_throttling: true,
        }
    }
}

/// Current resource metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub free_memory_mb: u64,
    pub state: ResourceState,
    pub background_tasks_frozen: bool,
}

pub struct ResourceGovernor {
    config: GovernorConfig,
    system: System,
    current_state: ResourceState,
    background_tasks_frozen: Arc<AtomicBool>,
    last_check_succeeded: bool,
}

impl ResourceGovernor {
    pub fn new(config: GovernorConfig) -> Self {
        Self {
            config,
            system: System::new_all(),
            current_state: ResourceState::Idle,
            background_tasks_frozen: Arc::new(AtomicBool::new(false)),
            last_check_succeeded: true,
        }
    }

    /// Run one monitoring tick. Updates system info and returns the current
    /// resource state. Call this on a periodic timer (every 1-5 seconds).
    pub fn tick(&mut self) -> ResourceMetrics {
        self.system.refresh_all();

        let cpu_percent = self.system.global_cpu_usage();
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let free_memory = self.system.free_memory();
        let memory_percent = if total_memory > 0 {
            (used_memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        self.current_state = self.classify_state(cpu_percent, memory_percent);
        self.last_check_succeeded = total_memory > 0;

        self.apply_throttling();

        ResourceMetrics {
            cpu_percent,
            memory_percent,
            total_memory_mb: total_memory / (1024 * 1024),
            used_memory_mb: used_memory / (1024 * 1024),
            free_memory_mb: free_memory / (1024 * 1024),
            state: self.current_state,
            background_tasks_frozen: self.background_tasks_frozen.load(Ordering::Relaxed),
        }
    }

    fn classify_state(&self, cpu: f32, memory: f32) -> ResourceState {
        if memory >= self.config.memory_critical_threshold {
            ResourceState::Critical
        } else if cpu >= self.config.cpu_high_threshold
            || memory >= self.config.memory_high_threshold
        {
            ResourceState::High
        } else if cpu >= self.config.cpu_idle_threshold
            || memory >= self.config.memory_moderate_threshold
        {
            ResourceState::Moderate
        } else {
            ResourceState::Idle
        }
    }

    fn apply_throttling(&mut self) {
        if !self.config.enable_adaptive_throttling {
            return;
        }

        let should_freeze = matches!(self.current_state, ResourceState::High | ResourceState::Critical);
        self.background_tasks_frozen.store(should_freeze, Ordering::Relaxed);

        if matches!(self.current_state, ResourceState::Critical) {
            tracing::warn!(
                "Resource governor: Critical state — emergency cache flush recommended (CPU: {:.1}%, Memory: {:.1}%)",
                self.system.global_cpu_usage(),
                self.system.used_memory() as f32 / self.system.total_memory().max(1) as f32 * 100.0
            );
        }
    }

    /// Check whether background tasks should be paused.
    pub fn should_pause_background_tasks(&self) -> bool {
        self.background_tasks_frozen.load(Ordering::Relaxed)
    }

    /// Get a shared flag that external components can check to decide whether
    /// to pause background work.
    pub fn background_freeze_flag(&self) -> Arc<AtomicBool> {
        self.background_tasks_frozen.clone()
    }

    /// Get the current resource state.
    pub fn current_state(&self) -> ResourceState {
        self.current_state
    }

    /// Pause background tasks (code index updates, cache maintenance, etc.).
    /// External systems should check `should_pause_background_tasks()` and
    /// suspend their work accordingly.
    pub fn pause_background_tasks(&self) {
        self.background_tasks_frozen.store(true, Ordering::Relaxed);
        tracing::info!("Resource governor: Background tasks paused");
    }

    /// Resume background tasks after a high-load period subsides.
    pub fn resume_background_tasks(&self) {
        self.background_tasks_frozen.store(false, Ordering::Relaxed);
        tracing::info!("Resource governor: Background tasks resumed");
    }

    /// Request an emergency memory trim — release all caches that can be
    /// reconstructed later. Callers should react by clearing their L1/L2
    /// caches.
    pub fn trim_memory(&self) {
        tracing::warn!(
            "Resource governor: Memory trim requested (state={:?})",
            self.current_state
        );
    }

    /// Whether the last monitoring check succeeded.
    pub fn healthy(&self) -> bool {
        self.last_check_succeeded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_idle() {
        let governor = ResourceGovernor::new(GovernorConfig::default());
        assert_eq!(governor.classify_state(10.0, 30.0), ResourceState::Idle);
    }

    #[test]
    fn test_classify_high() {
        let governor = ResourceGovernor::new(GovernorConfig::default());
        assert_eq!(governor.classify_state(85.0, 50.0), ResourceState::High);
        assert_eq!(governor.classify_state(30.0, 90.0), ResourceState::High);
    }

    #[test]
    fn test_classify_critical() {
        let config = GovernorConfig::default();
        let governor = ResourceGovernor::new(config);
        assert_eq!(governor.classify_state(95.0, 98.0), ResourceState::Critical);
    }

    #[test]
    fn test_background_freeze_flag() {
        let governor = ResourceGovernor::new(GovernorConfig::default());
        let flag = governor.background_freeze_flag();

        assert!(!governor.should_pause_background_tasks());

        governor.pause_background_tasks();
        assert!(flag.load(Ordering::Relaxed));
        assert!(governor.should_pause_background_tasks());

        governor.resume_background_tasks();
        assert!(!flag.load(Ordering::Relaxed));
    }
}
