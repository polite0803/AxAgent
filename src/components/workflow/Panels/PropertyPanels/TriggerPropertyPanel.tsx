import { Input, Select, Switch } from "antd";
import React from "react";
import type { TriggerNode, TriggerType, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface TriggerPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const TriggerPropertyPanel: React.FC<TriggerPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const triggerNode = node as TriggerNode;
  const triggerConfig = triggerNode.config || { type: "manual" as TriggerType, config: {} };

  const handleTypeChange = (type: TriggerType) => {
    let newConfig = { type, config: {} };
    switch (type) {
      case "schedule":
        newConfig = { type, config: { cron: "", timezone: "UTC", enabled: true } };
        break;
      case "webhook":
        newConfig = { type, config: { path: "", method: "GET", auth_type: "none" } };
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

  const renderTriggerConfig = () => {
    switch (triggerConfig.type) {
      case "schedule":
        return (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div>
              <label style={{ color: "#666", fontSize: 10 }}>Cron 表达式</label>
              <Input
                value={(triggerConfig.config as { cron?: string }).cron || ""}
                onChange={(e) => handleConfigChange("cron", e.target.value)}
                placeholder="* * * * *"
                size="small"
              />
            </div>
            <div>
              <label style={{ color: "#666", fontSize: 10 }}>时区</label>
              <Select
                value={(triggerConfig.config as { timezone?: string }).timezone || "UTC"}
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
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <label style={{ color: "#666", fontSize: 10 }}>启用</label>
              <Switch
                size="small"
                checked={(triggerConfig.config as { enabled?: boolean }).enabled ?? true}
                onChange={(checked) => handleConfigChange("enabled", checked)}
              />
            </div>
          </div>
        );

      case "webhook":
        return (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div>
              <label style={{ color: "#666", fontSize: 10 }}>路径</label>
              <Input
                value={(triggerConfig.config as { path?: string }).path || ""}
                onChange={(e) => handleConfigChange("path", e.target.value)}
                placeholder="/webhook/my-trigger"
                size="small"
              />
            </div>
            <div>
              <label style={{ color: "#666", fontSize: 10 }}>请求方法</label>
              <Select
                value={(triggerConfig.config as { method?: string }).method || "GET"}
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
              <label style={{ color: "#666", fontSize: 10 }}>认证类型</label>
              <Select
                value={(triggerConfig.config as { auth_type?: string }).auth_type || "none"}
                onChange={(value) => handleConfigChange("auth_type", value)}
                size="small"
                style={{ width: "100%" }}
                options={[
                  { value: "none", label: "无认证" },
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
              <label style={{ color: "#666", fontSize: 10 }}>事件类型</label>
              <Input
                value={(triggerConfig.config as { event_type?: string }).event_type || ""}
                onChange={(e) => handleConfigChange("event_type", e.target.value)}
                placeholder="message.created"
                size="small"
              />
            </div>
          </div>
        );

      default:
        return (
          <div style={{ color: "#666", fontSize: 11, padding: "8px 0" }}>
            手动触发无需配置
          </div>
        );
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>触发类型</label>
        <Select
          value={triggerConfig.type}
          onChange={handleTypeChange}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "manual", label: "👆 手动触发" },
            { value: "schedule", label: "⏰ 定时触发" },
            { value: "webhook", label: "🪝 Webhook" },
            { value: "event", label: "⚡ 事件触发" },
          ]}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          触发配置
        </label>
        {renderTriggerConfig()}
      </div>

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
