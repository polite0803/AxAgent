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

/// Acquire a `RwLock`/`Mutex` lock. `parking_lot` locks never poison, so this
/// is an identity expansion of the expression (which already yields the guard).
#[macro_export]
macro_rules! try_lock_or_log {
    ($expr:expr, $($msg:tt)+) => {
        $expr
    };
}
