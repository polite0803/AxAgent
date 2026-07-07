// SPDX-License-Identifier: AGPL-3.0-only

//! Migration crate.

pub use axagent_dao::ddl::run_initialization;
pub use axagent_harness::migration_types::{
    BackupInfo, DetectedPlatform, MigrationEntry, MigrationItem, MigrationReport,
};

mod migration;
pub use migration::*;
