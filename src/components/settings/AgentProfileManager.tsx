// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { useUIStore } from "@/stores";
import { useAgentStore } from "@/stores/feature/agentStore";
import type {
  AgentBehaviorMode,
  AgentProfile,
  CreateAgentProfileInput,
  ExpertCategory,
  UpdateAgentProfileInput,
} from "@/types";
import {
  Button,
  Card,
  Divider,
  Empty,
  Input,
  message,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Tag,
  theme,
  Tooltip,
  Typography,
} from "antd";
import {
  Bot,
  Code,
  Database,
  Edit,
  Edit3,
  FileText,
  FolderOpen,
  Globe,
  Plus,
  Search,
  Shield,
  Trash2,
  TrendingUp,
  Workflow,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const CATEGORY_ICONS: Record<string, React.ReactNode> = {
  general: <Bot size={14} />,
  development: <Code size={14} />,
  security: <Shield size={14} />,
  data: <Database size={14} />,
  devops: <TrendingUp size={14} />,
  design: <Workflow size={14} />,
  writing: <FileText size={14} />,
  business: <Globe size={14} />,
};

const emptyProfile = (): CreateAgentProfileInput => ({
  name: "",
  description: "",
  category: "general",
  icon: "🤖",
  agentRole: "executor",
  source: "custom",
  tags: [],
});

export function AgentProfileManager() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const isSmall = deviceLayout === "mobile" || deviceLayout === "tablet";
  const [profiles, setProfiles] = useState<AgentProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingProfile, setEditingProfile] = useState<AgentProfile | null>(
    null,
  );
  const [form, setForm] = useState<CreateAgentProfileInput>(() => emptyProfile());
  const [saving, setSaving] = useState(false);
  const [roleOptions, setRoleOptions] = useState<
    { value: string; label: string; activeDomains?: string[] }[]
  >([]);
  const [expertOptions, setExpertOptions] = useState<
    { value: string; label: string }[]
  >([]);

  const loadProfiles = useAgentStore((s) => s.loadProfiles);
  const getAllProfiles = useAgentStore((s) => s.getAllProfiles);
  const updateCustomProfile = useAgentStore((s) => s.updateCustomProfile);
  const createCustomProfile = useAgentStore((s) => s.createCustomProfile);
  const deleteCustomProfile = useAgentStore((s) => s.deleteCustomProfile);

  const loadRoles = useCallback(async () => {
    try {
      const roles: { id: string; name: string; active_domains?: string[] }[] = await invoke("list_agent_roles");
      setRoleOptions(
        Array.isArray(roles) ? roles.map((r) => ({ value: r.id, label: r.name, activeDomains: r.active_domains })) : [],
      );
    } catch {
      /* fallback */
    }
  }, []);

  const [importingRoles, setImportingRoles] = useState(false);
  const [roleEditorOpen, setRoleEditorOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<
    {
      id: string;
      name: string;
      description: string;
      systemPrompt: string;
      activeDomains: string[];
      source: string;
    } | null
  >(null);
  const [editRoleSaving, setEditRoleSaving] = useState(false);
  const handleImportRoles = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (!selected) {
        return;
      }
      setImportingRoles(true);
      const res = await invoke<{
        imported: number;
        skipped: number;
        errors: string[];
      }>("import_agent_roles", { path: selected });
      if (res.imported > 0) {
        message.success(
          t("agentProfile.importSuccess", { count: res.imported }),
        );
      }
      if (res.skipped > 0 || res.errors.length > 0) {
        message.warning(
          t("agentProfile.importSkipped", {
            count: res.skipped,
            errors: res.errors.length,
            detail: res.errors.slice(0, 3).join("; "),
          }),
        );
      }
      await loadRoles();
    } catch (e) {
      message.error(t("agentProfile.importFailed", { error: String(e) }));
    } finally {
      setImportingRoles(false);
    }
  };

  const handleEditRole = useCallback((roleId: string) => {
    const role = roleOptions.find((r) => r.value === roleId);
    if (!role) { return; }
    setEditingRole({
      id: roleId,
      name: role.label,
      description: "",
      systemPrompt: "",
      activeDomains: role.activeDomains ?? [],
      source: "custom",
    });
    setRoleEditorOpen(true);
  }, [roleOptions]);

  const handleRoleEditorOk = useCallback(async () => {
    if (!editingRole) { return; }
    setEditRoleSaving(true);
    try {
      await invoke("save_agent_role", {
        id: editingRole.id,
        name: editingRole.name,
        description: editingRole.description || null,
        systemPrompt: editingRole.systemPrompt,
        activeDomains: editingRole.activeDomains,
        source: editingRole.source,
      });
      message.success(t("agentProfile.saveSuccess"));
      setRoleEditorOpen(false);
      await loadRoles();
    } catch (e: any) {
      message.error(String(e));
    } finally {
      setEditRoleSaving(false);
    }
  }, [editingRole, loadRoles]);

  const loadExperts = useCallback(async () => {
    try {
      const experts: { id: string; name: string }[] = await invoke(
        "list_agency_experts",
      );
      setExpertOptions(Array.isArray(experts) ? experts.map((e) => ({ value: e.id, label: e.name })) : []);
    } catch {
      /* fallback */
    }
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      await loadProfiles();
      setProfiles(getAllProfiles());
    } finally {
      setLoading(false);
    }
  }, [loadProfiles, getAllProfiles]);

  const loadRef = useRef(load);
  const loadRolesRef = useRef(loadRoles);
  const loadExpertsRef = useRef(loadExperts);

  useEffect(() => {
    loadRef.current = load;
  }, [load]);

  useEffect(() => {
    loadRolesRef.current = loadRoles;
  }, [loadRoles]);

  useEffect(() => {
    loadExpertsRef.current = loadExperts;
  }, [loadExperts]);

  useEffect(() => {
    loadRef.current();
    loadRolesRef.current();
    loadExpertsRef.current();
  }, []);

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return profiles.filter(
      (p) =>
        !q
        || p.name.toLowerCase().includes(q)
        || p.description?.toLowerCase().includes(q)
        || p.tags?.some((t) => t.toLowerCase().includes(q)),
    );
  }, [profiles, search]);

  const grouped = useMemo(() => {
    const groups: Record<string, AgentProfile[]> = {};
    for (const p of filtered) {
      (groups[p.category] ??= []).push(p);
    }
    return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b));
  }, [filtered]);

  const catLabel = (cat: string) =>
    t(
      `chat.workflow.agentProfile${cat.charAt(0).toUpperCase() + cat.slice(1)}`,
    );

  const openCreate = () => {
    setEditingProfile(null);
    setForm(emptyProfile());
    setEditorOpen(true);
  };

  const openEdit = (p: AgentProfile) => {
    setEditingProfile(p);
    setForm({
      name: p.name,
      description: p.description ?? "",
      category: p.category,
      icon: p.icon,
      agentRole: p.agentRole ?? "",
      source: p.source,
      tags: p.tags ?? [],
      suggestedProviderId: p.suggestedProviderId,
      suggestedModelId: p.suggestedModelId,
      suggestedTemperature: p.suggestedTemperature,
      suggestedMaxTokens: p.suggestedMaxTokens,
      searchEnabled: p.searchEnabled,
      recommendPermissionMode: p.recommendPermissionMode,
      recommendedTools: p.recommendedTools,
      disallowedTools: p.disallowedTools,
      recommendedWorkflows: p.recommendedWorkflows,
      expertId: p.expertId ?? "",
    });
    setEditorOpen(true);
  };

  const handleSave = async () => {
    if (!form.name.trim()) {
      return;
    }
    setSaving(true);
    try {
      if (editingProfile) {
        await updateCustomProfile(
          editingProfile.id,
          form as UpdateAgentProfileInput,
        );
      } else {
        await createCustomProfile(form);
      }
      setEditorOpen(false);
      await load();
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    await deleteCustomProfile(id);
    await load();
  };

  const roleLabel = (role: string) => {
    const m: Record<string, string> = {
      coordinator: "Coordinator",
      researcher: "Researcher",
      developer: "Developer",
      reviewer: "Reviewer",
      browser: "Browser",
      synthesizer: "Synthesizer",
      planner: "Planner",
      executor: "Executor",
    };
    return m[role] ?? role;
  };

  const sourceLabel = (src: string) => {
    if (src === "builtin") {
      return t("chat.workflow.agentProfileBuiltin");
    }
    if (src === "agency") {
      return t("chat.workflow.agentProfileAgency");
    }
    return t("chat.workflow.agentProfileCustom");
  };

  return (
    <div>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 16,
          flexWrap: "wrap",
          gap: 8,
        }}
      >
        <Text strong style={{ fontSize: 13, color: token.colorTextSecondary }}>
          {t("chat.workflow.agentProfileTitle")}
        </Text>
        <Space wrap>
          <Input
            id="agent-profile-manager-input-31"
            size="small"
            prefix={<Search size={12} />}
            placeholder={t("chat.workflow.agentProfileSearch")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            style={{ width: isSmall ? 120 : 180 }}
            allowClear
          />
          <Button
            size="small"
            type="primary"
            icon={<Plus size={14} />}
            onClick={openCreate}
          >
            {t("chat.workflow.agentProfileCreate")}
          </Button>
          <Button
            size="small"
            icon={<FolderOpen size={14} />}
            onClick={handleImportRoles}
            loading={importingRoles}
          >
            {t("agentProfile.import")}
          </Button>
        </Space>
      </div>

      {loading
        ? (
          <div style={{ textAlign: "center", padding: 48 }}>
            <Spin />
          </div>
        )
        : filtered.length === 0
        ? (
          <Empty
            description={t("chat.workflow.agentProfileEmpty")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        )
        : (
          grouped.map(([category, items]) => (
            <div key={category} style={{ marginBottom: 20 }}>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  marginBottom: 10,
                }}
              >
                {CATEGORY_ICONS[category]}
                <Text style={{ fontSize: 12, color: token.colorTextDescription }}>
                  {catLabel(category)} · {items.length}
                </Text>
              </div>
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: isSmall ? "1fr" : "repeat(auto-fill, minmax(320px, 1fr))",
                  gap: 10,
                }}
              >
                {items.map((p) => (
                  <Card
                    key={p.id}
                    size="small"
                    hoverable
                    style={{
                      borderRadius: 10,
                      border: `1px solid ${token.colorBorderSecondary}`,
                    }}
                    onClick={() => openEdit(p)}
                  >
                    <div
                      style={{
                        display: "flex",
                        alignItems: "flex-start",
                        gap: 10,
                      }}
                    >
                      <span style={{ fontSize: 24 }}>{p.icon || "🤖"}</span>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                          }}
                        >
                          <Text strong style={{ fontSize: 13 }}>
                            {p.name}
                          </Text>
                          {p.agentRole && (
                            <Tag style={{ fontSize: 10, lineHeight: "16px" }}>
                              {roleLabel(p.agentRole)}
                            </Tag>
                          )}
                          <Tag
                            color={p.source === "builtin"
                              ? "blue"
                              : p.source === "agency"
                              ? "purple"
                              : "orange"}
                            style={{ fontSize: 10, lineHeight: "16px" }}
                          >
                            {sourceLabel(p.source)}
                          </Tag>
                        </div>
                        <Text
                          type="secondary"
                          style={{ fontSize: 12, display: "block", marginTop: 2 }}
                          ellipsis
                        >
                          {p.description || t("chat.workflow.agentProfileNoDesc")}
                        </Text>
                        <div
                          style={{
                            marginTop: 6,
                            display: "flex",
                            gap: 4,
                            flexWrap: "wrap",
                          }}
                        >
                          {p.recommendedTools?.slice(0, 4).map((t) => (
                            <Tag
                              key={t}
                              style={{ fontSize: 10, lineHeight: "16px" }}
                            >
                              {t}
                            </Tag>
                          ))}
                          {(p.recommendedTools?.length ?? 0) > 4 && (
                            <Text type="secondary" style={{ fontSize: 10 }}>
                              +{p.recommendedTools!.length - 4}
                            </Text>
                          )}
                        </div>
                      </div>
                      <div
                        style={{
                          display: "flex",
                          flexDirection: "column",
                          gap: 4,
                          alignItems: "flex-end",
                        }}
                      >
                        <Button
                          size="small"
                          type="text"
                          icon={<Edit size={12} />}
                          onClick={(e) => {
                            e.stopPropagation();
                            openEdit(p);
                          }}
                        />
                        <Popconfirm
                          title={t("chat.workflow.agentProfileDelete")}
                          onConfirm={(e) => {
                            e?.stopPropagation();
                            handleDelete(p.id);
                          }}
                          onCancel={(e) => e?.stopPropagation()}
                          okText={t("common.delete")}
                          cancelText={t("common.cancel")}
                        >
                          <Button
                            size="small"
                            type="text"
                            danger
                            icon={<Trash2 size={12} />}
                            onClick={(e) => e.stopPropagation()}
                          />
                        </Popconfirm>
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            </div>
          ))
        )}

      {/* Role Editor Modal */}
      <Modal
        title={t("chat.workflow.agentProfileEdit")}
        open={roleEditorOpen}
        onCancel={() => setRoleEditorOpen(false)}
        onOk={handleRoleEditorOk}
        confirmLoading={editRoleSaving}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
        width={560}
        destroyOnClose
      >
        {editingRole && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>ID</Text>
              <Input size="small" value={editingRole.id} disabled />
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>{t("common.name")}</Text>
              <Input
                size="small"
                value={editingRole.name}
                onChange={(e) => setEditingRole({ ...editingRole, name: e.target.value })}
              />
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>{t("common.description")}</Text>
              <Input
                size="small"
                value={editingRole.description}
                onChange={(e) => setEditingRole({ ...editingRole, description: e.target.value })}
              />
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>{t("chat.workflow.agentProfileSystemPrompt")}</Text>
              <Input.TextArea
                rows={6}
                size="small"
                value={editingRole.systemPrompt}
                onChange={(e) => setEditingRole({ ...editingRole, systemPrompt: e.target.value })}
              />
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>{t("settings.toolAccess")}</Text>
              <Select
                mode="multiple"
                value={editingRole.activeDomains}
                onChange={(v) => setEditingRole({ ...editingRole, activeDomains: v })}
                size="small"
                style={{ width: "100%" }}
                placeholder={t("settings.toolAccess")}
                options={[
                  { value: "core", label: "Core" },
                  { value: "general", label: "General" },
                  { value: "devops", label: "Devops" },
                  { value: "ai_media", label: "AI Media" },
                  { value: "invest", label: "Invest" },
                  { value: "opc", label: "OPC" },
                ]}
              />
            </div>
          </div>
        )}
      </Modal>

      <Modal
        title={editingProfile
          ? `${t("chat.workflow.agentProfileEdit")} ${editingProfile.name}`
          : t("chat.workflow.agentProfileCreate")}
        open={editorOpen}
        onCancel={() => setEditorOpen(false)}
        onOk={handleSave}
        confirmLoading={saving}
        width={680}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
      >
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: "12px 16px",
            maxHeight: "60vh",
            overflowY: "auto",
            paddingRight: 4,
          }}
        >
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("chat.workflow.agentProfileName")} *
            </Text>
            <Input
              size="small"
              value={form.name}
              onChange={(e) => setForm((prev) => ({ ...prev, name: e.target.value }))}
            />
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("chat.workflow.agentProfileIcon")}
            </Text>
            <Input
              id="agent-profile-manager-input-32"
              size="small"
              value={form.icon}
              onChange={(e) => setForm((prev) => ({ ...prev, icon: e.target.value }))}
              placeholder="🤖"
              maxLength={4}
            />
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("chat.workflow.agentProfileCategory")}
            </Text>
            <Select
              id="agent-profile-manager-select-33"
              size="small"
              style={{ width: "100%" }}
              value={form.category}
              onChange={(v) => setForm((prev) => ({ ...prev, category: v as ExpertCategory }))}
              options={Object.entries(CATEGORY_NAMES).map(([k, v]) => ({
                value: k,
                label: t(v),
              }))}
            />
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("chat.workflow.agentProfileRole")}
            </Text>
            <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
              <div style={{ flex: 1 }}>
                <Select
                  id="agent-profile-manager-select-34"
                  size="small"
                  style={{ width: "100%" }}
                  value={form.agentRole ?? ""}
                  onChange={(v) => setForm((prev) => ({ ...prev, agentRole: v || undefined }))}
                  options={[
                    { value: "", label: t("chat.workflow.agentProfileAutoRole") },
                    ...roleOptions,
                  ]}
                  allowClear
                />
              </div>
              <Tooltip title={t("common.edit")}>
                <Button
                  type="text"
                  size="small"
                  icon={<Edit3 size={14} />}
                  onClick={() => form.agentRole && handleEditRole(form.agentRole)}
                  disabled={!form.agentRole}
                />
              </Tooltip>
            </div>
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("agentProfile.expertLabel")}
            </Text>
            <Select
              id="agent-profile-manager-select-35"
              size="small"
              style={{ width: "100%" }}
              value={form.expertId ?? ""}
              onChange={(v) => setForm((prev) => ({ ...prev, expertId: v || undefined }))}
              options={[
                { value: "", label: t("agentProfile.none") },
                ...expertOptions,
              ]}
              allowClear
            />
          </div>
          <div
            style={{
              gridColumn: "span 2",
              padding: 8,
              background: token.colorBgContainerDisabled,
              borderRadius: 6,
              fontSize: 12,
              color: token.colorTextSecondary,
              lineHeight: 1.6,
            }}
          >
            {form.agentRole || form.expertId
              ? (
                <>
                  <div style={{ fontWeight: 500, marginBottom: 4 }}>
                    {t("settings.toolAccess") + " " + t("common.inherit")}
                  </div>
                  {form.agentRole && (
                    <div>
                      {t("chat.workflow.agentProfileRole")}: <Tag>{form.agentRole}</Tag> {"\u2192"}{" "}
                      {t("common.inherit")}
                    </div>
                  )}
                  {form.expertId && (
                    <div>
                      {t("agentProfile.expertLabel")}:{" "}
                      <Tag>{expertOptions.find((e) => e.value === form.expertId)?.label ?? form.expertId}</Tag>{" "}
                      {"\u2192"} {t("common.inherit")}
                    </div>
                  )}
                  <div style={{ marginTop: 4, color: token.colorTextQuaternary }}>
                    {t("settings.toolAccess") + ": " + t("common.inherit") + " (role + expert + extra/blocked)"}
                  </div>
                </>
              )
              : (
                <div style={{ color: token.colorTextQuaternary }}>
                  {t("settings.toolAccess") + ": Core + General (" + t("common.default") + ")"}
                </div>
              )}
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("chat.workflow.agentProfileDesc")}
            </Text>
            <Input
              id="agent-profile-manager-input-36"
              size="small"
              value={form.description}
              onChange={(e) => setForm((prev) => ({ ...prev, description: e.target.value }))}
            />
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <Divider style={{ margin: "4px 0 8px", fontSize: 12 }}>
              {t("chat.workflow.agentProfileRecommendedTools")}
            </Divider>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1fr",
                gap: 8,
              }}
            >
              <div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("chat.workflow.agentProfileRecommendedTools")}
                </Text>
                <Input
                  id="agent-profile-manager-input-37"
                  size="small"
                  value={form.recommendedTools?.join(", ") ?? ""}
                  onChange={(e) =>
                    setForm((prev) => ({
                      ...prev,
                      recommendedTools: e.target.value
                        .split(",")
                        .flatMap((s) => {
                          const r = s.trim();
                          return r ? [r] : [];
                        }),
                    }))}
                />
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("chat.workflow.agentProfileDisallowedTools")}
                </Text>
                <Input
                  id="agent-profile-manager-input-38"
                  size="small"
                  value={form.disallowedTools?.join(", ") ?? ""}
                  onChange={(e) =>
                    setForm((prev) => ({
                      ...prev,
                      disallowedTools: e.target.value
                        .split(",")
                        .flatMap((s) => {
                          const r = s.trim();
                          return r ? [r] : [];
                        }),
                    }))}
                />
              </div>
            </div>
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <Divider style={{ margin: "4px 0 8px", fontSize: 12 }}>
              {t("chat.workflow.agentProfileAdvanced")}
            </Divider>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1fr",
                gap: 8,
              }}
            >
              <div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("chat.workflow.agentProfilePermission")}
                </Text>
                <Select
                  id="agent-profile-manager-select-39"
                  size="small"
                  style={{ width: "100%" }}
                  value={form.recommendPermissionMode ?? ""}
                  onChange={(v) =>
                    setForm((prev) => ({
                      ...prev,
                      recommendPermissionMode: (v || undefined) as
                        | AgentBehaviorMode
                        | undefined,
                    }))}
                  options={[
                    { value: "", label: t("common.default") },
                    {
                      value: "accept_edits",
                      label: t("chat.agent.acceptEdits"),
                    },
                    { value: "full_access", label: t("chat.agent.fullAccess") },
                  ]}
                  allowClear
                />
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("chat.workflow.agentProfileTags")}
                </Text>
                <Input
                  id="agent-profile-manager-input-40"
                  size="small"
                  value={form.tags?.join(", ") ?? ""}
                  onChange={(e) =>
                    setForm((prev) => ({
                      ...prev,
                      tags: e.target.value.split(",").flatMap((s) => {
                        const r = s.trim();
                        return r ? [r] : [];
                      }),
                    }))}
                />
              </div>
            </div>
          </div>
        </div>
      </Modal>
    </div>
  );
}

const CATEGORY_NAMES: Record<string, string> = {
  general: "chat.workflow.agentProfileGeneral",
  development: "chat.workflow.agentProfileDevelopment",
  security: "chat.workflow.agentProfileSecurity",
  data: "chat.workflow.agentProfileData",
  devops: "chat.workflow.agentProfileDevops",
  design: "chat.workflow.agentProfileDesign",
  writing: "chat.workflow.agentProfileWriting",
  business: "chat.workflow.agentProfileBusiness",
};
