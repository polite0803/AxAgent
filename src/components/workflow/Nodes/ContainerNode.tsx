// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, PORT_SIZE } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { ClockCircleOutlined } from "@ant-design/icons";
import { Handle, Position } from "@xyflow/react";
import { Tag, theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { nodeIconFor } from "./nodeIcons";

export interface ContainerNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  childCount?: number;
  kind?: "decorative" | "executable";
  hasBranchTimeout?: boolean;
  nodeWidth?: number;
  nodeHeight?: number;
}

interface ContainerNodeProps {
  data: ContainerNodeData;
  selected: boolean;
  icon?: React.ReactNode;
  extraTags?: React.ReactNode;
  collapsedExtra?: React.ReactNode;
  disableHandles?: boolean;
  childLabel?: string;
}

const ContainerNodeComponent: React.FC<ContainerNodeProps> = ({
  data,
  selected,
  icon,
  extraTags,
  collapsedExtra,
  disableHandles,
  childLabel,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const resolvedIcon = icon ?? nodeIconFor(data.nodeType);

  const isCollapsed = useWorkflowEditorStore((s) => s.collapsedContainers[data.id] === true);
  const toggleCollapse = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      useWorkflowEditorStore.getState().toggleContainerCollapse(data.id);
    },
    [data.id],
  );

  const childCount = data.childCount ?? 0;

  // n8n 风格容器视觉
  const borderColor = selected ? token.colorPrimary : `${data.color}80`;
  const backgroundColor = isCollapsed
    ? `${token.colorBgElevated}`
    : `${data.color}08`; // 非常浅的背景色，体现容器包裹感

  return (
    <div
      style={{
        width: isCollapsed ? 160 : (data.nodeWidth ?? 240),
        height: isCollapsed ? 32 : (data.nodeHeight ?? 120),
        background: backgroundColor,
        border: `2px dashed ${borderColor}`,
        borderRadius: 8,
        opacity: data.enabled ? (data.kind === "decorative" ? 0.45 : 1) : 0.5,
        position: "relative",
        transition: "opacity 0.15s, border-color 0.15s, width 0.2s, height 0.2s",
        boxShadow: selected
          ? `0 0 0 1px ${data.color}40, 0 2px 6px rgba(0,0,0,0.06)`
          : "none",
        // n8n 风格：容器有内边距包裹子节点
        padding: isCollapsed ? 0 : "28px 12px 12px 12px",
        overflow: "visible",
      }}
    >
      {/* 标题栏 — 容器头部 (n8n 风格: 紧凑条带) */}
      <div
        className="workflow-container-drag-handle"
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          background: token.colorBgElevated,
          border: `1px solid ${borderColor}`,
          borderRadius: 6,
          padding: isCollapsed ? "4px 8px" : "4px 10px",
          zIndex: 10,
          cursor: "grab",
          boxShadow: "0 1px 2px rgba(0,0,0,0.04)",
          // 展开态标题栏定位在容器顶部内侧
          ...(isCollapsed ? {} : {
            position: "absolute",
            top: 10,
            left: 10,
            right: 10,
          }),
        }}
      >
        {/* 左侧强调色 (n8n 风格) */}
        <div
          style={{
            width: 3,
            height: 14,
            background: data.color,
            borderRadius: 1,
            flexShrink: 0,
          }}
        />

        {/* 图标块 */}
        <div
          style={{
            width: 18,
            height: 18,
            borderRadius: 3,
            background: `${data.color}20`,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 10,
            flexShrink: 0,
            lineHeight: 1,
          }}
        >
          {resolvedIcon}
        </div>

        <span
          style={{
            fontSize: 11,
            color: token.colorText,
            fontWeight: 600,
            lineHeight: "16px",
          }}
        >
          {data.title}
        </span>

        {/* 额外标签 */}
        {!isCollapsed && extraTags}

        {/* 折叠态：显示 ⊕ N */}
        {isCollapsed && childCount > 0 && (
          <Tag
            style={{
              margin: 0,
              fontSize: 10,
              padding: "0 5px",
              lineHeight: "16px",
              background: `${data.color}15`,
              border: `1px solid ${data.color}40`,
              color: data.color,
              fontWeight: 600,
            }}
          >
            ⊕{childCount}
          </Tag>
        )}

        {/* 折叠态额外内容 */}
        {isCollapsed && collapsedExtra}
      </div>

      {/* 超时/降级标记 */}
      {!isCollapsed && data.hasBranchTimeout && (
        <Tooltip
          title={t("workflow.containerNode.branchTimeout", {
            defaultValue: "Branch timeout configured",
          })}
        >
          <span
            style={{
              position: "absolute",
              top: 8,
              right: 40,
              fontSize: 10,
              lineHeight: "14px",
              padding: "1px 5px",
              borderRadius: 3,
              background: `${token.colorWarning}15`,
              border: `1px solid ${token.colorWarning}40`,
              color: token.colorWarning,
              fontWeight: 600,
              zIndex: 10,
              userSelect: "none",
            }}
          >
            <ClockCircleOutlined style={{ fontSize: 10 }} />
          </span>
        </Tooltip>
      )}

      {/* 展开/折叠按钮 */}
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
            top: 7,
            right: 8,
            cursor: "pointer",
            fontSize: 10,
            lineHeight: 1,
            padding: "3px 5px",
            borderRadius: 3,
            background: token.colorBgElevated,
            border: `1px solid ${token.colorBorder}`,
            zIndex: 10,
            opacity: 0.7,
            transition: "opacity 0.15s",
            userSelect: "none",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "1";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "0.7";
          }}
        >
          {isCollapsed ? "▼" : "▲"}
        </span>
      </Tooltip>

      {/* 展开态左下角：内部节点计数 */}
      {!isCollapsed && childCount > 0 && (
        <div
          style={{
            position: "absolute",
            bottom: 6,
            left: 10,
            display: "flex",
            alignItems: "center",
            gap: 3,
            fontSize: 10,
            color: data.color,
            fontWeight: 500,
            opacity: 0.7,
            zIndex: 5,
            userSelect: "none",
          }}
        >
          <span>⊕</span>
          <span>
            {childCount} {childLabel || t("workflow.containerNode.nodes", {
              defaultValue: "nodes",
            })}
          </span>
        </div>
      )}

      {/* Handle（使用精确位置计算） */}
      {!disableHandles && (
        <>
          <Handle
            type="target"
            position={Position.Top}
            style={{
              background: token.colorBgElevated,
              border: `2px solid ${data.color}`,
              width: PORT_SIZE,
              height: PORT_SIZE,
              pointerEvents: "all",
              ...getHandlePosition(
                isCollapsed ? 160 : (data.nodeWidth ?? 240),
                isCollapsed ? 32 : (data.nodeHeight ?? 120),
                "top",
              ),
            }}
          />
          <Handle
            type="source"
            position={Position.Bottom}
            style={{
              background: token.colorBgElevated,
              border: `2px solid ${data.color}`,
              width: PORT_SIZE,
              height: PORT_SIZE,
              pointerEvents: "all",
              ...getHandlePosition(
                isCollapsed ? 160 : (data.nodeWidth ?? 240),
                isCollapsed ? 32 : (data.nodeHeight ?? 120),
                "bottom",
              ),
            }}
          />
        </>
      )}
    </div>
  );
};

export const ContainerNode = memo(ContainerNodeComponent);
