// SPDX-License-Identifier: AGPL-3.0-only

import { theme } from "antd";
import React from "react";
import { nodeIconFor } from "./nodeIcons";

export interface NodeCardProps {
  /** 节点类型，用于自动解析图标 */
  nodeType: string;
  /** 显示标题 */
  title: string;
  /** 强调色（类型色）：左侧 4px 强调条 + 图标着色 */
  accent: string;
  selected?: boolean;
  enabled?: boolean;
  description?: string;
  /** 自定义图标（缺省按 nodeType 解析） */
  icon?: React.ReactNode;
  /** 标题行右侧内联内容（计数徽标、展开按钮等） */
  rightSlot?: React.ReactNode;
  /** 右上角状态徽标区（断点 / 校验 / 运行状态） */
  badge?: React.ReactNode;
  /** 标题行下方追加内容 */
  footer?: React.ReactNode;
  /** 透传到外层 wrapper（用于指定 width/height 等） */
  wrapperStyle?: React.CSSProperties;
  /** 透传 ReactFlow Handle 等绝对定位元素 */
  children?: React.ReactNode;
}

/**
 * 工作流节点统一卡片外壳。
 *
 * 视觉规范（Phase 1）：
 * - 中性卡片：colorBgContainer 背景 + 1px 细边框，左侧 4px 类型色强调条
 * - 图标统一为 antd 单色图标，类型色着色
 * - 状态指示集中到右上角 badge 区域，不再散落四角
 * - 选中：强调色 2px 环形描边
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
        minWidth: 120,
        maxWidth: 200,
        opacity: isDisabled ? 0.5 : 1,
        filter: isDisabled ? "grayscale(100%)" : "none",
        transition: "opacity 0.15s",
        ...wrapperStyle,
      }}
    >
      <div
        className="workflow-node-card"
        data-disabled={isDisabled}
        title={description || title}
        style={{
          background: token.colorBgContainer,
          border: `1px solid ${token.colorBorderSecondary}`,
          borderLeft: `4px solid ${accent}`,
          borderRadius: 8,
          padding: 0,
          boxShadow: selected
            ? `0 0 0 2px ${accent}55`
            : "0 1px 3px rgba(0,0,0,0.12)",
          transition: "box-shadow 0.15s, border-color 0.15s",
          position: "relative",
          width: "100%",
          overflow: "visible",
        }}
      >
        {/* 右上角状态徽标区 */}
        {badge && (
          <div
            style={{
              position: "absolute",
              top: -5,
              right: -5,
              display: "flex",
              alignItems: "center",
              gap: 3,
              zIndex: 10,
            }}
          >
            {badge}
          </div>
        )}

        <div
          style={{
            padding: "6px 10px",
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          {/* 图标块（类型色着色） */}
          <div
            style={{
              width: 22,
              height: 22,
              borderRadius: 4,
              background: `${accent}14`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: accent,
              fontSize: 14,
              flexShrink: 0,
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
              lineHeight: "22px",
            }}
          >
            {title}
          </span>

          {/* 右侧内联内容（计数 / 展开按钮等） */}
          {rightSlot}
        </div>

        {footer}
      </div>

      {children}
    </div>
  );
};
