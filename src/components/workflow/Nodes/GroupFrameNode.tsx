// SPDX-License-Identifier: AGPL-3.0-only

import type { NodeProps } from "@xyflow/react";
import { theme } from "antd";
import React, { memo, useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

interface GroupFrameData {
  id: string;
  type: "groupFrame";
  title: string;
  color: string;
  borderColor?: string;
  enabled: boolean;
  collapsed?: boolean;
}

/**
 * Group/Frame 节点。
 *
 * 用于对子图进行视觉分组，有标题栏、可折叠、可调整边框颜色。
 * 本身不包含执行逻辑，仅在编辑器中渲染为框。支持嵌套。
 */
const GroupFrameNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as GroupFrameData;
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [collapsed, setCollapsed] = useState(data.collapsed || false);

  const toggleCollapse = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setCollapsed((p) => !p);
  }, []);

  const borderColor = data.borderColor || token.colorBorderSecondary;

  return (
    <div
      style={{
        minWidth: collapsed ? 200 : 300,
        minHeight: collapsed ? 40 : 200,
        background: `${borderColor}08`,
        border: `2px solid ${selected ? token.colorPrimary : borderColor}`,
        borderRadius: 10,
        padding: collapsed ? "6px 10px" : 10,
        opacity: 0.85,
        position: "relative",
        boxShadow: selected ? `0 0 0 2px ${borderColor}40` : "none",
        transition: "min-width 0.25s, min-height 0.25s",
      }}
    >
      {/* 标题栏 */}
      <div
        className="react-flow__node-default"
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          userSelect: "none",
        }}
      >
        <span style={{ fontSize: 14, opacity: 0.6 }}>⊞</span>
        <span
          style={{
            fontSize: 12,
            fontWeight: 600,
            color: borderColor,
            flex: 1,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {data.title || t("workflow.groupNode.untitled", { defaultValue: "Group" })}
        </span>
        {collapsed && (
          <span
            style={{
              fontSize: 10,
              color: token.colorTextTertiary,
              background: `${borderColor}15`,
              padding: "0 6px",
              borderRadius: 3,
              lineHeight: "16px",
            }}
          >
            {t("workflow.groupNode.folded", { defaultValue: "folded" })}
          </span>
        )}
        <span
          onClick={toggleCollapse}
          style={{
            cursor: "pointer",
            fontSize: 12,
            color: token.colorTextTertiary,
            opacity: 0.6,
            padding: "2px 4px",
            borderRadius: 3,
            transition: "transform 0.2s",
            transform: collapsed ? "rotate(-90deg)" : "rotate(0deg)",
            userSelect: "none",
          }}
        >
          ▼
        </span>
      </div>
    </div>
  );
};

export const GroupFrameNode = memo(GroupFrameNodeComponent);
