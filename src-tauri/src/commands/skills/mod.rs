// SPDX-License-Identifier: AGPL-3.0-only
// Skills module - split into install, management, and analysis submodules

mod analysis;
mod install;
mod management;

pub use analysis::*;
pub use install::*;
pub use management::*;
