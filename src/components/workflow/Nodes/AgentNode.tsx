import type { ToolDef } from "@/components/workflow/types";
import { AGENT_ROLE_META } from "@/types";
import { Badge, Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

interface AgentNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  agentProfileId?: string;
  agentRole?: string;
  agentRoleIcon?: string;
  agentRoleDisplayName?: string;
  systemPrompt?: string;
  tools?: (ToolDef | string)[];
  contextSources?: string[];
  outputMode?: string;
  model?: string;
  validationState?: "error" | "warning";
}

const CYAN_BASE = "#13c2c2";
const CYAN_VAR = `var(--cyan, ${CYAN_BASE})`;

const AgentNodeComponent: React.FC<NodeProps<AgentNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = token.colorPrimary;

  const getBorderColor = () => {
    if (data.validationState === "error") {
      return token.colorError;
    }
    if (data.validationState === "warning") {
      return token.colorWarning;
    }
    if (selected) {
      return token.colorPrimary;
    }
    return color;
  };

  const borderColor = getBorderColor();

  const getOutputModeIcon = (mode: string): string => {
    switch (mode) {
      case "json":
        return "{}";
      case "text":
        return "📝";
      case "artifact":
        return "🎨";
      default:
        return "📝";
    }
  };

  const tools = data.tools || [];
  const contextSources = data.contextSources || [];

  const displayIcon = data.agentRoleIcon
    || (data.agentRole ? AGENT_ROLE_META[data.agentRole]?.icon : null)
    || "🤖";

  const displayName = data.agentRoleDisplayName
    || (data.agentRole
      ? t(AGENT_ROLE_META[data.agentRole]?.labelKey ?? "", data.agentRole)
      : null)
    || t("workflow.agentNode.agent");

  return (
    <div
      style={{
        minWidth: 200,
        maxWidth: 240,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: token.colorBgElevated,
          border: `2px solid ${borderColor}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${borderColor}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${color}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${color}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>{displayIcon}</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {displayName}
          </span>
          {data.model && (
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: `${color}30`,
                border: "none",
                color: token.colorText,
              }}
            >
              {data.model.length > 12
                ? `${data.model.slice(0, 12)}...`
                : data.model}
            </Tag>
          )}
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

          {data.systemPrompt && (
            <div
              style={{
                fontSize: 12,
                color: token.colorTextTertiary,
                marginBottom: 8,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {data.systemPrompt.slice(0, 40)}...
            </div>
          )}

          <div
            style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}
          >
            {tools.length > 0 && (
              <Badge
                count={tools.length}
                size="small"
                style={{
                  backgroundColor: token.colorSuccess,
                  fontSize: 9,
                }}
              >
                <Tag
                  style={{
                    margin: 0,
                    fontSize: 9,
                    padding: "0 4px",
                    background: `${token.colorSuccess}20`,
                    border: `1px solid ${token.colorSuccess}50`,
                    color: token.colorSuccess,
                  }}
                >
                  {t("workflow.agentNode.tools")}
                </Tag>
              </Badge>
            )}

            {contextSources.length > 0 && (
              <Badge
                count={contextSources.length}
                size="small"
                style={{
                  backgroundColor: CYAN_VAR,
                  fontSize: 9,
                }}
              >
                <Tag
                  style={{
                    margin: 0,
                    fontSize: 9,
                    padding: "0 4px",
                    background: `${CYAN_BASE}20`,
                    border: `1px solid ${CYAN_BASE}50`,
                    color: CYAN_VAR,
                  }}
                >
                  {t("workflow.agentNode.context")}
                </Tag>
              </Badge>
            )}

            {data.outputMode && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: `${color}20`,
                  border: `1px solid ${color}50`,
                  color: color,
                }}
              >
                {getOutputModeIcon(data.outputMode)} {data.outputMode}
              </Tag>
            )}
          </div>
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />
    </div>
  );
};

export const AgentNode = memo(AgentNodeComponent);
