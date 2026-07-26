// SPDX-License-Identifier: AGPL-3.0-only
// Skills module - split into install, management, analysis, and builtin_seed submodules

mod analysis;
mod builtin_seed;
mod install;
mod management;

pub use analysis::*;
pub use builtin_seed::seed_builtin_skills;
pub use install::*;
pub use management::*;
