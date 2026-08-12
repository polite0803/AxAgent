// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, getNodeSize, PORT_SIZE } from "@/lib/workflowLayout";
import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { ReloadOutlined } from "@ant-design/icons";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { NodeCard } from "./NodeCard";

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

  const getAccent = () => {
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

  const accent = getAccent();
  const isSkipped = effectiveExecState === "skipped";

  const nodeSize = getNodeSize(bd.type);
  const nodeWidth = nodeSize.width;
  const nodeHeight = nodeSize.height;

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
      style={{
        width: nodeWidth,
        height: nodeHeight,
        opacity: bd.enabled ? (isSkipped ? 0.4 : 1) : 0.5,
        filter: bd.enabled ? (isSkipped ? "grayscale(80%)" : "none") : "grayscale(100%)",
        transition: "opacity 0.15s",
      }}
    >
      <NodeCard
        nodeType={bd.nodeType}
        title={bd.title}
        accent={accent}
        selected={selected}
        enabled={bd.enabled}
        description={bd.description}
        badge={hasBreakpoint
          ? (
            <div
              title={t("workflow.node.breakpoint", { defaultValue: "Breakpoint" })}
              style={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                background: "#ff4d4f",
                border: "2px solid white",
                boxShadow: "0 1px 3px rgba(255,77,79,0.4)",
              }}
            />
          )
          : bd.validationState
          ? (
            <div
              title={bd.validationMessage}
              style={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                background: bd.validationState === "error" ? token.colorError : token.colorWarning,
                border: "2px solid white",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 7,
                fontWeight: 700,
                color: "#fff",
                boxShadow: "0 1px 3px rgba(0,0,0,0.2)",
                cursor: "pointer",
              }}
            >
              !
            </div>
          )
          : null}
        rightSlot={
          <>
            {statusDot && (
              <div
                style={{
                  width: 6,
                  height: 6,
                  borderRadius: "50%",
                  background: statusDot,
                  flexShrink: 0,
                }}
              />
            )}
            {bd.config?.tick_mode && (
              <ReloadOutlined
                title={t("workflow.node.tickMode")}
                style={{ fontSize: 10, color: token.colorTextTertiary }}
              />
            )}
            {bd.retry?.enabled && (
              <ReloadOutlined
                title={t("workflow.node.retryEnabled")}
                style={{ fontSize: 10, color: token.colorTextTertiary }}
              />
            )}
          </>
        }
      >
        {/* Handle（端口） */}
        <Handle
          type="target"
          position={Position.Top}
          style={{
            background: token.colorBgElevated,
            border: `2px solid ${accent}`,
            width: PORT_SIZE,
            height: PORT_SIZE,
            ...getHandlePosition(nodeWidth, nodeHeight, "top"),
          }}
        />
        <Handle
          type="source"
          position={Position.Bottom}
          style={{
            background: token.colorBgElevated,
            border: `2px solid ${accent}`,
            width: PORT_SIZE,
            height: PORT_SIZE,
            ...getHandlePosition(nodeWidth, nodeHeight, "bottom"),
          }}
        />
      </NodeCard>
    </div>
  );
};

export const BaseNode = memo(BaseNodeComponent);
