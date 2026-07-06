// SPDX-License-Identifier: AGPL-3.0-only

//! Reliability macros used to convert hard-panics into logged warnings.
//!
//! The macros in this module are the *gentle* counterpart of `.unwrap()` /
//! `.expect()`: when the operation would have panicked, they log a
//! structured warning at WARN level and return a configurable fallback
//! value.

/// Try to unwrap an `Option`/`Result`, logging a WARN and returning
/// `default` on failure.
#[macro_export]
macro_rules! try_unwrap_or_log {
    ($expr:expr, default = $default:expr, $($msg:tt)+) => {
        match $expr {
            Some(v) | Ok(v) => v,
            None | Err(e) => {
                tracing::warn!(
                    target: "axagent.reliability",
                    value = ?e,
                    "{} (defaulting)",
                    format!($($msg)+)
                );
                $default
            },
        }
    };
}

/// Try to unwrap an `Option`/`Result`, logging a WARN and early-returning
/// `$ret` on failure.
#[macro_export]
macro_rules! try_unwrap_or_return {
    ($expr:expr, $ret:expr, $($msg:tt)+) => {
        match $expr {
            Some(v) | Ok(v) => v,
            None | Err(e) => {
                tracing::warn!(
                    target: "axagent.reliability",
                    value = ?e,
                    "{} (returning)",
                    format!($($msg)+)
                );
                return $ret;
            },
        }
    };
}

/// Try to acquire a `RwLock`/`Mutex` lock, recovering the guard from
/// poisoning and logging a WARN instead of panicking.
#[macro_export]
macro_rules! try_lock_or_log {
    ($expr:expr, $($msg:tt)+) => {
        match $expr {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!(
                    target: "axagent.reliability",
                    "{} (recovering from poison)",
                    format!($($msg)+)
                );
                poisoned.into_inner()
            },
        }
    };
}
