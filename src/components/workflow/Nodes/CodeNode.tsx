import { Tag } from "antd";
import React, { memo } from "react";
import { Handle, type NodeProps, Position } from "reactflow";

interface CodeNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  language?: string;
  code?: string;
  outputVar?: string;
}

const CodeNodeComponent: React.FC<NodeProps<CodeNodeData>> = ({ data, selected }) => {
  const color = "#52c41a";
  const language = data.language || "javascript";
  const code = data.code || "";
  const outputVar = data.outputVar;

  const getLanguageIcon = (lang: string): string => {
    const icons: Record<string, string> = {
      javascript: "🟨",
      typescript: "🔷",
      python: "🐍",
      java: "☕",
      go: "🔵",
      rust: "🦀",
      php: "🐘",
      ruby: "💎",
      swift: "🍎",
      kotlin: "🟣",
      csharp: "🟩",
      cpp: "🔴",
      c: "⚪",
      html: "🌐",
      css: "🎨",
      sql: "🗃️",
      bash: "📟",
      shell: "📟",
      powershell: "📟",
    };
    return icons[lang.toLowerCase()] || "📝";
  };

  const getLanguageColor = (lang: string): string => {
    const colors: Record<string, string> = {
      javascript: "#f7df1e",
      typescript: "#3178c6",
      python: "#3776ab",
      java: "#007396",
      go: "#00add8",
      rust: "#ce422b",
      php: "#777bb4",
      ruby: "#cc342d",
      swift: "#fa7343",
      kotlin: "#7f52ff",
      csharp: "#239120",
      cpp: "#00599c",
      c: "#a8b9cc",
      html: "#e34f26",
      css: "#1572b6",
      sql: "#4479a1",
      bash: "#4eaa25",
      shell: "#89e051",
      powershell: "#5391fe",
    };
    return colors[lang.toLowerCase()] || "#888";
  };

  const lineCount = code.split("\n").length;

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
          <span style={{ fontSize: 14 }}>{getLanguageIcon(language)}</span>
          <span
            style={{
              fontSize: 11,
              color: getLanguageColor(language),
              fontWeight: 600,
            }}
          >
            {language.toUpperCase()}
          </span>
          {lineCount > 0 && lineCount <= 100 && (
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
              {lineCount} 行
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

          {code && (
            <div
              style={{
                fontSize: 9,
                color: "#666",
                fontFamily: "monospace",
                marginBottom: 6,
                padding: "4px 6px",
                background: "#252525",
                borderRadius: 4,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {code.slice(0, 60).replace(/\n/g, " ")}...
            </div>
          )}

          {outputVar && (
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: "#1890ff20",
                border: "1px solid #1890ff50",
                color: "#1890ff",
              }}
            >
              📤 {outputVar}
            </Tag>
          )}
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

export const CodeNode = memo(CodeNodeComponent);
