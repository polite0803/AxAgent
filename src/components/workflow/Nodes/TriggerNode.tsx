import { Button, theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";
import { useWorkflowEditorStore } from "@/stores/feature/workflowEditorStore";

const PURPLE_BASE = "#722ed1";
const PURPLE_VAR = `var(--purple, ${PURPLE_BASE})`;

interface TriggerNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  /** store 中的 config（TriggerConfig），通过 ...node 展开注入 */
  config?: {
    type: "manual" | "schedule" | "webhook" | "event";
    config: unknown;
  };
  /** 兼容旧字段，与 config 相同 */
  triggerConfig?: {
    type: "manual" | "schedule" | "webhook" | "event";
    config: unknown;
  };
}

type TriggerStatus = "active" | "disabled" | "unconfigured";

function getTriggerStatus(data: TriggerNodeData): TriggerStatus {
  if (!data.enabled) return "disabled";
  const tc = data.config || data.triggerConfig;
  if (!tc || !tc.config) return "unconfigured";
  return "active";
}

const STATUS_META: Record<
  TriggerStatus,
  { badge: string; labelKey: string; color: string }
> = {
  active: {
    badge: "\u{1F7E2}", // 🟢
    labelKey: "workflow.triggerNode.statusActive",
    color: "#52c41a",
  },
  disabled: {
    badge: "\u{1F7E1}", // 🟡
    labelKey: "workflow.triggerNode.statusDisabled",
    color: "#faad14",
  },
  unconfigured: {
    badge: "\u{1F534}", // 🔴
    labelKey: "workflow.triggerNode.statusUnconfigured",
    color: "#ff4d4f",
  },
};

const TriggerNodeComponent: React.FC<NodeProps<TriggerNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const setSelectedNode = useWorkflowEditorStore((s) => s.setSelectedNode);

  // 优先从 data.config 读取（store 通过 ...node 展开），fallback 到 data.triggerConfig
  const resolvedConfig = data.config || data.triggerConfig;
  const triggerType = resolvedConfig?.type || "manual";
  const color = PURPLE_VAR;

  const status = getTriggerStatus(data);
  const statusMeta = STATUS_META[status];

  const getTriggerIcon = (type: string): string => {
    switch (type) {
      case "manual":
        return "\u{1F446}"; // 👆
      case "schedule":
        return "\u23F0"; // ⏰
      case "webhook":
        return "\u{1FAA1}"; // 🪝
      case "event":
        return "\u26A1"; // ⚡
      default:
        return "\u26A1";
    }
  };

  const getTriggerDescription = (type: string, cfg: unknown): string => {
    switch (type) {
      case "manual":
        return t("workflow.triggerNode.manual");
      case "schedule": {
        const sc = cfg as { cron?: string; timezone?: string } | undefined;
        return sc?.cron ? `Cron: ${sc.cron}` : t("workflow.triggerNode.schedule");
      }
      case "webhook": {
        const wc = cfg as { path?: string; method?: string } | undefined;
        return wc?.path ? `${wc.method || "GET"} ${wc.path}` : "Webhook";
      }
      case "event": {
        const ec = cfg as { event_type?: string } | undefined;
        return ec?.event_type || t("workflow.triggerNode.event");
      }
      default:
        return "";
    }
  };

  const handleConfigure = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      setSelectedNode(data.id);
    },
    [data.id, setSelectedNode],
  );

  const tooltipMsg = (() => {
    if (status === "active") return t("workflow.triggerNode.statusActive");
    if (status === "disabled") return t("workflow.triggerNode.statusDisabled");
    return t("workflow.triggerNode.statusUnconfigured");
  })();

  return (
    <div
      style={{
        minWidth: 180,
        maxWidth: 220,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: token.colorBgElevated,
          border: `2px solid ${selected ? token.colorPrimary : color}`,
          borderRadius: 8,
          overflow: "visible",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
          position: "relative",
        }}
      >
        {/* 标题栏：紫色底色 */}
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${PURPLE_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${PURPLE_BASE}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>{getTriggerIcon(triggerType)}</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.5px",
            }}
          >
            {t("workflow.triggerNode.title")}
          </span>
        </div>

        {/* 内容区：标题 + 触发类型 Tag + 描述 */}
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

          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span
              style={{
                fontSize: 11,
                padding: "0 6px",
                borderRadius: 4,
                background: `${color}18`,
                color: color,
                fontWeight: 600,
                letterSpacing: "0.3px",
              }}
            >
              {triggerType.toUpperCase()}
            </span>
            <span
              style={{
                fontSize: 12,
                color: token.colorTextTertiary,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                flex: 1,
              }}
            >
              {getTriggerDescription(triggerType, resolvedConfig?.config)}
            </span>
          </div>
        </div>

        {/* ── 状态徽章（右下角） ────────────────────────────── */}
        <Tooltip title={tooltipMsg}>
          <div
            style={{
              position: "absolute",
              bottom: -6,
              right: -6,
              width: 18,
              height: 18,
              borderRadius: "50%",
              background: token.colorBgElevated,
              border: `2px solid ${statusMeta.color}`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 10,
              lineHeight: 1,
              cursor: "default",
              zIndex: 10,
            }}
          >
            {statusMeta.badge}
          </div>
        </Tooltip>

        {/* ── 补全配置按钮（仅未配置时显示） ────────────────── */}
        {status === "unconfigured" && (
          <Tooltip title={t("workflow.triggerNode.configureHint")}>
            <Button
              type="text"
              size="small"
              onClick={handleConfigure}
              style={{
                position: "absolute",
                bottom: -6,
                left: -6,
                fontSize: 11,
                padding: "0 4px",
                height: 20,
                lineHeight: "20px",
                borderRadius: 10,
                background: token.colorBgElevated,
                border: `1px solid ${token.colorBorderSecondary}`,
                color: token.colorPrimary,
                zIndex: 10,
              }}
            >
              {t("workflow.triggerNode.configure")}
            </Button>
          </Tooltip>
        )}
      </div>

      {/* 底部 Source Handle */}
      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: color,
          border: "none",
          width: 10,
          height: 10,
        }}
      />

      {/* 顶部三角形指示器 */}
      <div
        style={{
          position: "absolute",
          top: -10,
          left: "50%",
          transform: "translateX(-50%)",
          width: 0,
          height: 0,
          borderLeft: "6px solid transparent",
          borderRight: "6px solid transparent",
          borderBottom: `8px solid ${color}`,
        }}
      />
    </div>
  );
};

export const TriggerNode = memo(TriggerNodeComponent);
