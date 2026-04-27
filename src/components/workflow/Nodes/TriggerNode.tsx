import { Tag } from "antd";
import React, { memo } from "react";
import { Handle, type NodeProps, Position } from "reactflow";

interface TriggerNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  triggerConfig?: {
    type: "manual" | "schedule" | "webhook" | "event";
    config: unknown;
  };
}

const TriggerNodeComponent: React.FC<NodeProps<TriggerNodeData>> = ({ data, selected }) => {
  const triggerType = data.triggerConfig?.type || "manual";
  const color = "#722ed1";

  const getTriggerIcon = (type: string): string => {
    switch (type) {
      case "manual":
        return "👆";
      case "schedule":
        return "⏰";
      case "webhook":
        return "🪝";
      case "event":
        return "⚡";
      default:
        return "⚡";
    }
  };

  const getTriggerDescription = (type: string, config: unknown): string => {
    switch (type) {
      case "manual":
        return "手动触发";
      case "schedule":
        const scheduleConfig = config as { cron?: string; timezone?: string };
        return scheduleConfig.cron ? `Cron: ${scheduleConfig.cron}` : "定时触发";
      case "webhook":
        const webhookConfig = config as { path?: string; method?: string };
        return webhookConfig.path ? `${webhookConfig.method || "GET"} ${webhookConfig.path}` : "Webhook";
      case "event":
        const eventConfig = config as { event_type?: string };
        return eventConfig.event_type || "事件触发";
      default:
        return "";
    }
  };

  return (
    <div
      style={{
        minWidth: 180,
        maxWidth: 220,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: "#1e1e1e",
          border: `2px solid ${selected ? "#1890ff" : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "all 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${color}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${color}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>{getTriggerIcon(triggerType)}</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.5px",
            }}
          >
            触发器
          </span>
        </div>

        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: "#fff",
              fontWeight: 500,
              marginBottom: 6,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <Tag
              color={color}
              style={{
                margin: 0,
                fontSize: 10,
                padding: "0 4px",
                borderRadius: 4,
              }}
            >
              {triggerType.toUpperCase()}
            </Tag>
            <span
              style={{
                fontSize: 10,
                color: "#888",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                flex: 1,
              }}
            >
              {getTriggerDescription(triggerType, data.triggerConfig?.config)}
            </span>
          </div>
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: color,
          border: "none",
          width: 10,
          height: 10,
        }}
      />

      <div
        style={{
          position: "absolute",
          top: -10,
          left: "50%",
          transform: "translateX(-50%)",
          width: 0,
          height: 0,
          borderLeft: "6px solid transparent",
          borderRight: "6px solid transparent",
          borderBottom: `8px solid ${color}`,
        }}
      />
    </div>
  );
};

export const TriggerNode = memo(TriggerNodeComponent);
