//! Reliability macros used to convert hard-panics into logged warnings.
//!
//! The macros in this module are the *gentle* counterpart of `.unwrap()` /
//! `.expect()`: when the operation would have panicked, they log a
//! structured warning at WARN level and return a configurable fallback
//! value.  The intended use cases are:
//!
//! - Lookups in maps/collections where the "missing key" branch is
//!   expected to be unreachable in practice but is defensive coding.
//! - Decryption / parsing operations where a malformed input should
//!   be downgraded to "skip this item" rather than crash the worker.
//! - Internal synchronisation primitives (`Arc::strong_count`,
//!   `Mutex::lock`) where poisoning is fatal to a thread but the
//!   whole daemon should keep running.
//!
//! They are **not** a substitute for proper error handling.  Code that
//! may legitimately fail (network I/O, file parsing, SQL queries)
//! should still use `?` and `Result`.  These macros exist for the
//! large number of `unwrap()` calls in the codebase that were simply
//! defensive "should never happen" assertions; we want to keep that
//! invariant but log it so a misbehaving caller doesn't take the
//! daemon down with no trace.

/// Try to unwrap an `Option`/`Result`, logging a WARN and returning
/// `default` on failure.  Use only when the missing/error branch is
/// defensive ("should never happen at runtime, but if it does we want
/// the daemon to keep running").
///
/// # Examples
///
/// ```ignore
/// use crate::util::try_unwrap_or_log;
///
/// let v = try_unwrap_or_log!(
///     opt_value,
///     default = HashMap::new(),
///     "agent_session lookup failed; using empty session"
/// );
/// ```
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
