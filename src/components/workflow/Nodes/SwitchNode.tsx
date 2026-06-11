// SPDX-License-Identifier: AGPL-3.0-only
// @ts-nocheck

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";

const SWITCH_COLOR = "#722ed1";

interface SwitchNodeData extends Record<string, unknown> {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  cases?: Array<{ value: string; label: string }>;
  match_mode?: string;
  input_var?: string;
}

const SwitchNodeComponent: React.FC<NodeProps> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const cases = data.cases || [];
  // const matchMode = data.match_mode || "exact";
  const inputVar = data.input_var || "";

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
          border: "2px solid " + (selected ? token.colorPrimary : SWITCH_COLOR),
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? "0 0 0 2px " + SWITCH_COLOR + "40" : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: "1px solid " + SWITCH_COLOR + "30",
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: SWITCH_COLOR + "15",
          }}
        >
          <span style={{ fontSize: 14 }}>🔀</span>
          <span style={{ fontSize: 12, color: SWITCH_COLOR, fontWeight: 600 }}>
            {t("workflow.nodeTypes.switch")}
          </span>
          <Tag
            style={{
              margin: "0 0 0 auto",
              fontSize: 9,
              padding: "0 4px",
              border: "none",
              color: "#fff",
              background: SWITCH_COLOR,
            }}
          >
            {cases.length}
          </Tag>
        </div>
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
          {inputVar && <div style={{ fontSize: 10, color: SWITCH_COLOR, marginBottom: 4 }}>{inputVar}</div>}
          {cases.slice(0, 3).map((c, i) => (
            <div key={i} style={{ fontSize: 10, color: token.colorTextTertiary, padding: "1px 0" }}>
              #{i + 1} {c.label || c.value}
            </div>
          ))}
          {cases.length > 3 && (
            <div style={{ fontSize: 10, color: token.colorTextTertiary }}>+{cases.length - 3} more</div>
          )}
        </div>
      </div>
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: SWITCH_COLOR, border: "none", width: 8, height: 8 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: SWITCH_COLOR, border: "none", width: 8, height: 8 }}
      />
    </div>
  );
};

export const SwitchNode = memo(SwitchNodeComponent);
