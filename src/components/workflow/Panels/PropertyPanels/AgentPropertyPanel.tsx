import { ExpertSelector } from "@/components/chat/ExpertSelector";
import { ModelSelect } from "@/components/shared/ModelSelect";
import { invoke } from "@/lib/invoke";
import {
  useAgentProfileStore,
  useKnowledgeStore,
  useLocalToolStore,
  useProviderStore,
  useWorkflowEditorStore,
} from "@/stores";
import { usePromptTemplateStore } from "@/stores";
import { useExpertStore } from "@/stores/feature/expertStore";
import type { CreateAgentProfileInput, PromptTemplate } from "@/types";
import { Button, Divider, Input, InputNumber, message, Modal, Select, Tag, theme } from "antd";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
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
  const { token } = theme.useToken();
  const [messageApi, messageContextHolder] = message.useMessage();
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

  // 提示词模板选择
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<PromptTemplate | null>(null);
  const [variableValues, setVariableValues] = useState<Record<string, string>>({});
  const { templates, loadTemplates } = usePromptTemplateStore();
  const incrementUsage = usePromptTemplateStore((s) => s.incrementUsage);
  useEffect(() => {
    loadTemplates();
  }, []);

  const parseVariables = (content: string): string[] => {
    const matches = content.match(/\{([^}]+)\}/g) || [];
    return [...new Set(matches.map((m) => m.slice(1, -1)))];
  };

  const activeTemplates = templates.filter((t) => t.isActive);

  const handleSelectTemplate = (template: PromptTemplate) => {
    setSelectedTemplate(template);
    setVariableValues({});
    setTemplateModalOpen(true);
  };

  const handleApplyTemplate = () => {
    if (!selectedTemplate) { return; }
    let content = selectedTemplate.content;
    try {
      const schema = selectedTemplate.variablesSchema
        ? JSON.parse(selectedTemplate.variablesSchema)
        : {};
      for (const [varName] of Object.entries(schema)) {
        const value = variableValues[varName] || `{${varName}}`;
        content = content.replace(new RegExp(`\\{${varName}\\}`, "g"), value);
      }
    } catch {
      content = selectedTemplate.content;
    }
    handleConfigChange("system_prompt", content);
    handleConfigChange("promptTemplateId", selectedTemplate.id);
    setTemplateModalOpen(false);
    setSelectedTemplate(null);
    setVariableValues({});
    incrementUsage(selectedTemplate.id);
    messageApi.success(t("promptTemplates.applied"));
  };

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
  const templateToolDefs = useWorkflowEditorStore((s) => s.currentTemplate?.tool_defs);
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

  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const handleAIOptimizeSystemPrompt = async () => {
    if (config.agentProfileId) {
      messageApi.warning(t("workflow.props.expertNotFound"));
      return;
    }
    const current = config.system_prompt || "";
    if (!current.trim()) {
      messageApi.warning(t("workflow.aiPanel.enterPromptToOptimize"));
      return;
    }
    const result = await aiGenerate({
      systemPrompt:
        "你是一个 AI 智能体系统提示词优化专家。改进用户提供的 system_prompt，使角色定位更清晰、能力边界更明确、输出格式更可控。"
        + "保留原有工具调用约束和变量占位符（如 {varName}）。"
        + "只输出优化后的 system_prompt 正文，不要任何解释、前缀或 Markdown 标记。",
      userPrompt: current,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    handleConfigChange("system_prompt", result);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  const handleAIContextComplete = async () => {
    if (config.agentProfileId) {
      messageApi.warning(t("workflow.props.expertNotFound"));
      return;
    }
    const current = config.system_prompt || "";
    // 获取工作流上下文
    const store = useWorkflowEditorStore.getState();
    const upstreamEdgeIds = store.edges.filter((e) => e.target === agentNode.id).map((e) => e.source);
    const upstreamNodes = store.nodes.filter((n) => upstreamEdgeIds.includes(n.id));
    const downstreamEdgeIds = store.edges.filter((e) => e.source === agentNode.id).map((e) => e.target);
    const downstreamNodes = store.nodes.filter((n) => downstreamEdgeIds.includes(n.id));
    const contextInfo = [
      `当前节点: "${agentNode.title}" (类型: ${agentNode.type})`,
      upstreamNodes.length > 0
        ? `上游节点: ${upstreamNodes.map((n) => `"${n.title}"(${n.type})`).join(", ")}`
        : "无上游节点",
      downstreamNodes.length > 0
        ? `下游节点: ${downstreamNodes.map((n) => `"${n.title}"(${n.type})`).join(", ")}`
        : "无下游节点",
    ].join("\n");
    const result = await aiGenerate({
      systemPrompt: "你是工作流上下文补全助手。根据工作流上下文和当前提示词，生成可追加到提示词末尾的补充内容，"
        + "帮助智能体理解可用的上下文信息、上游数据来源和输出目标。"
        + "只输出纯文本补充内容，不要解释、前缀或 Markdown 标记。",
      userPrompt: current
        ? `工作流上下文:\n${contextInfo}\n\n当前提示词:\n${current}\n\n请根据工作流上下文，生成可以追加到提示词末尾的补充内容。`
        : `工作流上下文:\n${contextInfo}\n\n当前没有提示词。请根据工作流上下文生成一个初始提示词。`,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    handleConfigChange("system_prompt", current ? `${current}\n\n${result}` : result);
    messageApi.success(t("workflow.aiAssist.contextCompleteApplied"));
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
      {messageContextHolder}
      {/* 角色选择（全局 agent_roles） */}
      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
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
            color: token.colorTextTertiary,
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
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 4,
          }}
        >
          <label
            style={{ color: token.colorTextTertiary, fontSize: 12 }}
          >
            {t("workflow.props.systemPrompt")}
          </label>
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <AIAssistButton
              labelKey="optimize"
              loading={aiGenerating}
              onClick={handleAIOptimizeSystemPrompt}
              disabled={!!config.agentProfileId}
              compact
            />
            <AIAssistButton
              labelKey="contextComplete"
              loading={aiGenerating}
              onClick={handleAIContextComplete}
              disabled={!!config.agentProfileId}
              compact
            />
          </div>
        </div>
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
        <Button
          size="small"
          type="link"
          onClick={() => setTemplateModalOpen(true)}
          style={{ padding: 0, marginTop: 4 }}
          disabled={!!config.agentProfileId}
        >
          {t("promptTemplates.selectFromLibrary")}
        </Button>
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

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.modelRole")}
        </label>
        <Select
          value={config.model_role || undefined}
          onChange={(value) => handleConfigChange("model_role", value || undefined)}
          size="small"
          style={{ width: "100%" }}
          allowClear
          placeholder={t("workflow.props.modelRolePlaceholder")}
          options={[
            { value: "quick_think", label: t("workflow.props.modelRoleQuickThink") },
            { value: "deep_think", label: t("workflow.props.modelRoleDeepThink") },
          ]}
        />
        <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 2 }}>
          {t("workflow.props.modelRoleHint")}
        </div>
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label
            style={{
              display: "block",
              color: token.colorTextTertiary,
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
              color: token.colorTextTertiary,
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
            color: token.colorTextTertiary,
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

      <div style={{ marginBottom: 12 }}>
        <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 4 }}>
          {t("workflow.props.executionMode")}
        </label>
        <Select
          value={config.execution_mode ?? "react"}
          onChange={(v) => handleConfigChange("execution_mode", v === "react" ? undefined : v)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "react", label: t("workflow.props.executionReact") },
            { value: "plan", label: t("workflow.props.executionPlan") },
          ]}
        />
        <div style={{ fontSize: 11, color: "#666", marginTop: 2 }}>
          {t(`workflow.props.execution${config.execution_mode === "plan" ? "Plan" : "React"}Hint`)}
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
          id="agent-property-panel-input-79"
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.outputVarDefault")}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
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
                    background: token.colorBgContainer,
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    userSelect: "none",
                  }}
                >
                  <span style={{ fontSize: 12, fontWeight: 500, color: token.colorTextQuaternary }}>
                    🛠 {toolDef.name}
                  </span>
                  <span style={{ fontSize: 10, color: token.colorTextTertiary }}>
                    {expanded ? "▼" : "▶"}
                  </span>
                </div>
                {expanded && (
                  <div style={{ padding: "8px 10px", background: token.colorBgContainer }}>
                    <label
                      style={{
                        display: "block",
                        color: token.colorTextTertiary,
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
                        color: token.colorTextTertiary,
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
                color: token.colorTextTertiary,
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
              options={((): { value: string; label: string }[] => {
                const names = new Set<string>();
                const items: { value: string; label: string }[] = [];
                // Template tool_defs（Rhai 脚本工具）
                if (templateToolDefs && templateToolDefs.length > 0) {
                  items.push({ value: "", label: "── 模板 Rhai 工具 ──", disabled: true } as any);
                  templateToolDefs.forEach((td) => {
                    if (!names.has(td.tool_name)) {
                      names.add(td.tool_name);
                      items.push({ value: td.tool_name, label: `🦀 ${td.tool_name}` });
                    }
                  });
                }
                // 全局工具
                items.push({ value: "", label: "── 全局工具 ──", disabled: true } as any);
                (config.tools || []).forEach((td) => {
                  const name = typeof td === "string" ? td : td.name;
                  if (!names.has(name)) {
                    names.add(name);
                    items.push({ value: name, label: `🔧 ${name}` });
                  }
                });
                return items;
              })()}
            />
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 2 }}>
              {t("workflow.props.exposedToolsHint")}
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
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 2 }}>
              {t("workflow.props.maxToolRoundsHint")}
            </div>
          </div>
        </>
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

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.ragSourceIdsCount", {
            count: config.rag_source_ids?.length || 0,
          })}
        </label>
        <Select
          mode="tags"
          value={config.rag_source_ids || []}
          onChange={(value) => handleConfigChange("rag_source_ids", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.ragSourceIdsPlaceholder")}
          options={knowledgeBases
            .filter((kb) => kb.enabled)
            .map((kb) => ({
              value: `knowledge:${kb.id}`,
              label: `📚 ${kb.name}`,
            }))}
        />
        <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 2 }}>
          {t("workflow.props.ragSourceIdsHint")}
        </div>
      </div>

      <div
        style={{ borderTop: `1px solid ${token.colorBorderSecondary}`, paddingTop: 12, marginTop: 4 }}
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

      {/* 提示词模板选择 */}
      <Modal
        title={t("promptTemplates.selectFromLibrary")}
        open={templateModalOpen}
        onOk={handleApplyTemplate}
        onCancel={() => setTemplateModalOpen(false)}
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
        width={600}
      >
        {selectedTemplate
          ? (
            <div style={{ padding: "12px 0" }}>
              <p style={{ marginBottom: 8 }}>{t("promptTemplates.fillVariables")}</p>
              {Object.entries(
                selectedTemplate.variablesSchema
                  ? JSON.parse(selectedTemplate.variablesSchema)
                  : {},
              ).map(([varName, varType]) => (
                <div key={varName} style={{ marginBottom: 8 }}>
                  <label style={{ display: "block", fontSize: 12, marginBottom: 2 }}>
                    {varName} ({String(varType)})
                  </label>
                  <Input
                    placeholder={`${varName} (${String(varType)})`}
                    value={variableValues[varName] || ""}
                    onChange={(e) => setVariableValues((prev) => ({ ...prev, [varName]: e.target.value }))}
                  />
                </div>
              ))}
              {parseVariables(selectedTemplate.content).length > 0
                && Object.keys(
                    selectedTemplate.variablesSchema
                      ? JSON.parse(selectedTemplate.variablesSchema)
                      : {},
                  ).length === 0
                && (
                  <p style={{ color: token.colorWarning, fontSize: 12 }}>
                    {t("promptTemplates.hasVariables", {
                      variables: parseVariables(selectedTemplate.content).join(", "),
                    })}
                  </p>
                )}
            </div>
          )
          : (
            <div style={{ maxHeight: 400, overflowY: "auto" }}>
              {activeTemplates.length === 0
                ? (
                  <div style={{ textAlign: "center", padding: 24, color: token.colorTextTertiary }}>
                    {t("promptTemplates.noTemplates")}
                  </div>
                )
                : (
                  activeTemplates.map((template) => (
                    <div
                      key={template.id}
                      role="button"
                      tabIndex={0}
                      onClick={() => handleSelectTemplate(template)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") { handleSelectTemplate(template); }
                      }}
                      style={{
                        padding: "8px 12px",
                        cursor: "pointer",
                        borderBottom: `1px solid ${token.colorBorderSecondary}`,
                      }}
                    >
                      <div style={{ fontWeight: 500 }}>{template.name}</div>
                      <div style={{ fontSize: 12, color: token.colorTextTertiary }}>
                        {template.description || template.content.slice(0, 60) + "..."}
                      </div>
                    </div>
                  ))
                )}
            </div>
          )}
      </Modal>
    </div>
  );
};
