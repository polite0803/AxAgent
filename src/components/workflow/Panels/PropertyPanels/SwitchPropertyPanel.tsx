// SPDX-License-Identifier: AGPL-3.0-only

import { Button, Divider, Input, message, Select, Switch, theme } from "antd";
import { Plus, Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { SwitchNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const SwitchPropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const [messageApi, contextHolder] = message.useMessage();
  const sw = node as unknown as SwitchNode;
  const config = sw.config || {
    input_var: "",
    cases: [],
    default_case: "",
    match_mode: "exact" as const,
    output_var: "",
    use_llm: false,
    llm_prompt: "",
    llm_model: "",
  };

  const setCfg = (key: string, val: unknown) => onUpdate({ config: { ...config, [key]: val } });

  const isExpressionMode = config.match_mode === "expression";
  const isLlmMode = config.use_llm === true;

  const handleAISuggestCases = async () => {
    const result = await aiGenerate({
      systemPrompt: t("workflow.aiAssist.switch.systemPrompt", {
        schema: '{"value": "string", "label": "string"}',
      }),
      userPrompt: t("workflow.aiAssist.switch.casesHint", {
        current: config.cases.length,
        input: config.input_var,
      }),
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    try {
      const cleaned = result.replace(/^```\w*\s*|\s*```$/g, "").trim();
      const parsed = JSON.parse(cleaned) as Array<{ value: string; label: string }>;
      setCfg("cases", parsed);
      messageApi.success(t("workflow.aiAssist.applied"));
    } catch {
      messageApi.error(t("workflow.aiAssist.subWorkflow.parseFailed"));
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {contextHolder}
      <div>
        <label
          style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}
        >
          {t("workflow.props.inputVariable")}
        </label>
        <Input
          value={config.input_var}
          onChange={(e) => setCfg("input_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.switchInputVarPlaceholder")}
        />
      </div>

      {/* 匹配模式 */}
      <div>
        <label
          style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}
        >
          {t("workflow.props.matchMode")}
        </label>
        <Select
          value={config.match_mode}
          onChange={(v) => setCfg("match_mode", v)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "exact", label: t("workflow.props.matchModeExact", { defaultValue: "Exact" }) },
            { value: "contains", label: t("workflow.props.matchModeContains", { defaultValue: "Contains" }) },
            { value: "regex", label: t("workflow.props.matchModeRegex", { defaultValue: "Regex" }) },
            {
              value: "expression",
              label: t("workflow.props.matchModeExpression", { defaultValue: "Expression" }),
            },
          ]}
        />
        {isExpressionMode && (
          <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 4 }}>
            {t("workflow.props.expressionHint", {
              defaultValue: "Each case value is a Rhai expression. Use `_value` for input.",
            })}
          </div>
        )}
      </div>

      {/* LLM 智能路由 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.props.useLlmRouting", { defaultValue: "LLM Smart Routing" })}
        </label>
        <Switch
          size="small"
          checked={config.use_llm === true}
          onChange={(checked) => setCfg("use_llm", checked || undefined)}
        />
      </div>
      {isLlmMode && (
        <>
          <div>
            <label
              style={{
                display: "block",
                color: token.colorTextTertiary,
                fontSize: 12,
                marginBottom: 4,
              }}
            >
              {t("workflow.props.llmRoutingPrompt", { defaultValue: "Routing Prompt" })}
            </label>
            <Input.TextArea
              value={config.llm_prompt || ""}
              onChange={(e) => setCfg("llm_prompt", e.target.value || undefined)}
              size="small"
              rows={2}
              placeholder={t("workflow.props.llmRoutingPromptPlaceholder", {
                defaultValue: "Describe how to route inputs to cases...",
              })}
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
              {t("workflow.props.llmRoutingModel", { defaultValue: "Model (optional)" })}
            </label>
            <Input
              value={config.llm_model || ""}
              onChange={(e) => setCfg("llm_model", e.target.value || undefined)}
              size="small"
              placeholder={t("workflow.props.defaultModel", { defaultValue: "Use default model" })}
            />
          </div>
        </>
      )}

      {/* Cases 列表 */}
      <div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 4,
          }}
        >
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.cases")} ({config.cases.length})
          </label>
          <div style={{ display: "flex", gap: 4 }}>
            <AIAssistButton
              labelKey="suggest"
              loading={aiGenerating}
              onClick={handleAISuggestCases}
              compact
            />
            <Button
              type="link"
              size="small"
              icon={<Plus size={12} />}
              onClick={() => {
                const newCase = {
                  value: "",
                  label: t("workflow.caseLabel", {
                    n: config.cases.length + 1,
                    defaultValue: "Case " + (config.cases.length + 1),
                  }),
                };
                setCfg("cases", [...config.cases, newCase]);
              }}
            >
              {t("workflow.props.addCase")}
            </Button>
          </div>
        </div>
        {config.cases.map((c, i) => (
          <div
            key={c.value + "|" + i}
            style={{
              display: "flex",
              gap: 4,
              marginBottom: 4,
              alignItems: "center",
            }}
          >
            <span style={{ fontSize: 10, color: token.colorTextTertiary, minWidth: 20 }}>
              #{i + 1}
            </span>
            <Input
              size="small"
              style={{ width: 60 }}
              value={c.label}
              onChange={(e) => {
                const cases = [...config.cases];
                cases[i] = { ...cases[i], label: e.target.value };
                setCfg("cases", cases);
              }}
              placeholder={t("workflow.props.switchLabelPlaceholder")}
            />
            <Input
              size="small"
              style={{ flex: 1 }}
              value={c.value}
              onChange={(e) => {
                const cases = [...config.cases];
                cases[i] = { ...cases[i], value: e.target.value };
                setCfg("cases", cases);
              }}
              placeholder={isExpressionMode
                ? t("workflow.props.expressionPlaceholder", { defaultValue: "_value > 100" })
                : t("workflow.props.switchValuePlaceholder")}
            />
            <Button
              type="text"
              danger
              size="small"
              icon={<Trash2 size={11} />}
              onClick={() => {
                setCfg(
                  "cases",
                  // eslint-disable-next-line @typescript-eslint/no-explicit-any
                  config.cases.filter((_: any, j: number) => j !== i),
                );
              }}
            />
          </div>
        ))}
      </div>

      {/* 默认分支 */}
      <div>
        <label
          style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}
        >
          {t("workflow.props.defaultCase", { defaultValue: "Default case (fallback)" })}
        </label>
        <Input
          value={config.default_case || ""}
          onChange={(e) => setCfg("default_case", e.target.value || undefined)}
          size="small"
          placeholder={t("workflow.props.notSet")}
        />
      </div>

      <div>
        <label
          style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}
        >
          {t("workflow.props.outputVariable")}
        </label>
        <Input value={config.output_var} onChange={(e) => setCfg("output_var", e.target.value)} size="small" />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
