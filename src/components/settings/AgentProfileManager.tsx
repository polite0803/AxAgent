import { useAgentProfileStore } from "@/stores/feature/agentProfileStore";
import type { AgentProfile, CreateAgentProfileInput, ExpertCategory, UpdateAgentProfileInput } from "@/types/agentProfile";
import { Button, Card, Divider, Empty, Input, Modal, Popconfirm, Select, Space, Spin, Tag, Typography, theme } from "antd";
import { Bot, Code, Database, Edit, FileText, Globe, Plus, Search, Shield, Trash2, TrendingUp, Workflow } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;
const { TextArea } = Input;

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
  systemPrompt: "",
  agentRole: "executor",
  source: "custom",
  tags: [],
});

export function AgentProfileManager() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [profiles, setProfiles] = useState<AgentProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingProfile, setEditingProfile] = useState<AgentProfile | null>(null);
  const [form, setForm] = useState<CreateAgentProfileInput>(emptyProfile());
  const [saving, setSaving] = useState(false);

  const store = useAgentProfileStore();

  const load = async () => {
    setLoading(true);
    try {
      await store.loadProfiles();
      setProfiles(store.getAllProfiles());
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return profiles.filter((p) =>
      !q || p.name.toLowerCase().includes(q)
      || p.description?.toLowerCase().includes(q)
      || p.tags?.some((t) => t.toLowerCase().includes(q))
    );
  }, [profiles, search]);

  const grouped = useMemo(() => {
    const groups: Record<string, AgentProfile[]> = {};
    for (const p of filtered) {
      (groups[p.category] ??= []).push(p);
    }
    return Object.entries(groups).sort(([a], [b]) => a.localeCompare(b));
  }, [filtered]);

  const catLabel = (cat: string) => t(`chat.workflow.agentProfile${cat.charAt(0).toUpperCase() + cat.slice(1)}`, CATEGORY_LABELS[cat] ?? cat);

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
      systemPrompt: p.systemPrompt,
      agentRole: p.agentRole ?? "",
      source: p.source,
      tags: p.tags ?? [],
      suggestedProviderId: p.suggestedProviderId,
      suggestedModelId: p.suggestedModelId,
      suggestedTemperature: p.suggestedTemperature,
      suggestMaxTokens: p.suggestedMaxTokens,
      searchEnabled: p.searchEnabled,
      recommendPermissionMode: p.recommendPermissionMode,
      recommendedTools: p.recommendedTools,
      disallowedTools: p.disallowedTools,
      recommendedWorkflows: p.recommendedWorkflows,
    });
    setEditorOpen(true);
  };

  const handleSave = async () => {
    if (!form.name.trim()) return;
    setSaving(true);
    try {
      if (editingProfile) {
        await store.updateCustomProfile(editingProfile.id, form as UpdateAgentProfileInput);
      } else {
        await store.createCustomProfile(form);
      }
      setEditorOpen(false);
      await load();
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    await store.deleteCustomProfile(id);
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
    if (src === "builtin") return t("chat.workflow.agentProfileBuiltin");
    if (src === "agency") return t("chat.workflow.agentProfileAgency");
    return t("chat.workflow.agentProfileCustom");
  };

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <Text strong style={{ fontSize: 13, color: token.colorTextSecondary }}>
          {t("chat.workflow.agentProfileTitle")}
        </Text>
        <Space>
          <Input
            size="small"
            prefix={<Search size={12} />}
            placeholder={t("chat.workflow.agentProfileSearch")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            style={{ width: 180 }}
            allowClear
          />
          <Button size="small" type="primary" icon={<Plus size={14} />} onClick={openCreate}>
            {t("chat.workflow.agentProfileCreate")}
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
        ? <Empty description={t("chat.workflow.agentProfileEmpty")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
        : grouped.map(([category, items]) => (
          <div key={category} style={{ marginBottom: 20 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 10 }}>
              {CATEGORY_ICONS[category]}
              <Text style={{ fontSize: 12, color: token.colorTextDescription }}>
                {catLabel(category)} · {items.length}
              </Text>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: 10 }}>
              {items.map((p) => (
                <Card
                  key={p.id}
                  size="small"
                  hoverable
                  style={{ borderRadius: 10, border: `1px solid ${token.colorBorderSecondary}` }}
                  onClick={() => openEdit(p)}
                >
                  <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
                    <span style={{ fontSize: 24 }}>{p.icon || "🤖"}</span>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        <Text strong style={{ fontSize: 13 }}>{p.name}</Text>
                        {p.agentRole && <Tag style={{ fontSize: 10, lineHeight: "16px" }}>{roleLabel(p.agentRole)}</Tag>}
                        <Tag
                          color={p.source === "builtin" ? "blue" : p.source === "agency" ? "purple" : "orange"}
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
                      <div style={{ marginTop: 6, display: "flex", gap: 4, flexWrap: "wrap" }}>
                        {p.recommendedTools?.slice(0, 4).map((t) => (
                          <Tag key={t} style={{ fontSize: 10, lineHeight: "16px" }}>{t}</Tag>
                        ))}
                        {(p.recommendedTools?.length ?? 0) > 4 && (
                          <Text type="secondary" style={{ fontSize: 10 }}>+{p.recommendedTools!.length - 4}</Text>
                        )}
                      </div>
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4, alignItems: "flex-end" }}>
                      <Button
                        size="small"
                        type="text"
                        icon={<Edit size={12} />}
                        onClick={(e) => { e.stopPropagation(); openEdit(p); }}
                      />
                      <Popconfirm
                        title={t("chat.workflow.agentProfileDelete")}
                        onConfirm={(e) => { e?.stopPropagation(); handleDelete(p.id); }}
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
        ))}

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
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "12px 16px", maxHeight: "60vh", overflowY: "auto", paddingRight: 4 }}>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileName")} *</Text>
            <Input size="small" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} />
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileIcon")}</Text>
            <Input size="small" value={form.icon} onChange={(e) => setForm({ ...form, icon: e.target.value })} placeholder="🤖" maxLength={4} />
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileCategory")}</Text>
            <Select
              size="small"
              style={{ width: "100%" }}
              value={form.category}
              onChange={(v) => setForm({ ...form, category: v as ExpertCategory })}
              options={Object.entries(CATEGORY_NAMES).map(([k, v]) => ({ value: k, label: t(v) }))}
            />
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileRole")}</Text>
            <Select
              size="small"
              style={{ width: "100%" }}
              value={form.agentRole ?? ""}
              onChange={(v) => setForm({ ...form, agentRole: v || undefined })}
              options={[
                { value: "", label: t("chat.workflow.agentProfileAutoRole") },
                ...AGENT_ROLE_OPTIONS,
              ]}
              allowClear
            />
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileDesc")}</Text>
            <Input size="small" value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} />
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileSystemPrompt")}</Text>
            <TextArea
              size="small"
              rows={4}
              value={form.systemPrompt}
              onChange={(e) => setForm({ ...form, systemPrompt: e.target.value })}
            />
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <Divider style={{ margin: "4px 0 8px", fontSize: 12 }}>{t("chat.workflow.agentProfileRecommendedTools")}</Divider>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <div>
                <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileRecommendedTools")}</Text>
                <Input
                  size="small"
                  value={form.recommendedTools?.join(", ") ?? ""}
                  onChange={(e) => setForm({ ...form, recommendedTools: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })}
                />
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileDisallowedTools")}</Text>
                <Input
                  size="small"
                  value={form.disallowedTools?.join(", ") ?? ""}
                  onChange={(e) => setForm({ ...form, disallowedTools: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })}
                />
              </div>
            </div>
          </div>
          <div style={{ gridColumn: "span 2" }}>
            <Divider style={{ margin: "4px 0 8px", fontSize: 12 }}>{t("chat.workflow.agentProfileAdvanced")}</Divider>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <div>
                <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfilePermission")}</Text>
                <Select
                  size="small"
                  style={{ width: "100%" }}
                  value={form.recommendPermissionMode ?? ""}
                  onChange={(v) => setForm({ ...form, recommendPermissionMode: v || undefined })}
                  options={[
                    { value: "", label: t("common.default") },
                    { value: "accept_edits", label: t("chat.agent.acceptEdits") },
                    { value: "full_access", label: t("chat.agent.fullAccess") },
                  ]}
                  allowClear
                />
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: 11 }}>{t("chat.workflow.agentProfileTags")}</Text>
                <Input
                  size="small"
                  value={form.tags?.join(", ") ?? ""}
                  onChange={(e) => setForm({ ...form, tags: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })}
                />
              </div>
            </div>
          </div>
        </div>
      </Modal>
    </div>
  );
}

const CATEGORY_LABELS: Record<string, string> = {
  general: "通用",
  development: "开发",
  security: "安全",
  data: "数据",
  devops: "运维",
  design: "设计",
  writing: "写作",
  business: "商业",
};

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

const AGENT_ROLE_OPTIONS = [
  { value: "coordinator", label: "Coordinator" },
  { value: "researcher", label: "Researcher" },
  { value: "developer", label: "Developer" },
  { value: "reviewer", label: "Reviewer" },
  { value: "browser", label: "Browser" },
  { value: "synthesizer", label: "Synthesizer" },
  { value: "planner", label: "Planner" },
  { value: "executor", label: "Executor" },
];
