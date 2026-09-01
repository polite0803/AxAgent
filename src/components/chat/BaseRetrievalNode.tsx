// SPDX-License-Identifier: AGPL-3.0-only

/**
 * BaseRetrievalNode — shared rendering for knowledge/wiki retrieval results.
 *
 * Both KnowledgeRetrievalNode and WikiRetrievalNode are thin wrappers
 * around this component, differing only in the icon and i18n prefix.
 */

import { CITE_JUMP_EVENT, CiteItemsContext } from "@/components/chat/citeContext";
import { translateBackendError } from "@/lib/errorI18n";
import { invoke } from "@/lib/invoke";
import type { MemoryRetrievedItem, MemorySourceResult } from "@/lib/memoryUtils";
import { App, theme } from "antd";
import { AlertCircle, ChevronDown, ChevronRight, ThumbsDown, ThumbsUp, XCircle } from "lucide-react";
import type { NodeComponentProps } from "markstream-react";
import { type ComponentType, createContext, type CSSProperties, useCallback } from "react";
import { useContext, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * RetrievalMessageIdContext — 传递当前消息 ID 给检索结果节点。
 *
 * AssistantMarkdown 在渲染时用 Provider 包裹 NodeRenderer，
 * BaseRetrievalNode 通过 useContext 取 messageId，
 * 用于调用 update_retrieval_hit_feedback_by_ref 后端命令。
 * 未提供时（如 user 消息或预览模式），反馈按钮不渲染。
 */
export const RetrievalMessageIdContext = createContext<string | null>(null);

/** 反馈类型：positive（有用）/ negative（无用）/ irrelevant（无关） */
type FeedbackValue = "positive" | "negative" | "irrelevant";

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
    const { message: messageApi } = App.useApp();
    const [expanded, setExpanded] = useState(false);
    const [highlightedIdx, setHighlightedIdx] = useState<number | null>(null);
    const allEntries = useContext(CiteItemsContext);
    const highlightTimerRef = useRef<number | null>(null);
    // 从 Context 获取当前消息 ID，用于 RAG 反馈闭环
    const messageId = useContext(RetrievalMessageIdContext);
    // 反馈状态：key = `${document_id}#${chunk_ref}`，value = FeedbackValue
    const [feedbackMap, setFeedbackMap] = useState<Record<string, FeedbackValue>>({});
    // 正在提交反馈的 item key 集合，防止重复点击
    const [submittingKeys, setSubmittingKeys] = useState<Set<string>>(new Set());

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

    // 提交反馈到后端（RAG 反馈闭环）
    const handleFeedback = useCallback(
      async (item: MemoryRetrievedItem, feedback: FeedbackValue) => {
        if (!messageId || !item.document_id || !item.id) {
          return;
        }
        const key = `${item.document_id}#${item.id}`;
        // 切换反馈：再次点击同一反馈则取消
        const current = feedbackMap[key];
        const newFeedback = current === feedback ? null : feedback;

        setSubmittingKeys((prev) => new Set(prev).add(key));
        try {
          const ok = await invoke<boolean>(
            "update_retrieval_hit_feedback_by_ref",
            {
              messageId,
              documentId: item.document_id,
              chunkRef: item.id,
              feedback: newFeedback,
            },
          );
          if (ok) {
            setFeedbackMap((prev) => {
              const next = { ...prev };
              if (newFeedback) {
                next[key] = newFeedback;
              } else {
                delete next[key];
              }
              return next;
            });
            messageApi.success(
              t(`${i18nPrefix}.feedbackSubmitted`, {
                defaultValue: t("chat.knowledgeRetrieval.feedbackSubmitted"),
              }),
            );
          } else {
            messageApi.warning(
              t(`${i18nPrefix}.feedbackNotFound`, {
                defaultValue: t("chat.knowledgeRetrieval.feedbackNotFound"),
              }),
            );
          }
        } catch (e) {
          messageApi.error(translateBackendError(e));
        } finally {
          setSubmittingKeys((prev) => {
            const next = new Set(prev);
            next.delete(key);
            return next;
          });
        }
      },
      [messageId, feedbackMap, messageApi, t, i18nPrefix],
    );

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
                      {/* RAG 反馈闭环：仅当有 messageId 时渲染反馈按钮 */}
                      {messageId && item.document_id && item.id && (() => {
                        const key = `${item.document_id}#${item.id}`;
                        const current = feedbackMap[key];
                        const isSubmitting = submittingKeys.has(key);
                        const feedbackBtns: Array<{
                          value: FeedbackValue;
                          icon: typeof ThumbsUp;
                          color: string;
                          labelKey: string;
                        }> = [
                          {
                            value: "positive",
                            icon: ThumbsUp,
                            color: token.colorSuccess,
                            labelKey: `${i18nPrefix}.feedbackPositive`,
                          },
                          {
                            value: "negative",
                            icon: ThumbsDown,
                            color: token.colorError,
                            labelKey: `${i18nPrefix}.feedbackNegative`,
                          },
                          {
                            value: "irrelevant",
                            icon: XCircle,
                            color: token.colorTextTertiary,
                            labelKey: `${i18nPrefix}.feedbackIrrelevant`,
                          },
                        ];
                        return (
                          <span
                            style={{
                              display: "inline-flex",
                              alignItems: "center",
                              gap: 2,
                              marginLeft: 4,
                            }}
                          >
                            {feedbackBtns.map((btn) => {
                              const active = current === btn.value;
                              const BtnIcon = btn.icon;
                              return (
                                <button
                                  key={btn.value}
                                  type="button"
                                  disabled={isSubmitting}
                                  title={t(btn.labelKey, {
                                    defaultValue: t(`chat.knowledgeRetrieval.${
                                      btn.value === "positive"
                                        ? "feedbackPositive"
                                        : btn.value === "negative"
                                        ? "feedbackNegative"
                                        : "feedbackIrrelevant"
                                    }`),
                                  })}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    void handleFeedback(item, btn.value);
                                  }}
                                  style={{
                                    display: "inline-flex",
                                    alignItems: "center",
                                    justifyContent: "center",
                                    width: 18,
                                    height: 18,
                                    padding: 0,
                                    border: "none",
                                    borderRadius: 3,
                                    cursor: isSubmitting ? "wait" : "pointer",
                                    opacity: isSubmitting ? 0.5 : 1,
                                    backgroundColor: active
                                      ? btn.color === token.colorTextTertiary
                                        ? token.colorFillSecondary
                                        : `${btn.color}20`
                                      : "transparent",
                                    color: active ? btn.color : token.colorTextQuaternary,
                                    transition: "all 150ms ease",
                                  }}
                                >
                                  <BtnIcon size={11} />
                                </button>
                              );
                            })}
                          </span>
                        );
                      })()}
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
