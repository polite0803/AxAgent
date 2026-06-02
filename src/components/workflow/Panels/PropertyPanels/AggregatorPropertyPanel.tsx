import { Divider, Select, theme } from "antd";
import React from "react";
import type { AggregatorNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const AggregatorPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { token } = theme.useToken();
  const n = node as unknown as AggregatorNode;
  const c = n.config || { strategy: "all", input_sources: [], output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Strategy</label>
        <Select
          value={c.strategy}
          onChange={(v) => sc("strategy", v)}
          size="small"
          style={{ width: "100%" }}
          options={[{ value: "all", label: "All" }, { value: "any", label: "Any" }, { value: "race", label: "Race" }, {
            value: "majority",
            label: "Majority",
          }]}
        />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
