// SPDX-License-Identifier: AGPL-3.0-only

import { Divider, Input, Select, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { WebhookSendNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const WebhookSendPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const n = node as unknown as WebhookSendNode;
  const c = n.config || { url: "", method: "POST", body: "", headers: {}, output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>URL</label>
        <Input
          value={c.url}
          onChange={(e) => sc("url", e.target.value)}
          size="small"
          placeholder={t("workflow.props.webhookUrlPlaceholder")}
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Method</label>
        <Select
          value={c.method}
          onChange={(v) => sc("method", v)}
          size="small"
          style={{ width: "100%" }}
          options={["GET", "POST", "PUT", "PATCH", "DELETE"].map(m => ({ value: m, label: m }))}
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Body</label>
        <Input.TextArea value={c.body ?? ""} onChange={(e) => sc("body", e.target.value)} rows={3} size="small" />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
