import { theme } from "antd";
import React, { memo, useState } from "react";
import { useTranslation } from "react-i18next";
import { NODE_KIND_LABEL_KEYS, type NodeKind } from "./types";

/**
 * 语义化颜色 — 根据 NodeKind 从主题 token 解析可适配深色/浅色的颜色值。
 *
 * 颜色规范（引擎定义，不可在设计层覆盖）：
 * Input=黄  Output=红  Tool=绿  Agent=蓝
 * Condition=橙  Loop=紫  Container=青  Storage=粉
 */
// eslint-disable-next-line react-refresh/only-export-components
export function getNodeKindColor(kind: NodeKind): {
  base: string;
  bg: string;
  border: string;
} {
  switch (kind) {
    case "input":
      return { base: "#fadb14", bg: "#fadb1415", border: "#fadb1450" };
    case "output":
      return { base: "#f5222d", bg: "#f5222d15", border: "#f5222d50" };
    case "tool":
      return { base: "#52c41a", bg: "#52c41a15", border: "#52c41a50" };
    case "agent":
      return { base: "#1677ff", bg: "#1677ff15", border: "#1677ff50" };
    case "condition":
      return { base: "#fa8c16", bg: "#fa8c1615", border: "#fa8c1650" };
    case "loop":
      return { base: "#722ed1", bg: "#722ed115", border: "#722ed150" };
    case "container":
      return { base: "#13c2c2", bg: "#13c2c215", border: "#13c2c250" };
    case "storage":
      return { base: "#eb2f96", bg: "#eb2f9615", border: "#eb2f9650" };
    default:
      return { base: "#888", bg: "#88888815", border: "#88888850" };
  }
}

/**
 * 编辑器右上角的浮动颜色图例组件。
 * 可折叠，默认展开。颜色自动从 NODE_KIND_MAP 映射生成。
 */
export const WorkflowLegend: React.FC = memo(() => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [collapsed, setCollapsed] = useState(false);

  const kinds: NodeKind[] = [
    "input",
    "output",
    "tool",
    "agent",
    "condition",
    "loop",
    "container",
    "storage",
  ];

  return (
    <div
      onMouseDown={(e) => e.stopPropagation()}
      style={{
        background: token.colorBgElevated,
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: 8,
        fontSize: 11,
        lineHeight: "20px",
        userSelect: "none",
        boxShadow: "0 2px 8px rgba(0,0,0,0.15)",
        overflow: "hidden",
        transition: "width 0.2s, height 0.2s",
        width: collapsed ? 32 : "auto",
      }}
    >
      {/* 标题栏（可点击折叠） */}
      <div
        id="workflow-legend-header"
        role="button"
        tabIndex={0}
        aria-label={t("workflow.legend.toggleLabel", {
          action: t(collapsed ? "workflow.legend.expand" : "workflow.legend.collapse", {
            defaultValue: collapsed ? "Expand" : "Collapse",
          }),
          defaultValue: collapsed ? "Expand legend" : "Collapse legend",
        })}
        aria-expanded={!collapsed}
        aria-controls="workflow-legend-content"
        onClick={() => setCollapsed(!collapsed)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setCollapsed(!collapsed);
          }
        }}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 8,
          padding: collapsed ? "6px 8px" : "6px 12px",
          cursor: "pointer",
          borderBottom: collapsed ? "none" : `1px solid ${token.colorBorderSecondary}`,
          transition: "border 0.2s",
        }}
      >
        {!collapsed && (
          <span
            style={{
              fontWeight: 600,
              fontSize: 10,
              color: token.colorTextSecondary,
              textTransform: "uppercase",
              letterSpacing: "0.5px",
            }}
          >
            {t("workflow.legend.title", { defaultValue: "Node Colors" })}
          </span>
        )}
        <span
          style={{
            fontSize: 10,
            color: token.colorTextTertiary,
            transform: collapsed ? "rotate(-90deg)" : "rotate(0deg)",
            transition: "transform 0.2s",
            lineHeight: 1,
          }}
        >
          {collapsed ? "▶" : "▲"}
        </span>
      </div>

      {/* 图例内容 */}
      {!collapsed && (
        <div
          id="workflow-legend-content"
          role="region"
          aria-labelledby="workflow-legend-header"
          style={{ padding: "6px 12px 8px" }}
        >
          {kinds.map((kind) => {
            const colors = getNodeKindColor(kind);
            return (
              <div
                key={kind}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  marginBottom: 2,
                }}
              >
                <span
                  style={{
                    display: "inline-block",
                    width: 10,
                    height: 10,
                    borderRadius: "50%",
                    background: colors.base,
                    border: `1px solid ${colors.border}`,
                    flexShrink: 0,
                  }}
                />
                <span style={{ color: token.colorTextTertiary }}>
                  {t(NODE_KIND_LABEL_KEYS[kind])}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
});
