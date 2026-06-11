// SPDX-License-Identifier: AGPL-3.0-only

import { Button, Divider, Input, theme } from "antd";
import { Plus, Trash2 } from "lucide-react";
import React from "react";
import type { LlmClassifierNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const LlmClassifierPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { token } = theme.useToken();
  const n = node as unknown as LlmClassifierNode;
  const c = n.config || { categories: [], prompt: "", model: "", input_var: "", output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Input Variable</label>
        <Input value={c.input_var} onChange={(e) => sc("input_var", e.target.value)} size="small" />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Prompt</label>
        <Input.TextArea value={c.prompt} onChange={(e) => sc("prompt", e.target.value)} rows={3} size="small" />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Categories ({c.categories.length})</label>
        <div style={{ display: "flex", gap: 4, marginBottom: 4 }}>
          {c.categories.map((cat, i) => (
            <span
              key={i}
              style={{
                display: "flex",
                gap: 2,
                alignItems: "center",
                background: token.colorFillSecondary,
                borderRadius: 4,
                padding: "0 4px",
                fontSize: 11,
              }}
            >
              <Input
                size="small"
                value={cat}
                onChange={(e) => {
                  const cats = [...c.categories];
                  cats[i] = e.target.value;
                  sc("categories", cats);
                }}
                style={{ width: 60, fontSize: 11 }}
              />
              <Button
                type="text"
                size="small"
                danger
                icon={<Trash2 size={10} />}
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                onClick={() => sc("categories", c.categories.filter((_: any, j: number) => j !== i))}
              />
            </span>
          ))}
        </div>
        <Button
          size="small"
          icon={<Plus size={12} />}
          onClick={() => sc("categories", [...c.categories, "category_" + (c.categories.length + 1)])}
        >
          Add
        </Button>
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
