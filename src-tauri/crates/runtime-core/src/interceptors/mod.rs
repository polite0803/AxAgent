// SPDX-License-Identifier: AGPL-3.0-only

//! Concrete interceptor implementations migrated from harness.
//!
//! Each interceptor is a standalone struct implementing `HarnessInterceptor`.

pub mod business_rule;
pub mod consistency_check;
pub mod output_validation;
pub mod prompt_guard;
