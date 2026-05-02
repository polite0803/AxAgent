import { useExpertStore } from "@/stores/feature/expertStore";
import { EXPERT_CATEGORY_LABELS, type ExpertCategory, type ExpertRole } from "@/types/expert";
import { App, Button, Card, Empty, Input, Modal, Popover, Select, Space, Tag, Typography } from "antd";
import { ArrowDown, ArrowUp, ArrowUpDown, Check, Download, Pencil, Plus, Search, Trash2, Upload } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

const { Text } = Typography;

type SortField = "displayName" | "category" | "source" | "created";
type SortDirection = "asc" | "desc";

interface SortConfig {
  field: SortField;
  direction: SortDirection;
}

interface ExpertListViewProps {
  open: boolean;
  onClose: () => void;
  onSelect?: (roleId: string) => void;
  selectedRoleId?: string | null;
  showSelectMode?: boolean;
}

export function ExpertListView({
  open,
  onClose,
  onSelect,
  selectedRoleId,
  showSelectMode = false,
}: ExpertListViewProps) {
  const getAllRoles = useExpertStore((s) => s.getAllRoles);
  const customRoles = useExpertStore((s) => s.customRoles);
  const removeCustomRole = useExpertStore((s) => s.removeCustomRole);
  const exportCustomRoles = useExpertStore((s) => s.exportCustomRoles);
  const importCustomRoles = useExpertStore((s) => s.importCustomRoles);

  const [searchQuery, setSearchQuery] = useState("");
  const [showImportModal, setShowImportModal] = useState(false);
  const [importText, setImportText] = useState("");
  const [importing, setImporting] = useState(false);
  const [editingRole, setEditingRole] = useState<ExpertRole | null>(null);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showAddModal, setShowAddModal] = useState(false);
  const [newRole, setNewRole] = useState<Partial<ExpertRole>>({
    id: "",
    displayName: "",
    description: "",
    category: "general",
    icon: "\uD83E\uDD16",
    systemPrompt: "",
    source: "custom",
    tags: [],
  });
  const [groupBy, setGroupBy] = useState<"category" | "source" | "none">("category");
  const [sortConfig, setSortConfig] = useState<SortConfig>({ field: "displayName", direction: "asc" });
  const [selectedCategory, setSelectedCategory] = useState<ExpertCategory | "all">("all");
  const [selectedSource, setSelectedSource] = useState<string>("all");
  const app = App.useApp();

  useEffect(() => {
    if (!open) {
      setSearchQuery("");
      setShowImportModal(false);
      setShowEditModal(false);
      setShowAddModal(false);
    }
  }, [open]);

  const allRoles = getAllRoles();

  const filteredRoles = useMemo(() => {
    let roles = allRoles;

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      roles = roles.filter(
        (r) =>
          r.displayName.toLowerCase().includes(query)
          || r.description.toLowerCase().includes(query)
          || r.tags.some((t) => t.toLowerCase().includes(query)),
      );
    }

    if (selectedCategory !== "all") {
      roles = roles.filter((r) => r.category === selectedCategory);
    }

    if (selectedSource !== "all") {
      roles = roles.filter((r) => r.source === selectedSource);
    }

    roles.sort((a, b) => {
      let cmp = 0;
      switch (sortConfig.field) {
        case "displayName":
          cmp = a.displayName.localeCompare(b.displayName);
          break;
        case "category":
          cmp = a.category.localeCompare(b.category);
          break;
        case "source":
          cmp = a.source.localeCompare(b.source);
          break;
        default:
          cmp = 0;
      }
      return sortConfig.direction === "asc" ? cmp : -cmp;
    });

    return roles;
  }, [allRoles, searchQuery, selectedCategory, selectedSource, sortConfig]);

  const grouped = useMemo(() => {
    if (groupBy === "none") {
      return { all: filteredRoles };
    }
    const result: Record<string, ExpertRole[]> = {};
    for (const role of filteredRoles) {
      const key = groupBy === "category" ? role.category : role.source;
      if (!result[key]) { result[key] = []; }
      result[key].push(role);
    }
    return result;
  }, [filteredRoles, groupBy]);

  const handleExport = () => {
    const json = exportCustomRoles();
    if (!customRoles.length) {
      app.message.warning("没有可导出的自定义专家");
      return;
    }
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `expert-roles-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    app.message.success(`已导出 ${customRoles.length} 个专家`);
  };

  const handleImport = async () => {
    if (!importText.trim()) { return; }
    setImporting(true);
    try {
      const result = importCustomRoles(importText);
      if (result.count > 0) {
        app.message.success(`成功导入 ${result.count} 个专家`);
      }
      if (result.errors.length > 0) {
        app.message.warning(`导入完成，但有 ${result.errors.length} 个错误`);
      }
      setShowImportModal(false);
      setImportText("");
    } finally {
      setImporting(false);
    }
  };

  const handleDelete = (role: ExpertRole) => {
    Modal.confirm({
      title: "确认删除",
      content: `确定要删除专家「${role.displayName}」吗？`,
      okText: "删除",
      okType: "danger",
      onOk: () => {
        removeCustomRole(role.id);
        app.message.success("已删除");
      },
    });
  };

  const handleEdit = (role: ExpertRole) => {
    setEditingRole({ ...role });
    setShowEditModal(true);
  };

  const handleSaveEdit = () => {
    if (!editingRole) { return; }
    const store = useExpertStore.getState();
    store.updateCustomRole(editingRole);
    app.message.success("已保存");
    setShowEditModal(false);
    setEditingRole(null);
  };

  const handleAdd = () => {
    const role: ExpertRole = {
      id: newRole.id || `custom-${Date.now()}`,
      displayName: newRole.displayName || "",
      description: newRole.description || "",
      category: newRole.category || "general",
      icon: newRole.icon || "\uD83E\uDD16",
      systemPrompt: newRole.systemPrompt || "",
      source: "custom",
      tags: newRole.tags || [],
      suggestedProviderId: newRole.suggestedProviderId,
      suggestedModelId: newRole.suggestedModelId,
      suggestedTemperature: newRole.suggestedTemperature,
      suggestedMaxTokens: newRole.suggestedMaxTokens,
      searchEnabled: newRole.searchEnabled,
      recommendPermissionMode: newRole.recommendPermissionMode,
      recommendedTools: newRole.recommendedTools,
      recommendedWorkflows: newRole.recommendedWorkflows,
    };
    const store = useExpertStore.getState();
    store.addCustomRole(role);
    app.message.success("已添加");
    setShowAddModal(false);
    setNewRole({
      id: "",
      displayName: "",
      description: "",
      category: "general",
      icon: "\uD83E\uDD16",
      systemPrompt: "",
      source: "custom",
      tags: [],
    });
  };

  const toggleSort = (field: SortField) => {
    setSortConfig((prev) =>
      prev.field === field
        ? { field, direction: prev.direction === "asc" ? "desc" : "asc" }
        : { field, direction: "asc" }
    );
  };

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortConfig.field !== field) { return <ArrowUpDown size={12} opacity={0.4} />; }
    return sortConfig.direction === "asc" ? <ArrowUp size={12} /> : <ArrowDown size={12} />;
  };

  const SOURCE_LABELS: Record<string, { label: string; color: string }> = {
    builtin: { label: "内置", color: "purple" },
    agency: { label: "外部", color: "blue" },
    custom: { label: "自定义", color: "green" },
  };

  const categories = Object.keys(EXPERT_CATEGORY_LABELS) as ExpertCategory[];
  const sources = ["builtin", "agency", "custom"];

  const renderRoleCard = (role: ExpertRole) => {
    const isSelected = selectedRoleId === role.id;
    const isDefault = role.id === "general-assistant";
    const sourceInfo = SOURCE_LABELS[role.source] ?? SOURCE_LABELS.builtin;
    const canEdit = role.source === "custom";
    const canDelete = role.source === "custom";

    return (
      <Card
        key={role.id}
        size="small"
        hoverable={showSelectMode}
        onClick={() => {
          if (showSelectMode && onSelect) {
            onSelect(role.id);
            onClose();
          }
        }}
        style={{
          cursor: showSelectMode ? "pointer" : "default",
          border: isSelected ? "1.5px solid var(--color-border-info)" : undefined,
          background: isSelected
            ? "var(--color-background-info)"
            : isDefault
            ? "var(--color-background-secondary)"
            : undefined,
          opacity: canEdit || canDelete ? 1 : 0.85,
        }}
        bodyStyle={{ padding: 12 }}
      >
        <div style={{ display: "flex", alignItems: "flex-start", gap: 8 }}>
          <span style={{ fontSize: 20, flexShrink: 0 }}>{role.icon}</span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
              <Text strong style={{ fontSize: 13 }}>
                {role.displayName}
              </Text>
              {isSelected && <Check size={14} style={{ color: "var(--color-text-info)" }} />}
              <Tag color={sourceInfo.color} style={{ fontSize: 9, lineHeight: "14px", padding: "0 3px", margin: 0 }}>
                {sourceInfo.label}
              </Tag>
              {!showSelectMode && (canEdit || canDelete) && (
                <div style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
                  {canEdit && (
                    <Button
                      size="small"
                      type="text"
                      icon={<Pencil size={12} />}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleEdit(role);
                      }}
                      style={{ padding: "2px 4px", height: 22, width: 22 }}
                    />
                  )}
                  {canDelete && (
                    <Button
                      size="small"
                      type="text"
                      danger
                      icon={<Trash2 size={12} />}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDelete(role);
                      }}
                      style={{ padding: "2px 4px", height: 22, width: 22 }}
                    />
                  )}
                </div>
              )}
            </div>
            <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 2, lineHeight: "1.4" }} ellipsis>
              {role.description || "暂无描述"}
            </Text>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}>
              {role.tags.slice(0, 3).map((tag) => (
                <Tag key={tag} style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px", margin: 0 }}>
                  {tag}
                </Tag>
              ))}
              {role.systemPrompt && (
                <Popover
                  title={`${role.icon} ${role.displayName} - 能力详情`}
                  content={
                    <div
                      style={{
                        maxWidth: 400,
                        maxHeight: 240,
                        overflowY: "auto",
                        fontSize: 12,
                        lineHeight: 1.6,
                        whiteSpace: "pre-wrap",
                      }}
                    >
                      {role.systemPrompt}
                    </div>
                  }
                  trigger="click"
                >
                  <Tag color="blue" style={{ fontSize: 10, cursor: "pointer" }}>
                    查看详情
                  </Tag>
                </Popover>
              )}
            </div>
          </div>
        </div>
      </Card>
    );
  };

  return (
    <Modal
      title={showSelectMode ? "选择专家角色" : "专家管理"}
      open={open}
      onCancel={onClose}
      footer={null}
      width={900}
      destroyOnHidden
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <Input
            placeholder="搜索专家..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{ width: 200 }}
            allowClear
            prefix={<Search size={14} opacity={0.5} />}
          />
          <Select
            value={selectedCategory}
            onChange={setSelectedCategory}
            style={{ width: 100 }}
            options={[
              { value: "all", label: "全部分类" },
              ...categories.map((c) => ({ value: c, label: EXPERT_CATEGORY_LABELS[c] })),
            ]}
          />
          <Select
            value={selectedSource}
            onChange={setSelectedSource}
            style={{ width: 100 }}
            options={[
              { value: "all", label: "全部来源" },
              ...sources.map((s) => ({ value: s, label: SOURCE_LABELS[s].label })),
            ]}
          />
          <Select
            value={groupBy}
            onChange={setGroupBy}
            style={{ width: 100 }}
            options={[
              { value: "category", label: "按分类" },
              { value: "source", label: "按来源" },
              { value: "none", label: "不分组" },
            ]}
          />
          <div style={{ flex: 1 }} />
          {!showSelectMode && (
            <Space size={4}>
              <Button size="small" icon={<Plus size={14} />} onClick={() => setShowAddModal(true)}>
                新建
              </Button>
              <Button size="small" icon={<Upload size={14} />} onClick={() => setShowImportModal(true)}>
                导入
              </Button>
              <Button size="small" icon={<Download size={14} />} onClick={handleExport}>
                导出
              </Button>
            </Space>
          )}
        </div>

        <div
          style={{ display: "flex", gap: 8, alignItems: "center", fontSize: 11, color: "var(--color-text-secondary)" }}
        >
          <span>排序：</span>
          <Button
            type="link"
            size="small"
            style={{ fontSize: 11, padding: "0 4px", height: 20 }}
            onClick={() => toggleSort("displayName")}
          >
            名称 <SortIcon field="displayName" />
          </Button>
          <Button
            type="link"
            size="small"
            style={{ fontSize: 11, padding: "0 4px", height: 20 }}
            onClick={() => toggleSort("category")}
          >
            分类 <SortIcon field="category" />
          </Button>
          <Button
            type="link"
            size="small"
            style={{ fontSize: 11, padding: "0 4px", height: 20 }}
            onClick={() => toggleSort("source")}
          >
            来源 <SortIcon field="source" />
          </Button>
          <span style={{ marginLeft: 8 }}>共 {filteredRoles.length} 个专家</span>
        </div>

        <div style={{ maxHeight: "60vh", overflowY: "auto", paddingRight: 4 }} data-os-scrollbar>
          {Object.keys(grouped).length === 0 ? <Empty description="没有找到匹配的专家" style={{ marginTop: 40 }} /> : (
            Object.entries(grouped).map(([key, roles]) => (
              <div key={key} style={{ marginBottom: 16 }}>
                <Text
                  type="secondary"
                  style={{
                    fontSize: 11,
                    fontWeight: 500,
                    textTransform: "uppercase",
                    letterSpacing: "0.5px",
                    marginBottom: 8,
                    display: "block",
                  }}
                >
                  {groupBy === "category"
                    ? EXPERT_CATEGORY_LABELS[key as ExpertCategory] ?? key
                    : SOURCE_LABELS[key]?.label ?? key}
                  <span style={{ fontWeight: 400, opacity: 0.6 }}>({roles!.length})</span>
                </Text>
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: showSelectMode ? "1fr 1fr" : "1fr 1fr",
                    gap: 8,
                  }}
                >
                  {roles!.map(renderRoleCard)}
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      <Modal
        title="导入专家角色"
        open={showImportModal}
        onCancel={() => {
          setShowImportModal(false);
          setImportText("");
        }}
        onOk={handleImport}
        confirmLoading={importing}
        okText="导入"
        width={600}
      >
        <Text type="secondary" style={{ display: "block", marginBottom: 8, fontSize: 12 }}>
          粘贴 JSON 格式的专家角色数据（可从「导出」功能获取）
        </Text>
        <Input.TextArea
          value={importText}
          onChange={(e) => setImportText(e.target.value)}
          rows={10}
          placeholder='[{"id":"custom-1","displayName":"我的专家","description":"...","category":"development",...}]'
          style={{ fontFamily: "monospace", fontSize: 12 }}
        />
      </Modal>

      <Modal
        title={editingRole ? `编辑专家 - ${editingRole.displayName}` : "新建专家"}
        open={showEditModal || showAddModal}
        onCancel={() => {
          setShowEditModal(false);
          setShowAddModal(false);
          setEditingRole(null);
          setNewRole({
            id: "",
            displayName: "",
            description: "",
            category: "general",
            icon: "\uD83E\uDD16",
            systemPrompt: "",
            source: "custom",
            tags: [],
          });
        }}
        onOk={showAddModal ? handleAdd : handleSaveEdit}
        okText="保存"
        width={600}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={{ display: "flex", gap: 8 }}>
            <div style={{ flex: 1 }}>
              <label style={{ display: "block", fontSize: 12, marginBottom: 4, color: "#666" }}>ID</label>
              <Input
                value={showAddModal ? newRole.id : editingRole?.id}
                onChange={(e) =>
                  showAddModal
                    ? setNewRole({ ...newRole, id: e.target.value })
                    : setEditingRole({ ...editingRole!, id: e.target.value })}
                placeholder="唯一标识，如 my-expert"
                disabled={!showAddModal}
              />
            </div>
            <div style={{ flex: 1 }}>
              <label style={{ display: "block", fontSize: 12, marginBottom: 4, color: "#666" }}>图标</label>
              <Input
                value={showAddModal ? newRole.icon : editingRole?.icon}
                onChange={(e) =>
                  showAddModal
                    ? setNewRole({ ...newRole, icon: e.target.value })
                    : setEditingRole({ ...editingRole!, icon: e.target.value })}
                placeholder="emoji，如 🤖"
                style={{ width: 80 }}
              />
            </div>
          </div>
          <div>
            <label style={{ display: "block", fontSize: 12, marginBottom: 4, color: "#666" }}>显示名称</label>
            <Input
              value={showAddModal ? newRole.displayName : editingRole?.displayName}
              onChange={(e) =>
                showAddModal
                  ? setNewRole({ ...newRole, displayName: e.target.value })
                  : setEditingRole({ ...editingRole!, displayName: e.target.value })}
              placeholder="专家的显示名称"
            />
          </div>
          <div>
            <label style={{ display: "block", fontSize: 12, marginBottom: 4, color: "#666" }}>描述</label>
            <Input.TextArea
              value={showAddModal ? newRole.description : editingRole?.description}
              onChange={(e) =>
                showAddModal
                  ? setNewRole({ ...newRole, description: e.target.value })
                  : setEditingRole({ ...editingRole!, description: e.target.value })}
              rows={2}
              placeholder="一句话描述专家职责"
            />
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <div style={{ flex: 1 }}>
              <label style={{ display: "block", fontSize: 12, marginBottom: 4, color: "#666" }}>分类</label>
              <Select
                value={showAddModal ? newRole.category : editingRole?.category}
                onChange={(val) =>
                  showAddModal
                    ? setNewRole({ ...newRole, category: val })
                    : setEditingRole({ ...editingRole!, category: val })}
                style={{ width: "100%" }}
                options={categories.map((c) => ({ value: c, label: EXPERT_CATEGORY_LABELS[c] }))}
              />
            </div>
            <div style={{ flex: 1 }}>
              <label style={{ display: "block", fontSize: 12, marginBottom: 4, color: "#666" }}>标签（逗号分隔）</label>
              <Input
                value={(showAddModal ? newRole.tags : editingRole?.tags)?.join(", ")}
                onChange={(e) => {
                  const tags = e.target.value.split(",").map((t) => t.trim()).filter(Boolean);
                  showAddModal ? setNewRole({ ...newRole, tags }) : setEditingRole({ ...editingRole!, tags });
                }}
                placeholder="标签1, 标签2"
              />
            </div>
          </div>
          <div>
            <label style={{ display: "block", fontSize: 12, marginBottom: 4, color: "#666" }}>系统提示词</label>
            <Input.TextArea
              value={showAddModal ? newRole.systemPrompt : editingRole?.systemPrompt}
              onChange={(e) =>
                showAddModal
                  ? setNewRole({ ...newRole, systemPrompt: e.target.value })
                  : setEditingRole({ ...editingRole!, systemPrompt: e.target.value })}
              rows={6}
              placeholder="定义专家行为的系统提示词..."
            />
          </div>
        </div>
      </Modal>
    </Modal>
  );
}
