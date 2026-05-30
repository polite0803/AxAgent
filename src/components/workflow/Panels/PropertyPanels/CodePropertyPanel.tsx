import { MonacoEditor } from "@/components/shared/MonacoEditor";
import { invoke } from "@/lib/invoke";
import { useLocalToolStore, useProviderStore, useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Dropdown, Input, Select, Tag, theme, Tooltip } from "antd";
import { Sparkles, Wand2, Wrench } from "lucide-react";
import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
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
  { value: "rhai", label: "🦀 Rhai (动态工具)" },
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

/** Rhai 代码模板 */
const RHAI_TEMPLATES = [
  {
    key: "filter",
    label: "数据过滤器",
    code:
      "// 过滤数据\nfn main(data, threshold) {\n    let filtered = data.filter(|x| x.score > threshold);\n    filtered\n}",
  },
  {
    key: "transform",
    label: "数据转换",
    code:
      "// 转换数据格式\nfn main(input) {\n    let result = #{};\n    for item in input {\n        result[item.id] = item.value;\n    }\n    result\n}",
  },
  {
    key: "aggregate",
    label: "聚合计算",
    code:
      "// 聚合统计\nfn main(data) {\n    let total = 0.0;\n    let count = 0;\n    for item in data {\n        total += item.value;\n        count += 1;\n    }\n    #{ total: total, avg: total / count, count: count }\n}",
  },
  {
    key: "tool_chain",
    label: "工具链调用",
    code:
      '// 链式调用多个工具\nfn main(input) {\n    let data = tool("web_fetch", #{ url: input.url });\n    let result = tool("code_analyzer", #{ data: data });\n    result\n}',
  },
];

export const CodePropertyPanel: React.FC<CodePropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const codeNode = node as CodeNode;
  const config = codeNode.config || {
    language: "javascript",
    code: "",
    output_var: "",
  };

  const [aiGenerating, setAiGenerating] = useState(false);
  const [showTools, setShowTools] = useState(false);
  const { groups: toolGroups } = useLocalToolStore();
  const { nodes } = useWorkflowEditorStore();
  const { providers } = useProviderStore();

  // 收集工作流中的 Rhai 工具和已注册工具名
  const availableTools = useMemo(() => {
    const tools: { name: string; source: string }[] = [];
    // Rhai 代码节点
    for (const n of nodes) {
      if (n.type === "code" && (n as CodeNode).config?.language === "rhai") {
        const cn = n as CodeNode;
        const toolName = cn.config.tool_name ?? `code_${n.id}`;
        if (n.id !== codeNode.id) {
          tools.push({ name: toolName, source: "Rhai 脚本" });
        }
      }
    }
    // 全局工具
    for (const g of toolGroups) {
      if (!g.enabled) { continue; }
      for (const t of g.tools) {
        tools.push({ name: t.name, source: g.groupName });
      }
    }
    return tools;
  }, [nodes, toolGroups, codeNode.id]);

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const insertCode = (snippet: string) => {
    const newCode = (config.code ?? "") + snippet;
    handleConfigChange("code", newCode);
  };

  const insertToolCall = (toolName: string) => {
    if (config.language === "rhai") {
      insertCode(`\ntool("${toolName}", #{ /* args */ })`);
    } else {
      insertCode(`\n// tool: ${toolName}`);
    }
  };

  const handleAiGenerate = async () => {
    setAiGenerating(true);
    try {
      const provider = providers.find((p) => p.enabled && p.models.some((m) => m.enabled));
      if (!provider) {
        setAiGenerating(false);
        return;
      }
      const model = provider.models.find((m) => m.enabled);
      if (!model) {
        setAiGenerating(false);
        return;
      }

      const lang = config.language;
      const existingCode = config.code ?? "";
      const toolList = availableTools.map((t) => t.name).join(", ");

      const prompt = existingCode
        ? `优化以下 ${lang} 代码。可用的工具: ${toolList}。只输出代码不要解释。\n\n${existingCode}`
        : `写一段 ${lang} 代码。可用的工具: ${toolList}。只输出代码不要解释。`;

      const result = await invoke<{ content: string }>("send_message", {
        params: {
          conversationId: "",
          content: prompt,
          attachments: [],
          options: {},
        },
      });

      if (result?.content) {
        const codeMatch = result.content.match(/```[\w]*\n([\s\S]*?)```/);
        handleConfigChange("code", codeMatch ? codeMatch[1].trim() : result.content.trim());
      }
    } catch {
      // AI 生成失败，静默处理
    } finally {
      setAiGenerating(false);
    }
  };

  const getDefaultCode = (language: string): string => {
    switch (language) {
      case "javascript":
      case "typescript":
        return `// ${t("workflow.props.codeInputParams")}\n// ${
          t("workflow.props.codeReturnHint")
        }\n\nconst result = input;\nreturn result;`;
      case "python":
        return `${t("workflow.props.defaultCodePythonComment")}\n\nresult = input\nreturn result`;
      case "rhai":
        return '// Rhai 脚本作为动态工具\n// 参数通过 scope 注入\n// 支持 tool("name", args) 调用其他工具\n\nfn main(input) {\n    let result = input;\n    result\n}';
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

  const isRhai = config.language === "rhai";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {/* 语言 + AI 按钮 */}
      <div style={{ display: "flex", gap: 8, alignItems: "flex-end" }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.language")}
          </label>
          <Select
            value={config.language}
            onChange={handleLanguageChange}
            size="small"
            style={{ width: "100%" }}
            showSearch
            options={LANGUAGE_OPTIONS}
          />
        </div>
        <Tooltip title={t("workflow.props.aiGenerate")}>
          <Button
            size="small"
            icon={<Sparkles size={14} />}
            loading={aiGenerating}
            onClick={handleAiGenerate}
            type={aiGenerating ? "primary" : "default"}
          />
        </Tooltip>
        <Dropdown
          menu={{
            items: RHAI_TEMPLATES.map((tpl) => ({
              key: tpl.key,
              label: tpl.label,
              onClick: () => insertCode(tpl.code),
            })),
          }}
          trigger={["click"]}
        >
          <Tooltip title={t("workflow.props.codeTemplates")}>
            <Button size="small" icon={<Wand2 size={14} />} />
          </Tooltip>
        </Dropdown>
        <Tooltip title={showTools ? t("workflow.props.hideTools") : t("workflow.props.showTools")}>
          <Button
            size="small"
            type={showTools ? "primary" : "default"}
            icon={<Wrench size={14} />}
            onClick={() => setShowTools(!showTools)}
          />
        </Tooltip>
      </div>

      {/* 工具浏览器 */}
      {showTools && (
        <div
          style={{
            padding: 8,
            borderRadius: 6,
            border: `1px solid ${token.colorBorderSecondary}`,
            background: token.colorBgElevated,
            maxHeight: 160,
            overflow: "auto",
          }}
        >
          <div style={{ fontSize: 11, color: token.colorTextQuaternary, marginBottom: 6 }}>
            {t("workflow.props.availableTools", { count: availableTools.length })}
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {availableTools.map((tool) => (
              <Tag
                key={tool.name}
                style={{ cursor: "pointer", fontSize: 11 }}
                onClick={() => insertToolCall(tool.name)}
                color={tool.source === "Rhai 脚本" ? "purple" : "blue"}
              >
                <Wrench size={10} style={{ marginRight: 3 }} />
                {tool.name}
                <span style={{ opacity: 0.5, fontSize: 10, marginLeft: 3 }}>
                  ({tool.source})
                </span>
              </Tag>
            ))}
            {availableTools.length === 0 && (
              <span style={{ fontSize: 11, color: token.colorTextQuaternary }}>
                {t("workflow.props.noToolsAvailable")}
              </span>
            )}
          </div>
        </div>
      )}

      {/* 代码编辑区 */}
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.code")}
        </label>
        <div
          style={{
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: 6,
            overflow: "hidden",
            minHeight: 200,
          }}
        >
          <MonacoEditor
            value={config.code || ""}
            language={(config.language === "rhai" ? "rust" : config.language) as any}
            onChange={(v) => handleConfigChange("code", v || "")}
            height="300px"
          />
        </div>
      </div>

      {/* 输出变量 + tool_name */}
      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
            {t("workflow.props.outputVariable")}
          </label>
          <Input
            id="code-property-panel-input-86"
            value={config.output_var || ""}
            onChange={(e) => handleConfigChange("output_var", e.target.value)}
            size="small"
            placeholder={t("workflow.props.outputVarDefault")}
          />
        </div>
        {isRhai && (
          <div style={{ flex: 1 }}>
            <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
              {t("workflow.props.toolName")}
            </label>
            <Input
              value={config.tool_name ?? ""}
              onChange={(e) => handleConfigChange("tool_name", e.target.value || undefined)}
              size="small"
              placeholder="code_<id>"
            />
          </div>
        )}
      </div>

      <Divider style={{ margin: "4px 0", borderColor: token.colorBorderSecondary }} />

      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
