import { Divider, Input, Select, theme } from "antd";
import React from "react";
import type { FileOperationNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const FileOperationPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { token } = theme.useToken();
  const n = node as unknown as FileOperationNode;
  const c = n.config || { operation: "read", file_path: "", content: "", output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Operation</label>
        <Select
          value={c.operation}
          onChange={(v) => sc("operation", v)}
          size="small"
          style={{ width: "100%" }}
          options={[{ value: "read", label: "Read" }, { value: "write", label: "Write" }, {
            value: "delete",
            label: "Delete",
          }, { value: "exists", label: "Exists" }]}
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>File Path</label>
        <Input
          value={c.file_path}
          onChange={(e) => sc("file_path", e.target.value)}
          size="small"
          placeholder="/path/to/file"
        />
      </div>
      {c.operation === "write" && (
        <div>
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Content</label>
          <Input.TextArea
            value={c.content ?? ""}
            onChange={(e) => sc("content", e.target.value)}
            rows={5}
            size="small"
          />
        </div>
      )}
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Output Var</label>
        <Input value={c.output_var} onChange={(e) => sc("output_var", e.target.value)} size="small" />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
