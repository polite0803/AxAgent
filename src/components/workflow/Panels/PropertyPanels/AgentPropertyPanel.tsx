import { ExpertSelector } from "@/components/chat/ExpertSelector";
import { ModelSelect } from "@/components/shared/ModelSelect";
import { invoke } from "@/lib/invoke";
import { useAgentProfileStore, useKnowledgeStore, useLocalToolStore, useProviderStore } from "@/stores";
import { useExpertStore } from "@/stores/feature/expertStore";
import type { CreateAgentProfileInput } from "@/types";
import { Button, Divider, Input, InputNumber, message, Select, Tag } from "antd";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentNode, OutputMode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface AgentPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

interface AgentRoleRow {
  id: string;
  name: string;
}

export const AgentPropertyPanel: React.FC<AgentPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const agentNode = node as AgentNode;
  const config = agentNode.config || {
    system_prompt: "",
    context_sources: [],
    output_var: "",
    tools: [],
    output_mode: "text" as OutputMode,
  };

  const [expertSelectorOpen, setExpertSelectorOpen] = useState(false);
  const [globalRoles, setGlobalRoles] = useState<AgentRoleRow[]>([]);
  const [selectedRoleId, setSelectedRoleId] = useState<string | null>(null);
  const [creatingProfile, setCreatingProfile] = useState(false);

  const getExpert = useExpertStore((s) => s.getRoleById);
  const selectedExpert = config.agentProfileId
    ? getExpert(config.agentProfileId)
    : null;

  const { groups: toolGroups, loadGroups: loadToolGroups } = useLocalToolStore();
  const { bases: knowledgeBases, loadBases: loadKnowledgeBases } = useKnowledgeStore();
  const { providers, fetchProviders } = useProviderStore();

  // 加载全局角色列表
  useEffect(() => {
    invoke<AgentRoleRow[]>("list_agent_roles")
      .then((roles) => {
        setGlobalRoles(roles);
        // 从已选 profile 中恢复角色选择
        if (selectedExpert?.agentRole) {
          setSelectedRoleId(selectedExpert.agentRole);
        }
      })
      .catch((e) => console.error("[AgentPropertyPanel] Failed to load agent roles:", e));
  }, []);

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
  }, [
    toolGroups.length,
    knowledgeBases.length,
    providers.length,
    loadToolGroups,
    loadKnowledgeBases,
    fetchProviders,
  ]);

  const toolOptions = useMemo(() => {
    const options: { value: string; label: string }[] = [];
    for (const group of toolGroups) {
      if (!group.enabled) {
        continue;
      }
      for (const tool of group.tools) {
        options.push({
          value: tool.name,
          label: `${group.groupName} / ${tool.name}`,
        });
      }
    }
    return options;
  }, [toolGroups]);

  const contextSourceOptions = useMemo(() => {
    const options: { value: string; label: string }[] = [
      {
        value: "conversation_history",
        label: t("workflow.props.contextConversationHistory"),
      },
    ];
    for (const kb of knowledgeBases) {
      if (kb.enabled) {
        options.push({
          value: `knowledge_base::${kb.id}`,
          label: t("workflow.props.contextKnowledgeBase", { name: kb.name }),
        });
      }
    }
    return options;
  }, [knowledgeBases]);

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  // 角色+专家组合 → 创建或选择 AgentProfile
  const handleRoleExpertCombine = async (roleId: string, expertId: string) => {
    setCreatingProfile(true);
    try {
      const expert = getExpert(expertId);
      if (!expert) {
        message.error(t("workflow.props.expertNotFound"));
        return;
      }
      // 查找是否已有匹配的 profile
      const existingProfiles = useAgentProfileStore.getState().getAllProfiles();
      const matched = existingProfiles.find(
        (p) => p.agentRole === roleId && p.expertId === expertId,
      );
      if (matched) {
        handleConfigChange("agentProfileId", matched.id);
        message.success(t("workflow.props.profileMatched"));
        return;
      }

      // 创建新 AgentProfile：不合并专家提示词，执行时按优先级自动选取
      const input: CreateAgentProfileInput = {
        name: `${expert.name} (${roleId})`,
        description: expert.description ?? undefined,
        category: expert.category,
        icon: expert.icon,
        systemPrompt: "",
        agentRole: roleId,
        source: "custom",
        tags: expert.tags,
        expertId,
        suggestedProviderId: expert.suggestedProviderId,
        suggestedModelId: expert.suggestedModelId,
        suggestedTemperature: expert.suggestedTemperature,
        suggestedMaxTokens: expert.suggestedMaxTokens,
        searchEnabled: expert.searchEnabled,
        recommendPermissionMode: expert.recommendPermissionMode,
        recommendedTools: expert.recommendedTools,
        recommendedWorkflows: expert.recommendedWorkflows,
      };
      const profile = await useAgentProfileStore
        .getState()
        .createCustomProfile(input);
      handleConfigChange("agentProfileId", profile.id);
      message.success(t("workflow.props.profileCreated"));
    } catch (e) {
      message.error(
        t("workflow.props.profileCreateFailed", { error: String(e) }),
      );
    } finally {
      setCreatingProfile(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {/* 角色选择（全局 agent_roles） */}
      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.agentRole")}
        </label>
        <Select
          value={selectedRoleId}
          onChange={(roleId) => {
            setSelectedRoleId(roleId);
            // 如果已经选了专家，自动组合
            if (roleId && config.agentProfileId) {
              handleRoleExpertCombine(roleId, config.agentProfileId);
            }
          }}
          size="small"
          style={{ width: "100%" }}
          allowClear
          placeholder={t("workflow.props.selectRole")}
          options={globalRoles.map((r) => ({ value: r.id, label: r.name }))}
        />
      </div>

      {/* 专家/AgentProfile 选择 */}
      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.expertRole")}
        </label>
        {selectedExpert
          ? (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                flexWrap: "wrap",
              }}
            >
              <Tag
                closable
                onClose={() => {
                  handleConfigChange("agentProfileId", undefined);
                  setSelectedRoleId(null);
                }}
                style={{
                  margin: 0,
                  fontSize: 12,
                  padding: "2px 8px",
                  display: "flex",
                  alignItems: "center",
                  gap: 4,
                }}
              >
                {selectedExpert.icon} {selectedExpert.name}
              </Tag>
              {selectedExpert.agentRole && (
                <Tag color="blue" style={{ margin: 0, fontSize: 12 }}>
                  {t("workflow.props.roleTag", {
                    role: selectedExpert.agentRole,
                  })}
                </Tag>
              )}
              {!selectedExpert.agentRole && selectedRoleId && (
                <Button
                  size="small"
                  type="link"
                  loading={creatingProfile}
                  onClick={() =>
                    handleRoleExpertCombine(
                      selectedRoleId,
                      config.agentProfileId!,
                    )}
                >
                  {t("workflow.props.bindRole")}
                </Button>
              )}
            </div>
          )
          : (
            <Button
              size="small"
              type="dashed"
              block
              onClick={() => setExpertSelectorOpen(true)}
            >
              {t("workflow.props.selectExpert")}
            </Button>
          )}
      </div>

      <ExpertSelector
        open={expertSelectorOpen}
        selectedRoleId={config.agentProfileId ?? null}
        onSelect={(profileId) => {
          handleConfigChange("agentProfileId", profileId);
          setExpertSelectorOpen(false);
          // 从选中的 profile 中恢复角色选择
          const profile = getExpert(profileId);
          if (profile?.agentRole) {
            setSelectedRoleId(profile.agentRole);
          }
        }}
        onClose={() => setExpertSelectorOpen(false)}
      />

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.systemPrompt")}
        </label>
        <Input.TextArea
          id="agent-property-panel-input-textarea-76"
          value={config.system_prompt || ""}
          onChange={(e) => handleConfigChange("system_prompt", e.target.value)}
          rows={4}
          size="small"
          placeholder={t("workflow.props.systemPromptPlaceholder")}
        />
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.model")}
        </label>
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
          <label
            style={{
              display: "block",
              color: "#999",
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.temperature")}
          </label>
          <InputNumber
            id="agent-property-panel-inputnumber-77"
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
          <label
            style={{
              display: "block",
              color: "#999",
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.maxTokens")}
          </label>
          <InputNumber
            id="agent-property-panel-inputnumber-78"
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
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.outputMode")}
        </label>
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
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.outputVariable")}
        </label>
        <Input
          id="agent-property-panel-input-79"
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.outputVarDefault")}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.toolsCount", { count: config.tools?.length || 0 })}
        </label>
        <Select
          mode="multiple"
          value={(config.tools || []).map((td) => typeof td === "string" ? td : td.name)}
          onChange={(values: string[]) => {
            const existingMap = new Map(
              (config.tools || []).map((td) => [
                typeof td === "string" ? td : td.name,
                typeof td === "string" ? null : td,
              ]),
            );
            const newTools = values.map((name) =>
              existingMap.get(name) || {
                name,
                description: undefined,
                parameters: undefined,
              }
            );
            handleConfigChange("tools", newTools);
          }}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectTools")}
          showSearch
          optionFilterProp="label"
          options={toolOptions}
        />
      </div>

      {(config.tools?.length ?? 0) > 0 && (
        <div>
          <label
            style={{
              display: "block",
              color: "#999",
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.maxToolRounds")}
          </label>
          <InputNumber
            id="agent-property-panel-inputnumber-max-tool-rounds"
            value={config.max_tool_rounds ?? 5}
            onChange={(value) => handleConfigChange("max_tool_rounds", value ?? null)}
            min={1}
            max={50}
            step={1}
            size="small"
            style={{ width: "100%" }}
          />
          <div style={{ fontSize: 11, color: "#666", marginTop: 2 }}>
            {t("workflow.props.maxToolRoundsHint")}
          </div>
        </div>
      )}

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.contextSourcesCount", {
            count: config.context_sources?.length || 0,
          })}
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

      <div
        style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}
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
