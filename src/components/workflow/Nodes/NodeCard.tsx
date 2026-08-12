// SPDX-License-Identifier: AGPL-3.0-only

import { theme } from "antd";
import React from "react";
import { nodeIconFor } from "./nodeIcons";

export interface NodeCardProps {
  nodeType: string;
  title: string;
  accent: string;
  selected?: boolean;
  enabled?: boolean;
  description?: string;
  icon?: React.ReactNode;
  rightSlot?: React.ReactNode;
  badge?: React.ReactNode;
  footer?: React.ReactNode;
  wrapperStyle?: React.CSSProperties;
  children?: React.ReactNode;
}

/**
 * n8n 风格节点卡片 - 清爽扁平设计
 */
export const NodeCard: React.FC<NodeCardProps> = ({
  nodeType,
  title,
  accent,
  selected,
  enabled = true,
  description,
  icon,
  rightSlot,
  badge,
  footer,
  wrapperStyle,
  children,
}) => {
  const { token } = theme.useToken();
  const resolvedIcon = icon ?? nodeIconFor(nodeType);
  const isDisabled = !enabled;

  return (
    <div
      style={{
        position: "relative",
        minWidth: 100,
        opacity: isDisabled ? 0.5 : 1,
        filter: isDisabled ? "grayscale(100%)" : "none",
        transition: "opacity 0.15s",
        ...wrapperStyle,
      }}
    >
      {/* n8n 风格容器：无背景色，透明，子节点直接绘制在画布上 */}
      <div
        className="workflow-node-card"
        data-disabled={isDisabled}
        title={description || title}
        style={{
          position: "relative",
          width: "100%",
          height: "100%",
          overflow: "visible",
        }}
      >
        {/* 内容区域：n8n 风格紧凑布局 */}
        <div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: token.colorBgElevated,
            border: `1px solid ${selected ? accent : token.colorBorder}`,
            borderRadius: 6,
            padding: "4px 8px",
            display: "flex",
            alignItems: "center",
            gap: 6,
            boxShadow: selected
              ? `0 0 0 2px ${accent}40, 0 2px 6px rgba(0,0,0,0.08)`
              : "0 1px 3px rgba(0,0,0,0.06)",
            transition: "box-shadow 0.15s, border-color 0.15s",
          }}
        >
          {/* 左侧强调色边 (n8n 风格: 左侧 3px) */}
          <div
            style={{
              position: "absolute",
              left: 0,
              top: 4,
              bottom: 4,
              width: 3,
              background: accent,
              borderRadius: "0 2px 2px 0",
            }}
          />

          {/* 图标块 */}
          <div
            style={{
              width: 20,
              height: 20,
              borderRadius: 4,
              background: `${accent}20`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: accent,
              fontSize: 12,
              flexShrink: 0,
              marginLeft: 4,
              lineHeight: 1,
            }}
          >
            {resolvedIcon}
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
              lineHeight: "1.4",
            }}
          >
            {title}
          </span>

          {/* 右侧内联内容 */}
          {rightSlot}
        </div>

        {/* 右上角徽标区 */}
        {badge && (
          <div
            style={{
              position: "absolute",
              top: -6,
              right: -6,
              zIndex: 10,
            }}
          >
            {badge}
          </div>
        )}

        {footer}
      </div>

      {children}
    </div>
  );
};
