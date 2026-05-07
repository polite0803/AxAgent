import { invoke } from "@/lib/invoke";
import { useMemoryStore } from "@/stores/feature/memoryStore";
import { App, Button, Empty, Modal, Select, Spin, theme, Typography } from "antd";
import { Brain } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ExtractMemoriesModalProps {
  open: boolean;
  onClose: () => void;
  conversationId: string;
}

export function ExtractMemoriesModal({ open, onClose, conversationId }: ExtractMemoriesModalProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi } = App.useApp();

  const namespaces = useMemoryStore((s) => s.namespaces);
  const loadNamespaces = useMemoryStore((s) => s.loadNamespaces);
  const loading = useMemoryStore((s) => s.loading);

  const [selectedNamespaceId, setSelectedNamespaceId] = useState<string | null>(null);
  const [extracting, setExtracting] = useState(false);

  useEffect(() => {
    if (open) {
      setSelectedNamespaceId(null);
      loadNamespaces();
    }
  }, [open, loadNamespaces]);

  const handleOk = useCallback(async () => {
    if (!selectedNamespaceId || !conversationId) { return; }
    setExtracting(true);
    try {
      const count = await invoke<number>("extract_conversation_memories", {
        conversationId,
        namespaceId: selectedNamespaceId,
      });
      messageApi.success(t("chat.extractMemoriesSuccess", { count }));
      onClose();
    } catch (e) {
      messageApi.error(t("chat.extractMemoriesError"));
    } finally {
      setExtracting(false);
    }
  }, [selectedNamespaceId, conversationId, messageApi, t, onClose]);

  const namespaceOptions = namespaces.map((ns) => ({
    value: ns.id,
    label: ns.name,
  }));

  const hasNamespaces = namespaces.length > 0;

  return (
    <Modal
      title={
        <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
          <Brain size={16} style={{ color: token.colorPrimary }} />
          {t("chat.extractMemoriesTitle")}
        </span>
      }
      open={open}
      onCancel={onClose}
      destroyOnHidden
      width={440}
      footer={[
        <Button key="cancel" onClick={onClose} disabled={extracting}>
          {t("common.cancel")}
        </Button>,
        <Button
          key="ok"
          type="primary"
          onClick={handleOk}
          loading={extracting}
          disabled={!selectedNamespaceId || !hasNamespaces}
        >
          {extracting ? t("chat.extractMemoriesProcessing") : t("chat.extractMemories")}
        </Button>,
      ]}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16, marginTop: 8 }}>
        <Typography.Text type="secondary">
          {t("chat.extractMemoriesDesc")}
        </Typography.Text>
        {loading ? (
          <div style={{ display: "flex", justifyContent: "center", padding: "16px 0" }}>
            <Spin size="small" />
          </div>
        ) : hasNamespaces ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <Typography.Text style={{ fontSize: 13 }}>
              {t("chat.extractMemoriesSelectNamespace")}
            </Typography.Text>
            <Select
              value={selectedNamespaceId}
              onChange={setSelectedNamespaceId}
              options={namespaceOptions}
              placeholder={t("chat.extractMemoriesSelectNamespace")}
              style={{ width: "100%" }}
              disabled={extracting}
            />
          </div>
        ) : (
          <Empty
            description={t("chat.extractMemoriesNoNamespaces")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        )}
      </div>
    </Modal>
  );
}
