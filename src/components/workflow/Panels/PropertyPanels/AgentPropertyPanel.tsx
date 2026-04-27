import { Divider, Input, InputNumber, Select } from "antd";
import React, { useEffect, useMemo } from "react";
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
      { value: "conversation_history", label: "对话历史" },
    ];
    for (const kb of knowledgeBases) {
      if (kb.enabled) {
        options.push({ value: `knowledge_base::${kb.id}`, label: `知识库: ${kb.name}` });
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
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>Agent 角色</label>
        <Select
          value={config.role}
          onChange={(value) => handleConfigChange("role", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "researcher", label: "🔍 研究员" },
            { value: "planner", label: "📋 规划师" },
            { value: "developer", label: "💻 开发者" },
            { value: "reviewer", label: "👀 审核员" },
            { value: "synthesizer", label: "🔬 综合师" },
            { value: "executor", label: "⚙️ 执行者" },
          ]}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>系统提示词</label>
        <Input.TextArea
          value={config.system_prompt || ""}
          onChange={(e) => handleConfigChange("system_prompt", e.target.value)}
          rows={4}
          size="small"
          placeholder="定义 Agent 的行为和能力..."
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>模型</label>
        <ModelSelect
          value={config.model || undefined}
          onChange={(value) => handleConfigChange("model", value || "")}
          placeholder="选择模型..."
          allowClear
          style={{ width: "100%" }}
        />
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>温度</label>
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
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>最大 Token</label>
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
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>输出模式</label>
        <Select
          value={config.output_mode}
          onChange={(value) => handleConfigChange("output_mode", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "text", label: "📝 文本" },
            { value: "json", label: "{} JSON" },
            { value: "artifact", label: "🎨 工件" },
          ]}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>输出变量</label>
        <Input
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder="agent_output"
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          工具 ({config.tools?.length || 0})
        </label>
        <Select
          mode="multiple"
          value={config.tools || []}
          onChange={(value) => handleConfigChange("tools", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder="选择工具..."
          showSearch
          optionFilterProp="label"
          options={toolOptions}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          上下文源 ({config.context_sources?.length || 0})
        </label>
        <Select
          mode="multiple"
          value={config.context_sources || []}
          onChange={(value) => handleConfigChange("context_sources", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder="选择上下文源..."
          options={contextSourceOptions}
        />
      </div>

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
