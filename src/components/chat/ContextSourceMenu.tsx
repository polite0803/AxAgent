// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { useConversationStore, useKnowledgeStore, useLlmWikiStore, useMemoryStore } from "@/stores";
import { Badge, Button, Modal, theme } from "antd";
import { Database } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { SourcePickerPanel } from "./SourcePickerPanel";

export function ContextSourceMenu() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const [sourceModalOpen, setSourceModalOpen] = useState(false);

  const knowledgeBases = useKnowledgeStore((s) => s.bases);
  const memoryNamespaces = useMemoryStore((s) => s.namespaces);
  const wikis = useLlmWikiStore((s) => s.wikis);
  const activeConversationId = useConversationStore(
    (s) => s.activeConversationId,
  );
  const enabledKnowledgeBaseIds = useConversationStore(
    (s) => s.enabledKnowledgeBaseIds,
  );
  const activeMemoryNamespaceId = useConversationStore(
    (s) => s.activeMemoryNamespaceId,
  );
  const enabledWikiIds = useConversationStore((s) => s.enabledWikiIds);
  const toggleKnowledgeBase = useConversationStore(
    (s) => s.toggleKnowledgeBase,
  );
  const setActiveMemoryNamespace = useConversationStore(
    (s) => s.setActiveMemoryNamespace,
  );
  const toggleWiki = useConversationStore((s) => s.toggleWiki);

  const enabledCount = enabledKnowledgeBaseIds.length
    + (activeMemoryNamespaceId ? 1 : 0)
    + enabledWikiIds.length;

  const popoverContent = useMemo(() => {
    const safeKb = knowledgeBases ?? [];
    const safeMem = memoryNamespaces ?? [];
    const safeWikis = wikis ?? [];
    const totalSources = safeKb.length + safeMem.length + safeWikis.length;
    if (totalSources === 0) {
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
            onClick={() => {
              setSourceModalOpen(false);
              navigate("/knowledge");
            }}
          >
            {t("chat.connector.goConfig")}
          </Button>
        </div>
      );
    }
    return (
      <SourcePickerPanel
        conversationId={activeConversationId}
        knowledgeBases={safeKb}
        memoryNamespaces={safeMem}
        wikis={safeWikis}
        enabledKnowledgeBaseIds={enabledKnowledgeBaseIds}
        activeMemoryNamespaceId={activeMemoryNamespaceId}
        enabledWikiIds={enabledWikiIds}
        onToggleKb={toggleKnowledgeBase}
        onSetActiveMemory={setActiveMemoryNamespace}
        onToggleWiki={toggleWiki}
        onGoConfig={() => {
          setSourceModalOpen(false);
          navigate("/knowledge");
        }}
      />
    );
  }, [
    knowledgeBases,
    memoryNamespaces,
    wikis,
    enabledKnowledgeBaseIds,
    activeMemoryNamespaceId,
    enabledWikiIds,
    toggleKnowledgeBase,
    setActiveMemoryNamespace,
    toggleWiki,
    t,
    token,
    navigate,
    activeConversationId,
  ]);

  return (
    <>
      <Tooltip title={t("chat.sources.title")}>
        <Badge
          count={enabledCount}
          size="small"
          offset={[-4, 4]}
          color={token.colorPrimary}
        >
          <Button
            type="text"
            size="small"
            icon={<Database size={14} />}
            onClick={() => setSourceModalOpen(true)}
            style={enabledCount > 0 ? { color: token.colorPrimary } : undefined}
          />
        </Badge>
      </Tooltip>

      <Modal
        title={t("chat.sources.title")}
        open={sourceModalOpen}
        onCancel={() => setSourceModalOpen(false)}
        footer={
          <Button type="primary" onClick={() => setSourceModalOpen(false)}>
            {t("common.confirm")}
          </Button>
        }
        width={420}
        destroyOnHidden
      >
        {popoverContent}
      </Modal>
    </>
  );
}
