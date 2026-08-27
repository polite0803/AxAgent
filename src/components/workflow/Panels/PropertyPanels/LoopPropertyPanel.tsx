// SPDX-License-Identifier: AGPL-3.0-only

import { message } from "@/lib/toast";
import { useWorkflowEditorStore } from "@/stores";
import { Divider, Input, InputNumber, Select, Switch, Tag, theme } from "antd";
import { X } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { LoopNode, LoopNodeConfig, LoopType, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

const LOOP_TYPE_VALUES: readonly LoopType[] = ["forEach", "while", "doWhile", "until"];

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
  // 兼容旧版数据：config 可能整体缺失、缺字段，或 bodySteps 非数组（历史 schema 无该字段）。
  // 归一化后再使用，避免渲染属性面板时对 undefined 调 .map/.includes/.filter 抛 TypeError
  // 导致整页进入错误边界（"页面错误"）。
  const rawConfig = (loopNode.config ?? {}) as Partial<LoopNodeConfig>;
  const config: LoopNodeConfig = {
    loopType: "forEach",
    itemsVar: "",
    iterateeVar: "",
    maxIterations: 100,
    continueOnError: false,
    bodySteps: [],
    ...rawConfig,
  };
  if (!Array.isArray(config.bodySteps)) {
    config.bodySteps = [];
  }
  if (!LOOP_TYPE_VALUES.includes(config.loopType)) {
    // 旧 schema 的 "count"/"condition" 等遗留值归一化为 forEach，保证 Select 有匹配选项
    config.loopType = "forEach";
  }

  const { nodes } = useWorkflowEditorStore();

  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const [messageApi, contextHolder] = message.useMessage();

  const handleAIGenerateContinueCondition = async () => {
    const result = await aiGenerate({
      systemPrompt:
        "你是一个循环控制专家。根据用户的自然语言描述，输出一个布尔表达式字符串作为循环的 continueCondition 条件（如：'i < 10'、'${item}.status === \"active\"'）。"
        + "只输出表达式字符串本身，不要任何解释或 Markdown 标记。",
      userPrompt: config.continueCondition || t("workflow.aiAssist.loop.continueHint", { items: config.itemsVar }),
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    handleConfigChange("continueCondition", result.split("\n")[0].trim());
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAddStep = (nodeId: string) => {
    if (!config.bodySteps.includes(nodeId)) {
      handleConfigChange("bodySteps", [...config.bodySteps, nodeId]);
    }
  };

  const handleRemoveStep = (nodeId: string) => {
    handleConfigChange(
      "bodySteps",
      config.bodySteps.filter((id) => id !== nodeId),
    );
  };

  const availableNodes = nodes.filter(
    (n) => n.id !== node.id && !config.bodySteps.includes(n.id),
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
          value={config.loopType}
          onChange={(value) => handleConfigChange("loopType", value)}
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

      {config.loopType === "forEach" && (
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
              value={config.itemsVar || ""}
              onChange={(e) => handleConfigChange("itemsVar", e.target.value)}
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
              value={config.iterateeVar || ""}
              onChange={(e) => handleConfigChange("iterateeVar", e.target.value)}
              size="small"
              placeholder={t("workflow.props.iterateVarExample")}
            />
          </div>
        </>
      )}

      {config.loopType === "while" && (
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
            value={config.continueCondition || ""}
            onChange={(e) => handleConfigChange("continueCondition", e.target.value)}
            rows={2}
            size="small"
            placeholder={t("workflow.props.continueConditionExample")}
          />
        </div>
      )}

      {config.loopType === "until" && (
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
            value={config.continueCondition || ""}
            onChange={(e) => handleConfigChange("continueCondition", e.target.value)}
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
          value={config.maxIterations ?? 100}
          onChange={(value) => handleConfigChange("maxIterations", value)}
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
          checked={config.continueOnError ?? false}
          onChange={(checked) => handleConfigChange("continueOnError", checked)}
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
            count: config.bodySteps?.length || 0,
          })}
        </label>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {config.bodySteps.map((stepId) => (
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
          {config.bodySteps.length === 0 && (
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
