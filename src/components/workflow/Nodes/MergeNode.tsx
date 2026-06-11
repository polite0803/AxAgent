// SPDX-License-Identifier: AGPL-3.0-only
// @ts-nocheck

import type { NodeProps } from "@xyflow/react";
import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import type { MergeStrategy } from "../types/workflow.types";

const ORANGE_BASE = "#fa8c16";
const ORANGE_VAR = `var(--orange, ${ORANGE_BASE})`;

interface MergeNodeData extends Record<string, unknown> {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  mergeStrategy?: MergeStrategy;
  inputCount?: number;
  autoInputsFromBranches?: boolean;
}

const MergeNodeComponent: React.FC<NodeProps> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = ORANGE_VAR;
  const mergeStrategy = data.mergeStrategy || "all";
  const inputCount = data.inputCount || 2;
  const autoInputsFromBranches = data.autoInputsFromBranches !== false;

  const getMergeStrategyLabel = (strategy: MergeStrategy): string => {
    switch (strategy) {
      case "all":
        return t("workflow.mergeNode.waitAll");
      case "any":
        return t("workflow.mergeNode.waitAny");
      case "race":
        return t("workflow.mergeNode.race");
      case "majority":
        return t("workflow.mergeNode.majority");
      default:
        return String(strategy);
    }
  };

  // 容器节点：不需要 Handle，子节点通过 parentId 挂载
  return (
    <div
      style={{
        minWidth: 220,
        minHeight: 120,
        background: `${ORANGE_BASE}08`,
        border: `2px solid ${selected ? token.colorPrimary : ORANGE_BASE}40`,
        borderRadius: 12,
        padding: 12,
        opacity: data.enabled ? 1 : 0.5,
        position: "relative",
        boxShadow: selected ? `0 0 0 2px ${ORANGE_VAR}40` : "none",
      }}
    >
      {/* 标题栏 — 左上角 */}
      <div
        style={{
          position: "absolute",
          top: 8,
          left: 12,
          display: "flex",
          alignItems: "center",
          gap: 6,
          background: token.colorBgElevated,
          border: `1px solid ${ORANGE_BASE}30`,
          borderRadius: 6,
          padding: "4px 10px",
          zIndex: 10,
        }}
      >
        <span style={{ fontSize: 14 }}>🔗</span>
        <span style={{ fontSize: 12, color, fontWeight: 600 }}>
          {data.title}
        </span>
        <div style={{ display: "flex", gap: 4 }}>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${ORANGE_BASE}20`,
              border: `1px solid ${ORANGE_BASE}50`,
              color: ORANGE_VAR,
            }}
          >
            {inputCount} {t("workflow.mergeNode.inputs")}
          </Tag>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: token.colorBgContainer,
              border: `1px solid ${token.colorBorderSecondary}`,
              color: token.colorTextQuaternary,
            }}
          >
            {getMergeStrategyLabel(mergeStrategy)}
          </Tag>
        </div>
        {autoInputsFromBranches && (
          <span style={{ fontSize: 9, color: token.colorTextTertiary }}>
            {t("workflow.mergeNode.autoInputs")}
          </span>
        )}
      </div>

      {/* 子节点由 ReactFlow 根据 parentId 自动绘制在此容器内 */}
    </div>
  );
};

export const MergeNode = memo(MergeNodeComponent);
