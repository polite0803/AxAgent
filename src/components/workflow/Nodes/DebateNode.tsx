import { useWorkflowEditorStore } from "@/stores";
import { Tag, theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import type { NodeProps } from "reactflow";

const BLUE_BASE = "#1890ff";
const BLUE_VAR = `var(--blue, ${BLUE_BASE})`;

interface DebateNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  debaterSteps?: string[];
  maxRounds?: number;
  convergencePrompt?: string;
}

const DebateNodeComponent: React.FC<NodeProps<DebateNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = BLUE_VAR;
  const debaterCount = data.debaterSteps?.length || 0;
  const maxRounds = data.maxRounds || 2;

  const isCollapsed = useWorkflowEditorStore((s) => s.collapsedContainers.has(data.id));
  const toggleCollapse = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      useWorkflowEditorStore.getState().toggleContainerCollapse(data.id);
    },
    [data.id],
  );

  return (
    <div
      style={{
        minWidth: 400,
        minHeight: 200,
        background: `${BLUE_BASE}08`,
        border: `2px dashed ${selected ? token.colorPrimary : BLUE_BASE}40`,
        borderRadius: 12,
        padding: 12,
        opacity: data.enabled ? 1 : 0.5,
        position: "relative",
        boxShadow: selected ? `0 0 0 2px ${BLUE_VAR}40` : "none",
      }}
    >
      <div
        className="workflow-container-drag-handle"
        style={{
          position: "absolute",
          top: 8,
          left: 12,
          display: "flex",
          alignItems: "center",
          gap: 6,
          background: token.colorBgElevated,
          border: `1px solid ${BLUE_BASE}30`,
          borderRadius: 6,
          padding: "4px 10px",
          zIndex: 10,
          cursor: "grab",
        }}
      >
        <span style={{ fontSize: 14 }}>⚖️</span>
        <span style={{ fontSize: 12, color, fontWeight: 600 }}>
          {data.title}
        </span>
        <div style={{ display: "flex", gap: 4 }}>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${BLUE_BASE}20`,
              border: `1px solid ${BLUE_BASE}50`,
              color: BLUE_VAR,
            }}
          >
            {debaterCount} {t("workflow.debateNode.debaters", { defaultValue: "debaters" })}
          </Tag>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: token.colorBgContainer,
              border: `1px solid ${token.colorBorderSecondary}`,
              color: token.colorTextTertiary,
            }}
          >
            {maxRounds} {t("workflow.debateNode.rounds", { defaultValue: "rounds" })}
          </Tag>
        </div>
      </div>

      <Tooltip
        title={isCollapsed
          ? t("workflow.debateNode.expand", { defaultValue: "Expand" })
          : t("workflow.debateNode.collapse", { defaultValue: "Collapse" })}
      >
        <span
          onClick={toggleCollapse}
          style={{
            position: "absolute",
            top: 8,
            right: 12,
            cursor: "pointer",
            fontSize: 14,
            lineHeight: 1,
            padding: "2px 6px",
            borderRadius: 4,
            background: token.colorBgElevated,
            border: `1px solid ${BLUE_BASE}30`,
            zIndex: 10,
            opacity: 0.7,
            transition: "opacity 0.2s, transform 0.2s",
            transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)",
            display: "inline-block",
            userSelect: "none",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "1";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "0.7";
          }}
        >
          ▼
        </span>
      </Tooltip>
    </div>
  );
};

export const DebateNode = memo(DebateNodeComponent);
