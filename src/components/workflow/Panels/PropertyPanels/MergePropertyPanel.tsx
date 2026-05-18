import { Divider, Select } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { MergeNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface MergePropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const MergePropertyPanel: React.FC<MergePropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const mergeNode = node as MergeNode;
  const config = mergeNode.config || {
    merge_type: "all",
    inputs: [],
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.mergeType")}
        </label>
        <Select
          value={config.merge_type}
          onChange={(value) => handleConfigChange("merge_type", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "all", label: t("workflow.props.mergeAll") },
            { value: "first", label: t("workflow.props.mergeFirst") },
            { value: "last", label: t("workflow.props.mergeLast") },
          ]}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.inputCount")} ({config.inputs?.length || 0})
        </label>
        <div style={{ color: "#666", fontSize: 12 }}>
          {t("workflow.props.connectInputsHint")}
        </div>
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
