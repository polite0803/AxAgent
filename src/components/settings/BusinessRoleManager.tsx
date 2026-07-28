// SPDX-License-Identifier: AGPL-3.0-only

// BusinessRoleManager — 业务岗位管理组件
//
// 业务岗位（CEO/CTO/产品经理 等）表达「在组织里担什么责」，
// 与 AgentRole（executor/planner/researcher）正交。
// AgentProfile 通过 businessRoleId 绑定岗位，运行时按 4 层 prompt 拼接：
// BusinessRole.systemPrompt → AgentRole.systemPrompt → Expert.systemPrompt → 节点 inline。

import { showBackendError } from "@/lib/errorI18n";
import { message } from "@/lib/toast";
import { useBusinessRoleStore } from "@/stores/feature/businessRoleStore";
import type { BusinessRole, SaveBusinessRoleInput } from "@/types";
import { Button, Card, Empty, Input, Modal, Popconfirm, Select, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { Edit, Plus, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const emptyForm = (): SaveBusinessRoleInput => ({
  id: crypto.randomUUID(),
  name: "",
  description: null,
  responsibilities: [],
  decisionAuthority: null,
  reportsTo: null,
  managedExpertIds: [],
  requiredCertifications: [],
  activeDomains: [],
  systemPrompt: "",
  icon: "💼",
  color: null,
  source: "custom",
  sortOrder: 0,
});

/** 将逗号分隔字符串拆分为字符串数组 */
function splitList(input: string): string[] {
  return input
    .split(",")
    .flatMap((s) => {
      const r = s.trim();
      return r ? [r] : [];
    });
}

export function BusinessRoleManager() {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const fetchRoles = useBusinessRoleStore((s) => s.fetchRoles);
  const roles = useBusinessRoleStore((s) => s.roles);
  const loading = useBusinessRoleStore((s) => s.loading);
  const saveRole = useBusinessRoleStore((s) => s.saveRole);
  const deleteRole = useBusinessRoleStore((s) => s.deleteRole);

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<BusinessRole | null>(null);
  const [form, setForm] = useState<SaveBusinessRoleInput>(() => emptyForm());
  const [saving, setSaving] = useState(false);
  // 逗号分隔的临时输入态（responsibilities / certifications / managedExpertIds）
  const [respInput, setRespInput] = useState("");
  const [certInput, setCertInput] = useState("");
  const [managedInput, setManagedInput] = useState("");

  useEffect(() => {
    fetchRoles();
  }, [fetchRoles]);

  const sortedRoles = useMemo(() => {
    return [...roles].sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name));
  }, [roles]);

  const openCreate = () => {
    setEditingRole(null);
    const fresh = emptyForm();
    setForm(fresh);
    setRespInput("");
    setCertInput("");
    setManagedInput("");
    setEditorOpen(true);
  };

  const openEdit = (role: BusinessRole) => {
    setEditingRole(role);
    setForm({
      id: role.id,
      name: role.name,
      description: role.description,
      responsibilities: [...role.responsibilities],
      decisionAuthority: role.decisionAuthority ? JSON.stringify(role.decisionAuthority) : null,
      reportsTo: role.reportsTo,
      managedExpertIds: [...role.managedExpertIds],
      requiredCertifications: [...role.requiredCertifications],
      activeDomains: [...role.activeDomains],
      systemPrompt: role.systemPrompt,
      icon: role.icon,
      color: role.color,
      source: role.source,
      sortOrder: role.sortOrder,
    });
    setRespInput(role.responsibilities.join(", "));
    setCertInput(role.requiredCertifications.join(", "));
    setManagedInput(role.managedExpertIds.join(", "));
    setEditorOpen(true);
  };

  const handleSave = async () => {
    if (!form.name.trim()) {
      message.warning(t("businessRole.nameRequired"));
      return;
    }
    if (!form.systemPrompt.trim()) {
      message.warning(t("businessRole.systemPromptRequired"));
      return;
    }
    setSaving(true);
    try {
      const payload: SaveBusinessRoleInput = {
        ...form,
        responsibilities: splitList(respInput),
        requiredCertifications: splitList(certInput),
        managedExpertIds: splitList(managedInput),
      };
      await saveRole(payload);
      message.success(t("businessRole.saveSuccess"));
      setEditorOpen(false);
    } catch (e) {
      showBackendError(message, e);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteRole(id);
      message.success(t("businessRole.deleteSuccess"));
    } catch (e) {
      showBackendError(message, e);
    }
  };

  // reportsTo 候选项：除当前编辑角色外的所有角色
  const reportsToOptions = useMemo(() => {
    return roles
      .filter((r) => !editingRole || r.id !== editingRole.id)
      .map((r) => ({ value: r.id, label: r.name }));
  }, [roles, editingRole]);

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
          {t("businessRole.managerTitle")}
        </Text>
        <Button size="small" type="primary" icon={<Plus size={14} />} onClick={openCreate}>
          {t("businessRole.create")}
        </Button>
      </div>

      {loading
        ? (
          <div style={{ textAlign: "center", padding: 48 }}>
            <Spin />
          </div>
        )
        : sortedRoles.length === 0
        ? (
          <Empty
            description={t("businessRole.empty")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        )
        : (
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))",
              gap: 10,
            }}
          >
            {sortedRoles.map((role) => (
              <Card
                key={role.id}
                size="small"
                hoverable
                style={{
                  borderRadius: 10,
                  border: `1px solid ${token.colorBorderSecondary}`,
                }}
                onClick={() => openEdit(role)}
              >
                <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
                  <span style={{ fontSize: 24 }}>{role.icon || "💼"}</span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                      <Text strong style={{ fontSize: 13 }}>{role.name}</Text>
                      <Tag
                        color={role.source === "builtin" ? "blue" : "orange"}
                        style={{ fontSize: 10, lineHeight: "16px" }}
                      >
                        {role.source === "builtin"
                          ? t("businessRole.sourceBuiltin")
                          : t("businessRole.sourceCustom")}
                      </Tag>
                      {!role.isEnabled && (
                        <Tag color="default" style={{ fontSize: 10, lineHeight: "16px" }}>
                          {t("businessRole.disabled")}
                        </Tag>
                      )}
                    </div>
                    <Text
                      type="secondary"
                      style={{ fontSize: 12, display: "block", marginTop: 2 }}
                      ellipsis
                    >
                      {role.description || t("businessRole.noDesc")}
                    </Text>
                    {role.responsibilities.length > 0 && (
                      <div style={{ marginTop: 6, display: "flex", gap: 4, flexWrap: "wrap" }}>
                        {role.responsibilities.slice(0, 3).map((r, i) => (
                          <Tag key={i} style={{ fontSize: 10, lineHeight: "16px" }}>{r}</Tag>
                        ))}
                        {role.responsibilities.length > 3 && (
                          <Text type="secondary" style={{ fontSize: 10 }}>
                            +{role.responsibilities.length - 3}
                          </Text>
                        )}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", flexDirection: "column", gap: 4, alignItems: "flex-end" }}>
                    <Tooltip title={t("common.edit")}>
                      <Button
                        size="small"
                        type="text"
                        icon={<Edit size={12} />}
                        onClick={(e) => {
                          e.stopPropagation();
                          openEdit(role);
                        }}
                      />
                    </Tooltip>
                    {role.source !== "builtin" && (
                      <Popconfirm
                        title={t("businessRole.deleteConfirm")}
                        onConfirm={(e) => {
                          e?.stopPropagation();
                          handleDelete(role.id);
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
                    )}
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}

      <Modal
        title={editingRole
          ? `${t("businessRole.edit")} ${editingRole.name}`
          : t("businessRole.create")}
        open={editorOpen}
        onCancel={() => setEditorOpen(false)}
        onOk={handleSave}
        confirmLoading={saving}
        width={640}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
        destroyOnHidden
      >
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 12,
            maxHeight: "60vh",
            overflowY: "auto",
            paddingRight: 4,
          }}
        >
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "12px 16px" }}>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("common.name")} *
              </Text>
              <Input
                size="small"
                value={form.name}
                onChange={(e) => setForm((prev) => ({ ...prev, name: e.target.value }))}
              />
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("businessRole.icon")}
              </Text>
              <Input
                size="small"
                value={form.icon ?? ""}
                onChange={(e) => setForm((prev) => ({ ...prev, icon: e.target.value || null }))}
                placeholder="💼"
                maxLength={4}
              />
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("businessRole.reportsTo")}
              </Text>
              <Select
                size="small"
                style={{ width: "100%" }}
                value={form.reportsTo ?? ""}
                onChange={(v) => setForm((prev) => ({ ...prev, reportsTo: v || null }))}
                options={[{ value: "", label: t("businessRole.noReportsTo") }, ...reportsToOptions]}
                allowClear
              />
            </div>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("businessRole.activeDomains")}
              </Text>
              <Select
                mode="multiple"
                size="small"
                style={{ width: "100%" }}
                value={form.activeDomains ?? []}
                onChange={(v) => setForm((prev) => ({ ...prev, activeDomains: v }))}
                placeholder={t("businessRole.activeDomainsPlaceholder")}
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

          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("common.description")}
            </Text>
            <Input
              size="small"
              value={form.description ?? ""}
              onChange={(e) => setForm((prev) => ({ ...prev, description: e.target.value || null }))}
            />
          </div>

          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("businessRole.responsibilities")}
            </Text>
            <Input
              size="small"
              value={respInput}
              onChange={(e) => setRespInput(e.target.value)}
              placeholder={t("businessRole.responsibilitiesPlaceholder")}
            />
          </div>

          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("businessRole.requiredCertifications")}
            </Text>
            <Input
              size="small"
              value={certInput}
              onChange={(e) => setCertInput(e.target.value)}
              placeholder={t("businessRole.requiredCertificationsPlaceholder")}
            />
          </div>

          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("businessRole.managedExpertIds")}
            </Text>
            <Input
              size="small"
              value={managedInput}
              onChange={(e) => setManagedInput(e.target.value)}
              placeholder={t("businessRole.managedExpertIdsPlaceholder")}
            />
          </div>

          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("businessRole.systemPrompt")} *
            </Text>
            <Input.TextArea
              rows={6}
              size="small"
              value={form.systemPrompt}
              onChange={(e) => setForm((prev) => ({ ...prev, systemPrompt: e.target.value }))}
              placeholder={t("businessRole.systemPromptPlaceholder")}
            />
            <Text type="secondary" style={{ fontSize: 11, marginTop: 4, display: "block" }}>
              {t("businessRole.systemPromptHint")}
            </Text>
          </div>
        </div>
      </Modal>
    </div>
  );
}
