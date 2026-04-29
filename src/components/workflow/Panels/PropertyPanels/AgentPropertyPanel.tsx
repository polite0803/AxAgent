import { Divider, Input, InputNumber, Select } from "antd";
import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ModelSelect } from "@/components/shared/ModelSelect";
import { useKnowledgeStore, useLocalToolStore, useProviderStore } from "@/stores";
import type { AgentNode, AgentRole, OutputMode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface AgentPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const AgentPropertyPanel: React.FC<AgentPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const agentNode = node as AgentNode;
  const config = agentNode.config || {
    role: "developer" as AgentRole,
    system_prompt: "",
    context_sources: [],
    output_var: "",
    tools: [],
    output_mode: "text" as OutputMode,
  };

  const { groups: toolGroups, loadGroups: loadToolGroups } = useLocalToolStore();
  const { bases: knowledgeBases, loadBases: loadKnowledgeBases } = useKnowledgeStore();
  const { providers, fetchProviders } = useProviderStore();

  useEffect(() => {
    if (toolGroups.length === 0) {
      loadToolGroups();
    }
    if (knowledgeBases.length === 0) {
      loadKnowledgeBases();
    }
    if (providers.length === 0) {
      fetchProviders();
    }
  }, [toolGroups.length, knowledgeBases.length, providers.length, loadToolGroups, loadKnowledgeBases, fetchProviders]);

  const toolOptions = useMemo(() => {
    const options: { value: string; label: string }[] = [];
    for (const group of toolGroups) {
      if (!group.enabled) continue;
      for (const tool of group.tools) {
        options.push({ value: tool.toolName, label: `${group.groupName} / ${tool.toolName}` });
      }
    }
    return options;
  }, [toolGroups]);

  const contextSourceOptions = useMemo(() => {
    const options: { value: string; label: string }[] = [
      { value: "conversation_history", label: t("workflow.props.contextConversationHistory") },
    ];
    for (const kb of knowledgeBases) {
      if (kb.enabled) {
        options.push({ value: `knowledge_base::${kb.id}`, label: t("workflow.props.contextKnowledgeBase", { name: kb.name }) });
      }
    }
    return options;
  }, [knowledgeBases]);

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.agentRole")}</label>
        <Select
          value={config.role}
          onChange={(value) => handleConfigChange("role", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "researcher", label: t("workflow.props.roleResearcher") },
            { value: "planner", label: t("workflow.props.rolePlanner") },
            { value: "developer", label: t("workflow.props.roleDeveloper") },
            { value: "reviewer", label: t("workflow.props.roleReviewer") },
            { value: "synthesizer", label: t("workflow.props.roleSynthesizer") },
            { value: "executor", label: t("workflow.props.roleExecutor") },
          ]}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.systemPrompt")}</label>
        <Input.TextArea
          value={config.system_prompt || ""}
          onChange={(e) => handleConfigChange("system_prompt", e.target.value)}
          rows={4}
          size="small"
          placeholder={t("workflow.props.systemPromptPlaceholder")}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.model")}</label>
        <ModelSelect
          value={config.model || undefined}
          onChange={(value) => handleConfigChange("model", value || "")}
          placeholder={t("workflow.props.selectModel")}
          allowClear
          style={{ width: "100%" }}
        />
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.temperature")}</label>
          <InputNumber
            value={config.temperature ?? 0.7}
            onChange={(value) => handleConfigChange("temperature", value)}
            min={0}
            max={2}
            step={0.1}
            size="small"
            style={{ width: "100%" }}
          />
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.maxTokens")}</label>
          <InputNumber
            value={config.max_tokens ?? 2048}
            onChange={(value) => handleConfigChange("max_tokens", value)}
            min={100}
            max={128000}
            step={100}
            size="small"
            style={{ width: "100%" }}
          />
        </div>
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.outputMode")}</label>
        <Select
          value={config.output_mode}
          onChange={(value) => handleConfigChange("output_mode", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "text", label: t("workflow.props.outputText") },
            { value: "json", label: "{} JSON" },
            { value: "artifact", label: t("workflow.props.outputArtifact") },
          ]}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.outputVariable")}</label>
        <Input
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.outputVarDefault")}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          {t("workflow.props.toolsCount", { count: config.tools?.length || 0 })}
        </label>
        <Select
          mode="multiple"
          value={config.tools || []}
          onChange={(value) => handleConfigChange("tools", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectTools")}
          showSearch
          optionFilterProp="label"
          options={toolOptions}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          {t("workflow.props.contextSourcesCount", { count: config.context_sources?.length || 0 })}
        </label>
        <Select
          mode="multiple"
          value={config.context_sources || []}
          onChange={(value) => handleConfigChange("context_sources", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectContextSources")}
          options={contextSourceOptions}
        />
      </div>

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
