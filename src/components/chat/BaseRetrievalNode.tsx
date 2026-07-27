// SPDX-License-Identifier: AGPL-3.0-only

/**
 * BaseRetrievalNode — shared rendering for knowledge/wiki retrieval results.
 *
 * Both KnowledgeRetrievalNode and WikiRetrievalNode are thin wrappers
 * around this component, differing only in the icon and i18n prefix.
 */

import { CITE_JUMP_EVENT, CiteItemsContext } from "@/components/chat/citeContext";
import type { MemoryRetrievedItem, MemorySourceResult } from "@/lib/memoryUtils";
import { theme } from "antd";
import { AlertCircle, ChevronDown, ChevronRight } from "lucide-react";
import type { NodeComponentProps } from "markstream-react";
import type { ComponentType, CSSProperties } from "react";
import { useContext, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

export type BaseRetrievalNodeData = {
  type: string;
  content?: string;
  attrs?: Record<string, string> | [string, string][];
  loading?: boolean;
};

export type RetrievalNodeConfig = {
  i18nPrefix: string;
  Icon: ComponentType<{ size: number; style?: CSSProperties }>;
};

function getAttrValue(
  attrs: BaseRetrievalNodeData["attrs"],
  key: string,
): string | undefined {
  if (!attrs) {
    return undefined;
  }
  if (Array.isArray(attrs)) {
    const entry = attrs.find(([name]) => name === key);
    return entry?.[1];
  }
  return attrs[key];
}

function truncateContent(text: string, maxLen = 120): string {
  if (text.length <= maxLen) {
    return text;
  }
  return text.slice(0, maxLen) + "…";
}

export function createRetrievalNode(config: RetrievalNodeConfig) {
  const { i18nPrefix, Icon } = config;

  const Component = (props: NodeComponentProps<BaseRetrievalNodeData>) => {
    const { node } = props;
    const { token } = theme.useToken();
    const { t } = useTranslation();
    const [expanded, setExpanded] = useState(false);
    const [highlightedIdx, setHighlightedIdx] = useState<number | null>(null);
    const allEntries = useContext(CiteItemsContext);
    const highlightTimerRef = useRef<number | null>(null);

    if (!node) {
      return null;
    }

    const status = getAttrValue(node.attrs, "status") ?? (node.loading ? "searching" : "done");

    let sources: MemorySourceResult[] = [];
    if (node.content) {
      try {
        const parsed = JSON.parse(node.content);
        if (Array.isArray(parsed)) {
          sources = parsed;
        }
      } catch {
        // invalid JSON
      }
    }

    const totalItems = sources.reduce((sum, s) => sum + s.items.length, 0);

    // 引用追溯：计算本节点内每个 item 对应的全局 cite idx（用于 data-cite-idx 标记 + 跳转高亮匹配）
    // 匹配键：(item.id, item.document_id)，与 AssistantMarkdown 中 citeEntries 的扁平化顺序一致
    const itemCiteIndices = useMemo<number[]>(() => {
      const result: number[] = [];
      for (const src of sources) {
        for (const item of src.items) {
          const found = allEntries.find(
            (e) => e.item.id === item.id && e.item.document_id === item.document_id,
          );
          result.push(found?.globalIdx ?? -1);
        }
      }
      return result;
    }, [sources, allEntries]);

    // 引用追溯：监听 chip 点击事件，匹配本节点 item 则展开 + 高亮
    useEffect(() => {
      const handler = (e: Event) => {
        const detail = (e as CustomEvent).detail as { idx: number } | undefined;
        if (!detail) { return; }
        const localIdx = itemCiteIndices.indexOf(detail.idx);
        if (localIdx < 0) { return; }
        setExpanded(true);
        setHighlightedIdx(localIdx);
        // 滚动到对应 item
        requestAnimationFrame(() => {
          const el = document.querySelector(`[data-cite-idx="${detail.idx}"]`);
          if (el) {
            el.scrollIntoView({ behavior: "smooth", block: "center" });
          }
        });
        // 2.5s 后清除高亮
        if (highlightTimerRef.current !== null) {
          window.clearTimeout(highlightTimerRef.current);
        }
        highlightTimerRef.current = window.setTimeout(() => {
          setHighlightedIdx(null);
          highlightTimerRef.current = null;
        }, 2500);
      };
      window.addEventListener(CITE_JUMP_EVENT, handler);
      return () => {
        window.removeEventListener(CITE_JUMP_EVENT, handler);
        if (highlightTimerRef.current !== null) {
          window.clearTimeout(highlightTimerRef.current);
          highlightTimerRef.current = null;
        }
      };
    }, [itemCiteIndices]);

    // Searching state
    if (status === "searching") {
      return (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "8px 12px",
            marginBottom: 8,
            borderRadius: 8,
            backgroundColor: token.colorFillQuaternary,
          }}
        >
          <span
            className="animate-spin"
            style={{ display: "inline-flex", width: 16, height: 16 }}
          >
            <Icon size={16} style={{ color: token.colorPrimary }} />
          </span>
          <span style={{ color: token.colorTextSecondary, fontSize: 13 }}>
            {t(`${i18nPrefix}.searching`)}
          </span>
        </div>
      );
    }

    // Error state
    if (status === "error") {
      return (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "8px 12px",
            marginBottom: 8,
            borderRadius: 8,
            backgroundColor: token.colorErrorBg,
            color: token.colorError,
            fontSize: 13,
          }}
        >
          <AlertCircle size={16} />
          <span>{node.content || t(`${i18nPrefix}.error`)}</span>
        </div>
      );
    }

    // Done state — no results
    if (totalItems === 0) {
      return null;
    }

    return (
      <div
        style={{
          marginBottom: 8,
          borderRadius: 8,
          border: `1px solid ${token.colorBorderSecondary}`,
          overflow: "hidden",
        }}
      >
        {/* Header */}
        <div
          onClick={() => setExpanded(!expanded)}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              setExpanded(!expanded);
            }
          }}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "8px 12px",
            cursor: "pointer",
            backgroundColor: token.colorFillQuaternary,
            userSelect: "none",
          }}
        >
          <Icon size={14} style={{ color: token.colorPrimary }} />
          <span style={{ fontSize: 13, fontWeight: 500 }}>
            {t(`${i18nPrefix}.resultsCount`, { count: totalItems })}
          </span>
          <span style={{ marginLeft: "auto", color: token.colorTextTertiary }}>
            {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </span>
        </div>

        {/* Per-item overview */}
        <div
          style={{
            display: "flex",
            gap: 4,
            padding: "6px 12px",
            flexWrap: "wrap",
            borderTop: `1px solid ${token.colorBorderSecondary}`,
          }}
        >
          {sources.flatMap((src, si) =>
            src.items.map((item, ii) => (
              <span
                key={`${si}-${ii}`}
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  padding: "2px 8px",
                  fontSize: 12,
                  borderRadius: 4,
                  backgroundColor: token.colorFillSecondary,
                  color: token.colorTextSecondary,
                }}
              >
                <Icon size={10} style={{ flexShrink: 0 }} />
                <span
                  style={{
                    maxWidth: 120,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {item.document_name || item.document_id?.slice(0, 8) || "—"}
                </span>
                {item.id && <span style={{ opacity: 0.5 }}>#{item.id.slice(0, 6)}</span>}
                <span
                  style={{
                    color: token.colorPrimary,
                    fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                  }}
                >
                  {(1 / (1 + item.score)).toFixed(3)}
                </span>
              </span>
            ))
          )}
        </div>

        {/* Expanded details */}
        {expanded && (
          <div
            style={{
              padding: "8px 12px",
              borderTop: `1px solid ${token.colorBorderSecondary}`,
            }}
          >
            {sources.map((src, si) =>
              src.items.map((item: MemoryRetrievedItem, ii: number) => {
                // 扁平化 local idx（与 itemCiteIndices 对齐）
                let localIdx = 0;
                for (let s = 0; s < si; s++) {
                  localIdx += sources[s].items.length;
                }
                localIdx += ii;
                const citeIdx = itemCiteIndices[localIdx] ?? -1;
                const isHighlighted = highlightedIdx === localIdx;
                return (
                  <div
                    key={`${si}-${ii}`}
                    data-cite-idx={citeIdx >= 0 ? citeIdx : undefined}
                    style={{
                      marginBottom: ii < src.items.length - 1 || si < sources.length - 1 ? 8 : 0,
                      fontSize: 12,
                      padding: "4px 6px",
                      borderRadius: 4,
                      transition: "background-color 200ms ease",
                      backgroundColor: isHighlighted ? token.colorPrimaryBg : undefined,
                      outline: isHighlighted ? `2px solid ${token.colorPrimary}` : undefined,
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 4,
                        marginBottom: 2,
                      }}
                    >
                      <Icon
                        size={12}
                        style={{ color: token.colorPrimary, flexShrink: 0 }}
                      />
                      <span style={{ fontWeight: 500, color: token.colorText }}>
                        {item.document_name || item.document_id?.slice(0, 8) || "—"}
                      </span>
                      {item.id && (
                        <span
                          style={{ fontSize: 10, color: token.colorTextQuaternary }}
                        >
                          #{item.id.slice(0, 8)}
                        </span>
                      )}
                      {citeIdx >= 0 && (
                        <span
                          style={{
                            fontSize: 10,
                            color: token.colorPrimary,
                            fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                          }}
                        >
                          [cite:{citeIdx}]
                        </span>
                      )}
                      <span
                        style={{
                          marginLeft: "auto",
                          fontSize: 10,
                          color: token.colorTextQuaternary,
                        }}
                      >
                        {(1 / (1 + item.score)).toFixed(4)}
                      </span>
                    </div>
                    <p
                      style={{
                        margin: "2px 0 0 0",
                        color: token.colorTextSecondary,
                        lineHeight: 1.5,
                        display: "-webkit-box",
                        WebkitLineClamp: 3,
                        WebkitBoxOrient: "vertical",
                        overflow: "hidden",
                      }}
                    >
                      {truncateContent(item.content, 200)}
                    </p>
                  </div>
                );
              })
            )}
          </div>
        )}
      </div>
    );
  };

  return Component;
}
