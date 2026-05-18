import { Input, Select, Switch } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { TriggerNode, TriggerType, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface TriggerPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

/**
 * Extracted component for rendering the trigger config by type.
 * Fixes react-doctor/no-render-in-render by moving renderTriggerConfig() out of TriggerPropertyPanel.
 */
function TriggerConfig({
  triggerConfig,
  handleConfigChange,
}: {
  triggerConfig: { type: string; config: any };
  handleConfigChange: (key: string, value: unknown) => void;
}) {
  const { t } = useTranslation();

  switch (triggerConfig.type) {
    case "schedule":
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div>
            <label style={{ color: "#666", fontSize: 12 }}>
              {t("workflow.props.cronExpression")}
            </label>
            <Input
              id="trigger-property-panel-input-113"
              value={(triggerConfig.config.cron as string) || ""}
              onChange={(e) => handleConfigChange("cron", e.target.value)}
              placeholder="* * * * *"
              size="small"
            />
          </div>
          <div>
            <label style={{ color: "#666", fontSize: 12 }}>
              {t("workflow.props.timezone")}
            </label>
            <Select
              value={(triggerConfig.config.timezone as string) || "UTC"}
              onChange={(value) => handleConfigChange("timezone", value)}
              size="small"
              style={{ width: "100%" }}
              options={[
                { value: "UTC", label: "UTC" },
                { value: "Asia/Shanghai", label: "Asia/Shanghai" },
                { value: "America/New_York", label: "America/New_York" },
                { value: "America/Los_Angeles", label: "America/Los_Angeles" },
                { value: "Europe/London", label: "Europe/London" },
                { value: "Europe/Paris", label: "Europe/Paris" },
                { value: "Asia/Tokyo", label: "Asia/Tokyo" },
                { value: "Asia/Singapore", label: "Asia/Singapore" },
              ]}
            />
          </div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <label style={{ color: "#666", fontSize: 12 }}>
              {t("workflow.props.enabled")}
            </label>
            <Switch
              size="small"
              checked={(triggerConfig.config.enabled as boolean) ?? true}
              onChange={(checked) => handleConfigChange("enabled", checked)}
            />
          </div>
        </div>
      );

    case "webhook":
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div>
            <label style={{ color: "#666", fontSize: 12 }}>
              {t("workflow.props.webhookPath")}
            </label>
            <Input
              id="trigger-property-panel-input-114"
              value={(triggerConfig.config.path as string) || ""}
              onChange={(e) => handleConfigChange("path", e.target.value)}
              placeholder="/webhook/my-trigger"
              size="small"
            />
          </div>
          <div>
            <label style={{ color: "#666", fontSize: 12 }}>
              {t("workflow.props.httpMethod")}
            </label>
            <Select
              value={(triggerConfig.config.method as string) || "GET"}
              onChange={(value) => handleConfigChange("method", value)}
              size="small"
              style={{ width: "100%" }}
              options={[
                { value: "GET", label: "GET" },
                { value: "POST", label: "POST" },
                { value: "PUT", label: "PUT" },
                { value: "DELETE", label: "DELETE" },
              ]}
            />
          </div>
          <div>
            <label style={{ color: "#666", fontSize: 12 }}>
              {t("workflow.props.authType")}
            </label>
            <Select
              value={(triggerConfig.config.auth_type as string) || "none"}
              onChange={(value) => handleConfigChange("auth_type", value)}
              size="small"
              style={{ width: "100%" }}
              options={[
                { value: "none", label: t("workflow.props.authNone") },
                { value: "bearer", label: "Bearer Token" },
                { value: "api_key", label: "API Key" },
                { value: "basic", label: "Basic Auth" },
              ]}
            />
          </div>
        </div>
      );

    case "event":
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div>
            <label style={{ color: "#666", fontSize: 12 }}>
              {t("workflow.props.eventType")}
            </label>
            <Input
              id="trigger-property-panel-input-115"
              value={(triggerConfig.config.event_type as string) || ""}
              onChange={(e) => handleConfigChange("event_type", e.target.value)}
              size="small"
            />
          </div>
        </div>
      );

    default:
      return (
        <div style={{ color: "#666", fontSize: 12, padding: "8px 0" }}>
          {t("workflow.props.manualNoConfig")}
        </div>
      );
  }
}

export const TriggerPropertyPanel: React.FC<TriggerPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const triggerNode = node as TriggerNode;
  const triggerConfig = triggerNode.config || {
    type: "manual" as TriggerType,
    config: {},
  };

  const handleTypeChange = (type: TriggerType) => {
    let newConfig = { type, config: {} };
    switch (type) {
      case "schedule":
        newConfig = {
          type,
          config: { cron: "", timezone: "UTC", enabled: true },
        };
        break;
      case "webhook":
        newConfig = {
          type,
          config: { path: "", method: "GET", auth_type: "none" },
        };
        break;
      case "event":
        newConfig = { type, config: { event_type: "", filter: {} } };
        break;
    }
    onUpdate({ config: newConfig });
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({
      config: {
        ...triggerConfig,
        config: {
          ...(triggerConfig.config as Record<string, unknown>),
          [key]: value,
        },
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.triggerType")}
        </label>
        <Select
          value={triggerConfig.type}
          onChange={handleTypeChange}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "manual", label: t("workflow.props.manualTrigger") },
            { value: "schedule", label: t("workflow.props.scheduleTrigger") },
            { value: "webhook", label: "🪝 Webhook" },
            { value: "event", label: t("workflow.props.eventTrigger") },
          ]}
        />
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.triggerConfig")}
        </label>
        <TriggerConfig
          triggerConfig={triggerConfig}
          handleConfigChange={handleConfigChange}
        />
      </div>

      <div
        style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}
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
