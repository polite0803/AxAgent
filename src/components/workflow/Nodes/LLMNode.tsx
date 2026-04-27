import { Tag } from "antd";
import React, { memo } from "react";
import { Handle, type NodeProps, Position } from "reactflow";

interface LLMNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  model?: string;
  prompt?: string;
  temperature?: number;
  maxTokens?: number;
  tools?: string[];
}

const LLMNodeComponent: React.FC<NodeProps<LLMNodeData>> = ({ data, selected }) => {
  const color = "#13c2c2";

  const getModelIcon = (model: string): string => {
    if (!model) { return "🤖"; }
    const lowerModel = model.toLowerCase();
    if (lowerModel.includes("gpt") || lowerModel.includes("openai")) { return "🤖"; }
    if (lowerModel.includes("claude")) { return "🧠"; }
    if (lowerModel.includes("gemini")) { return "✨"; }
    if (lowerModel.includes("llama")) { return "🦙"; }
    if (lowerModel.includes("mistral")) { return "🌬️"; }
    if (lowerModel.includes("qwen")) { return "🔮"; }
    if (lowerModel.includes("deepseek")) { return "🔍"; }
    return "🤖";
  };

  const formatTemperature = (temp: number | undefined): string => {
    if (temp === undefined) { return ""; }
    return temp.toFixed(1);
  };

  const formatMaxTokens = (tokens: number | undefined): string => {
    if (!tokens) { return ""; }
    if (tokens >= 1000) { return `${(tokens / 1000).toFixed(0)}k`; }
    return `${tokens}`;
  };

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
          border: `2px solid ${selected ? "#1890ff" : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
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
          <span style={{ fontSize: 14 }}>{getModelIcon(data.model || "")}</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            LLM
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
              {data.model.length > 15 ? `${data.model.slice(0, 15)}...` : data.model}
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

          {data.prompt && (
            <div
              style={{
                fontSize: 10,
                color: "#888",
                marginBottom: 8,
                padding: 6,
                background: "#2a2a2a",
                borderRadius: 4,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              💬 {data.prompt.slice(0, 50)}...
            </div>
          )}

          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {data.temperature !== undefined && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: "#2a2a2a",
                  border: "1px solid #444",
                  color: "#aaa",
                }}
              >
                🌡️ {formatTemperature(data.temperature)}
              </Tag>
            )}

            {data.maxTokens !== undefined && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: "#2a2a2a",
                  border: "1px solid #444",
                  color: "#aaa",
                }}
              >
                📏 {formatMaxTokens(data.maxTokens)}
              </Tag>
            )}

            {data.tools && data.tools.length > 0 && (
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
                🔧 {data.tools.length}
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

export const LLMNode = memo(LLMNodeComponent);
