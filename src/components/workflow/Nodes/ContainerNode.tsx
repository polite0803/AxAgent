// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, PORT_SIZE } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { Handle, Position } from "@xyflow/react";
import { Tag, theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";

/**
 * 容器节点的共享 data 接口。
 * 各容器节点在 data 中应包含这些字段，ContainerNode 根据它们渲染。
 */
export interface ContainerNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  /** 容器子图节点数量（折叠态显示） */
  childCount?: number;
  /** 装饰容器标记（用于 parallel kind="decorative"） */
  kind?: "decorative" | "executable";
  /** 是否存在分支级别的超时/降级配置（显示 timeout 标记） */
  hasBranchTimeout?: boolean;
  /** 外部传入的宽度（WorkflowEditor 根据子节点 bbox 计算） */
  nodeWidth?: number;
  /** 外部传入的高度 */
  nodeHeight?: number;
}

interface ContainerNodeProps {
  /** ReactFlow NodeProps 的 data */
  data: ContainerNodeData;
  selected: boolean;
  /** 容器类型对应的 emoji/icon */
  icon?: string;
  /** 额外子元素（渲染在标题栏右侧的标签区域） */
  extraTags?: React.ReactNode;
  /** 折叠态下额外显示的内容（在计数下方） */
  collapsedExtra?: React.ReactNode;
  /** 是否禁用 Handle（装饰容器等） */
  disableHandles?: boolean;
  /** 内部子节点类型标签（如 "Agents"、"Debaters"），用于折叠态计数显示：⊕ N <label> */
  childLabel?: string;
}

/**
 * 通用复合节点渲染组件（n8n 紧凑风格）。
 *
 * 为 Parallel / Debate / Loop / SubWorkflow / Swarm 等内部含多个子节点的节点
 * 提供统一的视觉标识：
 * - 虚线边框 + 轻微背景色
 * - 紧凑标题栏（图标 + 标题 + 折叠按钮一行）
 * - 折叠态仅显示标题 + 内部节点数量
 */
const ContainerNodeComponent: React.FC<ContainerNodeProps> = ({
  data,
  selected,
  icon = "📦",
  extraTags,
  collapsedExtra,
  disableHandles,
  childLabel,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const isCollapsed = useWorkflowEditorStore((s) => s.collapsedContainers.has(data.id));
  const toggleCollapse = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      useWorkflowEditorStore.getState().toggleContainerCollapse(data.id);
    },
    [data.id],
  );

  const childCount = data.childCount ?? 0;

  return (
    <div
      style={{
        width: isCollapsed ? 160 : (data.nodeWidth ?? undefined),
        height: isCollapsed ? 34 : (data.nodeHeight ?? undefined),
        minWidth: isCollapsed ? 160 : 200,
        minHeight: isCollapsed ? 34 : 80,
        background: `${data.color}06`,
        border: `1.5px dashed ${selected ? token.colorPrimary : `${data.color}50`}`,
        borderRadius: 8,
        padding: isCollapsed ? "6px 8px" : 8,
        opacity: data.enabled ? (data.kind === "decorative" ? 0.55 : 1) : 0.5,
        position: "relative",
        transition: "opacity 0.15s, border-color 0.15s",
      }}
    >
      {/* 标题栏 — 紧凑单行 */}
      <div
        className="workflow-container-drag-handle"
        style={{
          display: "flex",
          alignItems: "center",
          gap: 5,
          background: isCollapsed || data.kind === "decorative"
            ? "transparent"
            : token.colorBgElevated,
          border: isCollapsed || data.kind === "decorative"
            ? "none"
            : `1px solid ${data.color}20`,
          borderRadius: 4,
          padding: "3px 6px",
          zIndex: 10,
          cursor: "grab",
          ...(isCollapsed ? {} : {
            position: "absolute",
            top: 6,
            left: 8,
          }),
        }}
      >
        {/* 图标色块 */}
        <div
          style={{
            width: 18,
            height: 18,
            borderRadius: 3,
            background: `${data.color}18`,
            border: `1px solid ${data.color}30`,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 10,
            flexShrink: 0,
            lineHeight: 1,
          }}
        >
          {icon}
        </div>

        <span style={{ fontSize: 11, color: data.color, fontWeight: 600, lineHeight: "18px" }}>
          {data.title}
        </span>

        {/* 额外标签 */}
        {!isCollapsed && extraTags}

        {/* 折叠态：显示 ⊕ N */}
        {isCollapsed && childCount > 0 && (
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              lineHeight: "16px",
              background: `${data.color}12`,
              border: `1px solid ${data.color}30`,
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
              right: 30,
              fontSize: 8,
              lineHeight: "12px",
              padding: "1px 4px",
              borderRadius: 2,
              background: `${token.colorWarning}15`,
              border: `1px solid ${token.colorWarning}40`,
              color: token.colorWarning,
              fontWeight: 600,
              zIndex: 10,
              userSelect: "none",
            }}
          >
            ⏱
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
            top: 6,
            right: 8,
            cursor: "pointer",
            fontSize: 10,
            lineHeight: 1,
            padding: "3px 5px",
            borderRadius: 3,
            background: isCollapsed ? "transparent" : token.colorBgElevated,
            border: isCollapsed ? "none" : `1px solid ${data.color}20`,
            zIndex: 10,
            opacity: 0.6,
            transition: "opacity 0.15s",
            userSelect: "none",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "1";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "0.6";
          }}
        >
          {isCollapsed ? "▶" : "▼"}
        </span>
      </Tooltip>

      {/* 展开态左下角：内部节点计数 */}
      {!isCollapsed && childCount > 0 && (
        <div
          style={{
            position: "absolute",
            bottom: 4,
            left: 8,
            display: "flex",
            alignItems: "center",
            gap: 3,
            fontSize: 9,
            color: data.color,
            fontWeight: 600,
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
              background: `${data.color}80`,
              border: `1.5px solid ${data.color}`,
              width: PORT_SIZE,
              height: PORT_SIZE,
              pointerEvents: "all",
              ...getHandlePosition(data.nodeWidth ?? 400, data.nodeHeight ?? 200, "top"),
            }}
          />
          <Handle
            type="source"
            position={Position.Bottom}
            style={{
              background: `${data.color}80`,
              border: `1.5px solid ${data.color}`,
              width: PORT_SIZE,
              height: PORT_SIZE,
              pointerEvents: "all",
              ...getHandlePosition(data.nodeWidth ?? 400, data.nodeHeight ?? 200, "bottom"),
            }}
          />
        </>
      )}
    </div>
  );
};

export const ContainerNode = memo(ContainerNodeComponent);
