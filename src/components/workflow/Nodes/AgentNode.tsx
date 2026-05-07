import { Badge, Tag } from "antd";
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
  agentRole?: string;
  systemPrompt?: string;
  tools?: string[];
  contextSources?: string[];
  outputMode?: string;
  model?: string;
  expertRoleId?: string;
  expertIcon?: string;
  expertName?: string;
  validationState?: "error" | "warning";
}

const AgentNodeComponent: React.FC<NodeProps<AgentNodeData>> = ({ data, selected }) => {
  const { t } = useTranslation("chat");
  const color = "#1890ff";

  const getBorderColor = () => {
    if (data.validationState === "error") { return "#ff4d4f"; }
    if (data.validationState === "warning") { return "#faad14"; }
    if (selected) { return "#1890ff"; }
    return color;
  };

  const borderColor = getBorderColor();

  const getRoleIcon = (role: string): string => {
    switch (role) {
      case "researcher":
        return "🔍";
      case "planner":
        return "📋";
      case "developer":
        return "💻";
      case "reviewer":
        return "👀";
      case "synthesizer":
        return "🔬";
      case "executor":
        return "⚙️";
      default:
        return "🤖";
    }
  };

  const getRoleLabel = (role: string): string => {
    const labels: Record<string, string> = {
      researcher: t("workflow.agentNode.roleResearcher"),
      planner: t("workflow.agentNode.rolePlanner"),
      developer: t("workflow.agentNode.roleDeveloper"),
      reviewer: t("workflow.agentNode.roleReviewer"),
      synthesizer: t("workflow.agentNode.roleSynthesizer"),
      executor: t("workflow.agentNode.roleExecutor"),
    };
    return labels[role] || role;
  };

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

  const agentRole = data.agentRole || "developer";
  const tools = data.tools || [];
  const contextSources = data.contextSources || [];

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
          background: "#1e1e1e",
          border: `2px solid ${borderColor}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${borderColor}40` : "none",
          transition: "all 0.2s",
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
          <span style={{ fontSize: 14 }}>
            {data.expertRoleId ? (data.expertIcon || "\uD83E\uDD16") : getRoleIcon(agentRole)}
          </span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            {data.expertRoleId
              ? (data.expertName || t("workflow.agentNode.expert"))
              : `Agent · ${getRoleLabel(agentRole)}`}
          </span>
          {data.model && (
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: `${color}30`,
                border: "none",
                color: "#fff",
              }}
            >
              {data.model.length > 12 ? `${data.model.slice(0, 12)}...` : data.model}
            </Tag>
          )}
        </div>

        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: "#fff",
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
                fontSize: 10,
                color: "#888",
                marginBottom: 8,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {data.systemPrompt.slice(0, 40)}...
            </div>
          )}

          <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}>
            {tools.length > 0 && (
              <Badge
                count={tools.length}
                size="small"
                style={{
                  backgroundColor: "#52c41a",
                  fontSize: 9,
                }}
              >
                <Tag
                  style={{
                    margin: 0,
                    fontSize: 9,
                    padding: "0 4px",
                    background: "#52c41a20",
                    border: "1px solid #52c41a50",
                    color: "#52c41a",
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
                  backgroundColor: "#13c2c2",
                  fontSize: 9,
                }}
              >
                <Tag
                  style={{
                    margin: 0,
                    fontSize: 9,
                    padding: "0 4px",
                    background: "#13c2c220",
                    border: "1px solid #13c2c250",
                    color: "#13c2c2",
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
                  border: "1px solid ${color}50",
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
