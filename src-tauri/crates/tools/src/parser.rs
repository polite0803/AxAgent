// SPDX-License-Identifier: AGPL-3.0-only

//! tools crate 的 DocumentParser trait 注入模块。
//!
//! tools crate 不再直接依赖 axagent-document-parser，
//! 通过本模块的 OnceLock 注入 DocumentParser 实现。
//! 由 wiring 层在启动时调用 `set_parser` 注入。

use std::sync::{Arc, OnceLock};

use axagent_harness::search_sources::DocumentParser;

static PARSER: OnceLock<Arc<dyn DocumentParser>> = OnceLock::new();

/// 由 wiring 层（init/state.rs）启动时调用一次。
pub fn set_parser(parser: Arc<dyn DocumentParser>) {
    let _ = PARSER.set(parser);
}

pub(crate) fn parser() -> &'static Arc<dyn DocumentParser> {
    PARSER.get().expect(
        "DocumentParser not initialized — call axagent_tools::parser::set_parser() in wiring layer",
    )
}
