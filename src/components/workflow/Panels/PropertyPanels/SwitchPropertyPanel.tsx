// SPDX-License-Identifier: AGPL-3.0-only

import { message } from "@/lib/toast";
import { Button, Divider, Input, Select, Switch, theme } from "antd";
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
  const sw = node as unknown as SwitchNode; // SAFE: WorkflowNode union narrowed to specific node type via config field access
  const config = sw.config || {
    inputVar: "",
    cases: [],
    defaultCase: "",
    matchMode: "exact" as const,
    outputVar: "",
    useLlm: false,
    llmPrompt: "",
    llmModel: "",
  };

  const setCfg = (key: string, val: unknown) => onUpdate({ config: { ...config, [key]: val } });

  const isExpressionMode = config.matchMode === "expression";
  const isLlmMode = config.useLlm === true;

  const handleAISuggestCases = async () => {
    const result = await aiGenerate({
      systemPrompt: t("workflow.aiAssist.switch.systemPrompt", {
        schema: '{"value": "string", "label": "string"}',
      }),
      userPrompt: t("workflow.aiAssist.switch.casesHint", {
        current: config.cases.length,
        input: config.inputVar,
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
          value={config.inputVar}
          onChange={(e) => setCfg("inputVar", e.target.value)}
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
          value={config.matchMode}
          onChange={(v) => setCfg("matchMode", v)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "exact", label: t("workflow.props.matchModeExact") },
            { value: "contains", label: t("workflow.props.matchModeContains") },
            { value: "regex", label: t("workflow.props.matchModeRegex") },
            {
              value: "expression",
              label: t("workflow.props.matchModeExpression"),
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
          {t("workflow.props.useLlmRouting")}
        </label>
        <Switch
          size="small"
          checked={config.useLlm === true}
          onChange={(checked) => setCfg("useLlm", checked || undefined)}
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
              {t("workflow.props.llmRoutingPrompt")}
            </label>
            <Input.TextArea
              value={config.llmPrompt || ""}
              onChange={(e) => setCfg("llmPrompt", e.target.value || undefined)}
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
              {t("workflow.props.llmRoutingModel")}
            </label>
            <Input
              value={config.llmModel || ""}
              onChange={(e) => setCfg("llmModel", e.target.value || undefined)}
              size="small"
              placeholder={t("workflow.props.defaultModel")}
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
                ? t("workflow.props.expressionPlaceholder")
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
                  config.cases.filter((_: unknown, j: number) => j !== i),
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
          {t("workflow.props.defaultCase")}
        </label>
        <Input
          value={config.defaultCase || ""}
          onChange={(e) => setCfg("defaultCase", e.target.value || undefined)}
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
        <Input value={config.outputVar} onChange={(e) => setCfg("outputVar", e.target.value)} size="small" />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
