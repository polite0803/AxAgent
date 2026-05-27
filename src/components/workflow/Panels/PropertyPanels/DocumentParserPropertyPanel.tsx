import { Divider, Input, Select, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
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

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
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
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.outputVariable")}
        </label>
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
