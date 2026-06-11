// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";

const ORANGE_BASE = "#fa8c16";
const ORANGE_VAR = `var(--orange, ${ORANGE_BASE})`;

interface DelayNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  delayType?: "fixed" | "random" | "cron";
  delayMs?: number;
  delayMinMs?: number;
  delayMaxMs?: number;
}

const DelayNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as DelayNodeData;
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = ORANGE_VAR;
  const delayType = data.delayType || "fixed";

  const formatDelay = (ms: number): string => {
    if (ms < 1000) {
      return `${ms}ms`;
    }
    if (ms < 60000) {
      return `${(ms / 1000).toFixed(1)}s`;
    }
    if (ms < 3600000) {
      return `${(ms / 60000).toFixed(1)}min`;
    }
    return `${(ms / 3600000).toFixed(1)}h`;
  };

  const getDelayDescription = (): string => {
    switch (delayType) {
      case "fixed":
        return data.delayMs ? formatDelay(data.delayMs) : t("workflow.delayNode.notConfigured");
      case "random":
        return data.delayMinMs && data.delayMaxMs
          ? `${formatDelay(data.delayMinMs)} ~ ${formatDelay(data.delayMaxMs)}`
          : t("workflow.delayNode.notConfigured");
      case "cron":
        return "Cron";
      default:
        return t("workflow.delayNode.notConfigured");
    }
  };

  return (
    <div
      style={{
        minWidth: 160,
        maxWidth: 200,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: token.colorBgElevated,
          border: `2px solid ${selected ? token.colorPrimary : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${ORANGE_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${ORANGE_BASE}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>⏱️</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.delayNode.title")}
          </span>
        </div>

        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: token.colorText,
              fontWeight: 500,
              marginBottom: 6,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>

          <div
            style={{
              fontSize: 12,
              color: token.colorTextTertiary,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {getDelayDescription()}
          </div>
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />
    </div>
  );
};

export const DelayNode = memo(DelayNodeComponent);
