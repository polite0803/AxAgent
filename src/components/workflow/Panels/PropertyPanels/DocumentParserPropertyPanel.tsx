import { Divider, Input, message, Select, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { DocumentParserNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface DocumentParserPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const DocumentParserPropertyPanel: React.FC<
  DocumentParserPropertyPanelProps
> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const documentParserNode = node as DocumentParserNode;
  const config = documentParserNode.config || {
    input_var: "",
    parser_type: "text",
    output_var: "",
  };

  const PARSER_TYPE_OPTIONS = [
    { value: "pdf", label: "📄 PDF" },
    { value: "markdown", label: "📝 Markdown" },
    { value: "html", label: "🌐 HTML" },
    { value: "json", label: "{} JSON" },
    { value: "xml", label: "📋 XML" },
    { value: "csv", label: "📊 CSV" },
    { value: "text", label: t("workflow.props.plainText") },
  ];

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const [messageApi, messageContextHolder] = message.useMessage();
  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const handleAISuggestOutputVar = async () => {
    if (!config.input_var) {
      messageApi.warning(t("workflow.aiAssist.failed"));
      return;
    }
    const result = await aiGenerate({
      systemPrompt: "你是一名变量命名助手。基于 input_var 名 + 解析器类型，生成一个简洁的 snake_case 输出变量名。"
        + "只输出变量名本身，不要任何前缀、解释或 Markdown 标记。",
      userPrompt: `input_var: ${config.input_var}\nparser_type: ${config.parser_type}`,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    const cleaned = result.replace(/[^A-Za-z0-9_]/g, "").toLowerCase();
    if (!cleaned) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    handleConfigChange("output_var", cleaned);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {messageContextHolder}
      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.inputVariable")}
        </label>
        <Input
          id="document-parser-property-panel-input-93"
          value={config.input_var || ""}
          onChange={(e) => handleConfigChange("input_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.inputVarDocument")}
        />
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.parserType")}
        </label>
        <Select
          value={config.parser_type}
          onChange={(value) => handleConfigChange("parser_type", value)}
          size="small"
          style={{ width: "100%" }}
          options={PARSER_TYPE_OPTIONS}
        />
      </div>

      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 4,
          }}
        >
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.outputVariable")}
          </label>
          <AIAssistButton
            labelKey="suggest"
            loading={aiGenerating}
            onClick={handleAISuggestOutputVar}
            compact
          />
        </div>
        <Input
          id="document-parser-property-panel-input-94"
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.outputVarParsed")}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      <div
        style={{ borderTop: `1px solid ${token.colorBorderSecondary}`, paddingTop: 12, marginTop: 4 }}
      >
        <BasePropertyPanel
          node={node}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      </div>
    </div>
  );
};
