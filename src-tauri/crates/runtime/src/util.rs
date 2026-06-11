//! Reliability helpers shared across runtime submodules.
//!
//! This module hosts the small recovery primitives used to downgrade
//! "should never happen" failure modes (poisoned mutexes, missing
//! expected values) from hard panics into logged warnings.  They are
//! intended for *defensive* call sites only — code that may legitimately
//! fail (I/O, network, SQL) should keep using `?` / `Result`.

use std::sync::{LockResult, MutexGuard, PoisonError};

/// Recover from a poisoned mutex by logging a warning and returning
/// the inner guard.  A poisoned lock indicates that another thread
/// panicked while holding the lock; rather than cascading the panic
/// and tearing down the daemon, we surface a WARN and proceed with
/// the (potentially inconsistent) inner data.
///
/// This mirrors the `try_unwrap_or_log!` macro in the main binary's
/// `util` module, but specialised for `LockResult` so it preserves
/// the typed guard.
#[allow(dead_code)]
pub fn lock_or_recover<'a, T>(
    result: LockResult<MutexGuard<'a, T>>,
    lock_name: &'static str,
) -> MutexGuard<'a, T> {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                target: "axagent.reliability",
                lock = lock_name,
                "mutex poisoned; recovering with inner data (last holder panicked)"
            );
            PoisonError::into_inner(poisoned)
        }
    }
}
