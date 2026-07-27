// SPDX-License-Identifier: AGPL-3.0-only

import { KnowledgeBaseIcon } from "@/components/shared/KnowledgeBaseIcon";
import { NamespaceIcon } from "@/components/shared/NamespaceIcon";
import { useContextSourceStore } from "@/stores";
import type { KnowledgeBase, MemoryNamespace, Wiki } from "@/types";
import { Button, Checkbox, Radio, theme } from "antd";
import { BookOpen, Brain, ChevronDown, ChevronRight, Library } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ContextSourceDocsPanel } from "./ContextSourceDocsPanel";

interface SourcePickerPanelProps {
  conversationId: string | null;
  knowledgeBases: KnowledgeBase[];
  memoryNamespaces: MemoryNamespace[];
  wikis: Wiki[];
  enabledKnowledgeBaseIds: string[];
  activeMemoryNamespaceId: string | null;
  enabledWikiIds: string[];
  onToggleKb: (id: string) => void;
  onSetActiveMemory: (id: string | null) => void;
  onToggleWiki: (id: string) => void;
  onGoConfig: () => void;
}

/**
 * 多文档协同版的 ContextSourcePicker。
 *
 * 在原容器（KB / memory namespace / wiki）级勾选之上，新增「展开」按钮：
 * 展开后会显示该容器下的文档列表，用户可勾选/取消勾选文档以限制 RAG 检索范围。
 *
 * 数据流：
 * - 容器勾选状态：来自 props（preferenceStore + conversationStore 联动）
 * - 文档勾选状态：来自 contextSourceStore.sources（持久化在 context_sources.doc_ids_json）
 * - 切换容器：调用 props 回调 → preferenceStore.toggleKnowledgeBase → 后端 update_conversation
 *             → sync_context_sources（diff 同步，保留 doc_ids）→ 前端 loadSources(force) 刷新
 * - 切换文档：直接调用 contextSourceStore.setDocIds → 后端 set_context_source_doc_ids
 */
export function SourcePickerPanel({
  conversationId,
  knowledgeBases,
  memoryNamespaces,
  wikis,
  enabledKnowledgeBaseIds,
  activeMemoryNamespaceId,
  enabledWikiIds,
  onToggleKb,
  onSetActiveMemory,
  onToggleWiki,
  onGoConfig,
}: SourcePickerPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const contextSources = useContextSourceStore((s) => s.sources);
  const loadSources = useContextSourceStore((s) => s.loadSources);

  // 切换会话时加载 context sources
  useEffect(() => {
    if (conversationId) {
      void loadSources(conversationId, true);
    }
  }, [conversationId, loadSources]);

  // 容器勾选状态变化时重新加载（sync_context_sources 已在后端执行）
  useEffect(() => {
    if (conversationId) {
      void loadSources(conversationId, true);
    }
    // 监听三个 enabled 列表的变化
  }, [
    conversationId,
    enabledKnowledgeBaseIds,
    activeMemoryNamespaceId,
    enabledWikiIds,
    loadSources,
  ]);

  // 展开状态：key = `${sourceType}:${refId}`
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const toggleExpand = (key: string) => {
    setExpandedKey((prev) => (prev === key ? null : key));
  };

  const getDocIds = (sourceType: string, refId: string): string[] => {
    const src = contextSources.find(
      (s) => s.type === sourceType && s.refId === refId,
    );
    return src?.docIds ?? [];
  };

  const renderExpandIcon = (key: string, enabled: boolean) => {
    if (!enabled) {
      return null;
    }
    const expanded = expandedKey === key;
    return (
      <button
        type="button"
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          toggleExpand(key);
        }}
        style={{
          background: "transparent",
          border: "none",
          padding: 0,
          marginLeft: 4,
          cursor: "pointer",
          display: "inline-flex",
          alignItems: "center",
          color: token.colorTextTertiary,
        }}
        title={t("chat.sources.toggleDocs")}
      >
        {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
      </button>
    );
  };

  if (!conversationId) {
    return (
      <div style={{ padding: "8px 0", minWidth: 200 }}>
        <div
          style={{
            color: token.colorTextSecondary,
            fontSize: 12,
            marginBottom: 8,
          }}
        >
          {t("chat.sources.empty")}
        </div>
        <Button
          type="link"
          size="small"
          style={{ padding: 0, fontSize: 12 }}
          onClick={onGoConfig}
        >
          {t("chat.connector.goConfig")}
        </Button>
      </div>
    );
  }

  return (
    <div style={{ minWidth: 260, maxHeight: 480, overflowY: "auto" }}>
      {knowledgeBases.length > 0 && (
        <div
          style={{
            marginBottom: knowledgeBases.length > 0
                && (memoryNamespaces.length > 0 || wikis.length > 0)
              ? 8
              : 0,
          }}
        >
          <div
            style={{
              fontSize: 12,
              fontWeight: 600,
              color: token.colorTextSecondary,
              textTransform: "uppercase",
              letterSpacing: 0.5,
              marginBottom: 4,
              display: "flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            <BookOpen size={11} />
            {t("chat.knowledge.title")}
          </div>
          {knowledgeBases.map((kb) => {
            const enabled = enabledKnowledgeBaseIds.includes(kb.id);
            const key = `knowledge:${kb.id}`;
            const expanded = enabled && expandedKey === key;
            return (
              <div key={kb.id}>
                <div style={{ padding: "2px 0", display: "flex", alignItems: "center" }}>
                  <Checkbox checked={enabled} onChange={() => onToggleKb(kb.id)}>
                    <span
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 6,
                        fontSize: 13,
                      }}
                    >
                      <KnowledgeBaseIcon kb={kb} size={14} />
                      {kb.name}
                    </span>
                  </Checkbox>
                  {renderExpandIcon(key, enabled)}
                </div>
                {expanded && (
                  <ContextSourceDocsPanel
                    conversationId={conversationId}
                    sourceType="knowledge"
                    refId={kb.id}
                    selectedDocIds={getDocIds("knowledge", kb.id)}
                  />
                )}
              </div>
            );
          })}
        </div>
      )}
      {memoryNamespaces.length > 0 && (
        <div
          style={{
            marginBottom: memoryNamespaces.length > 0 && wikis.length > 0 ? 8 : 0,
          }}
        >
          <div
            style={{
              fontSize: 12,
              fontWeight: 600,
              color: token.colorTextSecondary,
              textTransform: "uppercase",
              letterSpacing: 0.5,
              marginBottom: 4,
              display: "flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            <Brain size={11} />
            {t("chat.memory.title")}
          </div>
          <Radio.Group
            value={activeMemoryNamespaceId}
            onChange={(e) => onSetActiveMemory(e.target.value || null)}
            style={{ display: "flex", flexDirection: "column", gap: 2 }}
          >
            {memoryNamespaces.map((ns) => {
              const enabled = activeMemoryNamespaceId === ns.id;
              const key = `memory:${ns.id}`;
              const expanded = enabled && expandedKey === key;
              return (
                <div key={ns.id}>
                  <div style={{ display: "flex", alignItems: "center" }}>
                    <Radio value={ns.id}>
                      <span
                        style={{
                          fontSize: 13,
                          display: "inline-flex",
                          alignItems: "center",
                          gap: 6,
                        }}
                      >
                        <NamespaceIcon ns={ns} size={16} />
                        {ns.name}
                      </span>
                    </Radio>
                    {renderExpandIcon(key, enabled)}
                  </div>
                  {expanded && (
                    <ContextSourceDocsPanel
                      conversationId={conversationId}
                      sourceType="memory"
                      refId={ns.id}
                      selectedDocIds={getDocIds("memory", ns.id)}
                    />
                  )}
                </div>
              );
            })}
          </Radio.Group>
        </div>
      )}
      {wikis.length > 0 && (
        <div>
          <div
            style={{
              fontSize: 12,
              fontWeight: 600,
              color: token.colorTextSecondary,
              textTransform: "uppercase",
              letterSpacing: 0.5,
              marginBottom: 4,
              display: "flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            <Library size={11} />
            {t("chat.wiki.title")}
          </div>
          {wikis.map((wiki) => {
            const enabled = enabledWikiIds.includes(wiki.id);
            const key = `wiki:${wiki.id}`;
            const expanded = enabled && expandedKey === key;
            return (
              <div key={wiki.id}>
                <div style={{ padding: "2px 0", display: "flex", alignItems: "center" }}>
                  <Checkbox checked={enabled} onChange={() => onToggleWiki(wiki.id)}>
                    <span
                      style={{
                        fontSize: 13,
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 6,
                      }}
                    >
                      <Library size={14} />
                      {wiki.name}
                    </span>
                  </Checkbox>
                  {renderExpandIcon(key, enabled)}
                </div>
                {expanded && (
                  <ContextSourceDocsPanel
                    conversationId={conversationId}
                    sourceType="wiki"
                    refId={wiki.id}
                    selectedDocIds={getDocIds("wiki", wiki.id)}
                  />
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
