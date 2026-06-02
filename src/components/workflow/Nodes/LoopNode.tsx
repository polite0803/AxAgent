import { useWorkflowEditorStore } from "@/stores";
import { Tag, theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Handle, Position } from "reactflow";
import type { NodeProps } from "reactflow";

const ORANGE_BASE = "#fa8c16";
const ORANGE_VAR = `var(--orange, ${ORANGE_BASE})`;

interface LoopNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  loopType?: "count" | "condition" | "forEach";
  maxIterations?: number;
  loopCondition?: string;
  collectionVar?: string;
}

const LoopNodeComponent: React.FC<NodeProps<LoopNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = ORANGE_VAR;
  const loopType = data.loopType || "count";

  const isCollapsed = useWorkflowEditorStore((s) => s.collapsedContainers.has(data.id));
  const toggleCollapse = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      useWorkflowEditorStore.getState().toggleContainerCollapse(data.id);
    },
    [data.id],
  );

  const getLoopDescription = (): string => {
    switch (loopType) {
      case "count":
        return data.maxIterations
          ? `${data.maxIterations}x`
          : t("workflow.loopNode.notConfigured");
      case "condition":
        return data.loopCondition || t("workflow.loopNode.notConfigured");
      case "forEach":
        return data.collectionVar
          ? `∈ ${data.collectionVar}`
          : t("workflow.loopNode.notConfigured");
      default:
        return t("workflow.loopNode.notConfigured");
    }
  };

  // 容器节点：不需要 Handle，ReactFlow 子节点通过 parentId 自动渲染在此区域内
  return (
    <div
      style={{
        minWidth: 400,
        minHeight: 200,
        background: `${ORANGE_BASE}08`,
        border: `2px dashed ${selected ? token.colorPrimary : ORANGE_BASE}40`,
        borderRadius: 12,
        padding: 12,
        opacity: data.enabled ? 1 : 0.5,
        position: "relative",
        boxShadow: selected ? `0 0 0 2px ${ORANGE_VAR}40` : "none",
      }}
    >
      {/* 标题栏 — 左上角 */}
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
          border: `1px solid ${ORANGE_BASE}30`,
          borderRadius: 6,
          padding: "4px 10px",
          zIndex: 10,
          cursor: "grab",
        }}
      >
        <span style={{ fontSize: 14 }}>🔁</span>
        <span style={{ fontSize: 12, color, fontWeight: 600 }}>
          {data.title}
        </span>
        <div style={{ display: "flex", gap: 4 }}>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${ORANGE_BASE}20`,
              border: `1px solid ${ORANGE_BASE}50`,
              color: ORANGE_VAR,
            }}
          >
            {loopType.toUpperCase()}
          </Tag>
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
            {getLoopDescription()}
          </Tag>
        </div>
      </div>

      {/* 折叠/展开按钮 — 右上角 */}
      <Tooltip
        title={isCollapsed
          ? t("workflow.parallelNode.expand")
          : t("workflow.parallelNode.collapse")}
      >
        <span
          className="react-flow__nodrag"
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
            border: `1px solid ${ORANGE_BASE}30`,
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

      {/* 折叠态 Handle：始终挂载但视觉不可见，供边重定向到容器自身 */}
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: "transparent", border: "none", width: 1, height: 1, top: 0 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: "transparent", border: "none", width: 1, height: 1, bottom: 0 }}
      />
    </div>
  );
};

export const LoopNode = memo(LoopNodeComponent);
