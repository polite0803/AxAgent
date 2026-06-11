// SPDX-License-Identifier: AGPL-3.0-only

//! Document text extraction — now lives in `axagent-document-parser`.
//! This module re-exports from the dedicated crate for backward compatibility.

pub use axagent_document_parser::{extract_text, mime_from_extension};
