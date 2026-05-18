import { useExpertStore } from "@/stores/feature/expertStore";
import { EXPERT_CATEGORY_KEYS } from "@/types";
import type { AgentProfile, ExpertCategory } from "@/types";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { App, Button, Card, Input, Modal, Popconfirm, Popover, Select, Space, Tag, Typography } from "antd";
import { ArrowDown, ArrowUp, Check, Download, FileDown, FolderOpen, Info, Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface ExpertSelectorProps {
  open: boolean;
  onClose: () => void;
  onSelect: (roleId: string) => void;
  selectedRoleId: string | null;
}

export function ExpertSelector({
  open,
  onClose,
  onSelect,
  selectedRoleId,
}: ExpertSelectorProps) {
  const getAllRoles = useExpertStore((s) => s.getAllRoles);
  const importAgencyExperts = useExpertStore((s) => s.importAgencyExperts);
  const loadAgencyRoles = useExpertStore((s) => s.loadAgencyRoles);
  const clearAgencyExperts = useExpertStore((s) => s.clearAgencyExperts);
  const deleteAgencyExpert = useExpertStore((s) => s.deleteAgencyExpert);
  const updateAgencyExpert = useExpertStore((s) => s.updateAgencyExpert);
  const exportAgencyExperts = useExpertStore((s) => s.exportAgencyExperts);
  const agencyRoles = useExpertStore((s) => s.agencyRoles);
  const agencyLoaded = useExpertStore((s) => s.agencyLoaded);
  const removeCustomRole = useExpertStore((s) => s.removeCustomRole);
  const updateCustomRole = useExpertStore((s) => s.updateCustomRole);
  const exportCustomRoles = useExpertStore((s) => s.exportCustomRoles);
  const importBuiltinPresets = useExpertStore((s) => s.importBuiltinPresets);
  const hasFullBuiltinPresets = useExpertStore((s) => s.hasFullBuiltinPresets);

  const [searchQuery, setSearchQuery] = useState("");
  const [importPath, setImportPath] = useState("");
  const [showImport, setShowImport] = useState(false);
  const [importing, setImporting] = useState(false);
  const [sortBy, setSortBy] = useState<"name" | "category" | "source">(
    "category",
  );
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const [sourceFilter, setSourceFilter] = useState<string>("all");
  const [editingExpert, setEditingExpert] = useState<AgentProfile | null>(null);
  const [editName, setEditName] = useState("");
  const [editDesc, setEditDesc] = useState("");
  const [editPrompt, setEditPrompt] = useState("");
  const [editCategory, setEditCategory] = useState<ExpertCategory>("general");
  const [saving, setSaving] = useState(false);
  const [showAddModal, setShowAddModal] = useState(false);
  const [newRole, setNewRole] = useState<Partial<AgentProfile>>({
    id: "",
    name: "",
    description: "",
    category: "general",
    icon: "🤖",
    systemPrompt: "",
    source: "custom",
    tags: [],
  });
  const app = App.useApp();
  const { t } = useTranslation();

  // Load agency roles on mount
  useEffect(() => {
    if (!agencyLoaded) {
      loadAgencyRoles();
    }
  }, []);

  const allRoles = getAllRoles();
  const filteredRoles = (() => {
    let roles = allRoles;
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      roles = roles.filter(
        (r) =>
          r.name.toLowerCase().includes(q)
          || (r.description || "").toLowerCase().includes(q)
          || r.tags.some((t) => t.toLowerCase().includes(q)),
      );
    }
    if (sourceFilter !== "all") {
      roles = roles.filter((r) => r.source === sourceFilter);
    }
    // Sort
    const sorted = [...roles];
    const dir = sortDir === "asc" ? 1 : -1;
    if (sortBy === "name") {
      sorted.sort((a, b) => dir * a.name.localeCompare(b.name, "zh"));
    } else if (sortBy === "category") {
      const order = EXPERT_CATEGORY_KEYS;
      sorted.sort(
        (a, b) =>
          dir
          * (order.indexOf(a.category) - order.indexOf(b.category)
            || a.name.localeCompare(b.name, "zh")),
      );
    } else if (sortBy === "source") {
      const sourceOrder: Record<string, number> = {
        builtin: 0,
        agency: 1,
        custom: 2,
      };
      sorted.sort(
        (a, b) =>
          dir
          * ((sourceOrder[a.source] ?? 3) - (sourceOrder[b.source] ?? 3)
            || a.name.localeCompare(b.name, "zh")),
      );
    }
    return sorted;
  })();

  const grouped: Partial<Record<ExpertCategory, typeof filteredRoles>> = {};
  for (const role of filteredRoles) {
    if (!grouped[role.category]) {
      grouped[role.category] = [];
    }
    grouped[role.category]!.push(role);
  }

  const handleImport = async () => {
    if (!importPath.trim()) {
      return;
    }
    setImporting(true);
    try {
      const result = await importAgencyExperts(importPath.trim());
      if (result.count > 0) {
        const parts = [
          t("expertSelector.importSuccess", { count: result.count }),
        ];
        if (result.workflows_created && result.workflows_created > 0) {
          parts.push(
            t("expertSelector.importWorkflows", {
              count: result.workflows_created,
            }),
          );
        }
        if (result.tools_matched && result.tools_matched > 0) {
          parts.push(
            t("expertSelector.importTools", { count: result.tools_matched }),
          );
        }
        app.message.success(parts.join("，"));
      }
      if (result.errors.length > 0) {
        const errorPreview = result.errors.slice(0, 3).join("; ");
        app.message.warning(
          t("expertSelector.importErrors", {
            count: result.errors.length,
            errors: errorPreview,
            more: result.errors.length > 3
              ? t("expertSelector.moreErrors", {
                count: result.errors.length,
              })
              : "",
          }),
          8,
        );
        console.warn("全部导入错误:", result.errors);
      }
      setShowImport(false);
    } catch (e) {
      app.message.error(t("expertSelector.importFailed", { error: String(e) }));
    } finally {
      setImporting(false);
    }
  };

  const handleClear = async () => {
    await clearAgencyExperts();
    app.message.success(t("expertSelector.externalCleared"));
  };

  const handleExport = async () => {
    try {
      const json = await exportAgencyExperts();
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `agency-experts-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
      app.message.success(t("expertSelector.exportSuccess"));
    } catch (e) {
      app.message.error(t("expertSelector.exportFailed", { error: String(e) }));
    }
  };

  const handleExportCustom = () => {
    const json = exportCustomRoles();
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `custom-experts-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
    app.message.success(t("expertSelector.customExported"));
  };

  const handleImportFromFolder = async () => {
    try {
      const folderPath = await openDialog({
        directory: true,
        multiple: false,
        title: t("expertSelector.selectFolder"),
      });
      if (!folderPath) {
        return;
      }
      setImporting(true);
      const result = await importAgencyExperts(folderPath);
      if (result.count > 0) {
        const parts = [
          t("expertSelector.importSuccess", { count: result.count }),
        ];
        if (result.workflows_created && result.workflows_created > 0) {
          parts.push(
            t("expertSelector.importWorkflows", {
              count: result.workflows_created,
            }),
          );
        }
        if (result.tools_matched && result.tools_matched > 0) {
          parts.push(
            t("expertSelector.importTools", { count: result.tools_matched }),
          );
        }
        app.message.success(parts.join("，"));
      }
      if (result.errors.length > 0) {
        const errorPreview = result.errors.slice(0, 3).join("; ");
        app.message.warning(
          t("expertSelector.importErrors", {
            count: result.errors.length,
            errors: errorPreview,
            more: result.errors.length > 3
              ? t("expertSelector.moreErrors", {
                count: result.errors.length,
              })
              : "",
          }),
          8,
        );
      }
    } catch (e) {
      app.message.error(t("expertSelector.importFailed", { error: String(e) }));
    } finally {
      setImporting(false);
    }
  };

  const handleDeleteExpert = async (role: AgentProfile) => {
    if (role.source === "agency") {
      await deleteAgencyExpert(role.id);
      app.message.success(t("expertSelector.deleted", { name: role.name }));
    } else if (role.source === "custom") {
      removeCustomRole(role.id);
      app.message.success(t("expertSelector.deleted", { name: role.name }));
    }
  };

  const handleEditOpen = (role: AgentProfile) => {
    setEditingExpert(role);
    setEditName(role.name);
    setEditDesc(role.description ?? "");
    setEditPrompt(role.systemPrompt || "");
    setEditCategory(role.category as ExpertCategory);
  };

  const handleEditSave = async () => {
    if (!editingExpert) {
      return;
    }
    setSaving(true);
    try {
      if (editingExpert.source === "agency") {
        await updateAgencyExpert(editingExpert.id, {
          name: editName,
          description: editDesc,
          system_prompt: editPrompt,
          category: editCategory,
        });
      } else if (editingExpert.source === "custom") {
        updateCustomRole({
          ...editingExpert,
          name: editName,
          description: editDesc,
          systemPrompt: editPrompt,
          category: editCategory,
        });
      }
      app.message.success(t("expertSelector.expertUpdated"));
      setEditingExpert(null);
    } catch (e) {
      app.message.error(t("expertSelector.saveFailed", { error: String(e) }));
    } finally {
      setSaving(false);
    }
  };

  const SOURCE_LABELS: Record<string, { label: string; color: string }> = {
    builtin: { label: t("expertSelector.source.builtin"), color: "purple" },
    agency: { label: t("expertSelector.source.agency"), color: "blue" },
    custom: { label: t("expertSelector.source.custom"), color: "green" },
  };

  return (
    <Modal
      title={t("expertSelector.title")}
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
      destroyOnHidden
    >
      <div
        style={{
          display: "flex",
          gap: 6,
          marginBottom: 12,
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <Input
          id="expert-selector-input-13"
          placeholder={t("expertSelector.searchPlaceholder")}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          style={{ flex: 1, minWidth: 160 }}
          allowClear
        />
        <Select
          size="small"
          value={sourceFilter}
          onChange={(v) => setSourceFilter(v)}
          style={{ width: 90 }}
          options={[
            { value: "all", label: t("expertSelector.filter.all") },
            { value: "builtin", label: t("expertSelector.source.builtin") },
            { value: "agency", label: t("expertSelector.source.agency") },
            { value: "custom", label: t("expertSelector.source.custom") },
          ]}
        />
        <Select
          size="small"
          value={sortBy}
          onChange={(v) => setSortBy(v)}
          style={{ width: 80 }}
          options={[
            { value: "category", label: t("expertSelector.sort.category") },
            { value: "name", label: t("expertSelector.sort.name") },
            { value: "source", label: t("expertSelector.sort.source") },
          ]}
        />
        <Button
          size="small"
          type="text"
          icon={sortDir === "asc" ? <ArrowUp size={14} /> : <ArrowDown size={14} />}
          onClick={() => setSortDir((d) => (d === "asc" ? "desc" : "asc"))}
          title={sortDir === "asc"
            ? t("expertSelector.sortDir")
            : t("expertSelector.sortDirDesc")}
        />
        <Button
          size="small"
          icon={<Plus size={14} />}
          onClick={() => setShowAddModal(true)}
          title={t("expertSelector.newCustomTitle")}
        />
        <Button
          size="small"
          icon={<FileDown size={14} />}
          onClick={handleExport}
          title={t("expertSelector.exportExternal")}
        />
        <Button
          size="small"
          icon={<FolderOpen size={14} />}
          onClick={handleImportFromFolder}
          title={t("expertSelector.importFromFolder")}
        />
        <Button
          size="small"
          icon={<Download size={14} />}
          onClick={handleExportCustom}
          title={t("expertSelector.exportCustom")}
        />
        {!hasFullBuiltinPresets()
          ? (
            <Button
              size="small"
              type="primary"
              ghost
              icon={<Plus size={14} />}
              onClick={() => {
                importBuiltinPresets();
                app.message.success(t("expertSelector.builtinImported"));
              }}
              title={t("expertSelector.importBuiltin")}
            >
              {t("expertSelector.importBuiltinBtn")}
            </Button>
          )
          : (
            <Button
              size="small"
              type="text"
              disabled
              icon={<Check size={14} />}
              title={t("expertSelector.builtinAlreadyImported")}
            >
              {t("expertSelector.builtinImportedLabel")}
            </Button>
          )}
      </div>

      {/* Import section */}
      {!showImport
        ? (
          <div style={{ marginBottom: 12 }}>
            <Space size={8}>
              {agencyRoles.length > 0
                ? (
                  <Tag color="blue" style={{ cursor: "default" }}>
                    {t("expertSelector.importedCount", {
                      count: agencyRoles.length,
                    })}
                  </Tag>
                )
                : (
                  <Button
                    size="small"
                    icon={<Download size={14} />}
                    onClick={() => setShowImport(true)}
                  >
                    {t("expertSelector.import")}
                  </Button>
                )}
              {agencyRoles.length > 0 && (
                <>
                  <Button
                    size="small"
                    icon={<FolderOpen size={14} />}
                    onClick={() => setShowImport(true)}
                  >
                    {t("expertSelector.reimport")}
                  </Button>
                  <Button
                    size="small"
                    danger
                    icon={<Trash2 size={14} />}
                    onClick={handleClear}
                  >
                    {t("expertSelector.clear")}
                  </Button>
                </>
              )}
            </Space>
          </div>
        )
        : (
          <div
            style={{
              marginBottom: 12,
              padding: 12,
              background: "var(--color-background-secondary)",
              borderRadius: 8,
            }}
          >
            <Text
              type="secondary"
              style={{ fontSize: 12, display: "block", marginBottom: 6 }}
            >
              {t("expertSelector.importPathHint")}
            </Text>
            <div style={{ display: "flex", gap: 8 }}>
              <Input
                id="expert-selector-input-14"
                size="small"
                placeholder="~/"
                value={importPath}
                onChange={(e) => setImportPath(e.target.value)}
                style={{ flex: 1 }}
              />
              <Button
                size="small"
                type="primary"
                loading={importing}
                onClick={handleImport}
              >
                {t("expertSelector.import")}
              </Button>
              <Button size="small" onClick={() => setShowImport(false)}>
                {t("common.cancel")}
              </Button>
            </div>
          </div>
        )}

      <div
        style={{ maxHeight: "55vh", overflowY: "auto", paddingRight: 4 }}
        data-os-scrollbar
      >
        {filteredRoles.length === 0
          ? (
            <div
              style={{
                textAlign: "center",
                padding: "48px 0",
                color: "var(--color-text-quaternary)",
              }}
            >
              <Text
                type="secondary"
                style={{ fontSize: 14, display: "block", marginBottom: 8 }}
              >
                {t("expertSelector.noMatch")}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {searchQuery
                  ? t("expertSelector.tryModifySearch")
                  : sourceFilter !== "all"
                  ? t("expertSelector.tryChangeSource")
                  : t("expertSelector.tryCreateNew")}
              </Text>
            </div>
          )
          : (
            Object.entries(grouped).map(([category, roles]) => (
              <div key={category} style={{ marginBottom: 20 }}>
                <Text
                  type="secondary"
                  style={{
                    fontSize: 12,
                    fontWeight: 500,
                    textTransform: "uppercase",
                    letterSpacing: "0.5px",
                    marginBottom: 8,
                    display: "block",
                  }}
                >
                  {t("expertCategory." + category)}
                </Text>
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: 8,
                  }}
                >
                  {roles!.map((role) => {
                    const isSelected = selectedRoleId === role.id;
                    const isDefault = role.id === "general-assistant";
                    const isBuiltin = role.source === "builtin";
                    const sourceInfo = SOURCE_LABELS[role.source] ?? SOURCE_LABELS.builtin;

                    return (
                      <Card
                        key={role.id}
                        size="small"
                        hoverable
                        onClick={() => {
                          onSelect(role.id);
                          onClose();
                        }}
                        style={{
                          cursor: "pointer",
                          border: isSelected
                            ? "1.5px solid var(--color-border-info)"
                            : undefined,
                          background: isSelected
                            ? "var(--color-background-info)"
                            : isDefault
                            ? "var(--color-background-secondary)"
                            : undefined,
                        }}
                      >
                        <div
                          style={{
                            display: "flex",
                            alignItems: "flex-start",
                            gap: 8,
                          }}
                        >
                          <span
                            style={{ fontSize: 18, flexShrink: 0, marginTop: 1 }}
                          >
                            {role.icon}
                          </span>
                          <div style={{ flex: 1, minWidth: 0 }}>
                            <div
                              style={{
                                display: "flex",
                                alignItems: "center",
                                gap: 4,
                                flexWrap: "wrap",
                              }}
                            >
                              <Text strong style={{ fontSize: 13 }}>
                                {role.name}
                              </Text>
                              {isSelected && (
                                <Check
                                  size={14}
                                  style={{
                                    color: "var(--color-text-info)",
                                    flexShrink: 0,
                                  }}
                                />
                              )}
                              <Tag
                                color={sourceInfo.color}
                                style={{
                                  fontSize: 9,
                                  lineHeight: "14px",
                                  padding: "0 3px",
                                  margin: 0,
                                }}
                              >
                                {sourceInfo.label}
                              </Tag>
                            </div>
                            <div
                              style={{
                                display: "flex",
                                alignItems: "flex-start",
                                justifyContent: "space-between",
                              }}
                            >
                              <Text
                                type="secondary"
                                style={{
                                  fontSize: 12,
                                  display: "block",
                                  marginTop: 2,
                                  lineHeight: "1.4",
                                  flex: 1,
                                }}
                                ellipsis
                              >
                                {role.description}
                              </Text>
                              {!isBuiltin && (
                                <Space
                                  size={2}
                                  style={{ marginLeft: 4, flexShrink: 0 }}
                                  onClick={(e: React.MouseEvent) => e.stopPropagation()}
                                >
                                  <Button
                                    type="text"
                                    size="small"
                                    icon={<Pencil size={12} />}
                                    onClick={() => handleEditOpen(role)}
                                    style={{
                                      padding: "0 2px",
                                      height: 20,
                                      minWidth: 20,
                                    }}
                                  />
                                  <Popconfirm
                                    title={t("expertSelector.confirmDelete")}
                                    onConfirm={() => handleDeleteExpert(role)}
                                    okText={t("common.delete")}
                                    cancelText={t("common.cancel")}
                                  >
                                    <Button
                                      type="text"
                                      size="small"
                                      danger
                                      icon={<Trash2 size={12} />}
                                      style={{
                                        padding: "0 2px",
                                        height: 20,
                                        minWidth: 20,
                                      }}
                                    />
                                  </Popconfirm>
                                </Space>
                              )}
                            </div>
                            <div
                              style={{
                                display: "flex",
                                flexWrap: "wrap",
                                gap: 4,
                                marginTop: 6,
                              }}
                            >
                              {role.recommendedWorkflows
                                && role.recommendedWorkflows.length > 0 && (
                                <Tag
                                  color="purple"
                                  style={{
                                    fontSize: 10,
                                    lineHeight: "16px",
                                    padding: "0 4px",
                                    margin: 0,
                                  }}
                                >
                                  {t("expertSelector.workflows", {
                                    count: role.recommendedWorkflows.length,
                                  })}
                                </Tag>
                              )}
                              {role.recommendedTools
                                && role.recommendedTools.length > 0 && (
                                <Tag
                                  color="cyan"
                                  style={{
                                    fontSize: 10,
                                    lineHeight: "16px",
                                    padding: "0 4px",
                                    margin: 0,
                                  }}
                                >
                                  {t("expertSelector.tools", {
                                    count: role.recommendedTools.length,
                                  })}
                                </Tag>
                              )}
                              {role.tags.slice(0, 3).map((tag) => (
                                <Tag
                                  key={tag}
                                  style={{
                                    fontSize: 10,
                                    lineHeight: "16px",
                                    padding: "0 4px",
                                    margin: 0,
                                  }}
                                >
                                  {tag}
                                </Tag>
                              ))}
                              {role.systemPrompt && (
                                <Popover
                                  title={`${role.icon} ${role.name} - ${t("expertSelector.capabilityDetail")}`}
                                  content={
                                    <div
                                      style={{
                                        maxWidth: 360,
                                        maxHeight: 200,
                                        overflowY: "auto",
                                        fontSize: 12,
                                        lineHeight: 1.6,
                                        whiteSpace: "pre-wrap",
                                      }}
                                    >
                                      {role.systemPrompt.slice(0, 600)}
                                      {role.systemPrompt.length > 600
                                        ? "..."
                                        : ""}
                                    </div>
                                  }
                                  trigger="click"
                                >
                                  <Tag
                                    color="blue"
                                    style={{
                                      fontSize: 10,
                                      lineHeight: "16px",
                                      padding: "0 4px",
                                      margin: 0,
                                      cursor: "pointer",
                                    }}
                                    onClick={(e: React.MouseEvent) => e.stopPropagation()}
                                  >
                                    <Info size={10} style={{ marginRight: 2 }} /> {t("expertSelector.detail")}
                                  </Tag>
                                </Popover>
                              )}
                            </div>
                          </div>
                        </div>
                      </Card>
                    );
                  })}
                </div>
              </div>
            ))
          )}
      </div>

      {/* Edit Expert Modal */}
      <Modal
        title={`${t("expertSelector.editTitle")}: ${editingExpert?.name || ""}`}
        open={!!editingExpert}
        onCancel={() => setEditingExpert(null)}
        onOk={handleEditSave}
        confirmLoading={saving}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
        width={560}
        destroyOnHidden
      >
        {editingExpert && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <label
                style={{
                  display: "block",
                  fontSize: 12,
                  color: "#999",
                  marginBottom: 4,
                }}
              >
                {t("expertSelector.name")}
              </label>
              <Input
                id="expert-selector-input-15"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                size="small"
              />
            </div>
            <div>
              <label
                style={{
                  display: "block",
                  fontSize: 12,
                  color: "#999",
                  marginBottom: 4,
                }}
              >
                {t("expertSelector.desc")}
              </label>
              <Input
                id="expert-selector-input-16"
                value={editDesc}
                onChange={(e) => setEditDesc(e.target.value)}
                size="small"
              />
            </div>
            <div>
              <label
                style={{
                  display: "block",
                  fontSize: 12,
                  color: "#999",
                  marginBottom: 4,
                }}
              >
                {t("expertSelector.category")}
              </label>
              <Select
                value={editCategory}
                onChange={(v) => setEditCategory(v)}
                size="small"
                style={{ width: "100%" }}
                options={EXPERT_CATEGORY_KEYS.map((k) => ({
                  value: k,
                  label: t("expertCategory." + k),
                }))}
              />
            </div>
            <div>
              <label
                style={{
                  display: "block",
                  fontSize: 12,
                  color: "#999",
                  marginBottom: 4,
                }}
              >
                {t("expertSelector.prompt")}
              </label>
              <Input.TextArea
                id="expert-selector-input-textarea-17"
                value={editPrompt}
                onChange={(e) => setEditPrompt(e.target.value)}
                rows={10}
                size="small"
              />
            </div>
          </div>
        )}
      </Modal>

      {/* New Custom Expert Modal */}
      <Modal
        title={t("expertSelector.newCustomTitle")}
        open={showAddModal}
        onCancel={() => setShowAddModal(false)}
        onOk={() => {
          // eslint-disable-next-line react-doctor/rendering-hydration-mismatch-time
          const role: AgentProfile = {
            id: newRole.id || `custom-${Date.now()}`,
            name: newRole.name || t("expertSelector.unnamed"),
            description: newRole.description || "",
            category: newRole.category || "general",
            icon: newRole.icon || "🤖",
            systemPrompt: newRole.systemPrompt || "",
            source: "custom",
            agentRole: null,
            tags: newRole.tags || [],
            sortOrder: 0,
            isEnabled: true,
            // eslint-disable-next-line react-doctor/rendering-hydration-mismatch-time
            createdAt: Date.now(),
            // eslint-disable-next-line react-doctor/rendering-hydration-mismatch-time
            updatedAt: Date.now(),
            suggestedProviderId: newRole.suggestedProviderId,
            suggestedModelId: newRole.suggestedModelId,
            suggestedTemperature: newRole.suggestedTemperature,
            suggestedMaxTokens: newRole.suggestedMaxTokens,
            searchEnabled: newRole.searchEnabled,
            recommendPermissionMode: newRole.recommendPermissionMode,
            recommendedTools: newRole.recommendedTools,
            recommendedWorkflows: newRole.recommendedWorkflows,
          };
          useExpertStore.getState().addCustomRole(role);
          app.message.success(t("expertSelector.added"));
          setShowAddModal(false);
          setNewRole({
            id: "",
            name: "",
            description: "",
            category: "general",
            icon: "🤖",
            systemPrompt: "",
            source: "custom",
            tags: [],
          } as Partial<AgentProfile>);
        }}
        okText={t("common.create")}
        cancelText={t("common.cancel")}
        width={520}
        destroyOnHidden
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={{ display: "flex", gap: 12 }}>
            <div style={{ flex: 1 }}>
              <label
                style={{
                  display: "block",
                  fontSize: 12,
                  color: "#999",
                  marginBottom: 4,
                }}
              >
                {t("expertSelector.name")} *
              </label>
              <Input
                value={newRole.name}
                onChange={(e) => setNewRole((r) => ({ ...r, name: e.target.value }))}
                size="small"
                placeholder={t("expertSelector.namePlaceholder")}
              />
            </div>
            <div style={{ width: 60 }}>
              <label
                style={{
                  display: "block",
                  fontSize: 12,
                  color: "#999",
                  marginBottom: 4,
                }}
              >
                {t("expertSelector.icon")}
              </label>
              <Input
                id="expert-selector-input-18"
                value={newRole.icon}
                onChange={(e) => setNewRole((r) => ({ ...r, icon: e.target.value }))}
                size="small"
                maxLength={4}
              />
            </div>
          </div>
          <div>
            <label
              style={{
                display: "block",
                fontSize: 12,
                color: "#999",
                marginBottom: 4,
              }}
            >
              {t("expertSelector.desc")}
            </label>
            <Input
              id="expert-selector-input-19"
              value={newRole.description ?? ""}
              onChange={(e) => setNewRole((r) => ({ ...r, description: e.target.value }))}
              size="small"
            />
          </div>
          <div>
            <label
              style={{
                display: "block",
                fontSize: 12,
                color: "#999",
                marginBottom: 4,
              }}
            >
              {t("expertSelector.category")}
            </label>
            <Select
              value={newRole.category}
              onChange={(v) => setNewRole((r) => ({ ...r, category: v }))}
              size="small"
              style={{ width: "100%" }}
              options={EXPERT_CATEGORY_KEYS.map((k) => ({
                value: k,
                label: t("expertCategory." + k),
              }))}
            />
          </div>
          <div>
            <label
              style={{
                display: "block",
                fontSize: 12,
                color: "#999",
                marginBottom: 4,
              }}
            >
              {t("expertSelector.prompt")}
            </label>
            <Input.TextArea
              id="expert-selector-input-textarea-20"
              value={newRole.systemPrompt}
              onChange={(e) => setNewRole((r) => ({ ...r, systemPrompt: e.target.value }))}
              rows={8}
              size="small"
              placeholder={t("expertSelector.promptPlaceholder")}
            />
          </div>
        </div>
      </Modal>
    </Modal>
  );
}
