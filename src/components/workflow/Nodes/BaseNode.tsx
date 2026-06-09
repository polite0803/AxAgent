import { useWorkflowEditorStore } from "@/stores";
import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { theme } from "antd";
import React, { memo, useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";
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
}

/** 端口折叠阈值：当节点有 N 条以上输入或输出边时，默认折叠端口 */
const PORT_COLLAPSE_THRESHOLD = 4;

const BaseNodeComponent: React.FC<NodeProps<BaseNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const typeInfo = NODE_TYPE_MAP[data.nodeType] || {
    labelKey: "",
    color: token.colorTextTertiary,
  };

  const nodeStatuses = useWorkEngineStore((s) => s.nodeStatuses);
  const breakpoints = useWorkEngineStore((s) => s.breakpoints);
  const runtimeStatus = nodeStatuses[data.id];
  const hasBreakpoint = breakpoints.includes(data.id);

  const effectiveExecState = runtimeStatus || data.executionState;

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
  const [isPortCollapsed, setIsPortCollapsed] = useState(shouldCollapseByDefault);
  const [isHovering, setIsHovering] = useState(false);

  // 当边数降低到阈值以下时，自动恢复展开态
  useEffect(() => {
    if (!shouldCollapseByDefault && isPortCollapsed) {
      setIsPortCollapsed(false);
    }
  }, [shouldCollapseByDefault, isPortCollapsed]);

  const togglePorts = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      setIsPortCollapsed((prev) => !prev);
    },
    [],
  );

  const getBorderColor = () => {
    if (data.validationState === "error") { return token.colorError; }
    if (data.validationState === "warning") { return token.colorWarning; }
    if (effectiveExecState === "running") { return token.colorPrimary; }
    if (effectiveExecState === "completed") { return token.colorSuccess; }
    if (effectiveExecState === "failed" || effectiveExecState === "timeout") { return token.colorError; }
    if (effectiveExecState === "paused") { return token.colorWarning; }
    if (hasBreakpoint) { return "#ff4d4f"; }
    if (selected) { return token.colorPrimary; }
    return data.color;
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
        opacity: data.enabled ? (isSkipped ? 0.4 : 1) : 0.5,
        filter: data.enabled ? (isSkipped ? "grayscale(80%)" : "none") : "grayscale(100%)",
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

        {data.validationState && data.validationMessage && (
          <div
            title={data.validationMessage}
            style={{
              position: "absolute",
              top: -6,
              left: -6,
              width: 14,
              height: 14,
              borderRadius: "50%",
              background: data.validationState === "error" ? token.colorError : token.colorWarning,
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
            borderBottom: `1px solid ${data.color}40`,
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span style={{ fontSize: 16 }}>{getNodeIcon(data.nodeType)}</span>
          <span
            style={{
              fontSize: 12,
              color: data.color,
              fontWeight: 500,
              flex: 1,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {typeInfo.labelKey ? t(typeInfo.labelKey) : data.nodeType}
          </span>
          <div style={{ display: "flex", gap: 2, alignItems: "center" }}>
            {(data as any).config?.tick_mode && (
              <span title={t("workflow.node.tickMode")} style={{ fontSize: 10 }}>🔄</span>
            )}
            {(data as any).retry?.enabled && (
              <span title={t("workflow.node.retryEnabled")} style={{ fontSize: 10 }}>🔄</span>
            )}
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
              marginBottom: data.description ? 4 : 0,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>
          {data.description && (
            <div
              style={{
                fontSize: 12,
                color: token.colorTextTertiary,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {data.description}
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
              {data.nodeType && (
                <div>
                  {t("workflow.node.type", { defaultValue: "Type" })}: {data.nodeType}
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
                background: `${data.color}20`,
                border: `1px solid ${data.color}50`,
                color: data.color,
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
                background: `${data.color}20`,
                border: `1px solid ${data.color}50`,
                color: data.color,
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
                background: data.color,
                border: "none",
                width: 8,
                height: 8,
              }}
            />
            <Handle
              type="source"
              position={Position.Bottom}
              style={{
                background: data.color,
                border: "none",
                width: 8,
                height: 8,
              }}
            />

            {/* 容器内子节点：3 端口出口（左/中/右），减少边交叉 */}
            {data.parentId && (
              <>
                <Handle
                  type="source"
                  position={Position.Bottom}
                  id="port-0"
                  style={{
                    background: data.color,
                    border: "1px solid transparent",
                    width: 6,
                    height: 6,
                    left: "25%",
                    opacity: 0.5,
                  }}
                />
                <Handle
                  type="source"
                  position={Position.Bottom}
                  id="port-1"
                  style={{
                    background: data.color,
                    border: "1px solid transparent",
                    width: 6,
                    height: 6,
                    left: "50%",
                    opacity: 0.5,
                  }}
                />
                <Handle
                  type="source"
                  position={Position.Bottom}
                  id="port-2"
                  style={{
                    background: data.color,
                    border: "1px solid transparent",
                    width: 6,
                    height: 6,
                    left: "75%",
                    opacity: 0.5,
                  }}
                />
              </>
            )}

            {["condition", "merge"].includes(data.nodeType) && (
              <>
                <Handle
                  type="target"
                  position={Position.Left}
                  id="left-handle"
                  style={{
                    background: data.color,
                    border: "none",
                    width: 6,
                    height: 6,
                    top: "50%",
                  }}
                />
                <Handle
                  type="source"
                  position={Position.Right}
                  id="right-handle"
                  style={{
                    background: data.color,
                    border: "none",
                    width: 6,
                    height: 6,
                    top: "50%",
                  }}
                />
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
