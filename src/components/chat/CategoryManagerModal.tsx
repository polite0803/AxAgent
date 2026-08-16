// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { message } from "@/lib/toast";
import { useCategoryStore, useConversationStore } from "@/stores";
import type { ConversationCategory } from "@/types";
import { Avatar, Button, Empty, Modal, Popconfirm, theme } from "antd";
import { FolderOpen, Pencil, Plus, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { type CategoryEditFormData, CategoryEditModal } from "./CategoryEditModal";

interface CategoryManagerModalProps {
  open: boolean;
  onClose: () => void;
}

type EditTarget = { id: string } & CategoryEditFormData;

export function CategoryManagerModal({
  open,
  onClose,
}: CategoryManagerModalProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const {
    categories,
    loading,
    fetchCategories,
    createCategory,
    updateCategory,
    deleteCategory,
  } = useCategoryStore();

  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [editingCategory, setEditingCategory] = useState<EditTarget | null>(
    null,
  );
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      void fetchCategories();
    }
  }, [open, fetchCategories]);

  const handleCreate = useCallback(
    async (data: CategoryEditFormData) => {
      setSaving(true);
      try {
        await createCategory({
          name: data.name,
          iconType: data.iconType,
          iconValue: data.iconValue,
          systemPrompt: data.systemPrompt,
          defaultProviderId: data.defaultProviderId,
          defaultModelId: data.defaultModelId,
          defaultTemperature: data.defaultTemperature,
          defaultMaxTokens: data.defaultMaxTokens,
          defaultTopP: data.defaultTopP,
          defaultFrequencyPenalty: data.defaultFrequencyPenalty,
        });
        setCreateModalOpen(false);
        message.success(t("chat.createCategory") + " " + t("common.success"));
        // 同步：新分类可能被赋值到已有对话的 categoryId
        useConversationStore.getState().fetchConversations();
      } finally {
        setSaving(false);
      }
    },
    [createCategory, t],
  );

  const handleEdit = useCallback(
    async (data: CategoryEditFormData) => {
      if (!editingCategory) {
        return;
      }
      setSaving(true);
      try {
        await updateCategory(editingCategory.id, {
          name: data.name,
          iconType: data.iconType,
          iconValue: data.iconValue,
          systemPrompt: data.systemPrompt,
          defaultProviderId: data.defaultProviderId,
          defaultModelId: data.defaultModelId,
          defaultTemperature: data.defaultTemperature,
          defaultMaxTokens: data.defaultMaxTokens,
          defaultTopP: data.defaultTopP,
          defaultFrequencyPenalty: data.defaultFrequencyPenalty,
        });
        setEditingCategory(null);
        message.success(t("chat.editCategory") + " " + t("common.success"));
        // 同步：分类重命名可能影响侧栏分类视图
        useConversationStore.getState().fetchConversations();
      } finally {
        setSaving(false);
      }
    },
    [editingCategory, updateCategory, t],
  );

  const handleDelete = useCallback(
    async (category: ConversationCategory) => {
      await deleteCategory(category.id);
      message.success(t("chat.deleteCategory") + " " + t("common.success"));
      // 同步：后端将关联对话的 categoryId 置 null，前端需要刷新
      useConversationStore.getState().fetchConversations();
    },
    [deleteCategory, t],
  );

  const openEdit = useCallback((category: ConversationCategory) => {
    setEditingCategory({
      id: category.id,
      name: category.name,
      iconType: category.iconType,
      iconValue: category.iconValue,
      systemPrompt: category.systemPrompt,
      defaultProviderId: category.defaultProviderId,
      defaultModelId: category.defaultModelId,
      defaultTemperature: category.defaultTemperature,
      defaultMaxTokens: category.defaultMaxTokens,
      defaultTopP: category.defaultTopP,
      defaultFrequencyPenalty: category.defaultFrequencyPenalty,
    });
  }, []);

  return (
    <>
      <Modal
        title={t("chat.manageCategories")}
        open={open}
        onCancel={onClose}
        footer={null}
        width={560}
        mask={{ enabled: true, blur: true }}
        destroyOnHidden
      >
        <div
          style={{
            marginBottom: 12,
            display: "flex",
            justifyContent: "flex-end",
          }}
        >
          <Button
            type="primary"
            icon={<Plus size={14} />}
            onClick={() => setCreateModalOpen(true)}
          >
            {t("chat.createCategory")}
          </Button>
        </div>

        {loading
          ? (
            <div style={{ padding: "16px 0" }}>
              {Array.from({ length: 3 }).map((_, i) => (
                <div
                  key={i}
                  className="ax-skeleton"
                  style={{ height: 48, marginBottom: 8, borderRadius: 6 }}
                />
              ))}
            </div>
          )
          : categories.length === 0
          ? (
            <Empty
              description={t("chat.noCategories")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )
          : (
            <div className="divide-y divide-gray-100">
              {categories.map((category) => (
                <div
                  key={category.id}
                  style={{
                    padding: "12px 0",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    <Avatar
                      size={28}
                      icon={<FolderOpen size={14} />}
                      style={{
                        backgroundColor: token.colorFillSecondary,
                        color: token.colorTextSecondary,
                      }}
                    />
                    <div>
                      <div style={{ fontWeight: 500 }}>{category.name}</div>
                      {category.systemPrompt
                        ? (
                          <div
                            style={{
                              color: "var(--text-secondary, rgba(0,0,0,0.45))",
                              fontSize: 13,
                              marginTop: 2,
                            }}
                          >
                            <span
                              style={{
                                maxWidth: 200,
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                display: "inline-block",
                              }}
                            >
                              {category.systemPrompt}
                            </span>
                          </div>
                        )
                        : null}
                    </div>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    <Tooltip title={t("chat.editCategory")}>
                      <Button
                        type="text"
                        size="small"
                        icon={<Pencil size={14} />}
                        onClick={() => openEdit(category)}
                      />
                    </Tooltip>
                    <Popconfirm
                      title={t("chat.deleteCategoryConfirm")}
                      onConfirm={() => handleDelete(category)}
                      okButtonProps={{ danger: true }}
                    >
                      <Tooltip title={t("chat.deleteCategory")}>
                        <Button
                          type="text"
                          size="small"
                          danger
                          icon={<Trash2 size={14} />}
                        />
                      </Tooltip>
                    </Popconfirm>
                  </div>
                </div>
              ))}
            </div>
          )}
      </Modal>

      <CategoryEditModal
        open={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
        onOk={handleCreate}
        confirmLoading={saving}
      />

      {editingCategory && (
        <CategoryEditModal
          open={!!editingCategory}
          onClose={() => setEditingCategory(null)}
          onOk={handleEdit}
          title={t("chat.editCategory")}
          initialName={editingCategory.name}
          initialIconType={editingCategory.iconType}
          initialIconValue={editingCategory.iconValue}
          initialSystemPrompt={editingCategory.systemPrompt}
          initialDefaultProviderId={editingCategory.defaultProviderId}
          initialDefaultModelId={editingCategory.defaultModelId}
          initialDefaultTemperature={editingCategory.defaultTemperature}
          initialDefaultMaxTokens={editingCategory.defaultMaxTokens}
          initialDefaultTopP={editingCategory.defaultTopP}
          initialDefaultFrequencyPenalty={editingCategory.defaultFrequencyPenalty}
          confirmLoading={saving}
        />
      )}
    </>
  );
}
