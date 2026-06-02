import { Divider, Input, InputNumber, Select, theme } from "antd";
import React from "react";
import type { ApprovalNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const ApprovalPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { token } = theme.useToken();
  const n = node as unknown as ApprovalNode;
  const c = n.config
    || { message: "", approver: "", timeout_secs: 86400, timeout_action: "auto_reject", output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Message</label>
        <Input.TextArea value={c.message} onChange={(e) => sc("message", e.target.value)} rows={3} size="small" />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Approver</label>
        <Input value={c.approver ?? ""} onChange={(e) => sc("approver", e.target.value)} size="small" />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Timeout (s)</label>
        <InputNumber
          value={c.timeout_secs}
          onChange={(v) => sc("timeout_secs", v ?? 86400)}
          size="small"
          style={{ width: "100%" }}
          min={0}
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Timeout Action</label>
        <Select
          value={c.timeout_action}
          onChange={(v) => sc("timeout_action", v)}
          size="small"
          style={{ width: "100%" }}
          options={[{ value: "auto_reject", label: "Auto Reject" }, { value: "auto_approve", label: "Auto Approve" }]}
        />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
