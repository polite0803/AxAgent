import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, InputNumber, Switch, Tag, theme } from "antd";
import { X } from "lucide-react";
import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowNode } from "../types";
import { buildBatchUpdate } from "./batchEditUtils";

interface Props {
  selectedNodeIds: Set<string>;
  onClose: () => void;
}

export const BatchEditPanel: React.FC<Props> = ({ selectedNodeIds, onClose }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const nodes = useWorkflowEditorStore((s) => s.nodes);
  const updateNode = useWorkflowEditorStore((s) => s.updateNode);
  const selectedNodes = useMemo(
    () => nodes.filter((n) => selectedNodeIds.has(n.id)),
    [nodes, selectedNodeIds],
  );

  const [timeout, setTimeout_] = useState<number | null>(null);
  const [retryEnabled, setRetryEnabled] = useState<boolean | null>(null);
  const [enabled, setEnabled] = useState<boolean | null>(null);

  const apply = () => {
    for (const node of selectedNodes) {
      const updates = buildBatchUpdate(node as WorkflowNode, { timeout, retryEnabled, enabled });
      if (Object.keys(updates).length === 0) { continue; }
      updateNode(node.id, updates);
    }
    onClose();
  };

  if (selectedNodes.length === 0) { return null; }

  return (
    <div
      style={{
        position: "absolute",
        bottom: "100%",
        right: 0,
        width: 320,
        background: token.colorBgElevated,
        borderRadius: 8,
        border: "1px solid " + token.colorBorderSecondary,
        boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
        padding: 12,
        zIndex: 100,
        display: "flex",
        flexDirection: "column",
        gap: 8,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontSize: 13, fontWeight: 600, color: token.colorText }}>
          {t("workflow.batchEdit.title")}
        </span>
        <Button type="text" size="small" icon={<X size={14} />} onClick={onClose} />
      </div>

      <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
        {selectedNodes.map((n) => <Tag key={n.id} style={{ fontSize: 10, margin: 0 }}>{n.id}</Tag>)}
      </div>

      <Divider style={{ margin: "4px 0" }} />

      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <label style={{ fontSize: 12, color: token.colorTextTertiary }}>{t("workflow.props.timeout")}</label>
          <InputNumber
            size="small"
            value={timeout}
            onChange={(v) => setTimeout_(v ?? null)}
            style={{ width: 100 }}
            min={0}
            max={300}
            placeholder={t("workflow.batchEdit.noChange")}
          />
        </div>

        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <label style={{ fontSize: 12, color: token.colorTextTertiary }}>{t("workflow.props.retryEnabled")}</label>
          <Switch size="small" checked={retryEnabled ?? false} onChange={(v) => setRetryEnabled(v)} />
        </div>

        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <label style={{ fontSize: 12, color: token.colorTextTertiary }}>{t("workflow.props.enabled")}</label>
          <Switch size="small" checked={enabled ?? true} onChange={(v) => setEnabled(v)} />
        </div>
      </div>

      <Button type="primary" size="small" onClick={apply} style={{ marginTop: 4 }}>
        {t("workflow.batchEdit.apply", { count: selectedNodes.length })}
      </Button>
    </div>
  );
};
