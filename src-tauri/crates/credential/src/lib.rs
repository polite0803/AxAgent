// SPDX-License-Identifier: AGPL-3.0-only

mod error;
pub mod manager;
pub mod store;
pub mod types;

pub use error::{CredentialError, Result};
pub use manager::CredentialManager;
pub use store::{CredentialMeta, CredentialStore};
pub use types::{Credential, CredentialRef, CredentialType, SmtpConfig};
