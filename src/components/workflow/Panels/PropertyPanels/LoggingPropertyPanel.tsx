import { Divider, Input, Select, theme } from "antd";
import React from "react";
import type { LoggingNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const LoggingPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { token } = theme.useToken();
  const n = node as unknown as LoggingNode;
  const c = n.config || { level: "info", message: "", output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Level</label>
        <Select
          value={c.level}
          onChange={(v) => sc("level", v)}
          size="small"
          style={{ width: "100%" }}
          options={[{ value: "debug", label: "DEBUG" }, { value: "info", label: "INFO" }, {
            value: "warn",
            label: "WARN",
          }, { value: "error", label: "ERROR" }]}
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Message</label>
        <Input.TextArea value={c.message} onChange={(e) => sc("message", e.target.value)} rows={3} size="small" />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
