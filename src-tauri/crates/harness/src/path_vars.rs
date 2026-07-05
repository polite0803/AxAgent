// SPDX-License-Identifier: AGPL-3.0-only

//! Path encoding/decoding trait for cross-device portability.
//!
//! Defines the `PathEncoder` trait that replaces absolute path prefixes with
//! variables (`{{AxAgent_HOME}}`, `{{DOCUMENTS}}`, `{{HOME}}`) when writing to
//! the database, and resolves them back to absolute paths when reading.
//! Implementations live in the storage crate.

/// Encodes and decodes absolute paths for portable storage.
pub trait PathEncoder: Send + Sync {
    /// Replace known absolute path prefixes with variables.
    fn encode_path(&self, absolute_path: &str) -> String;
    /// Resolve variables back to platform-specific absolute paths.
    fn decode_path(&self, encoded_path: &str) -> String;
}
