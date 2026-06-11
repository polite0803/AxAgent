// SPDX-License-Identifier: AGPL-3.0-only

import { Divider, Input, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { DataTransformerNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";
interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}
export const DataTransformerPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const n = node as unknown as DataTransformerNode;
  const c = n.config || { input_var: "", expression: "", output_var: "" };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Input Variable</label>
        <Input
          value={c.input_var}
          onChange={(e) => sc("input_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.inputVarPlaceholder")}
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>Expression</label>
        <Input.TextArea
          value={c.expression}
          onChange={(e) => sc("expression", e.target.value)}
          rows={3}
          size="small"
          placeholder="$json.path"
        />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
