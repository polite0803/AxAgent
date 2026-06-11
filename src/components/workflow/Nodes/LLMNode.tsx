// SPDX-License-Identifier: AGPL-3.0-only
// @ts-nocheck

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { Tag, theme } from "antd";
import React, { memo } from "react";

const CYAN_BASE = "#13c2c2";
const CYAN_VAR = `var(--cyan, ${CYAN_BASE})`;

interface LLMNodeData extends Record<string, unknown> {
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

const LLMNodeComponent: React.FC<NodeProps> = ({
  data,
  selected,
}) => {
  const { token } = theme.useToken();
  const color = CYAN_VAR;

  const getModelIcon = (model: string): string => {
    if (!model) {
      return "🤖";
    }
    const lowerModel = model.toLowerCase();
    if (lowerModel.includes("gpt") || lowerModel.includes("openai")) {
      return "🤖";
    }
    if (lowerModel.includes("claude")) {
      return "🧠";
    }
    if (lowerModel.includes("gemini")) {
      return "✨";
    }
    if (lowerModel.includes("llama")) {
      return "🦙";
    }
    if (lowerModel.includes("mistral")) {
      return "🌬️";
    }
    if (lowerModel.includes("qwen")) {
      return "🔮";
    }
    if (lowerModel.includes("deepseek")) {
      return "🔍";
    }
    return "🤖";
  };

  const formatTemperature = (temp: number | undefined): string => {
    if (temp === undefined) {
      return "";
    }
    return temp.toFixed(1);
  };

  const formatMaxTokens = (tokens: number | undefined): string => {
    if (!tokens) {
      return "";
    }
    if (tokens >= 1000) {
      return `${(tokens / 1000).toFixed(0)}k`;
    }
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
          background: token.colorBgElevated,
          border: `2px solid ${selected ? token.colorPrimary : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${CYAN_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${CYAN_BASE}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>{getModelIcon(data.model || "")}</span>
          <span
            style={{
              fontSize: 12,
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
                background: `${CYAN_BASE}30`,
                border: "none",
                color: token.colorText,
              }}
            >
              {data.model.length > 15
                ? `${data.model.slice(0, 15)}...`
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

          {data.prompt && (
            <div
              style={{
                fontSize: 12,
                color: token.colorTextTertiary,
                marginBottom: 8,
                padding: 6,
                background: token.colorFillQuaternary,
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
                  background: token.colorFillQuaternary,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  color: token.colorTextQuaternary,
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
                  background: token.colorFillQuaternary,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  color: token.colorTextQuaternary,
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
                  background: `${token.colorSuccess}20`,
                  border: `1px solid ${token.colorSuccess}50`,
                  color: token.colorSuccess,
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
