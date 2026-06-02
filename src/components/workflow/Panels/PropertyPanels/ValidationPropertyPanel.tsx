import { MinusCircleOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Input, message, Select, theme } from "antd";
import { Sparkles } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNodeAIAssist } from "../../Hooks";
import type { ValidationNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface ValidationPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ValidationPropertyPanel: React.FC<
  ValidationPropertyPanelProps
> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [messageApi, messageContextHolder] = message.useMessage();
  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const validationNode = node as ValidationNode;
  const config = validationNode.config || {
    assertions: [],
    on_fail: "stop" as const,
    max_retries: 0,
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAssertionChange = (
    index: number,
    field: string,
    value: string,
  ) => {
    const newAssertions = [...(config.assertions || [])];
    newAssertions[index] = { ...newAssertions[index], [field]: value };
    handleConfigChange("assertions", newAssertions);
  };

  const handleAddAssertion = () => {
    const newAssertions = [
      ...(config.assertions || []),
      { type: "equals" as const, expected: "", actual: "" },
    ];
    handleConfigChange("assertions", newAssertions);
  };

  const handleRemoveAssertion = (index: number) => {
    const newAssertions = (config.assertions || []).filter(
      (_, i) => i !== index,
    );
    handleConfigChange("assertions", newAssertions);
  };

  const handleAIGenerateAssertions = async () => {
    const result = await aiGenerate({
      systemPrompt:
        "你是一名数据校验规则生成助手。基于节点的 context（上游节点的输出变量、节点描述），生成合理的断言规则列表。"
        + "只输出严格合法的 JSON 数组，数组中每个对象的 type ∈ {equals, contains, matches, exists, custom}。"
        + '不要任何前缀、解释、Markdown 标记。示例：[{"type":"equals","expected":"200","actual":"${http.status}"}]',
      userPrompt: `Node title: ${node.title || ""}\nNode description: ${
        node.description || ""
      }\nNode id: ${node.id}\n\nExisting assertions (可保留也可重写):\n${
        JSON.stringify(config.assertions || [], null, 2)
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
    handleConfigChange("assertions", parsed);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {messageContextHolder}
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
            {t("workflow.props.assertions")}
          </label>
          <div style={{ display: "flex", gap: 4 }}>
            <Button
              size="small"
              type="dashed"
              icon={<Sparkles size={12} />}
              onClick={handleAIGenerateAssertions}
              loading={aiGenerating}
              style={{ fontSize: 12 }}
            >
              {t("workflow.aiAssist.btn.generate")}
            </Button>
            <Button
              size="small"
              type="dashed"
              icon={<PlusOutlined />}
              onClick={handleAddAssertion}
              style={{ fontSize: 12 }}
            >
              {t("workflow.props.add")}
            </Button>
          </div>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {/* assertions use index-based callbacks, safe to use index as key */}
          {(config.assertions || []).map((assertion, index) => (
            <div
              key={`assertion-${index}`}
              style={{ background: token.colorBgContainer, borderRadius: 4, padding: 8 }}
            >
              <div
                style={{
                  display: "flex",
                  gap: 4,
                  alignItems: "center",
                  marginBottom: 4,
                }}
              >
                <Select
                  value={assertion.type}
                  onChange={(value) => handleAssertionChange(index, "type", value)}
                  size="small"
                  style={{ flex: 1 }}
                  options={[
                    { value: "equals", label: t("workflow.props.equals") },
                    { value: "contains", label: t("workflow.props.contains") },
                    { value: "matches", label: t("workflow.props.matches") },
                    { value: "exists", label: t("workflow.props.exists") },
                    { value: "custom", label: t("workflow.props.custom") },
                  ]}
                />
                <MinusCircleOutlined
                  onClick={() => handleRemoveAssertion(index)}
                  style={{ color: token.colorError, cursor: "pointer", fontSize: 12 }}
                />
              </div>
              {assertion.type !== "exists" && (
                <div style={{ display: "flex", gap: 4 }}>
                  <Input
                    id="validation-property-panel-input-116"
                    value={assertion.expected || ""}
                    onChange={(e) => handleAssertionChange(index, "expected", e.target.value)}
                    size="small"
                    placeholder={t("workflow.props.expectedValue")}
                    style={{ flex: 1 }}
                  />
                  <Input
                    id="validation-property-panel-input-117"
                    value={assertion.actual || ""}
                    onChange={(e) => handleAssertionChange(index, "actual", e.target.value)}
                    size="small"
                    placeholder={t("workflow.props.actualValue")}
                    style={{ flex: 1 }}
                  />
                </div>
              )}
              {assertion.type === "custom" && (
                <Input
                  id="validation-property-panel-input-118"
                  value={assertion.expression || ""}
                  onChange={(e) => handleAssertionChange(index, "expression", e.target.value)}
                  size="small"
                  placeholder={t("workflow.props.customExpression")}
                  style={{ marginTop: 4 }}
                />
              )}
            </div>
          ))}

          {(config.assertions || []).length === 0 && (
            <div
              style={{
                color: token.colorTextTertiary,
                fontSize: 12,
                textAlign: "center",
                padding: 8,
              }}
            >
              {t("workflow.props.clickToAddAssertion")}
            </div>
          )}
        </div>
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
          {t("workflow.props.failStrategy")}
        </label>
        <Select
          value={config.on_fail || "stop"}
          onChange={(value) => handleConfigChange("on_fail", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "stop", label: t("workflow.props.stop") },
            { value: "retry", label: t("workflow.props.retry") },
            { value: "continue", label: t("workflow.props.continue") },
          ]}
        />
      </div>

      {config.on_fail === "retry" && (
        <div>
          <label
            style={{
              display: "block",
              color: token.colorTextTertiary,
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.maxRetries")}
          </label>
          <Input
            id="validation-property-panel-input-119"
            type="number"
            value={config.max_retries ?? 0}
            onChange={(e) => handleConfigChange("max_retries", parseInt(e.target.value) || 0)}
            size="small"
            min={0}
          />
        </div>
      )}

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
