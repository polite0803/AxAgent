// SPDX-License-Identifier: AGPL-3.0-only

import { Divider, Input, Select, theme } from "antd";
import React from "react";

import type { NotificationNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const NotificationPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { token } = theme.useToken();
  const n = node as unknown as NotificationNode; // SAFE: WorkflowNode union narrowed to specific node type via config field access
  const c = n.config
    || { channel: "webhook", message: "", webhookUrl: "", recipients: [], subject: "", enabled: true, outputVar: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Channel</label>
        <Select
          value={c.channel}
          onChange={(v) => sc("channel", v)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "webhook", label: "Webhook" },
            { value: "slack", label: "Slack" },
            { value: "wecom", label: "WeCom" },
            { value: "dingtalk", label: "DingTalk" },
            { value: "feishu", label: "Feishu" },
            { value: "log", label: "Log Only" },
          ]}
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Message</label>
        <Input.TextArea value={c.message} onChange={(e) => sc("message", e.target.value)} rows={3} size="small" />
      </div>
      {c.channel === "webhook" && (
        <div>
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Webhook URL</label>
          <Input value={c.webhookUrl ?? ""} onChange={(e) => sc("webhookUrl", e.target.value)} size="small" />
        </div>
      )}
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Output Var</label>
        <Input value={c.outputVar} onChange={(e) => sc("outputVar", e.target.value)} size="small" />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
