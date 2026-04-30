import { useExpertStore } from "@/stores/feature/expertStore";
import { EXPERT_CATEGORY_LABELS } from "@/types/expert";
import type { ExpertCategory } from "@/types/expert";
import { Button, Card, Input, Modal, Popover, Space, Tag, Typography, App } from "antd";
import { Check, Download, Trash2, FolderOpen, Info } from "lucide-react";
import { useState, useEffect } from "react";

const { Text } = Typography;

interface ExpertSelectorProps {
  open: boolean;
  onClose: () => void;
  onSelect: (roleId: string) => void;
  selectedRoleId: string | null;
}

export function ExpertSelector({ open, onClose, onSelect, selectedRoleId }: ExpertSelectorProps) {
  const getAllRoles = useExpertStore((s) => s.getAllRoles);
  const importAgencyExperts = useExpertStore((s) => s.importAgencyExperts);
  const loadAgencyRoles = useExpertStore((s) => s.loadAgencyRoles);
  const clearAgencyExperts = useExpertStore((s) => s.clearAgencyExperts);
  const agencyRoles = useExpertStore((s) => s.agencyRoles);
  const agencyLoaded = useExpertStore((s) => s.agencyLoaded);

  const [searchQuery, setSearchQuery] = useState("");
  const [importPath, setImportPath] = useState("");
  const [showImport, setShowImport] = useState(false);
  const [importing, setImporting] = useState(false);
  const app = App.useApp();

  // Load agency roles on mount
  useEffect(() => {
    if (!agencyLoaded) {
      loadAgencyRoles();
    }
  }, []);

  const allRoles = getAllRoles();
  const filteredRoles = searchQuery.trim()
    ? allRoles.filter(
      (r) =>
        r.displayName.toLowerCase().includes(searchQuery.toLowerCase())
        || r.description.toLowerCase().includes(searchQuery.toLowerCase())
        || r.tags.some((t) => t.includes(searchQuery.toLowerCase())),
    )
    : allRoles;

  const grouped: Partial<Record<ExpertCategory, typeof filteredRoles>> = {};
  for (const role of filteredRoles) {
    if (!grouped[role.category]) {
      grouped[role.category] = [];
    }
    grouped[role.category]!.push(role);
  }

  const handleImport = async () => {
    if (!importPath.trim()) return;
    setImporting(true);
    try {
      const result = await importAgencyExperts(importPath.trim());
      if (result.count > 0) {
        const parts = [`成功导入 ${result.count} 个专家`];
        if (result.workflows_created && result.workflows_created > 0) {
          parts.push(`自动创建 ${result.workflows_created} 个工作流`);
        }
        if (result.tools_matched && result.tools_matched > 0) {
          parts.push(`匹配 ${result.tools_matched} 个工具`);
        }
        app.message.success(parts.join("，"));
      }
      if (result.errors.length > 0) {
        app.message.warning(`导入完成，但有 ${result.errors.length} 个错误`);
        console.warn("Import errors:", result.errors.slice(0, 5));
      }
      setShowImport(false);
    } catch (e) {
      app.message.error(`导入失败: ${String(e)}`);
    } finally {
      setImporting(false);
    }
  };

  const handleClear = async () => {
    await clearAgencyExperts();
    app.message.success("已清除所有外部专家");
  };

  const SOURCE_LABELS: Record<string, { label: string; color: string }> = {
    builtin: { label: "内置", color: "purple" },
    agency: { label: "外部", color: "blue" },
    custom: { label: "自定义", color: "green" },
  };

  return (
    <Modal
      title="选择专家角色"
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
      destroyOnHidden
    >
      <Input
        placeholder="搜索专家..."
        value={searchQuery}
        onChange={(e) => setSearchQuery(e.target.value)}
        style={{ marginBottom: 12 }}
        allowClear
      />

      {/* Import section */}
      {!showImport ? (
        <div style={{ marginBottom: 12 }}>
          <Space size={8}>
            {agencyRoles.length > 0
              ? (
                <Tag color="blue" style={{ cursor: "default" }}>
                  已导入 {agencyRoles.length} 个外部专家
                </Tag>
              )
              : (
                <Button size="small" icon={<Download size={14} />} onClick={() => setShowImport(true)}>
                  导入 agency-agents-zh
                </Button>
              )}
            {agencyRoles.length > 0 && (
              <>
                <Button size="small" icon={<FolderOpen size={14} />} onClick={() => setShowImport(true)}>
                  重新导入
                </Button>
                <Button size="small" danger icon={<Trash2 size={14} />} onClick={handleClear}>
                  清除
                </Button>
              </>
            )}
          </Space>
        </div>
      ) : (
        <div style={{ marginBottom: 12, padding: 12, background: "var(--color-background-secondary)", borderRadius: 8 }}>
          <Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 6 }}>
            输入 agency-agents-zh 本地仓库路径（如 ~/agency-agents-zh）
          </Text>
          <div style={{ display: "flex", gap: 8 }}>
            <Input
              size="small"
              placeholder="~/agency-agents-zh"
              value={importPath}
              onChange={(e) => setImportPath(e.target.value)}
              style={{ flex: 1 }}
            />
            <Button size="small" type="primary" loading={importing} onClick={handleImport}>
              导入
            </Button>
            <Button size="small" onClick={() => setShowImport(false)}>
              取消
            </Button>
          </div>
        </div>
      )}

      <div style={{ maxHeight: "55vh", overflowY: "auto", paddingRight: 4 }} data-os-scrollbar>
        {Object.entries(grouped).map(([category, roles]) => (
          <div key={category} style={{ marginBottom: 20 }}>
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
              {EXPERT_CATEGORY_LABELS[category as ExpertCategory]}
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
                      border: isSelected ? "1.5px solid var(--color-border-info)" : undefined,
                      background: isSelected ? "var(--color-background-info)" : isDefault ? "var(--color-background-secondary)" : undefined,
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "flex-start", gap: 8 }}>
                      <span style={{ fontSize: 18, flexShrink: 0, marginTop: 1 }}>{role.icon}</span>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 4, flexWrap: "wrap" }}>
                          <Text strong style={{ fontSize: 13 }}>
                            {role.displayName}
                          </Text>
                          {isSelected && <Check size={14} style={{ color: "var(--color-text-info)", flexShrink: 0 }} />}
                          <Tag color={sourceInfo.color} style={{ fontSize: 9, lineHeight: "14px", padding: "0 3px", margin: 0 }}>
                            {sourceInfo.label}
                          </Tag>
                        </div>
                        <Text
                          type="secondary"
                          style={{ fontSize: 11, display: "block", marginTop: 2, lineHeight: "1.4" }}
                          ellipsis
                        >
                          {role.description}
                        </Text>
                        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}>
                          {role.recommendedWorkflows && role.recommendedWorkflows.length > 0 && (
                            <Tag
                              color="purple"
                              style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px", margin: 0 }}
                            >
                              {role.recommendedWorkflows.length} 工作流
                            </Tag>
                          )}
                          {role.recommendedTools && role.recommendedTools.length > 0 && (
                            <Tag
                              color="cyan"
                              style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px", margin: 0 }}
                            >
                              {role.recommendedTools.length} 工具
                            </Tag>
                          )}
                          {role.tags.slice(0, 3).map((tag) => (
                            <Tag key={tag} style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px", margin: 0 }}>
                              {tag}
                            </Tag>
                          ))}
                          {role.systemPrompt && (
                            <Popover
                              title={`${role.icon} ${role.displayName} - 能力详情`}
                              content={
                                <div style={{ maxWidth: 360, maxHeight: 200, overflowY: "auto", fontSize: 12, lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                                  {role.systemPrompt.slice(0, 600)}{role.systemPrompt.length > 600 ? "..." : ""}
                                </div>
                              }
                              trigger="click"
                            >
                              <Tag
                                color="blue"
                                style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px", margin: 0, cursor: "pointer" }}
                                onClick={(e: React.MouseEvent) => e.stopPropagation()}
                              >
                                <Info size={10} style={{ marginRight: 2 }} /> 详
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
        ))}
      </div>
    </Modal>
  );
}
