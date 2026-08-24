// SPDX-License-Identifier: AGPL-3.0-only

//! 搜索层 trait 注入模块。
//!
//! search crate 不再直接依赖 axagent-dao / axagent-document-parser。
//! 这 5 个 trait object 由 wiring 层在启动时调用 `set_sources` 注入。
//! 内部模块通过 `OnceLock` 静态读取。
//!
//! v2: 新增 `UnifiedKnowledgeSource` 支持，提供四类知识源的统一访问。

use std::sync::{Arc, OnceLock};

use axagent_harness::search_sources::{
    DocumentParser, KnowledgeSource, MemorySource, SettingsSource, UnifiedKnowledgeSource,
    WikiSource,
};

static KNOWLEDGE: OnceLock<Arc<dyn KnowledgeSource>> = OnceLock::new();
static MEMORY: OnceLock<Arc<dyn MemorySource>> = OnceLock::new();
static WIKI: OnceLock<Arc<dyn WikiSource>> = OnceLock::new();
static SETTINGS: OnceLock<Arc<dyn SettingsSource>> = OnceLock::new();
static PARSER: OnceLock<Arc<dyn DocumentParser>> = OnceLock::new();

// 统一知识源注册表（v2）
static UNIFIED_SOURCES: OnceLock<Vec<Arc<dyn UnifiedKnowledgeSource>>> = OnceLock::new();

/// 由 wiring 层（runtime/gateway）启动时调用一次。
#[allow(clippy::too_many_arguments)]
pub fn set_sources(
    knowledge: Arc<dyn KnowledgeSource>,
    memory: Arc<dyn MemorySource>,
    wiki: Arc<dyn WikiSource>,
    settings: Arc<dyn SettingsSource>,
    parser: Arc<dyn DocumentParser>,
) {
    let _ = KNOWLEDGE.set(knowledge);
    let _ = MEMORY.set(memory);
    let _ = WIKI.set(wiki);
    let _ = SETTINGS.set(settings);
    let _ = PARSER.set(parser);
}

/// 注册统一知识源实现（v2）
pub fn set_unified_sources(sources: Vec<Arc<dyn UnifiedKnowledgeSource>>) {
    let _ = UNIFIED_SOURCES.set(sources);
}

pub(crate) fn knowledge() -> &'static Arc<dyn KnowledgeSource> {
    KNOWLEDGE.get().expect("KnowledgeSource not initialized — call axagent_search::sources::set_sources() in wiring layer")
}

pub(crate) fn memory() -> &'static Arc<dyn MemorySource> {
    MEMORY.get().expect("MemorySource not initialized — call axagent_search::sources::set_sources() in wiring layer")
}

pub(crate) fn wiki() -> &'static Arc<dyn WikiSource> {
    WIKI.get().expect(
        "WikiSource not initialized — call axagent_search::sources::set_sources() in wiring layer",
    )
}

pub(crate) fn settings() -> &'static Arc<dyn SettingsSource> {
    SETTINGS.get().expect("SettingsSource not initialized — call axagent_search::sources::set_sources() in wiring layer")
}

pub fn parser() -> &'static Arc<dyn DocumentParser> {
    PARSER.get().expect("DocumentParser not initialized — call axagent_search::sources::set_sources() in wiring layer")
}

/// 获取所有注册的统一知识源
#[allow(dead_code)]
pub fn unified_sources() -> &'static [Arc<dyn UnifiedKnowledgeSource>] {
    UNIFIED_SOURCES.get().map(|v| v.as_slice()).unwrap_or(&[])
}
