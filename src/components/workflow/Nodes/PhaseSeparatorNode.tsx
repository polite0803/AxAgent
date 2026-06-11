// SPDX-License-Identifier: AGPL-3.0-only

import type { NodeProps } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";

interface PhaseSeparatorData {
  id: string;
  type: "phaseSeparator";
  label: string;
  color: string;
}

/**
 * 阶段分隔线节点。
 *
 * 渲染为半透明水平横线 + 阶段标签文字，
 * 不参与执行逻辑，仅用于大型工作流按阶段阅读和维护。
 */
const PhaseSeparatorNodeComponent: React.FC<
  NodeProps
> = ({ data: _data, selected }) => {
  const data = _data as unknown as PhaseSeparatorData;
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const label = data.label || t("workflow.phaseSeparator.defaultLabel", {
    defaultValue: "Phase",
  });

  return (
    <div
      style={{
        width: "100%",
        height: 32,
        position: "relative",
        cursor: "row-resize",
        userSelect: "none",
      }}
    >
      {/* 主线 */}
      <div
        style={{
          position: "absolute",
          top: "50%",
          left: 0,
          right: 0,
          height: 2,
          background: selected
            ? token.colorPrimary
            : token.colorTextQuaternary,
          opacity: selected ? 0.6 : 0.3,
          transform: "translateY(-50%)",
          transition: "opacity 0.2s, background 0.2s",
        }}
      />

      {/* 选中环（两端装饰） */}
      {selected && (
        <>
          <div
            style={{
              position: "absolute",
              top: "50%",
              left: -4,
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: token.colorPrimary,
              opacity: 0.5,
              transform: "translateY(-50%)",
            }}
          />
          <div
            style={{
              position: "absolute",
              top: "50%",
              right: -4,
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: token.colorPrimary,
              opacity: 0.5,
              transform: "translateY(-50%)",
            }}
          />
        </>
      )}

      {/* 标签 */}
      <div
        style={{
          position: "absolute",
          top: "50%",
          left: 16,
          transform: "translateY(-50%)",
          display: "flex",
          alignItems: "center",
          gap: 6,
          background: token.colorBgContainer,
          padding: "0 8px",
          fontSize: 11,
          fontWeight: 600,
          color: selected
            ? token.colorPrimary
            : token.colorTextSecondary,
          letterSpacing: "0.3px",
          borderRadius: 4,
          lineHeight: "22px",
          userSelect: "none",
          transition: "color 0.2s",
        }}
      >
        {/* 分隔符图标 */}
        <span style={{ fontSize: 10, opacity: 0.5 }}>━</span>
        {label}
      </div>
    </div>
  );
};

export const PhaseSeparatorNode = memo(PhaseSeparatorNodeComponent);
