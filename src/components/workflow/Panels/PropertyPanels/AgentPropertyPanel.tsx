import { ExpertSelector } from "@/components/chat/ExpertSelector";
import { ModelSelect } from "@/components/shared/ModelSelect";
import { invoke } from "@/lib/invoke";
import { useAgentProfileStore, useKnowledgeStore, useLocalToolStore, useProviderStore } from "@/stores";
import { useExpertStore } from "@/stores/feature/expertStore";
import type { CreateAgentProfileInput } from "@/types";
import { Button, Divider, Input, InputNumber, message, Modal, Select, Tag } from "antd";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentNode, OutputMode, ToolDef, WorkflowNode } from "../../types";
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
    exposed_tools: [],
    output_mode: "text" as OutputMode,
  };

  const [expertSelectorOpen, setExpertSelectorOpen] = useState(false);
  const [globalRoles, setGlobalRoles] = useState<AgentRoleRow[]>([]);
  const [selectedRoleId, setSelectedRoleId] = useState<string | null>(null);
  const [creatingProfile, setCreatingProfile] = useState(false);

  // 快捷编辑 Expert/Role 提示词
  const [promptEditor, setPromptEditor] = useState<{
    open: boolean;
    type: "expert" | "role";
    id: string;
    name: string;
    prompt: string;
    saving: boolean;
  }>({ open: false, type: "expert", id: "", name: "", prompt: "", saving: false });

  // 每个工具的参数编辑展开状态
  const [expandedTools, setExpandedTools] = useState<Set<string>>(new Set());

  const openExpertPromptEditor = async (expertId: string, expertName: string) => {
    try {
      const experts: { id: string; system_prompt: string }[] = await invoke("list_agency_experts");
      const expert = experts.find((e) => e.id === expertId);
      setPromptEditor({
        open: true,
        type: "expert",
        id: expertId,
        name: expertName,
        prompt: expert?.system_prompt || "",
        saving: false,
      });
    } catch {
      setPromptEditor({
        open: true,
        type: "expert",
        id: expertId,
        name: expertName,
        prompt: "",
        saving: false,
      });
    }
  };

  const openRolePromptEditor = async (roleId: string, roleName: string) => {
    try {
      const roles: { id: string; system_prompt: string }[] = await invoke("list_agent_roles");
      const role = roles.find((r) => r.id === roleId);
      setPromptEditor({
        open: true,
        type: "role",
        id: roleId,
        name: roleName,
        prompt: role?.system_prompt || "",
        saving: false,
      });
    } catch {
      setPromptEditor({
        open: true,
        type: "role",
        id: roleId,
        name: roleName,
        prompt: "",
        saving: false,
      });
    }
  };

  const savePrompt = async () => {
    setPromptEditor((prev) => ({ ...prev, saving: true }));
    try {
      if (promptEditor.type === "expert") {
        await invoke("update_agency_expert", {
          request: {
            id: promptEditor.id,
            system_prompt: promptEditor.prompt,
          },
        });
      } else {
        await invoke("update_agent_role", {
          id: promptEditor.id,
          system_prompt: promptEditor.prompt,
        });
      }
      message.success(t("workflow.props.promptSaved"));
      setPromptEditor((prev) => ({ ...prev, open: false, saving: false }));
    } catch (e) {
      message.error(t("workflow.props.promptSaveFailed", { error: String(e) }));
      setPromptEditor((prev) => ({ ...prev, saving: false }));
    }
  };

  const getExpert = useExpertStore((s) => s.getRoleById);
  // useExpertStore 和 useAgentProfileStore 是两个独立数据源，需要合并查找
  const getProfileById = useAgentProfileStore((s) => s.getProfileById);
  const agentProfilesLoaded = useAgentProfileStore((s) => s.loaded);
  const loadAgentProfiles = useAgentProfileStore((s) => s.loadProfiles);
  const selectedExpert = config.agentProfileId
    ? (getExpert(config.agentProfileId) ?? getProfileById(config.agentProfileId))
    : null;

  const { groups: toolGroups, loadGroups: loadToolGroups } = useLocalToolStore();
  const { bases: knowledgeBases, loadBases: loadKnowledgeBases } = useKnowledgeStore();
  const { providers, fetchProviders } = useProviderStore();

  // 加载 AgentProfile 数据，角色列表从 expertStore 读取（同源）
  const getAllRoles = useExpertStore((s) => s.getAllRoles);
  useEffect(() => {
    if (!agentProfilesLoaded) {
      loadAgentProfiles();
    }
    const roles = getAllRoles();
    setGlobalRoles(roles.map((r) => ({ id: r.id, name: r.name })));
  }, [agentProfilesLoaded, loadAgentProfiles, getAllRoles]);

  // 从已选 profile 中恢复角色选择（独立 effect，确保 selectedExpert 就绪后同步）
  useEffect(() => {
    if (selectedExpert?.agentRole) {
      setSelectedRoleId(selectedExpert.agentRole);
    }
  }, [selectedExpert?.agentRole]);

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

  // ── 单工具参数编辑 ──

  const toggleToolExpand = (name: string) => {
    setExpandedTools((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  };

  const updateToolDef = (idx: number, updated: ToolDef) => {
    const newTools = [...(config.tools || [])];
    newTools[idx] = updated;
    handleConfigChange("tools", newTools);
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
                  cursor: "pointer",
                }}
                onClick={() => {
                  const expertId = selectedExpert.expertId || config.agentProfileId;
                  if (expertId) {
                    openExpertPromptEditor(expertId, selectedExpert.name);
                  }
                }}
              >
                {selectedExpert.icon} {selectedExpert.name}
              </Tag>
              {selectedExpert.agentRole && (
                <Tag
                  color="blue"
                  style={{ margin: 0, fontSize: 12, cursor: "pointer" }}
                  onClick={() =>
                    openRolePromptEditor(
                      selectedExpert.agentRole!,
                      selectedExpert.agentRole!,
                    )}
                >
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
          readOnly={!!config.agentProfileId}
          disabled={!!config.agentProfileId}
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

      {/* 单工具参数编辑 */}
      {config.tools?.length > 0 && (
        <div style={{ marginTop: 8 }}>
          {config.tools.map((td, idx) => {
            if (typeof td === "string") { return null; }
            const toolDef = td as ToolDef;
            const expanded = expandedTools.has(toolDef.name);
            return (
              <div
                key={toolDef.name}
                style={{
                  marginBottom: 6,
                  border: "1px solid #333",
                  borderRadius: 6,
                  overflow: "hidden",
                }}
              >
                <div
                  onClick={() => toggleToolExpand(toolDef.name)}
                  style={{
                    padding: "6px 10px",
                    background: "#1a1a2e",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    userSelect: "none",
                  }}
                >
                  <span style={{ fontSize: 12, fontWeight: 500, color: "#ccc" }}>
                    🛠 {toolDef.name}
                  </span>
                  <span style={{ fontSize: 10, color: "#666" }}>
                    {expanded ? "▼" : "▶"}
                  </span>
                </div>
                {expanded && (
                  <div style={{ padding: "8px 10px", background: "#0d0d1a" }}>
                    <label
                      style={{
                        display: "block",
                        color: "#999",
                        fontSize: 11,
                        marginBottom: 2,
                      }}
                    >
                      {t("workflow.props.toolDescription")}
                    </label>
                    <Input
                      id={`agent-tool-desc-${idx}`}
                      value={toolDef.description || ""}
                      onChange={(e) => updateToolDef(idx, { ...toolDef, description: e.target.value })}
                      size="small"
                      style={{ width: "100%", marginBottom: 8 }}
                      placeholder={t("workflow.props.toolDescriptionPlaceholder")}
                    />
                    <label
                      style={{
                        display: "block",
                        color: "#999",
                        fontSize: 11,
                        marginBottom: 2,
                      }}
                    >
                      {t("workflow.props.toolParameters")}
                    </label>
                    <Input.TextArea
                      id={`agent-tool-params-${idx}`}
                      value={toolDef.parameters
                        ? JSON.stringify(toolDef.parameters, null, 2)
                        : ""}
                      onChange={(e) => {
                        if (!e.target.value.trim()) {
                          updateToolDef(idx, { ...toolDef, parameters: undefined });
                          return;
                        }
                        try {
                          const parsed = JSON.parse(e.target.value);
                          updateToolDef(idx, { ...toolDef, parameters: parsed });
                        } catch {
                          // 键入非法 JSON 时保留旧值，避免覆盖
                        }
                      }}
                      rows={4}
                      style={{
                        width: "100%",
                        fontFamily: "monospace",
                        fontSize: 11,
                      }}
                      placeholder={t("workflow.props.toolParametersPlaceholder")}
                    />
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {(config.tools?.length ?? 0) > 0 && (
        <>
          {/* 暴露给 LLM 的工具选择 */}
          <div style={{ marginBottom: 12 }}>
            <label
              style={{
                display: "block",
                color: "#999",
                fontSize: 12,
                marginBottom: 4,
              }}
            >
              {t("workflow.props.exposedTools", {
                count: config.exposed_tools?.length || config.tools?.length || 0,
              })}
            </label>
            <Select
              mode="multiple"
              value={config.exposed_tools?.length
                ? config.exposed_tools
                : (config.tools || []).map((td) => typeof td === "string" ? td : td.name)}
              onChange={(values: string[]) => handleConfigChange("exposed_tools", values)}
              size="small"
              style={{ width: "100%" }}
              placeholder={t("workflow.props.exposedToolsPlaceholder")}
              showSearch
              options={(config.tools || []).map((td) => {
                const name = typeof td === "string" ? td : td.name;
                return { value: name, label: name };
              })}
            />
            <div style={{ fontSize: 11, color: "#666", marginTop: 2 }}>
              {t("workflow.props.exposedToolsHint")}
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
        </>
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

      {/* 快捷编辑 Expert/Role 提示词 */}
      <Modal
        title={promptEditor.type === "expert"
          ? t("workflow.props.editExpertPrompt", { name: promptEditor.name })
          : t("workflow.props.editRolePrompt", { name: promptEditor.name })}
        open={promptEditor.open}
        onOk={savePrompt}
        onCancel={() => setPromptEditor((prev) => ({ ...prev, open: false }))}
        confirmLoading={promptEditor.saving}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
        width={600}
      >
        <Input.TextArea
          value={promptEditor.prompt}
          onChange={(e) => setPromptEditor((prev) => ({ ...prev, prompt: e.target.value }))}
          rows={12}
          style={{ fontFamily: "monospace", fontSize: 13 }}
        />
      </Modal>
    </div>
  );
};
