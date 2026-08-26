// SPDX-License-Identifier: AGPL-3.0-only

import { Divider, Input, InputNumber, Select, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { MultiAgentNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface MultiAgentPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const MultiAgentPropertyPanel: React.FC<MultiAgentPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const n = node as unknown as MultiAgentNode;
  const c = n.config
    || { task: "", role: undefined, model: undefined, outputVar: "multiAgentOutput", mode: "auto", maxRounds: 5 };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.property.task")}
        </label>
        <Input.TextArea
          rows={4}
          value={c.task}
          onChange={(e) => sc("task", e.target.value)}
          placeholder={t("workflow.placeholder.task")}
          size="small"
        />
      </div>

      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.property.role")}
        </label>
        <Input
          value={c.role ?? ""}
          onChange={(e) => sc("role", e.target.value || undefined)}
          placeholder={t("workflow.placeholder.role")}
          size="small"
        />
      </div>

      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.property.model")}
        </label>
        <Input
          value={c.model ?? ""}
          onChange={(e) => sc("model", e.target.value || undefined)}
          placeholder={t("workflow.placeholder.model")}
          size="small"
        />
      </div>

      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.property.outputVar")}
        </label>
        <Input
          value={c.outputVar}
          onChange={(e) => sc("outputVar", e.target.value)}
          placeholder={t("workflow.placeholder.outputVar")}
          size="small"
        />
      </div>

      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.property.mode")}
        </label>
        <Select
          value={c.mode}
          onChange={(mode) => sc("mode", mode)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "auto", label: t("workflow.mode.auto") },
            { value: "swarm", label: t("workflow.mode.swarm") },
            { value: "debate", label: t("workflow.mode.debate") },
          ]}
        />
      </div>

      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.property.maxRounds")}
        </label>
        <InputNumber
          min={1}
          max={20}
          value={c.maxRounds}
          onChange={(v) => sc("maxRounds", v ?? 5)}
          size="small"
          style={{ width: "100%" }}
        />
      </div>

      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
