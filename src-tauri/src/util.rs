// SPDX-License-Identifier: AGPL-3.0-only

/// Reliability macros — re-exported from `axagent-harness`.
macro_rules! try_lock_or_log {
    ($expr:expr, $($msg:tt)+) => {
        axagent_harness::try_lock_or_log!($expr, $($msg)+)
    };
}
macro_rules! try_unwrap_or_log {
    ($expr:expr, default = $default:expr, $($msg:tt)+) => {
        axagent_harness::try_unwrap_or_log!($expr, default = $default, $($msg)+)
    };
}
macro_rules! try_unwrap_or_return {
    ($expr:expr, $ret:expr, $($msg:tt)+) => {
        axagent_harness::try_unwrap_or_return!($expr, $ret, $($msg)+)
    };
}
