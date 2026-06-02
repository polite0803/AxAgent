import { Divider, Input, InputNumber, message, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { DatabaseQueryNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const DatabaseQueryPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const [messageApi, contextHolder] = message.useMessage();
  const dq = node as unknown as DatabaseQueryNode;
  const config = dq.config || { query: "", params: [], connection_name: "", timeout_secs: 30, output_var: "" };

  const setCfg = (key: string, val: unknown) => onUpdate({ config: { ...config, [key]: val } });

  const handleAIGenerateSQL = async () => {
    const current = config.query || "";
    const hint = current.trim() || t("workflow.aiAssist.dbQuery.sqlHint");
    const result = await aiGenerate({
      systemPrompt: "你是一个 SQL 专家。根据用户自然语言描述输出标准 SQL 语句，使用 ? 作为占位符。"
        + "只输出 SQL 本身，不要任何解释、Markdown 标记或分号。",
      userPrompt: hint,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    const cleaned = result.split(";")[0].trim().replace(/^```\w*\s*|\s*```$/g, "");
    setCfg("query", cleaned);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {contextHolder}
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.connection")}
        </label>
        <Input
          value={config.connection_name ?? ""}
          onChange={(e) => setCfg("connection_name", e.target.value || undefined)}
          size="small"
          placeholder={t("workflow.props.defaultConnection")}
        />
      </div>
      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.query")}
          </label>
          <AIAssistButton
            labelKey="generate"
            loading={aiGenerating}
            onClick={handleAIGenerateSQL}
            compact
          />
        </div>
        <Input.TextArea
          value={config.query}
          onChange={(e) => setCfg("query", e.target.value)}
          rows={5}
          size="small"
          placeholder={t("workflow.props.queryPlaceholder")}
          style={{ fontFamily: "monospace", fontSize: 11 }}
        />
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.timeout")}
        </label>
        <InputNumber
          value={config.timeout_secs}
          onChange={(v) => setCfg("timeout_secs", v ?? 30)}
          size="small"
          style={{ width: "100%" }}
          min={1}
          max={300}
          addonAfter="s"
        />
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.outputVariable")}
        </label>
        <Input value={config.output_var} onChange={(e) => setCfg("output_var", e.target.value)} size="small" />
      </div>
      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
