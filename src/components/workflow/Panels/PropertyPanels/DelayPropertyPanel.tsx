import { Divider, Input, InputNumber, Select } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { DelayNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface DelayPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

/**
 * Extracted component for rendering the delay config.
 * Fixes react-doctor/no-render-in-render by moving renderDelayConfig() out of DelayPropertyPanel.
 */
function DelayConfig({ config, handleConfigChange }: {
  config: Record<string, any>;
  handleConfigChange: (key: string, value: unknown) => void;
}) {
  const { t } = useTranslation();

  switch (config.delay_type as string) {
    case "seconds":
      return (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.delaySeconds")}
          </label>
          <InputNumber
            id="delay-property-panel-inputnumber-89"
            value={(config.seconds as number) ?? 5}
            onChange={(value) => handleConfigChange("seconds", value)}
            min={1}
            max={31536000}
            size="small"
            style={{ width: "100%" }}
          />
          <div style={{ fontSize: 10, color: "#666", marginTop: 4 }}>
            {((config.seconds as number) || 5) >= 60
              ? `≈ ${Math.floor(((config.seconds as number) || 5) / 60)} ${t("workflow.props.minutes")}`
              : `${(config.seconds as number) || 5} ${t("workflow.props.seconds")}`}
          </div>
        </div>
      );

    case "minutes":
      return (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.delayMinutes")}
          </label>
          <InputNumber
            id="delay-property-panel-inputnumber-90"
            value={Math.floor(((config.seconds as number) || 5) / 60)}
            onChange={(value) => handleConfigChange("seconds", ((value as number) || 1) * 60)}
            min={1}
            max={525600}
            size="small"
            style={{ width: "100%" }}
          />
        </div>
      );

    case "hours":
      return (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.delayHours")}
          </label>
          <InputNumber
            id="delay-property-panel-inputnumber-91"
            value={Math.floor(((config.seconds as number) || 5) / 3600)}
            onChange={(value) => handleConfigChange("seconds", ((value as number) || 1) * 3600)}
            min={1}
            max={8760}
            size="small"
            style={{ width: "100%" }}
          />
        </div>
      );

    case "until":
      return (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.delayUntil")}
          </label>
          <Input
            id="delay-property-panel-input-92"
            value={(config.until as string) || ""}
            onChange={(e) => handleConfigChange("until", e.target.value)}
            size="small"
            placeholder={t("workflow.props.delayUntilPlaceholder")}
          />
          <div style={{ fontSize: 10, color: "#666", marginTop: 4 }}>
            {t("workflow.props.delayUntilHint")}
          </div>
        </div>
      );

    default:
      return null;
  }
}

export const DelayPropertyPanel: React.FC<DelayPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const delayNode = node as DelayNode;
  const config = delayNode.config || {
    delay_type: "seconds",
    seconds: 5,
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.delayType")}
        </label>
        <Select
          value={config.delay_type}
          onChange={(value) => handleConfigChange("delay_type", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "seconds", label: t("workflow.props.seconds") },
            { value: "minutes", label: t("workflow.props.minutes") },
            { value: "hours", label: t("workflow.props.hours") },
            { value: "until", label: t("workflow.props.untilSpecifiedTime") },
          ]}
        />
      </div>

      <DelayConfig config={config} handleConfigChange={handleConfigChange} />

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
