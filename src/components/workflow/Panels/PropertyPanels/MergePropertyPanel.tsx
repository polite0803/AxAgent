import { Divider, Select, Switch, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { MergeNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface MergePropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const MergePropertyPanel: React.FC<MergePropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const mergeNode = node as MergeNode;
  const config = mergeNode.config || {
    merge_type: "all",
    inputs: [],
    auto_inputs_from_branches: true,
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.props.autoInputsFromBranches")}
        </label>
        <Switch
          size="small"
          checked={config.auto_inputs_from_branches !== false}
          onChange={(checked) => handleConfigChange("auto_inputs_from_branches", checked)}
        />
      </div>
      <div style={{ color: token.colorTextTertiary, fontSize: 12 }}>
        {t("workflow.props.autoInputsFromBranchesHint")}
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.mergeType")}
        </label>
        <Select
          value={config.merge_type}
          onChange={(value) => handleConfigChange("merge_type", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "all", label: t("workflow.props.mergeAll") },
            { value: "any", label: t("workflow.props.mergeAny") },
            { value: "race", label: t("workflow.props.mergeRace") },
            { value: "majority", label: t("workflow.props.mergeMajority") },
          ]}
        />
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.inputCount")} ({config.inputs?.length || 0})
        </label>
        <div style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {config.auto_inputs_from_branches !== false
            ? t("workflow.props.connectInputsHint")
            : t("workflow.props.connectInputsManually")}
        </div>
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      <div
        style={{ borderTop: `1px solid ${token.colorBorderSecondary}`, paddingTop: 12, marginTop: 4 }}
      >
        <BasePropertyPanel
          node={node}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      </div>
    </div>
  );
};
