import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

const HTTP_COLOR = "#eb2f96";

interface HttpRequestNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  url?: string;
  method?: string;
}

const HttpRequestNodeComponent: React.FC<NodeProps<HttpRequestNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const method = data.method || "GET";
  const url = data.url || "";
  const urlPreview = url.length > 25 ? url.slice(0, 25) + "..." : url;

  const getMethodColor = (m: string) => {
    switch (m.toUpperCase()) {
      case "GET":
        return "#52c41a";
      case "POST":
        return "#1677ff";
      case "PUT":
        return "#fa8c16";
      case "PATCH":
        return "#722ed1";
      case "DELETE":
        return "#ff4d4f";
      default:
        return token.colorTextQuaternary;
    }
  };

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
          border: "2px solid " + (selected ? token.colorPrimary : HTTP_COLOR),
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? "0 0 0 2px " + HTTP_COLOR + "40" : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: "1px solid " + HTTP_COLOR + "30",
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: HTTP_COLOR + "15",
          }}
        >
          <span style={{ fontSize: 14 }}>🌐</span>
          <span style={{ fontSize: 12, color: HTTP_COLOR, fontWeight: 600 }}>
            {t("workflow.nodeTypes.httpRequest")}
          </span>
          <Tag
            style={{
              margin: "0 0 0 auto",
              fontSize: 9,
              padding: "0 4px",
              border: "none",
              color: "#fff",
              background: getMethodColor(method),
            }}
          >
            {method.toUpperCase()}
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

          {url && (
            <div
              style={{
                fontSize: 11,
                color: getMethodColor(method),
                padding: "4px 6px",
                background: getMethodColor(method) + "10",
                borderRadius: 4,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                fontFamily: "monospace",
              }}
            >
              {urlPreview}
            </div>
          )}
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{ background: HTTP_COLOR, border: "none", width: 8, height: 8 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: HTTP_COLOR, border: "none", width: 8, height: 8 }}
      />
    </div>
  );
};

export const HttpRequestNode = memo(HttpRequestNodeComponent);
