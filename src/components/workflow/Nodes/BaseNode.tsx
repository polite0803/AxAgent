// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, getNodeSize, PORT_SIZE } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo, useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

export interface BaseNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  validationState?: "error" | "warning";
  validationMessage?: string;
  executionState?: "running" | "completed" | "failed" | "timeout" | "skipped" | "paused";
  parentId?: string;
  config?: { tick_mode?: boolean };
  retry?: { enabled?: boolean };
}

/** 端口折叠阈值：当节点有 N 条以上输入或输出边时，默认折叠端口 */
const PORT_COLLAPSE_THRESHOLD = 4;

const BaseNodeComponent: React.FC<NodeProps> = ({
  data,
  selected,
}) => {
  const bd = data as unknown as BaseNodeData;
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const nodeStatuses = useWorkEngineStore((s) => s.nodeStatuses);
  const breakpoints = useWorkEngineStore((s) => s.breakpoints);
  const runtimeStatus = nodeStatuses[bd.id];
  const hasBreakpoint = breakpoints.includes(bd.id);

  const effectiveExecState = runtimeStatus || bd.executionState;

  // ── 端口计数 ──
  const inboundCount = useWorkflowEditorStore(
    useCallback((s) => s.edges.filter((e) => e.target === data.id).length, [data.id]),
  );
  const outboundCount = useWorkflowEditorStore(
    useCallback((s) => s.edges.filter((e) => e.source === data.id).length, [data.id]),
  );

  // ── 端口折叠状态 ──
  const shouldCollapseByDefault = inboundCount >= PORT_COLLAPSE_THRESHOLD
    || outboundCount >= PORT_COLLAPSE_THRESHOLD;
  const [userCollapsed, setUserCollapsed] = useState(shouldCollapseByDefault);
  const [isHovering, setIsHovering] = useState(false);

  // 当边数低于阈值时自动展开，达到阈值时使用用户折叠偏好
  const isPortCollapsed = shouldCollapseByDefault ? userCollapsed : false;

  const togglePorts = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      setUserCollapsed((prev) => !prev);
    },
    [],
  );

  const getBorderColor = () => {
    if (bd.validationState === "error") { return token.colorError; }
    if (bd.validationState === "warning") { return token.colorWarning; }
    if (effectiveExecState === "running") { return token.colorPrimary; }
    if (effectiveExecState === "completed") { return token.colorSuccess; }
    if (effectiveExecState === "failed" || effectiveExecState === "timeout") { return token.colorError; }
    if (effectiveExecState === "paused") { return token.colorWarning; }
    if (hasBreakpoint) { return "#ff4d4f"; }
    if (selected) { return token.colorPrimary; }
    return bd.color;
  };

  const borderColor = getBorderColor();
  const isRunning = effectiveExecState === "running";
  const isSkipped = effectiveExecState === "skipped";

  // 端口折叠时节点宽度缩减
  const isWide = shouldCollapseByDefault && isPortCollapsed && !isHovering;

  // 节点尺寸（用于计算 Handle 位置）
  const nodeSize = getNodeSize(bd.type);
  const nodeWidth = isWide ? 120 : nodeSize.width;
  const nodeHeight = nodeSize.height;

  // 状态指示色块
  const statusDot = effectiveExecState === "running"
    ? token.colorPrimary
    : effectiveExecState === "completed"
    ? token.colorSuccess
    : effectiveExecState === "failed" || effectiveExecState === "timeout"
    ? token.colorError
    : effectiveExecState === "paused"
    ? token.colorWarning
    : null;

  return (
    <div
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
      style={{
        width: nodeWidth,
        height: nodeHeight,
        overflow: "hidden",
        opacity: bd.enabled ? (isSkipped ? 0.4 : 1) : 0.5,
        filter: bd.enabled ? (isSkipped ? "grayscale(80%)" : "none") : "grayscale(100%)",
        transition: "width 0.15s, opacity 0.15s",
      }}
    >
      <div
        className="workflow-node-card"
        title={bd.description || bd.title}
        style={{
          background: token.colorBgContainer,
          border: `1.5px solid ${borderColor}`,
          borderRadius: 8,
          padding: 0,
          boxShadow: selected
            ? `0 0 0 1.5px ${borderColor}40`
            : "0 1px 3px rgba(0,0,0,0.08)",
          transition: "box-shadow 0.15s",
          animation: isRunning ? "nodePulse 1.5s ease-in-out infinite" : "none",
          position: "relative",
          ...(data.parentId ? { borderLeftWidth: 3, borderLeftColor: token.colorTextQuaternary } : {}),
        }}
      >
        {hasBreakpoint && (
          <div
            style={{
              position: "absolute",
              top: -4,
              right: -4,
              width: 10,
              height: 10,
              borderRadius: "50%",
              background: "#ff4d4f",
              border: "1.5px solid white",
              zIndex: 10,
            }}
          />
        )}

        {bd.validationState && bd.validationMessage && (
          <div
            title={bd.validationMessage}
            style={{
              position: "absolute",
              top: -4,
              left: -4,
              width: 10,
              height: 10,
              borderRadius: "50%",
              background: bd.validationState === "error" ? token.colorError : token.colorWarning,
              border: "1.5px solid white",
              zIndex: 10,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 7,
              fontWeight: 700,
              color: "#fff",
              cursor: "pointer",
            }}
          >
            !
          </div>
        )}

        {/* n8n 风格：单行 — 图标色块 + 标题 + 状态 */}
        <div
          style={{
            padding: "6px 10px",
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          {/* 图标色块 */}
          <div
            style={{
              width: 22,
              height: 22,
              borderRadius: 4,
              background: `${bd.color}18`,
              border: `1px solid ${bd.color}30`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 12,
              flexShrink: 0,
              lineHeight: 1,
            }}
          >
            {getNodeIcon(bd.nodeType)}
          </div>

          {/* 标题 */}
          <span
            style={{
              fontSize: 11,
              color: token.colorText,
              fontWeight: 500,
              flex: 1,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              lineHeight: "22px",
            }}
          >
            {bd.title}
          </span>

          {/* 状态指示 */}
          {statusDot && (
            <div
              style={{
                width: 6,
                height: 6,
                borderRadius: "50%",
                background: statusDot,
                flexShrink: 0,
                animation: effectiveExecState === "running" ? "nodePulse 1.5s ease-in-out infinite" : "none",
              }}
            />
          )}
          {bd.config?.tick_mode && (
            <span title={t("workflow.node.tickMode")} style={{ fontSize: 8, flexShrink: 0 }}>🔄</span>
          )}
          {bd.retry?.enabled && (
            <span title={t("workflow.node.retryEnabled")} style={{ fontSize: 8, flexShrink: 0 }}>🔄</span>
          )}
        </div>

        {/* Hover 工具提示 */}
        {isHovering && !data.parentId && (
          <div
            style={{
              padding: "3px 8px",
              fontSize: 9,
              color: token.colorTextTertiary,
              background: token.colorBgElevated,
              border: `1px solid ${token.colorBorderSecondary}`,
              borderRadius: 4,
              lineHeight: "14px",
              whiteSpace: "nowrap",
            }}
          >
            {t("workflow.node.inputs", { defaultValue: "In" })}: {inboundCount} |{" "}
            {t("workflow.node.outputs", { defaultValue: "Out" })}: {outboundCount}
          </div>
        )}
      </div>

      {/* ── 端口渲染：折叠或展开 ── */}
      {shouldCollapseByDefault && isPortCollapsed && !isHovering
        ? (
          // 折叠态：显示计数标签
          <>
            <div
              onClick={togglePorts}
              title={t("workflow.node.clickToExpandPorts", { defaultValue: "Click to expand ports" })}
              style={{
                position: "absolute",
                top: -16,
                left: "50%",
                transform: "translateX(-50%)",
                fontSize: 8,
                lineHeight: "12px",
                padding: "0 5px",
                borderRadius: 3,
                background: `${bd.color}15`,
                border: `1px solid ${bd.color}40`,
                color: bd.color,
                whiteSpace: "nowrap",
                cursor: "pointer",
                zIndex: 5,
                userSelect: "none",
              }}
            >
              {inboundCount}
            </div>
            <div
              onClick={togglePorts}
              title={t("workflow.node.clickToExpandPorts", { defaultValue: "Click to expand ports" })}
              style={{
                position: "absolute",
                bottom: -16,
                left: "50%",
                transform: "translateX(-50%)",
                fontSize: 8,
                lineHeight: "12px",
                padding: "0 5px",
                borderRadius: 3,
                background: `${bd.color}15`,
                border: `1px solid ${bd.color}40`,
                color: bd.color,
                whiteSpace: "nowrap",
                cursor: "pointer",
                zIndex: 5,
                userSelect: "none",
              }}
            >
              {outboundCount}
            </div>
          </>
        )
        : (
          // 展开态：标准 Handle（使用精确位置计算）
          <>
            <Handle
              type="target"
              position={Position.Top}
              style={{
                background: bd.color,
                border: "none",
                width: PORT_SIZE,
                height: PORT_SIZE,
                ...getHandlePosition(nodeWidth, nodeHeight, "top"),
              }}
            />
            <Handle
              type="source"
              position={Position.Bottom}
              style={{
                background: bd.color,
                border: "none",
                width: PORT_SIZE,
                height: PORT_SIZE,
                ...getHandlePosition(nodeWidth, nodeHeight, "bottom"),
              }}
            />

            {/* 容器内子节点：动态端口出口 */}
            {data.parentId && (
              <>
                {Array.from({ length: 6 }).map((_, i) => {
                  const leftPct = `${((i + 1) / 7) * 100}%`;
                  return (
                    <Handle
                      key={`port-${i}`}
                      type="source"
                      position={Position.Bottom}
                      id={`port-${i}`}
                      style={{
                        background: bd.color,
                        border: "1px solid transparent",
                        width: 5,
                        height: 5,
                        left: leftPct,
                        opacity: 0.3,
                      }}
                    />
                  );
                })}
              </>
            )}

            {shouldCollapseByDefault && (
              <div
                onClick={togglePorts}
                title={t("workflow.node.clickToCollapsePorts", { defaultValue: "Click to collapse ports" })}
                style={{
                  position: "absolute",
                  bottom: -6,
                  right: -6,
                  fontSize: 7,
                  lineHeight: "10px",
                  padding: "0 3px",
                  borderRadius: 2,
                  background: token.colorBgElevated,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  color: token.colorTextTertiary,
                  cursor: "pointer",
                  zIndex: 5,
                  userSelect: "none",
                  opacity: isHovering ? 1 : 0.4,
                  transition: "opacity 0.15s",
                }}
              >
                ⊟
              </div>
            )}
          </>
        )}
    </div>
  );
};

function getNodeIcon(type: string): string {
  const icons: Record<string, string> = {
    trigger: "⚡",
    agent: "🤖",
    llm: "🧠",
    llmClassifier: "🏷️",
    condition: "🔀",
    switch: "🔀",
    parallel: "⚡",
    loop: "🔄",
    debate: "💬",
    swarm: "🐝",
    merge: "🔗",
    aggregator: "∑",
    delay: "⏱",
    tool: "🔧",
    code: "💻",
    subWorkflow: "📦",
    workflowRef: "🔗",
    documentParser: "📄",
    vectorRetrieve: "🔍",
    storage: "💾",
    databaseQuery: "🗄️",
    httpRequest: "🌐",
    validation: "✅",
    notification: "🔔",
    approval: "✔️",
    fileOperation: "📁",
    dataTransformer: "🔄",
    webhookSend: "📨",
    logging: "📝",
    email: "📧",
    end: "🏁",
    _phaseSeparator: "➖",
    groupFrame: "▭",
  };
  return icons[type] || "📦";
}

export const BaseNode = memo(BaseNodeComponent);
