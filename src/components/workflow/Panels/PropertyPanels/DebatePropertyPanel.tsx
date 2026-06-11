// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, InputNumber, Select, Tag, theme } from "antd";
import { Plus, Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { DebateNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface Props {
  node: WorkflowNode;
  onUpdate: (u: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

const MODEL_ROLE_OPTIONS = [
  { value: "", label: i18n.t("workflow.debateNode.defaultRole") },
  { value: "quick_think", label: "Quick Think" },
  { value: "deep_think", label: "Deep Think" },
];

export const DebatePropertyPanel: React.FC<Props> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const n = node as unknown as DebateNode;
  const c = n.config || {
    debater_steps: [],
    max_rounds: 2,
    topic_var: "topic",
    output_var: "debate_result",
  };
  const sc = (k: string, v: unknown) => onUpdate({ config: { ...c, [k]: v } });

  const allNodes = useWorkflowEditorStore((s) => s.nodes);
  const addNode = useWorkflowEditorStore((s) => s.addNode);
  const setParentRef = useWorkflowEditorStore((s) => s.setParentRef);
  const debaterSteps: string[] = c.debater_steps || [];

  const childNodes = debaterSteps
    .map((id) => allNodes.find((n) => n.id === id))
    .filter(Boolean) as WorkflowNode[];

  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();

  const handleAIOptimizeConvergence = async () => {
    const current = c.convergence_prompt || "";
    const result = await aiGenerate({
      systemPrompt: "你是一个辩论收敛提示词优化专家。改进用户提供的 convergence_prompt，"
        + "使收敛判断更精准、输出格式更可控。只输出优化后的提示词正文，不要解释或前缀。",
      userPrompt: current || "请生成一个通用的辩论收敛提示词，用于判断多轮辩论是否达成共识并总结结论。",
    });
    if (result) {
      sc("convergence_prompt", result);
    }
  };

  const handleAIAddDebater = async () => {
    const topicDesc = c.topic_var || "general";
    const existingCount = debaterSteps.length;
    const stance = existingCount === 0
      ? t("workflow.debateNode.stancePro")
      : existingCount === 1
      ? t("workflow.debateNode.stanceCon")
      : t("workflow.debateNode.stanceNth", { count: existingCount + 1 });
    const result = await aiGenerate({
      systemPrompt: "你是一个辩论 Agent 设计专家。根据辩论主题，生成一个辩手的 system_prompt。"
        + "只输出 system_prompt 正文，不要解释、不要 Markdown 标记、不要前缀。",
      userPrompt: `辩论主题变量: ${topicDesc}\n辩手立场: ${stance}\n请为该辩手生成一段专业、有针对性的 system_prompt。`,
    });
    const id = `node-${crypto.randomUUID()}`;
    const position = { x: 50 + existingCount * 30, y: 80 + existingCount * 80 };
    const newNode: WorkflowNode = {
      id,
      type: "agent",
      title: `${stance} Debater`,
      position,
      config: {
        system_prompt: result || t("workflow.debateNode.stanceSystemPromptFallback", { stance }),
        context_sources: [],
        output_var: `${id}_output`,
        tools: [],
        exposed_tools: [],
        output_mode: "text",
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as unknown as WorkflowNode;
    addNode(newNode);
    setParentRef(id, n.id);
    sc("debater_steps", [...debaterSteps, id]);
  };

  const addDebater = () => {
    const id = `node-${crypto.randomUUID()}`;
    const position = { x: 50 + debaterSteps.length * 30, y: 80 + debaterSteps.length * 80 };
    const newNode: WorkflowNode = {
      id,
      type: "agent",
      title: t("workflow.debateNode.newDebater", { defaultValue: "Debater" }) + ` ${debaterSteps.length + 1}`,
      position,
      config: {
        system_prompt: "",
        context_sources: [],
        output_var: `${id}_output`,
        tools: [],
        exposed_tools: [],
        output_mode: "text",
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as unknown as WorkflowNode;
    addNode(newNode);
    setParentRef(id, n.id);
    sc("debater_steps", [...debaterSteps, id]);
  };

  const removeDebater = (stepId: string) => {
    const updated = debaterSteps.filter((id) => id !== stepId);
    sc("debater_steps", updated);
    setParentRef(stepId, null);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.nodeConfig.topic_var", { defaultValue: "Topic Variable" })}
        </label>
        <Input
          value={c.topic_var}
          onChange={(e) => sc("topic_var", e.target.value)}
          size="small"
        />
      </div>
      <div>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.nodeConfig.max_rounds", { defaultValue: "Max Rounds" })}
        </label>
        <InputNumber
          value={c.max_rounds}
          onChange={(v) => sc("max_rounds", v ?? 2)}
          size="small"
          min={1}
          max={20}
          style={{ width: "100%" }}
        />
      </div>
      <Divider style={{ margin: "8px 0" }}>
        {t("workflow.nodeConfig.debaters", { defaultValue: "Debaters (child Agent nodes)" })}
      </Divider>
      {childNodes.map((child) => (
        <div
          key={child.id}
          style={{
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: 6,
            padding: 8,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 8,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6, flex: 1, minWidth: 0 }}>
            <Tag color="blue" style={{ margin: 0, fontSize: 11, flexShrink: 0 }}>
              Agent
            </Tag>
            <span style={{ fontSize: 12, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {child.title}
            </span>
          </div>
          <Button
            type="text"
            size="small"
            icon={<Trash2 size={14} />}
            onClick={() => removeDebater(child.id)}
            danger
          />
        </div>
      ))}
      {debaterSteps.length === 0 && (
        <div style={{ fontSize: 11, color: token.colorTextTertiary, textAlign: "center" }}>
          {t("workflow.debateNode.noDebatersHint", { defaultValue: "Add debater Agent nodes inside this container" })}
        </div>
      )}
      <div style={{ display: "flex", gap: 8 }}>
        <Button
          type="dashed"
          size="small"
          icon={<Plus size={14} />}
          onClick={addDebater}
          style={{ flex: 1 }}
        >
          {t("workflow.nodeConfig.add_debater", { defaultValue: "Add Debater" })}
        </Button>
        <AIAssistButton
          labelKey="generate"
          loading={aiGenerating}
          onClick={handleAIAddDebater}
          compact
        />
      </div>
      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.nodeConfig.convergence_prompt", { defaultValue: "Convergence Prompt (optional)" })}
          </label>
          <AIAssistButton
            labelKey="optimize"
            loading={aiGenerating}
            onClick={handleAIOptimizeConvergence}
            compact
          />
        </div>
        <Input.TextArea
          value={c.convergence_prompt || ""}
          onChange={(e) => sc("convergence_prompt", e.target.value || undefined)}
          placeholder={t("workflow.nodeConfig.convergence_prompt_placeholder", {
            defaultValue: "LLM prompt to judge if debate has converged",
          })}
          size="small"
          rows={2}
        />
      </div>
      <div>
        <label style={{ display: "block", color: token.colorTextTertiary, fontSize: 12, marginBottom: 4 }}>
          {t("workflow.nodeConfig.convergence_model_role", { defaultValue: "Convergence Model Role" })}
        </label>
        <Select
          value={c.convergence_model_role || ""}
          onChange={(v) => sc("convergence_model_role", v || undefined)}
          size="small"
          options={MODEL_ROLE_OPTIONS}
          style={{ width: "100%" }}
        />
      </div>
      <Divider style={{ margin: "8px 0" }} />
      <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
    </div>
  );
};
