// SPDX-License-Identifier: AGPL-3.0-only

import { useKnowledgeStore } from "@/stores";
import { useMemoryStore } from "@/stores";
import { useContextSourceStore } from "@/stores";
import { useWikiStore } from "@/stores";
import type { KnowledgeDocument, MemoryItem, Note } from "@/types";
import { Checkbox, Empty, theme, Tooltip } from "antd";
import { FileText, Loader2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

type SourceType = "knowledge" | "memory" | "wiki";

interface ContextSourceDocsPanelProps {
  conversationId: string;
  sourceType: SourceType;
  refId: string;
  /** 当前已选中的 doc_ids（来自 contextSourceStore.sources 中对应行） */
  selectedDocIds: string[];
}

/**
 * 多文档协同：在 ContextSourcePicker 的每个已启用容器行下方，
 * 渲染该容器下的文档列表，允许用户勾选/取消勾选文档以限制 RAG 检索范围。
 *
 * 文档列表懒加载：组件首次 mount 时调用对应 store 的 loadXxx；
 * 勾选状态通过 useContextSourceStore.setDocIds 持久化到 context_sources 表的 doc_ids_json 字段。
 */
export function ContextSourceDocsPanel({
  conversationId,
  sourceType,
  refId,
  selectedDocIds,
}: ContextSourceDocsPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const setDocIds = useContextSourceStore((s) => s.setDocIds);

  // 各 store 的文档加载与列表（按 sourceType 选择性订阅）
  const knowledgeDocuments = useKnowledgeStore((s) => s.documents);
  const loadKnowledgeDocuments = useKnowledgeStore((s) => s.loadDocuments);
  const memoryItems = useMemoryStore((s) => s.items);
  const loadMemoryItems = useMemoryStore((s) => s.loadItems);
  const wikiNotes = useWikiStore((s) => s.notes);
  const loadWikiNotes = useWikiStore((s) => s.loadNotes);

  const [loading, setLoading] = useState(false);
  const [loadedKey, setLoadedKey] = useState<string | null>(null);

  // 懒加载：mount 时加载一次（用 refId 作为 key 避免重复加载）
  useEffect(() => {
    const key = `${sourceType}:${refId}`;
    if (loadedKey === key) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    (async () => {
      try {
        if (sourceType === "knowledge") {
          await loadKnowledgeDocuments(refId);
        } else if (sourceType === "memory") {
          await loadMemoryItems(refId);
        } else if (sourceType === "wiki") {
          await loadWikiNotes(refId);
        }
        if (!cancelled) {
          setLoadedKey(key);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sourceType, refId, loadedKey, loadKnowledgeDocuments, loadMemoryItems, loadWikiNotes]);

  const docs: Array<{ id: string; title: string; subtitle?: string }> = useMemo(() => {
    if (sourceType === "knowledge") {
      return (knowledgeDocuments as KnowledgeDocument[])
        .filter((d) => d.knowledgeBaseId === refId)
        .map((d) => ({ id: d.id, title: d.title }));
    }
    if (sourceType === "memory") {
      return (memoryItems as MemoryItem[])
        .filter((m) => m.namespaceId === refId)
        .map((m) => ({ id: m.id, title: m.title }));
    }
    // wiki
    return (wikiNotes as Note[])
      .filter((n) => n.vaultId === refId)
      .map((n) => ({ id: n.id, title: n.title, subtitle: n.filePath }));
  }, [sourceType, refId, knowledgeDocuments, memoryItems, wikiNotes]);

  const handleToggle = (docId: string, checked: boolean) => {
    const next = checked
      ? [...selectedDocIds, docId]
      : selectedDocIds.filter((id) => id !== docId);
    void setDocIds(conversationId, sourceType, refId, next);
  };

  const handleSelectAll = () => {
    void setDocIds(conversationId, sourceType, refId, docs.map((d) => d.id));
  };

  const handleClear = () => {
    void setDocIds(conversationId, sourceType, refId, []);
  };

  if (loading) {
    return (
      <div
        style={{
          padding: "6px 12px 6px 28px",
          display: "flex",
          alignItems: "center",
          gap: 6,
          color: token.colorTextTertiary,
          fontSize: 12,
        }}
      >
        <Loader2 size={12} className="animate-spin" />
        {t("chat.sources.loadingDocs")}
      </div>
    );
  }

  if (docs.length === 0) {
    return (
      <div
        style={{
          padding: "6px 12px 6px 28px",
          color: token.colorTextTertiary,
          fontSize: 12,
        }}
      >
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("chat.sources.noDocs")}
          styles={{ root: { margin: 0, fontSize: 12 } }}
        />
      </div>
    );
  }

  const allSelected = docs.length > 0 && selectedDocIds.length === docs.length;
  const partialSelected = selectedDocIds.length > 0 && !allSelected;

  return (
    <div
      style={{
        padding: "4px 8px 4px 24px",
        borderTop: `1px dashed ${token.colorBorderSecondary}`,
        marginTop: 2,
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 4,
        }}
      >
        <span
          style={{
            fontSize: 11,
            color: token.colorTextTertiary,
          }}
        >
          {t("chat.sources.docsHint")}
        </span>
        <div style={{ display: "flex", gap: 8 }}>
          <Tooltip title={t("chat.sources.selectAll")}>
            <a
              onClick={handleSelectAll}
              style={{
                fontSize: 11,
                color: allSelected ? token.colorTextDisabled : token.colorPrimary,
                cursor: allSelected ? "default" : "pointer",
                pointerEvents: allSelected ? "none" : "auto",
              }}
            >
              {t("chat.sources.selectAll")}
            </a>
          </Tooltip>
          <Tooltip title={t("chat.sources.clearAll")}>
            <a
              onClick={handleClear}
              style={{
                fontSize: 11,
                color: selectedDocIds.length === 0 ? token.colorTextDisabled : token.colorPrimary,
                cursor: selectedDocIds.length === 0 ? "default" : "pointer",
                pointerEvents: selectedDocIds.length === 0 ? "none" : "auto",
              }}
            >
              {t("chat.sources.clearAll")}
            </a>
          </Tooltip>
        </div>
      </div>
      <Checkbox
        checked={allSelected}
        indeterminate={partialSelected}
        onChange={(e) => (e.target.checked ? handleSelectAll() : handleClear())}
        style={{
          fontSize: 12,
          color: token.colorTextSecondary,
          marginBottom: 4,
        }}
      >
        {t("chat.sources.selectAllInContainer")}
      </Checkbox>
      <div
        style={{
          maxHeight: 200,
          overflowY: "auto",
          paddingRight: 4,
        }}
      >
        {docs.map((doc) => {
          const checked = selectedDocIds.includes(doc.id);
          return (
            <div
              key={doc.id}
              style={{
                padding: "1px 0",
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              <Checkbox checked={checked} onChange={(e) => handleToggle(doc.id, e.target.checked)}>
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                    fontSize: 12,
                    color: checked ? token.colorText : token.colorTextSecondary,
                  }}
                >
                  <FileText size={11} />
                  <span
                    style={{
                      maxWidth: 180,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                    title={doc.title}
                  >
                    {doc.title}
                  </span>
                </span>
              </Checkbox>
            </div>
          );
        })}
      </div>
      {selectedDocIds.length > 0 && (
        <div
          style={{
            marginTop: 4,
            fontSize: 11,
            color: token.colorTextTertiary,
          }}
        >
          {t("chat.sources.selectedCount", { count: selectedDocIds.length })}
        </div>
      )}
    </div>
  );
}
