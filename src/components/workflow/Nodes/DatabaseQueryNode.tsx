// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";

const DB_COLOR = "#13c2c2";

interface DatabaseQueryNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  query?: string;
}

const DatabaseQueryNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as DatabaseQueryNodeData;
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const query = data.query || "";
  const preview = query.length > 40 ? query.slice(0, 40) + "..." : query;

  return (
    <div
      style={{
        minWidth: 180,
        maxWidth: 240,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: token.colorBgElevated,
          border: "2px solid " + (selected ? token.colorPrimary : DB_COLOR),
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? "0 0 0 2px " + DB_COLOR + "40" : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: "1px solid " + DB_COLOR + "30",
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: DB_COLOR + "15",
          }}
        >
          <span style={{ fontSize: 14 }}>🗄️</span>
          <span style={{ fontSize: 12, color: DB_COLOR, fontWeight: 600 }}>
            {t("workflow.nodeTypes.databaseQuery")}
          </span>
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
          {query && (
            <div
              style={{
                fontSize: 10,
                color: DB_COLOR,
                padding: "4px 6px",
                background: DB_COLOR + "10",
                borderRadius: 4,
                fontFamily: "monospace",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {preview}
            </div>
          )}
        </div>
      </div>
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: DB_COLOR, border: "none", width: 8, height: 8 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: DB_COLOR, border: "none", width: 8, height: 8 }}
      />
    </div>
  );
};

export const DatabaseQueryNode = memo(DatabaseQueryNodeComponent);
