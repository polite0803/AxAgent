// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkflowEditorStore } from "@/stores";
import { Divider, Input, InputNumber, message, Select, Switch, Tag, theme } from "antd";
import { X } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { LoopNode, LoopType, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface LoopPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const LoopPropertyPanel: React.FC<LoopPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const loopNode = node as LoopNode;
  const config = loopNode.config || {
    loop_type: "forEach" as LoopType,
    items_var: "",
    iteratee_var: "",
    max_iterations: 100,
    continue_on_error: false,
    body_steps: [],
  };

  const { nodes } = useWorkflowEditorStore();

  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const [messageApi, contextHolder] = message.useMessage();

  const handleAIGenerateContinueCondition = async () => {
    const result = await aiGenerate({
      systemPrompt:
        "你是一个循环控制专家。根据用户的自然语言描述，输出一个布尔表达式字符串作为循环的 continue_condition 条件（如：'i < 10'、'${item}.status === \"active\"'）。"
        + "只输出表达式字符串本身，不要任何解释或 Markdown 标记。",
      userPrompt: config.continue_condition || t("workflow.aiAssist.loop.continueHint", { items: config.items_var }),
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    handleConfigChange("continue_condition", result.split("\n")[0].trim());
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAddStep = (nodeId: string) => {
    if (!config.body_steps.includes(nodeId)) {
      handleConfigChange("body_steps", [...config.body_steps, nodeId]);
    }
  };

  const handleRemoveStep = (nodeId: string) => {
    handleConfigChange(
      "body_steps",
      config.body_steps.filter((id) => id !== nodeId),
    );
  };

  const availableNodes = nodes.filter(
    (n) => n.id !== node.id && !config.body_steps.includes(n.id),
  );

  const getNodeLabel = (nodeId: string) => {
    const found = nodes.find((n) => n.id === nodeId);
    return found ? `${found.title || found.id} (${found.type})` : nodeId;
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {contextHolder}
      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.loopType")}
        </label>
        <Select
          value={config.loop_type}
          onChange={(value) => handleConfigChange("loop_type", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "forEach", label: t("workflow.props.loopForEach") },
            { value: "while", label: t("workflow.props.loopWhile") },
            { value: "doWhile", label: t("workflow.props.loopDoWhile") },
            { value: "until", label: t("workflow.props.loopUntil") },
          ]}
        />
      </div>

      {config.loop_type === "forEach" && (
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
              {t("workflow.props.arrayVar")}
            </label>
            <Input
              id="loop-property-panel-input-100"
              value={config.items_var || ""}
              onChange={(e) => handleConfigChange("items_var", e.target.value)}
              size="small"
              placeholder={t("workflow.props.itemsVarExample")}
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
              {t("workflow.props.iterateVar")}
            </label>
            <Input
              id="loop-property-panel-input-101"
              value={config.iteratee_var || ""}
              onChange={(e) => handleConfigChange("iteratee_var", e.target.value)}
              size="small"
              placeholder={t("workflow.props.iterateVarExample")}
            />
          </div>
        </>
      )}

      {config.loop_type === "while" && (
        <div>
          <label
            style={{
              display: "block",
              color: token.colorTextTertiary,
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.continueCondition")}
          </label>
          <AIAssistButton
            labelKey="generate"
            loading={aiGenerating}
            onClick={handleAIGenerateContinueCondition}
            compact
          />
          <Input.TextArea
            id="loop-property-panel-input-textarea-102"
            value={config.continue_condition || ""}
            onChange={(e) => handleConfigChange("continue_condition", e.target.value)}
            rows={2}
            size="small"
            placeholder={t("workflow.props.continueConditionExample")}
          />
        </div>
      )}

      {config.loop_type === "until" && (
        <div>
          <label
            style={{
              display: "block",
              color: token.colorTextTertiary,
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.stopCondition")}
          </label>
          <Input.TextArea
            id="loop-property-panel-input-textarea-103"
            value={config.continue_condition || ""}
            onChange={(e) => handleConfigChange("continue_condition", e.target.value)}
            rows={2}
            size="small"
            placeholder={t("workflow.props.stopConditionExample")}
          />
        </div>
      )}

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.maxIterations")}
        </label>
        <InputNumber
          id="loop-property-panel-inputnumber-104"
          value={config.max_iterations ?? 100}
          onChange={(value) => handleConfigChange("max_iterations", value)}
          min={1}
          max={10000}
          size="small"
          style={{ width: "100%" }}
        />
      </div>

      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.props.continueOnError")}
        </label>
        <Switch
          size="small"
          checked={config.continue_on_error ?? false}
          onChange={(checked) => handleConfigChange("continue_on_error", checked)}
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
          {t("workflow.props.loopBodySteps", {
            count: config.body_steps?.length || 0,
          })}
        </label>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {config.body_steps.map((stepId) => (
            <Tag
              key={stepId}
              closable
              onClose={() => handleRemoveStep(stepId)}
              style={{
                background: token.colorFillQuaternary,
                border: "1px solid #444",
                color: token.colorTextQuaternary,
              }}
              closeIcon={<X size={10} />}
            >
              {getNodeLabel(stepId)}
            </Tag>
          ))}
          {config.body_steps.length === 0 && (
            <div style={{ color: token.colorTextTertiary, fontSize: 12 }}>
              {t("workflow.props.noLoopSteps")}
            </div>
          )}
        </div>
      </div>

      {availableNodes.length > 0 && (
        <div>
          <label
            style={{
              display: "block",
              color: token.colorTextTertiary,
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.addStep")}
          </label>
          <Select
            placeholder={t("workflow.props.selectNodeToAdd")}
            size="small"
            style={{ width: "100%" }}
            onChange={handleAddStep}
            options={availableNodes.map((n) => ({
              value: n.id,
              label: `${n.title || n.id} (${n.type})`,
            }))}
          />
        </div>
      )}

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
