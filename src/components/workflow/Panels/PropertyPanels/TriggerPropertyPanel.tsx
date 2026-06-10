import { Input, message, Select, Switch, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
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
  triggerConfig: { type: string; config: unknown };
  handleConfigChange: (key: string, value: unknown) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const [messageApi, contextHolder] = message.useMessage();

  /* eslint-disable @typescript-eslint/no-explicit-any */
  const handleAISuggestCron = async () => {
    const current = ((triggerConfig.config as any).cron as string) || "";
    const hint = current.trim() || t("workflow.aiAssist.trigger.cronHint");
    const result = await aiGenerate({
      systemPrompt:
        "你是一个 cron 表达式专家。根据用户的自然语言描述，输出标准 5 段式 cron 表达式（minute hour day-of-month month day-of-week）。"
        + "只输出 cron 字符串本身，不要任何解释或 Markdown 标记。",
      userPrompt: hint,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    const cleaned = result.split("\n")[0].trim().replace(/^```\w*\s*|\s*```$/g, "");
    handleConfigChange("cron", cleaned);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  switch (triggerConfig.type) {
    case "schedule":
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {contextHolder}
          <div>
            <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.cronExpression")}
            </label>
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <Input
                id="trigger-property-panel-input-113"
                value={((triggerConfig.config as any).cron as string) || ""}
                onChange={(e) => handleConfigChange("cron", e.target.value)}
                placeholder="* * * * *"
                size="small"
              />
              <AIAssistButton
                labelKey="suggest"
                loading={aiGenerating}
                onClick={handleAISuggestCron}
                compact
              />
            </div>
          </div>
          <div>
            <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.timezone")}
            </label>
            <Select
              value={((triggerConfig.config as any).timezone as string) || "UTC"}
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
            <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.enabled")}
            </label>
            <Switch
              size="small"
              checked={((triggerConfig.config as any).enabled as boolean) ?? true}
              onChange={(checked) => handleConfigChange("enabled", checked)}
            />
          </div>
        </div>
      );

    case "webhook":
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div>
            <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.webhookPath")}
            </label>
            <Input
              id="trigger-property-panel-input-114"
              value={((triggerConfig.config as any).path as string) || ""}
              onChange={(e) => handleConfigChange("path", e.target.value)}
              placeholder="/webhook/my-trigger"
              size="small"
            />
          </div>
          <div>
            <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.httpMethod")}
            </label>
            <Select
              value={((triggerConfig.config as any).method as string) || "GET"}
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
            <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.authType")}
            </label>
            <Select
              value={((triggerConfig.config as any).auth_type as string) || "none"}
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
            <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.eventType")}
            </label>
            <Input
              id="trigger-property-panel-input-115"
              value={((triggerConfig.config as any).event_type as string) || ""}
              onChange={(e) => handleConfigChange("event_type", e.target.value)}
              size="small"
            />
          </div>
        </div>
      );

    default:
      return (
        <div style={{ color: token.colorTextTertiary, fontSize: 12, padding: "8px 0" }}>
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
  const { token } = theme.useToken();
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
          ...(triggerConfig.config as any),
          [key]: value,
        },
      },
    });
  };
  /* eslint-enable @typescript-eslint/no-explicit-any */

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
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
            color: token.colorTextTertiary,
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
