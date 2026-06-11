// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { useCategoryStore, useConversationStore } from "@/stores";
import type { ConversationCategory } from "@/types";
import { Avatar, Button, Empty, List, message, Modal, Popconfirm, theme } from "antd";
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
          icon_type: data.icon_type,
          icon_value: data.icon_value,
          system_prompt: data.system_prompt,
          default_provider_id: data.default_provider_id,
          default_model_id: data.default_model_id,
          default_temperature: data.default_temperature,
          default_max_tokens: data.default_max_tokens,
          default_top_p: data.default_top_p,
          default_frequency_penalty: data.default_frequency_penalty,
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
          icon_type: data.icon_type,
          icon_value: data.icon_value,
          system_prompt: data.system_prompt,
          default_provider_id: data.default_provider_id,
          default_model_id: data.default_model_id,
          default_temperature: data.default_temperature,
          default_max_tokens: data.default_max_tokens,
          default_top_p: data.default_top_p,
          default_frequency_penalty: data.default_frequency_penalty,
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
      icon_type: category.icon_type,
      icon_value: category.icon_value,
      system_prompt: category.system_prompt,
      default_provider_id: category.default_provider_id,
      default_model_id: category.default_model_id,
      default_temperature: category.default_temperature,
      default_max_tokens: category.default_max_tokens,
      default_top_p: category.default_top_p,
      default_frequency_penalty: category.default_frequency_penalty,
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
            <List
              dataSource={categories}
              renderItem={(category) => (
                <List.Item
                  actions={[
                    <Tooltip title={t("chat.editCategory")} key="edit">
                      <Button
                        type="text"
                        size="small"
                        icon={<Pencil size={14} />}
                        onClick={() => openEdit(category)}
                      />
                    </Tooltip>,
                    <Popconfirm
                      key="delete"
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
                    </Popconfirm>,
                  ]}
                >
                  <List.Item.Meta
                    avatar={
                      <Avatar
                        size={28}
                        icon={<FolderOpen size={14} />}
                        style={{
                          backgroundColor: token.colorFillSecondary,
                          color: token.colorTextSecondary,
                        }}
                      />
                    }
                    title={category.name}
                    description={category.system_prompt
                      ? (
                        <span
                          style={{
                            maxWidth: 200,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            display: "inline-block",
                          }}
                        >
                          {category.system_prompt}
                        </span>
                      )
                      : undefined}
                  />
                </List.Item>
              )}
            />
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
          initialIconType={editingCategory.icon_type}
          initialIconValue={editingCategory.icon_value}
          initialSystemPrompt={editingCategory.system_prompt}
          initialDefaultProviderId={editingCategory.default_provider_id}
          initialDefaultModelId={editingCategory.default_model_id}
          initialDefaultTemperature={editingCategory.default_temperature}
          initialDefaultMaxTokens={editingCategory.default_max_tokens}
          initialDefaultTopP={editingCategory.default_top_p}
          initialDefaultFrequencyPenalty={editingCategory.default_frequency_penalty}
          confirmLoading={saving}
        />
      )}
    </>
  );
}
