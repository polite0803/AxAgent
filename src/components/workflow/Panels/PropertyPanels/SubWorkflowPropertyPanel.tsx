// SPDX-License-Identifier: AGPL-3.0-only

import { message } from "@/lib/toast";
import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, Select, Switch, theme } from "antd";
import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { SubWorkflowNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface SubWorkflowPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const SubWorkflowPropertyPanel: React.FC<
  SubWorkflowPropertyPanelProps
> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const subWorkflowNode = node as SubWorkflowNode;
  const config = subWorkflowNode.config || {
    subWorkflowId: "",
    inputMapping: {},
    outputVar: "",
    isAsync: false,
  };

  const { templates, loadTemplates, currentTemplate, expandedSubWorkflows } = useWorkflowEditorStore();

  useEffect(() => {
    if (templates.length === 0) {
      loadTemplates();
    }
  }, [templates.length, loadTemplates]);

  const expandedData = expandedSubWorkflows[node.id];
  const isExpanded = !!expandedData && !expandedData.isLoading;
  const isLoading = !!expandedData?.isLoading;

  const handleToggleExpand = () => {
    useWorkflowEditorStore.getState().toggleExpandSubWorkflow(node.id, config.subWorkflowId);
  };

  const workflowOptions = useMemo(
    () => templates.flatMap((t) => t.id !== currentTemplate?.id ? [{ value: t.id, label: t.name }] : []),
    [templates, currentTemplate?.id],
  );

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const [messageApi, contextHolder] = message.useMessage();

  const handleAISuggestInputMapping = async () => {
    if (!config.subWorkflowId) {
      messageApi.warning(t("workflow.aiAssist.subWorkflow.needPick"));
      return;
    }
    const result = await aiGenerate({
      systemPrompt:
        "你是一个工作流编排助手。根据当前节点的子工作流 id，输出建议的 input_mapping（一个 JSON 对象），键名为子工作流入参，值为上游变量路径（如 ${nodeId.output}）。"
        + "只输出 JSON 字符串，不要任何解释或 Markdown 标记。",
      userPrompt: JSON.stringify({ current_mapping: config.inputMapping, sub_workflow_id: config.subWorkflowId }),
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    try {
      const cleaned = result.replace(/^```\w*\s*|\s*```$/g, "").trim();
      const parsed = JSON.parse(cleaned) as Record<string, string>;
      onUpdate({ config: { ...config, inputMapping: { ...config.inputMapping, ...parsed } } });
      messageApi.success(t("workflow.aiAssist.applied"));
    } catch {
      messageApi.error(t("workflow.aiAssist.subWorkflow.parseFailed"));
    }
  };

  const handleAddInputMapping = () => {
    const existingKeys = Object.keys(config.inputMapping || {});
    let nextIndex = existingKeys.length + 1;
    let newKey = t("workflow.paramKey", { n: nextIndex, defaultValue: `param_${nextIndex}` });
    while (Object.prototype.hasOwnProperty.call(config.inputMapping || {}, newKey)) {
      nextIndex += 1;
      newKey = t("workflow.paramKey", { n: nextIndex, defaultValue: `param_${nextIndex}` });
    }
    onUpdate({
      config: {
        ...config,
        inputMapping: {
          ...config.inputMapping,
          [newKey]: "",
        },
      },
    });
  };

  const handleUpdateInputMapping = (key: string, value: string) => {
    onUpdate({
      config: {
        ...config,
        inputMapping: {
          ...config.inputMapping,
          [key]: value,
        },
      },
    });
  };

  const handleDeleteInputMapping = (key: string) => {
    const newMapping = { ...config.inputMapping };
    delete newMapping[key];
    onUpdate({
      config: {
        ...config,
        inputMapping: newMapping,
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {contextHolder}
      <div>
        <label
          htmlFor="sub-workflow-select"
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.subWorkflowLabel")}
        </label>
        <Select
          id="sub-workflow-select"
          value={config.subWorkflowId || undefined}
          onChange={(value) => handleConfigChange("subWorkflowId", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectSubWorkflow")}
          showSearch
          optionFilterProp="label"
          options={workflowOptions}
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
          {t("workflow.props.asyncExecution")}
        </label>
        <Switch
          size="small"
          checked={config.isAsync ?? false}
          onChange={(checked) => handleConfigChange("isAsync", checked)}
        />
      </div>

      {/* 展开/折叠按钮 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.subWorkflowNode.expand")}
        </label>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {isExpanded && expandedData && (
            <span style={{ fontSize: 11, color: token.colorTextTertiary }}>
              {t("workflow.nodesCount", {
                count: expandedData.nodes.length,
                defaultValue: `${expandedData.nodes.length} nodes`,
              })}
            </span>
          )}
          <Button
            size="small"
            loading={isLoading}
            onClick={handleToggleExpand}
            disabled={!config.subWorkflowId}
          >
            {isExpanded ? t("workflow.subWorkflowNode.collapse") : t("workflow.subWorkflowNode.expand")}
          </Button>
        </div>
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

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
            {t("workflow.props.inputMapping")}
          </label>
          <div style={{ display: "flex", gap: 4 }}>
            <AIAssistButton
              labelKey="suggest"
              loading={aiGenerating}
              onClick={handleAISuggestInputMapping}
              compact
            />
            <Button type="link" size="small" onClick={handleAddInputMapping}>
              {t("workflow.props.addMapping")}
            </Button>
          </div>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {Object.entries(config.inputMapping || {}).map(([key, value]) => (
            <div
              key={key}
              style={{ display: "flex", gap: 4, alignItems: "center" }}
            >
              <Input
                id="sub-workflow-property-panel-input-107"
                value={key}
                size="small"
                disabled
                style={{ width: 80, fontSize: 12 }}
              />
              <span style={{ color: token.colorTextTertiary, fontSize: 12 }}>←</span>
              <Input
                id="sub-workflow-property-panel-input-108"
                value={String(value)}
                onChange={(e) => handleUpdateInputMapping(key, e.target.value)}
                size="small"
                placeholder={t("workflow.props.variable")}
                style={{ flex: 1 }}
              />
              <Button
                type="link"
                size="small"
                danger
                onClick={() => handleDeleteInputMapping(key)}
              >
                {t("workflow.props.delete")}
              </Button>
            </div>
          ))}

          {Object.keys(config.inputMapping || {}).length === 0 && (
            <div
              style={{
                color: token.colorTextTertiary,
                fontSize: 12,
                textAlign: "center",
                padding: 8,
              }}
            >
              {t("workflow.props.clickToAddMapping")}
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
          {t("workflow.props.outputVariable")}
        </label>
        <Input
          id="sub-workflow-property-panel-input-109"
          value={config.outputVar || ""}
          onChange={(e) => handleConfigChange("outputVar", e.target.value)}
          size="small"
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
