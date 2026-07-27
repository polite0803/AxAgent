// SPDX-License-Identifier: AGPL-3.0-only

import type { MemoryRetrievedItem } from "@/lib/memoryUtils";
import React from "react";

/**
 * 扁平化后的引用条目：携带 item 本身 + 所属 source 的元信息（用于 popover 展示）。
 * 顺序与后端 `rebuild_context_with_citations` 的 cite_idx 严格一致。
 */
export type CiteFlatEntry = {
  item: MemoryRetrievedItem;
  sourceType: "knowledge" | "memory" | "wiki" | string;
  containerName?: string;
  /** 引用追溯跳转高亮：该 item 在扁平化数组中的全局序号，对应 [cite:N] 的 N */
  globalIdx: number;
};

/** 引用追溯跳转事件：CiteRefNode 点击时 dispatch，BaseRetrievalNode 监听后展开+高亮对应 item */
export const CITE_JUMP_EVENT = "axagent-cite-jump";
export type CiteJumpDetail = { idx: number };

/**
 * AssistantMarkdown 在渲染时从 message content 中解析所有 retrieval 标签
 * （knowledge-retrieval / memory-retrieval / wiki-retrieval），扁平化为 CiteFlatEntry[]
 * 注入此 Context，供 CiteRefNode 按 `n` 索引取条目，BaseRetrievalNode 按 globalIdx 标记 item。
 */
export const CiteItemsContext = React.createContext<CiteFlatEntry[]>([]);
