import { Button, Divider, Input, InputNumber, message, Select, Switch, theme } from "antd";
import { Plus, Sparkles, Trash2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNodeAIAssist } from "../../Hooks";
import type { CompareOperator, Condition, ConditionNode, LogicalOperator, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

/** 检测值的实际类型，返回序列化友好的 JS 原始值 */
function detectTypedValue(raw: unknown): string | number | boolean | null {
  if (raw === null || raw === undefined) { return ""; }
  if (typeof raw === "number") { return raw; }
  if (typeof raw === "boolean") { return raw; }
  const str = String(raw).trim();
  // 空字符串 → 空字符串
  if (str === "") { return ""; }
  // 尝试解析数字
  if (/^-?\d+(\.\d+)?$/.test(str)) { return Number(str); }
  // 尝试解析布尔
  if (str === "true") { return true; }
  if (str === "false") { return false; }
  return str;
}

interface ConditionPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ConditionPropertyPanel: React.FC<ConditionPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [messageApi, messageContextHolder] = message.useMessage();
  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const conditionNode = node as ConditionNode;
  const config = conditionNode.config || {
    conditions: [],
    logical_op: "and" as LogicalOperator,
  };

  /** 根据值的实际类型渲染对应的输入组件，确保 value 以正确类型存储 */
  const ValueInput: React.FC<{ value: unknown; onChange: (val: string | number | boolean | null) => void }> = React
    .memo(
      ({ value, onChange }) => {
        const typed = detectTypedValue(value);
        if (typeof typed === "boolean") {
          return (
            <Switch
              size="small"
              checked={typed}
              onChange={(checked) => onChange(checked)}
              style={{ flex: 1 }}
            />
          );
        }
        if (typeof typed === "number") {
          return (
            <InputNumber
              size="small"
              value={typed}
              onChange={(num) => onChange(num ?? "")}
              style={{ flex: 1, minWidth: 80 }}
              placeholder={t("workflow.props.conditionValue")}
            />
          );
        }
        return (
          <Input
            id="condition-property-value-input"
            value={String(typed ?? "")}
            onChange={(e) => onChange(detectTypedValue(e.target.value))}
            size="small"
            placeholder={t("workflow.props.conditionValue")}
            style={{ flex: 1 }}
          />
        );
      },
    );

  const OPERATOR_OPTIONS: { value: CompareOperator; label: string }[] = [
    { value: "eq", label: "=" },
    { value: "ne", label: "≠" },
    { value: "gt", label: ">" },
    { value: "lt", label: "<" },
    { value: "gte", label: "≥" },
    { value: "lte", label: "≤" },
    { value: "contains", label: t("workflow.props.opContains") },
    { value: "notContains", label: t("workflow.props.opNotContains") },
    { value: "startsWith", label: t("workflow.props.opStartsWith") },
    { value: "endsWith", label: t("workflow.props.opEndsWith") },
    { value: "regexMatch", label: t("workflow.props.opRegexMatch") },
    { value: "isEmpty", label: t("workflow.props.opIsEmpty") },
    { value: "isNotEmpty", label: t("workflow.props.opIsNotEmpty") },
  ];

  const handleAddCondition = () => {
    const newCondition: Condition = {
      var_path: "",
      operator: "eq",
      value: "",
    };
    onUpdate({
      config: {
        ...config,
        conditions: [...config.conditions, newCondition],
      },
    });
  };

  const handleUpdateCondition = (
    index: number,
    updates: Partial<Condition>,
  ) => {
    const newConditions = [...config.conditions];
    newConditions[index] = { ...newConditions[index], ...updates };
    onUpdate({
      config: {
        ...config,
        conditions: newConditions,
      },
    });
  };

  const handleDeleteCondition = (index: number) => {
    const newConditions = config.conditions.filter((_, i) => i !== index);
    onUpdate({
      config: {
        ...config,
        conditions: newConditions,
      },
    });
  };

  const handleLogicalOpChange = (logical_op: LogicalOperator) => {
    onUpdate({
      config: {
        ...config,
        logical_op,
      },
    });
  };

  const handleAIGenerateConditions = async () => {
    const result = await aiGenerate({
      systemPrompt: "你是一名条件规则生成助手。基于节点描述和 logical_op，生成合理的条件数组。"
        + "只输出严格合法的 JSON 数组，每个对象的 operator ∈ {eq, ne, gt, lt, gte, lte, contains, notContains, startsWith, endsWith, regexMatch, isEmpty, isNotEmpty}。"
        + "value 字段：isEmpty/isNotEmpty 时省略；其余时根据语义给出合理值（数字/字符串/布尔）。"
        + "var_path 必须是 ${...} 形式引用上游变量。不要任何前缀、解释、Markdown 标记。"
        + '示例：[{"var_path":"${http.status}","operator":"eq","value":200}]',
      userPrompt: `Node title: ${node.title || ""}\nNode description: ${
        node.description || ""
      }\nlogical_op: ${config.logical_op}\n\nExisting conditions (可保留也可重写):\n${
        JSON.stringify(config.conditions, null, 2)
      }`,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    let parsed: unknown;
    try {
      const jsonStart = result.indexOf("[");
      const jsonEnd = result.lastIndexOf("]");
      if (jsonStart === -1 || jsonEnd === -1) {
        throw new Error("no json array");
      }
      parsed = JSON.parse(result.slice(jsonStart, jsonEnd + 1));
    } catch {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    if (!Array.isArray(parsed)) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    onUpdate({
      config: {
        ...config,
        conditions: parsed as Condition[],
      },
    });
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  const handleAIOptimizeRoutingPrompt = async () => {
    const result = await aiGenerate({
      systemPrompt:
        "你是一名 LLM 路由提示词优化助手。基于节点的 natural-language 描述，生成更清晰、具体、可被 LLM 理解的路由判断提示词。"
        + "只输出提示词本身（多行可），不要任何前缀、解释、Markdown 标记。",
      userPrompt: `Node title: ${node.title || ""}\nNode description: ${
        node.description || ""
      }\n\nCurrent routing_prompt (可为空):\n${config.routing_prompt || ""}`,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    onUpdate({ config: { ...config, routing_prompt: result } });
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
          {t("workflow.props.logicalOp")}
        </label>
        <Select
          value={config.logical_op}
          onChange={handleLogicalOpChange}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "and", label: t("workflow.props.andAllTrue") },
            { value: "or", label: t("workflow.props.orAnyTrue") },
          ]}
        />
      </div>

      {/* LLM 动态路由开关 */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.props.llmRouting")}
        </label>
        <Switch
          size="small"
          checked={config.judge_by_llm ?? false}
          onChange={(checked) => onUpdate({ config: { ...config, judge_by_llm: checked || undefined } })}
        />
      </div>

      {config.judge_by_llm && (
        <>
          <Divider style={{ margin: "6px 0", borderColor: "#333" }} />
          <div style={{ marginBottom: 10 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                marginBottom: 4,
              }}
            >
              <label style={{ color: "#999", fontSize: 12 }}>
                {t("workflow.props.routingPrompt")}
              </label>
              <Button
                size="small"
                type="dashed"
                icon={<Sparkles size={12} />}
                onClick={handleAIOptimizeRoutingPrompt}
                loading={aiGenerating}
                style={{ fontSize: 12 }}
              >
                {t("workflow.aiAssist.btn.optimize")}
              </Button>
            </div>
            <Input.TextArea
              value={config.routing_prompt ?? ""}
              onChange={(e) =>
                onUpdate({
                  config: {
                    ...config,
                    routing_prompt: e.target.value || undefined,
                  },
                })}
              rows={3}
              size="small"
              style={{ width: "100%" }}
              placeholder={t("workflow.props.routingPromptPlaceholder")}
            />
          </div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
              {t("workflow.props.routingModel")}
            </label>
            <Input
              value={config.routing_model ?? ""}
              onChange={(e) =>
                onUpdate({
                  config: {
                    ...config,
                    routing_model: e.target.value || undefined,
                  },
                })}
              size="small"
              style={{ width: "100%" }}
              placeholder={t("workflow.props.routingModelPlaceholder")}
            />
          </div>
        </>
      )}

      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 8,
          }}
        >
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            {t("workflow.props.conditions")} ({config.conditions.length})
          </label>
          <div style={{ display: "flex", gap: 4 }}>
            <Button
              type="dashed"
              size="small"
              icon={<Sparkles size={12} />}
              onClick={handleAIGenerateConditions}
              loading={aiGenerating}
            >
              {t("workflow.aiAssist.btn.generate")}
            </Button>
            <Button
              type="dashed"
              size="small"
              icon={<Plus size={12} />}
              onClick={handleAddCondition}
            >
              {t("workflow.props.addCondition")}
            </Button>
          </div>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {config.conditions.map((condition, index) => (
            <div
              key={`cond-${index}`}
              style={{
                padding: 8,
                background: token.colorBgElevated,
                borderRadius: 6,
                border: "1px solid #333",
              }}
            >
              <div style={{ marginBottom: 8 }}>
                <Input
                  id="condition-property-panel-input-87"
                  value={condition.var_path}
                  onChange={(e) => handleUpdateCondition(index, { var_path: e.target.value })}
                  size="small"
                  placeholder={t("workflow.props.conditionVarPath")}
                />
              </div>

              <div
                style={{
                  display: "flex",
                  gap: 4,
                  marginBottom: condition.operator === "isEmpty"
                      || condition.operator === "isNotEmpty"
                    ? 0
                    : 8,
                }}
              >
                <Select
                  value={condition.operator}
                  onChange={(value) => handleUpdateCondition(index, { operator: value })}
                  size="small"
                  style={{ flex: 1 }}
                  options={OPERATOR_OPTIONS}
                />

                {condition.operator !== "isEmpty"
                  && condition.operator !== "isNotEmpty" && (
                  <ValueInput
                    value={condition.value}
                    onChange={(val) => handleUpdateCondition(index, { value: val })}
                  />
                )}

                <Button
                  type="text"
                  danger
                  size="small"
                  icon={<Trash2 size={12} />}
                  onClick={() => handleDeleteCondition(index)}
                />
              </div>
            </div>
          ))}

          {config.conditions.length === 0 && (
            <div
              style={{
                color: token.colorTextTertiary,
                fontSize: 12,
                textAlign: "center",
                padding: 16,
              }}
            >
              {t("workflow.props.clickToAddCondition")}
            </div>
          )}
        </div>
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
