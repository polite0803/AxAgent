// SPDX-License-Identifier: AGPL-3.0-only
//! Reliability helpers shared across runtime submodules.
//!
//! This module hosts the small recovery primitives used to downgrade
//! "should never happen" failure modes from hard panics into logged
//! warnings.  They are intended for *defensive* call sites only — code
//! that may legitimately fail (I/O, network, SQL) should keep using
//! `?` / `Result`.
