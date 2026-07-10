// SPDX-License-Identifier: AGPL-3.0-only

//! Re-export from harness — canonical definitions live in axagent_harness::plugin_hook.
//! This file exists so that `axagent_runtime_core::plugin_hooks::*` continues to
//! resolve for existing callers without requiring import path changes.

pub use axagent_harness::plugin_hook::*;
