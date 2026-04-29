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

  const renderDelayConfig = () => {
    switch (config.delay_type) {
      case "seconds":
        return (
          <div>
            <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>延迟秒数</label>
            <InputNumber
              value={config.seconds ?? 5}
              onChange={(value) => handleConfigChange("seconds", value)}
              min={1}
              max={31536000}
              size="small"
              style={{ width: "100%" }}
            />
            <div style={{ fontSize: 10, color: "#666", marginTop: 4 }}>
              {(config.seconds || 5) >= 60
                ? `≈ ${Math.floor((config.seconds || 5) / 60)} 分钟`
                : `${config.seconds || 5} 秒`}
            </div>
          </div>
        );

      case "minutes":
        return (
          <div>
            <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>延迟分钟数</label>
            <InputNumber
              value={Math.floor((config.seconds || 5) / 60)}
              onChange={(value) => handleConfigChange("seconds", (value || 1) * 60)}
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
            <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>延迟小时数</label>
            <InputNumber
              value={Math.floor((config.seconds || 5) / 3600)}
              onChange={(value) => handleConfigChange("seconds", (value || 1) * 3600)}
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
            <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>延迟到</label>
            <Input
              value={config.until || ""}
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
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.delayType")}</label>
        <Select
          value={config.delay_type}
          onChange={(value) => handleConfigChange("delay_type", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "seconds", label: "秒" },
            { value: "minutes", label: "分钟" },
            { value: "hours", label: "小时" },
            { value: "until", label: "直到指定时间" },
          ]}
        />
      </div>

      {renderDelayConfig()}

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
