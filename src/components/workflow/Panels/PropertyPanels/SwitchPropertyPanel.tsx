import { Button, Divider, Input, message, Select, theme } from "antd";
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
  const config = sw.config || { input_var: "", cases: [], default_case: "", match_mode: "exact", output_var: "" };

  const setCfg = (key: string, val: unknown) => onUpdate({ config: { ...config, [key]: val } });

  const handleAISuggestCases = async () => {
    const result = await aiGenerate({
      systemPrompt:
        '你是一个工作流分支设计专家。根据用户的描述，输出 switch 节点的 cases 数组，每项为 {"value": "string", "label": "string"}。'
        + "只输出 JSON 数组字符串，不要任何解释或 Markdown 标记。",
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
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.inputVariable")}
        </label>
        <Input
          value={config.input_var}
          onChange={(e) => setCfg("input_var", e.target.value)}
          size="small"
          placeholder="node_id.output_field"
        />
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.matchMode")}
        </label>
        <Select
          value={config.match_mode}
          onChange={(v) => setCfg("match_mode", v)}
          size="small"
          style={{ width: "100%" }}
          options={[{ value: "exact", label: "Exact" }, { value: "contains", label: "Contains" }, {
            value: "startsWith",
            label: "Starts With",
          }]}
        />
      </div>
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
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
                setCfg(
                  "cases",
                  [...config.cases, { value: "", label: "Case " + (config.cases.length + 1) }],
                );
              }}
            >
              {t("workflow.props.addCase")}
            </Button>
          </div>
        </div>
        {config.cases.map((c, i) => (
          <div key={i} style={{ display: "flex", gap: 4, marginBottom: 4, alignItems: "center" }}>
            <span style={{ fontSize: 10, color: token.colorTextTertiary, minWidth: 20 }}>#{i + 1}</span>
            <Input
              size="small"
              style={{ width: 60 }}
              value={c.label}
              onChange={(e) => {
                const cases = [...config.cases];
                cases[i] = { ...cases[i], label: e.target.value };
                setCfg("cases", cases);
              }}
              placeholder="label"
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
              placeholder="value"
            />
            <Button
              type="text"
              danger
              size="small"
              icon={<Trash2 size={11} />}
              onClick={() => {
                setCfg(
                  "cases",
                  config.cases.filter((_: any, j: number) =>
                    j !== i
                  ),
                );
              }}
            />
          </div>
        ))}
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
