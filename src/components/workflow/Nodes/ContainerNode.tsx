// SPDX-License-Identifier: AGPL-3.0-only

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
 * 通用复合节点渲染组件。
 *
 * 为 Parallel / Debate / Loop / SubWorkflow / Swarm 等内部含多个子节点的节点
 * 提供统一的视觉标识：
 * - 双线边框（虚线 + 内阴影）
 * - 左下角内部节点计数（⊕ N <label>）
 * - ▶/▼ 展开折叠按钮
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
        width: isCollapsed ? 220 : (data.nodeWidth ?? undefined),
        height: isCollapsed ? 48 : (data.nodeHeight ?? undefined),
        minWidth: isCollapsed ? 220 : 0,
        minHeight: isCollapsed ? 48 : 0,
        background: `${data.color}08`,
        border: `2px dashed ${selected ? token.colorPrimary : `${data.color}50`}`,
        // 双线效果：虚线边框 + 内阴影模拟第二条线
        boxShadow: selected
          ? `0 0 0 2px ${data.color}40, inset 0 0 0 2px ${data.color}15`
          : `inset 0 0 0 2px ${data.color}15`,
        borderRadius: 12,
        padding: isCollapsed ? "8px 12px" : 12,
        opacity: data.enabled ? (data.kind === "decorative" ? 0.65 : 1) : 0.5,
        position: "relative",
        transition: "width 0.25s, height 0.25s, opacity 0.2s, border-color 0.2s",
      }}
    >
      {/* 标题栏 — 左上角 */}
      <div
        className="workflow-container-drag-handle"
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          background: isCollapsed
            ? "transparent"
            : data.kind === "decorative"
            ? "transparent"
            : token.colorBgElevated,
          border: isCollapsed || data.kind === "decorative"
            ? "none"
            : `1px solid ${data.color}30`,
          borderRadius: 6,
          padding: "4px 8px",
          zIndex: 10,
          cursor: "grab",
          // 展开态固定左上角
          ...(isCollapsed ? {} : {
            position: "absolute",
            top: 8,
            left: 12,
          }),
        }}
      >
        <span style={{ fontSize: 14 }}>{icon}</span>
        <span style={{ fontSize: 12, color: data.color, fontWeight: 600 }}>
          {data.title}
        </span>

        {/* 额外标签（各容器节点自定义） */}
        {!isCollapsed && extraTags}

        {/* 折叠态：显示 ⊕ N <label> */}
        {isCollapsed && childCount > 0 && (
          <Tag
            style={{
              margin: 0,
              fontSize: 10,
              padding: "1px 6px",
              background: `${data.color}15`,
              border: `1px solid ${data.color}40`,
              color: data.color,
              fontWeight: 600,
            }}
          >
            ⊕ {childCount} {childLabel || t("workflow.containerNode.nodes", {
              defaultValue: "nodes",
            })}
          </Tag>
        )}

        {/* 折叠态额外内容 */}
        {isCollapsed && collapsedExtra}
      </div>

      {/* 超时/降级标记 — 右上角展开态显示 */}
      {!isCollapsed && data.hasBranchTimeout && (
        <Tooltip
          title={t("workflow.containerNode.branchTimeout", {
            defaultValue: "Branch timeout configured",
          })}
        >
          <span
            style={{
              position: "absolute",
              top: 10,
              right: 38,
              fontSize: 9,
              lineHeight: "14px",
              padding: "1px 5px",
              borderRadius: 3,
              background: `${token.colorWarning}20`,
              border: `1px solid ${token.colorWarning}50`,
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

      {/* 展开/折叠按钮 — ▶ / ▼ */}
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
            fontSize: isCollapsed ? 12 : 14,
            lineHeight: 1,
            padding: "4px 8px",
            borderRadius: 4,
            background: isCollapsed ? "transparent" : token.colorBgElevated,
            border: isCollapsed ? "none" : `1px solid ${data.color}30`,
            zIndex: 10,
            opacity: 0.7,
            transition: "opacity 0.2s",
            userSelect: "none",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "1";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.opacity = "0.7";
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
            bottom: 8,
            left: 12,
            display: "flex",
            alignItems: "center",
            gap: 4,
            fontSize: 10,
            color: data.color,
            fontWeight: 600,
            opacity: 0.8,
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

      {/* 端口透传 Handle：仅非装饰容器时渲染 */}
      {!disableHandles && (
        <>
          <Handle
            type="target"
            position={Position.Top}
            style={{
              background: "transparent",
              border: "none",
              width: 8,
              height: 8,
              top: 0,
              pointerEvents: "all",
            }}
          />
          <Handle
            type="source"
            position={Position.Bottom}
            style={{
              background: "transparent",
              border: "none",
              width: 8,
              height: 8,
              bottom: 0,
              pointerEvents: "all",
            }}
          />
        </>
      )}
    </div>
  );
};

export const ContainerNode = memo(ContainerNodeComponent);
