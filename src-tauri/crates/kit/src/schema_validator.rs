// SPDX-License-Identifier: AGPL-3.0-only

//! JSON Schema 校验工具 —— 由 `axagent-harness` 提供实现，本模块仅为向后兼容的 re-export。

pub use axagent_harness::schema_validator::{validate_against_schema, validate_recursive};
