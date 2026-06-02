import { useWorkflowEditorStore } from "@/stores";
import { Tag, theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

const MAGENTA_BASE = "#eb2f96";
const MAGENTA_VAR = `var(--magenta, ${MAGENTA_BASE})`;

interface SubWorkflowNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  subWorkflowId?: string;
  subWorkflowName?: string;
  inputMapping?: Record<string, string>;
  outputMapping?: Record<string, string>;
}

const SubWorkflowNodeComponent: React.FC<NodeProps<SubWorkflowNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = MAGENTA_VAR;
  const subWorkflowName = data.subWorkflowName;
  const inputMapping = data.inputMapping || {};
  const outputMapping = data.outputMapping || {};

  const inputCount = Object.keys(inputMapping).length;
  const outputCount = Object.keys(outputMapping).length;

  const expandedData = useWorkflowEditorStore((s) => s.expandedSubWorkflows[data.id]);
  const toggleExpand = useCallback(() => {
    useWorkflowEditorStore.getState().toggleExpandSubWorkflow(data.id, data.subWorkflowId);
  }, [data.id, data.subWorkflowId]);

  const isExpanded = !!expandedData && !expandedData.isLoading;
  const isLoading = !!expandedData?.isLoading;
  const childCount = isExpanded ? expandedData?.nodes?.length || 0 : 0;
  const childEdgeCount = isExpanded ? expandedData?.edges?.length || 0 : 0;

  if (isExpanded) {
    return (
      <div
        style={{
          minWidth: 400,
          minHeight: 200,
          background: `${MAGENTA_BASE}08`,
          border: `2px dashed ${selected ? token.colorPrimary : MAGENTA_BASE}40`,
          borderRadius: 12,
          padding: 12,
          opacity: data.enabled ? 1 : 0.5,
          position: "relative",
          boxShadow: selected ? `0 0 0 2px ${MAGENTA_VAR}40` : "none",
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
            border: `1px solid ${MAGENTA_BASE}30`,
            borderRadius: 6,
            padding: "4px 10px",
            zIndex: 10,
            cursor: "grab",
          }}
        >
          <span style={{ fontSize: 14 }}>🔄</span>
          <span style={{ fontSize: 12, color, fontWeight: 600 }}>
            {data.title}
          </span>
          {subWorkflowName && (
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: `${MAGENTA_BASE}20`,
                border: `1px solid ${MAGENTA_BASE}50`,
                color: MAGENTA_VAR,
              }}
            >
              📋 {subWorkflowName}
            </Tag>
          )}
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${MAGENTA_BASE}15`,
              border: `1px solid ${MAGENTA_BASE}40`,
              color: MAGENTA_BASE,
            }}
          >
            🔓 {childCount} nodes · {childEdgeCount} edges
          </Tag>
        </div>

        <Tooltip title={t("workflow.subWorkflowNode.collapse")}>
          <span
            className="react-flow__nodrag"
            onClick={(e) => {
              e.stopPropagation();
              toggleExpand();
            }}
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
              border: `1px solid ${MAGENTA_BASE}30`,
              zIndex: 10,
              opacity: 0.7,
              transition: "opacity 0.2s, transform 0.2s",
              transform: "rotate(0deg)",
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
  }

  return (
    <div
      style={{
        minWidth: 200,
        maxWidth: 240,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
        position: "relative",
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
          className="workflow-container-drag-handle"
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${MAGENTA_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${MAGENTA_BASE}15`,
            cursor: "grab",
          }}
        >
          <span style={{ fontSize: 14 }}>🔄</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
              flex: 1,
            }}
          >
            {t("workflow.subWorkflowNode.title")}
          </span>

          {data.subWorkflowId && (
            <Tooltip title={t("workflow.subWorkflowNode.expand")}>
              <span
                className="react-flow__nodrag"
                onClick={(e) => {
                  e.stopPropagation();
                  toggleExpand();
                }}
                style={{
                  cursor: "pointer",
                  fontSize: 14,
                  lineHeight: 1,
                  padding: "2px 4px",
                  borderRadius: 4,
                  opacity: isLoading ? 0.5 : 0.7,
                  transition: "opacity 0.2s, transform 0.2s",
                  transform: "rotate(0deg)",
                  display: "inline-block",
                }}
                onMouseEnter={(e) => {
                  (e.target as HTMLElement).style.opacity = "1";
                }}
                onMouseLeave={(e) => {
                  (e.target as HTMLElement).style.opacity = "0.7";
                }}
              >
                {isLoading ? "⏳" : "▼"}
              </span>
            </Tooltip>
          )}
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

          {subWorkflowName && (
            <div
              style={{
                fontSize: 12,
                color: color,
                marginBottom: 6,
                padding: "4px 6px",
                background: `${MAGENTA_BASE}15`,
                borderRadius: 4,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                fontWeight: 500,
              }}
            >
              📋 {subWorkflowName}
            </div>
          )}

          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {inputCount > 0 && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: token.colorBgContainer,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  color: token.colorTextQuaternary,
                }}
              >
                📥 {t("workflow.subWorkflowNode.inputCount", { count: inputCount })}
              </Tag>
            )}

            {outputCount > 0 && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: `${token.colorPrimary}20`,
                  border: `1px solid ${token.colorPrimary}50`,
                  color: token.colorPrimary,
                }}
              >
                📤 {t("workflow.subWorkflowNode.outputCount", { count: outputCount })}
              </Tag>
            )}
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

export const SubWorkflowNode = memo(SubWorkflowNodeComponent);
