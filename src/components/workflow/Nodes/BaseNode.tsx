import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { theme } from "antd";
import React, { memo } from "react";
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

  return (
    <div
      style={{
        minWidth: 160,
        maxWidth: 200,
        opacity: data.enabled ? (isSkipped ? 0.4 : 1) : 0.5,
        filter: data.enabled ? (isSkipped ? "grayscale(80%)" : "none") : "grayscale(100%)",
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
            }}
          >
            {typeInfo.labelKey ? t(typeInfo.labelKey) : data.nodeType}
          </span>
          <div style={{ display: "flex", gap: 2, alignItems: "center" }}>
            {(data as any).config?.tick_mode && (
              <span title={t("workflow.node.tickMode")} style={{ fontSize: 10 }}>�?</span>
            )}
            {(data as any).retry?.enabled && (
              <span title={t("workflow.node.retryEnabled")} style={{ fontSize: 10 }}>🔄</span>
            )}
            {effectiveExecState === "running" && <span style={{ fontSize: 10, color: token.colorPrimary }}>�?</span>}
            {effectiveExecState === "completed" && <span style={{ fontSize: 10, color: token.colorSuccess }}>�?</span>}
            {(effectiveExecState === "failed" || effectiveExecState === "timeout") && (
              <span style={{ fontSize: 10, color: token.colorError }}>�?</span>
            )}
            {effectiveExecState === "paused" && <span style={{ fontSize: 10, color: token.colorWarning }}>�?</span>}
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
        </div>
      </div>

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
    </div>
  );
};

function getNodeIcon(type: string): string {
  const icons: Record<string, string> = {
    trigger: "�?",
    agent: "🤖",
    llm: "🧠",
    condition: "�?",
    parallel: "�?",
    loop: "🔄",
    merge: "🔗",
    delay: "�?",
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
