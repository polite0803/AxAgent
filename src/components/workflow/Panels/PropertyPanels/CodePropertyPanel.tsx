import { Divider, Input, Select } from "antd";
import React from "react";
import type { CodeNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface CodePropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

const LANGUAGE_OPTIONS = [
  { value: "javascript", label: "🟨 JavaScript" },
  { value: "typescript", label: "🔷 TypeScript" },
  { value: "python", label: "🐍 Python" },
  { value: "java", label: "☕ Java" },
  { value: "go", label: "🔵 Go" },
  { value: "rust", label: "🦀 Rust" },
  { value: "php", label: "🐘 PHP" },
  { value: "ruby", label: "💎 Ruby" },
  { value: "swift", label: "🍎 Swift" },
  { value: "kotlin", label: "🟣 Kotlin" },
  { value: "csharp", label: "🟩 C#" },
  { value: "cpp", label: "🔴 C++" },
];

export const CodePropertyPanel: React.FC<CodePropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const codeNode = node as CodeNode;
  const config = codeNode.config || {
    language: "javascript",
    code: "",
    output_var: "",
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const getDefaultCode = (language: string): string => {
    switch (language) {
      case "javascript":
      case "typescript":
        return "// 输入参数: input\n// 返回值将存储到 output_var\n\nconst result = input;\nreturn result;";
      case "python":
        return "# 输入参数: input\n# 返回值将存储到 output_var\n\nresult = input\nreturn result";
      default:
        return `// ${language} code\n// Input: input\n// Output: output_var\n\n`;
    }
  };

  const handleLanguageChange = (language: string) => {
    const shouldUpdateCode = !config.code || config.code.includes("// Input:");
    onUpdate({
      config: {
        ...config,
        language,
        code: shouldUpdateCode ? getDefaultCode(language) : config.code,
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>语言</label>
        <Select
          value={config.language}
          onChange={handleLanguageChange}
          size="small"
          style={{ width: "100%" }}
          showSearch
          options={LANGUAGE_OPTIONS}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          代码
          <span style={{ color: "#666", fontWeight: 400, marginLeft: 4 }}>
            (输入: input, 输出: return)
          </span>
        </label>
        <Input.TextArea
          value={config.code || ""}
          onChange={(e) => handleConfigChange("code", e.target.value)}
          rows={8}
          size="small"
          style={{
            fontFamily: "Monaco, Consolas, monospace",
            fontSize: 11,
            background: "#1a1a1a",
          }}
          placeholder="输入你的代码..."
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>输出变量</label>
        <Input
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder="code_output"
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
