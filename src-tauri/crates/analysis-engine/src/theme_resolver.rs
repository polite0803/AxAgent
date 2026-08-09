//! 通用主题解析服务 — 将用户主题词解析为概念节点 + 成员股票
//!
//! 核心流程（三段式，任意主题词可解析，代码零业务硬编码）：
//! 1. 对每个主题词调用 `ConceptIndex::resolve()` 做本体对齐（基线本体 + lemonhu 图谱别名）
//! 2. 未命中时调用 astock 概念板块**动态搜索**（东财概念板块 → 成分股），作为实时兜底
//! 3. 仍未命中的主题词加入 `unresolved` 供上层降级处理（serenity 由 LLM 语义消费）

use crate::concept_index::ConceptIndex;
use axagent_astock_data::AStockClient;

/// 主题解析结果
#[derive(Debug, Clone)]
pub struct ThemeResolution {
    /// 成功解析的主题列表
    pub resolved: Vec<ResolvedTheme>,
    /// 未能匹配到任何概念的原始查询词
    pub unresolved: Vec<String>,
}

/// 单个已解析主题
#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    /// 原始查询词
    pub query: String,
    /// 解析后的规范概念 id（本体命中为概念 id；动态搜索命中为 `board_{board_code}`）
    pub concept_id: String,
    /// 概念中文显示名
    pub display: String,
    /// 该概念下的成员股票代码集合
    pub members: Vec<String>,
}

/// 批量主题解析管线（同步版：仅本体对齐）
///
/// 对每个主题词执行 `resolve → members` 流程，
/// 返回已解析和未解析的分类结果。不触发网络请求。
pub fn resolve_themes_pipeline(themes: &[String], index: &ConceptIndex) -> ThemeResolution {
    let mut resolved = Vec::new();
    let mut unresolved = Vec::new();

    for theme in themes {
        if let Some(concept_id) = index.resolve(theme) {
            let concept_id = concept_id.to_string();
            let display = index.display(&concept_id).unwrap_or(&concept_id).to_string();
            let members: Vec<String> = index.members(&concept_id).into_iter().collect();
            resolved.push(ResolvedTheme { query: theme.clone(), concept_id, display, members });
        } else {
            unresolved.push(theme.clone());
        }
    }

    ThemeResolution { resolved, unresolved }
}

/// 批量主题解析管线（异步版：本体对齐 + 动态搜索兜底）
///
/// ① 本体对齐命中 → 取概念成员；
/// ② 未命中且提供 `client` → `search_concept_boards` 动态搜索东财概念板块，
///    取首个板块的 `get_concept_board_members` 成分股作为成员（concept_id=`board_{code}`）；
/// ③ 仍未命中 → 记入 `unresolved`（不静默丢）。
///
/// `client` 为 `None` 时退化为纯本体对齐（等价同步版），便于无网络环境降级。
pub async fn resolve_themes_pipeline_with_search(
    themes: &[String],
    index: &ConceptIndex,
    client: Option<&AStockClient>,
) -> ThemeResolution {
    let mut resolved = Vec::new();
    let mut unresolved = Vec::new();

    for theme in themes {
        // ① 本体对齐
        if let Some(concept_id) = index.resolve(theme) {
            let concept_id = concept_id.to_string();
            let display = index.display(&concept_id).unwrap_or(&concept_id).to_string();
            let members: Vec<String> = index.members(&concept_id).into_iter().collect();
            resolved.push(ResolvedTheme { query: theme.clone(), concept_id, display, members });
            continue;
        }

        // ② 动态搜索东财概念板块（实时兜底）
        if let Some(client) = client {
            match client.search_concept_boards(theme).await {
                Ok(boards) if !boards.is_empty() => {
                    let board = &boards[0];
                    match client.get_concept_board_members(&board.board_code).await {
                        Ok(members) if !members.is_empty() => {
                            resolved.push(ResolvedTheme {
                                query: theme.clone(),
                                concept_id: format!("board_{}", board.board_code),
                                display: board.board_name.clone(),
                                members: members.iter().map(|m| m.stock_code.clone()).collect(),
                            });
                            continue;
                        },
                        _ => {
                            tracing::debug!(
                                "[theme_resolver] 概念板块 {}({}) 无成分股数据",
                                board.board_name,
                                board.board_code
                            );
                        },
                    }
                },
                Ok(_) => {
                    tracing::debug!("[theme_resolver] 主题词 '{theme}' 未命中概念板块搜索");
                },
                Err(e) => {
                    tracing::warn!("[theme_resolver] 搜索概念板块失败 '{theme}': {e}");
                },
            }
        }

        // ③ 仍未命中
        unresolved.push(theme.clone());
    }

    ThemeResolution { resolved, unresolved }
}
