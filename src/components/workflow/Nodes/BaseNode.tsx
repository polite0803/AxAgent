// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkflowEditorStore } from "@/stores";
import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo, useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { NODE_TYPE_MAP } from "../types";

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
  const typeInfo = NODE_TYPE_MAP[bd.nodeType] || {
    labelKey: "",
    color: token.colorTextTertiary,
  };

  const nodeStatuses = useWorkEngineStore((s) => s.nodeStatuses);
  const breakpoints = useWorkEngineStore((s) => s.breakpoints);
  const runtimeStatus = nodeStatuses[bd.id];
  const hasBreakpoint = breakpoints.includes(bd.id);

  const effectiveExecState = runtimeStatus || bd.executionState;

  // ── 端口计数 ──
  const edges = useWorkflowEditorStore((s) => s.edges);
  const inboundCount = useMemo(
    () => edges.filter((e) => e.target === data.id).length,
    [edges, data.id],
  );
  const outboundCount = useMemo(
    () => edges.filter((e) => e.source === data.id).length,
    [edges, data.id],
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

  return (
    <div
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
      style={{
        minWidth: isWide ? 120 : 160,
        maxWidth: isWide ? 150 : 200,
        opacity: bd.enabled ? (isSkipped ? 0.4 : 1) : 0.5,
        filter: bd.enabled ? (isSkipped ? "grayscale(80%)" : "none") : "grayscale(100%)",
        transition: "min-width 0.2s, max-width 0.2s, opacity 0.2s",
      }}
    >
      <div
        className="workflow-node-card"
        style={{
          background: token.colorBgContainer,
          border: `2px solid ${borderColor}`,
          borderRadius: 8,
          padding: 0,
          boxShadow: selected ? `0 0 0 2px ${borderColor}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
          animation: isRunning ? "nodePulse 1.5s ease-in-out infinite" : "none",
          position: "relative",
          ...(data.parentId ? { borderLeftWidth: 4, borderLeftColor: token.colorTextQuaternary } : {}),
        }}
        onMouseEnter={() => {
          setIsHovering(true);
        }}
        onMouseLeave={() => {
          setIsHovering(false);
        }}
      >
        {hasBreakpoint && (
          <div
            style={{
              position: "absolute",
              top: -6,
              right: -6,
              width: 14,
              height: 14,
              borderRadius: "50%",
              background: "#ff4d4f",
              border: "2px solid white",
              zIndex: 10,
            }}
          />
        )}

        {bd.validationState && bd.validationMessage && (
          <div
            title={bd.validationMessage}
            style={{
              position: "absolute",
              top: -6,
              left: -6,
              width: 14,
              height: 14,
              borderRadius: "50%",
              background: bd.validationState === "error" ? token.colorError : token.colorWarning,
              border: "2px solid white",
              zIndex: 10,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 10,
              fontWeight: 700,
              color: "#fff",
              cursor: "pointer",
            }}
          >
            !
          </div>
        )}

        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${bd.color}40`,
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span style={{ fontSize: 16 }}>{getNodeIcon(bd.nodeType)}</span>
          <span
            style={{
              fontSize: 12,
              color: bd.color,
              fontWeight: 500,
              flex: 1,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {typeInfo.labelKey ? t(typeInfo.labelKey) : bd.nodeType}
          </span>
          <div style={{ display: "flex", gap: 2, alignItems: "center" }}>
            {bd.config?.tick_mode && <span title={t("workflow.node.tickMode")} style={{ fontSize: 10 }}>🔄</span>}
            {bd.retry?.enabled && <span title={t("workflow.node.retryEnabled")} style={{ fontSize: 10 }}>🔄</span>}
            {effectiveExecState === "running" && <span style={{ fontSize: 10, color: token.colorPrimary }}>⏳</span>}
            {effectiveExecState === "completed" && <span style={{ fontSize: 10, color: token.colorSuccess }}>✓</span>}
            {(effectiveExecState === "failed" || effectiveExecState === "timeout") && (
              <span style={{ fontSize: 10, color: token.colorError }}>✗</span>
            )}
            {effectiveExecState === "paused" && <span style={{ fontSize: 10, color: token.colorWarning }}>⏸</span>}
          </div>
        </div>

        <div style={{ padding: "8px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: token.colorText,
              fontWeight: 500,
              marginBottom: bd.description ? 4 : 0,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {bd.title}
          </div>
          {bd.description && (
            <div
              style={{
                fontSize: 12,
                color: token.colorTextTertiary,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {bd.description}
            </div>
          )}

          {/* Hover 工具提示额外信息 */}
          {isHovering && !data.parentId && (
            <div
              style={{
                marginTop: 6,
                padding: "4px 6px",
                fontSize: 10,
                color: token.colorTextTertiary,
                background: token.colorBgLayout,
                borderRadius: 4,
                lineHeight: "16px",
              }}
            >
              <div>
                {t("workflow.node.inputs", { defaultValue: "Inputs" })}: {inboundCount} |{" "}
                {t("workflow.node.outputs", { defaultValue: "Outputs" })}: {outboundCount}
              </div>
              {bd.nodeType && (
                <div>
                  {t("workflow.node.type", { defaultValue: "Type" })}: {bd.nodeType}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* ── 端口渲染：折叠或展开 ── */}
      {shouldCollapseByDefault && isPortCollapsed && !isHovering
        ? (
          // 折叠态：显示计数标签（点击可展开）
          <>
            {/* 输入端口计数 */}
            <div
              onClick={togglePorts}
              title={t("workflow.node.clickToExpandPorts", {
                defaultValue: "Click to expand ports",
              })}
              style={{
                position: "absolute",
                top: -20,
                left: "50%",
                transform: "translateX(-50%)",
                fontSize: 9,
                lineHeight: "14px",
                padding: "0 6px",
                borderRadius: 3,
                background: `${bd.color}20`,
                border: `1px solid ${bd.color}50`,
                color: bd.color,
                whiteSpace: "nowrap",
                cursor: "pointer",
                zIndex: 5,
                userSelect: "none",
              }}
            >
              {inboundCount} {t("workflow.node.inputs", { defaultValue: "inputs" })}
            </div>
            {/* 输出端口计数 */}
            <div
              onClick={togglePorts}
              title={t("workflow.node.clickToExpandPorts", {
                defaultValue: "Click to expand ports",
              })}
              style={{
                position: "absolute",
                bottom: -20,
                left: "50%",
                transform: "translateX(-50%)",
                fontSize: 9,
                lineHeight: "14px",
                padding: "0 6px",
                borderRadius: 3,
                background: `${bd.color}20`,
                border: `1px solid ${bd.color}50`,
                color: bd.color,
                whiteSpace: "nowrap",
                cursor: "pointer",
                zIndex: 5,
                userSelect: "none",
              }}
            >
              {outboundCount} {t("workflow.node.outputs", { defaultValue: "outputs" })}
            </div>
          </>
        )
        : (
          // 展开态：渲染标准 Handle + 容器子节点 3 端口
          <>
            <Handle
              type="target"
              position={Position.Top}
              style={{
                background: bd.color,
                border: "none",
                width: 8,
                height: 8,
              }}
            />
            <Handle
              type="source"
              position={Position.Bottom}
              style={{
                background: bd.color,
                border: "none",
                width: 8,
                height: 8,
              }}
            />

            {/* 容器内子节点：动态端口出口（最多 6 端口均匀分布），减少边交叉 */}
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
                        width: 6,
                        height: 6,
                        left: leftPct,
                        opacity: 0.4,
                      }}
                    />
                  );
                })}
              </>
            )}

            {/* 端口已展开时的提示：点击折叠（端口密集节点） */}
            {shouldCollapseByDefault && (
              <div
                onClick={togglePorts}
                title={t("workflow.node.clickToCollapsePorts", {
                  defaultValue: "Click to collapse ports",
                })}
                style={{
                  position: "absolute",
                  bottom: -8,
                  right: -8,
                  fontSize: 8,
                  lineHeight: "12px",
                  padding: "0 4px",
                  borderRadius: 2,
                  background: token.colorBgElevated,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  color: token.colorTextTertiary,
                  cursor: "pointer",
                  zIndex: 5,
                  userSelect: "none",
                  opacity: isHovering ? 1 : 0.5,
                  transition: "opacity 0.15s",
                }}
              >
                📦
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
    condition: "🔀",
    parallel: "⚡",
    loop: "🔄",
    merge: "🔗",
    delay: "⏱",
    tool: "🔧",
    code: "💻",
    subWorkflow: "📦",
    workflowRef: "🔗",
    documentParser: "📄",
    vectorRetrieve: "🔍",
    end: "🏁",
  };
  return icons[type] || "📦";
}

export const BaseNode = memo(BaseNodeComponent);
