// SPDX-License-Identifier: AGPL-3.0-only

// 阅读列表设置面板
// 对接 useReadingListStore，提供列表管理与条目管理（CRUD + 状态切换 + 重排）

import { List } from "@/components/common/AntdList";
import { message } from "@/lib/toast";
import { useReadingListStore } from "@/stores";
import type { ReadingList, ReadingListItem } from "@/types";
import {
  App as AntdApp,
  Button,
  Card,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Tag,
  Typography,
} from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Paragraph, Text } = Typography;

// 阅读状态 → Tag 颜色映射
const STATUS_COLOR: Record<string, string> = {
  unread: "default",
  reading: "processing",
  read: "success",
  skipped: "warning",
};

/** 单个阅读条目 */
function ReadingItemRow({
  item,
  onChangeStatus,
  onEdit,
  onDelete,
}: {
  item: ReadingListItem;
  onChangeStatus: (id: string, status: string) => void;
  onEdit: (item: ReadingListItem) => void;
  onDelete: (id: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <List.Item
      actions={[
        <Select
          key="status"
          size="small"
          value={item.status}
          onChange={(val) => onChangeStatus(item.id, val)}
          style={{ width: 110 }}
          options={[
            { label: t("readingList.statusUnread"), value: "unread" },
            { label: t("readingList.statusReading"), value: "reading" },
            { label: t("readingList.statusRead"), value: "read" },
            { label: t("readingList.statusSkipped"), value: "skipped" },
          ]}
        />,
        <Button key="edit" size="small" onClick={() => onEdit(item)}>
          {t("readingList.editItem")}
        </Button>,
        <Popconfirm
          key="delete"
          title={t("common.confirm")}
          okText={t("common.confirm")}
          cancelText={t("common.cancel")}
          onConfirm={() => onDelete(item.id)}
        >
          <Button size="small" danger>
            {t("readingList.deleteItem")}
          </Button>
        </Popconfirm>,
      ]}
    >
      <List.Item.Meta
        title={
          <Space>
            <Text strong>{item.title}</Text>
            <Tag color={STATUS_COLOR[item.status] ?? "default"}>
              {t(`readingList.status${item.status.charAt(0).toUpperCase()}${item.status.slice(1)}`)}
            </Tag>
            {item.priority > 0 && (
              <Tag color="orange">
                P{item.priority}
              </Tag>
            )}
          </Space>
        }
        description={
          <Space orientation="vertical" size={0}>
            {item.notes && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {item.notes}
              </Text>
            )}
            {item.externalUrl && (
              <a
                href={item.externalUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{ fontSize: 12 }}
              >
                {item.externalUrl}
              </a>
            )}
            {item.documentId && (
              <Text type="secondary" style={{ fontSize: 11 }}>
                {t("readingList.itemDocumentId")}: {item.documentId}
              </Text>
            )}
          </Space>
        }
      />
    </List.Item>
  );
}

// ── 列表编辑 Modal ──────────────────────────────────────────────────────

function ListFormModal({
  open,
  editing,
  onClose,
  onSubmit,
}: {
  open: boolean;
  editing: ReadingList | null;
  onClose: () => void;
  onSubmit: (values: { name: string; description?: string }) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [form] = Form.useForm<{ name: string; description?: string }>();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      form.setFieldsValue({
        name: editing?.name ?? "",
        description: editing?.description ?? "",
      });
    }
  }, [open, editing, form]);

  const handleOk = useCallback(async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);
      await onSubmit(values);
      onClose();
    } catch {
      // validateFields 失败时无需处理，表单自身会显示错误
    } finally {
      setSaving(false);
    }
  }, [form, onSubmit, onClose]);

  return (
    <Modal
      title={editing ? t("readingList.edit") : t("readingList.create")}
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      confirmLoading={saving}
      okText={t("common.confirm")}
      cancelText={t("common.cancel")}
    >
      <Form form={form} layout="vertical">
        <Form.Item
          name="name"
          label={t("readingList.name")}
          rules={[{ required: true, message: t("readingList.required") }]}
        >
          <Input placeholder={t("readingList.name")} />
        </Form.Item>
        <Form.Item name="description" label={t("readingList.description")}>
          <Input.TextArea rows={3} placeholder={t("readingList.description")} />
        </Form.Item>
      </Form>
    </Modal>
  );
}

// ── 条目编辑 Modal ────────────────────────────────────────────────────

function ItemFormModal({
  open,
  editing,
  readingListId,
  onClose,
  onSubmit,
}: {
  open: boolean;
  editing: ReadingListItem | null;
  readingListId: string | null;
  onClose: () => void;
  onSubmit: (values: {
    title: string;
    notes?: string;
    externalUrl?: string;
    documentId?: string;
    priority?: number;
    status?: string;
  }) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [form] = Form.useForm<{
    title: string;
    notes?: string;
    externalUrl?: string;
    documentId?: string;
    priority?: number;
    status?: string;
  }>();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      form.setFieldsValue({
        title: editing?.title ?? "",
        notes: editing?.notes ?? "",
        externalUrl: editing?.externalUrl ?? "",
        documentId: editing?.documentId ?? "",
        priority: editing?.priority ?? 0,
        status: editing?.status ?? "unread",
      });
    }
  }, [open, editing, form]);

  const handleOk = useCallback(async () => {
    if (!readingListId) {
      return;
    }
    try {
      const values = await form.validateFields();
      setSaving(true);
      await onSubmit(values);
      onClose();
    } catch {
      // 校验失败由表单自身处理
    } finally {
      setSaving(false);
    }
  }, [form, readingListId, onSubmit, onClose]);

  return (
    <Modal
      title={editing ? t("readingList.editItem") : t("readingList.addItem")}
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      confirmLoading={saving}
      okText={t("common.confirm")}
      cancelText={t("common.cancel")}
    >
      <Form form={form} layout="vertical">
        <Form.Item
          name="title"
          label={t("readingList.itemTitle")}
          rules={[{ required: true, message: t("readingList.required") }]}
        >
          <Input placeholder={t("readingList.itemTitle")} />
        </Form.Item>
        <Form.Item name="externalUrl" label={t("readingList.itemExternalUrl")}>
          <Input placeholder={t("readingList.itemExternalUrlPlaceholder")} />
        </Form.Item>
        <Form.Item name="documentId" label={t("readingList.itemDocumentId")}>
          <Input placeholder={t("readingList.itemDocumentIdPlaceholder")} />
        </Form.Item>
        <Form.Item name="notes" label={t("readingList.itemNotes")}>
          <Input.TextArea rows={3} placeholder={t("readingList.itemNotes")} />
        </Form.Item>
        <Form.Item name="priority" label={t("readingList.itemPriority")}>
          <InputNumber min={0} max={10} style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item name="status" label={t("readingList.itemStatus")}>
          <Select
            options={[
              { label: t("readingList.statusUnread"), value: "unread" },
              { label: t("readingList.statusReading"), value: "reading" },
              { label: t("readingList.statusRead"), value: "read" },
              { label: t("readingList.statusSkipped"), value: "skipped" },
            ]}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
}

// ── 主面板 ────────────────────────────────────────────────────────────

export function ReadingListSettings() {
  const { t } = useTranslation();
  const { modal } = AntdApp.useApp();
  const {
    lists,
    items,
    selectedListId,
    loading,
    loadLists,
    createList,
    updateList,
    deleteList,
    setSelectedList,
    loadItems,
    createItem,
    updateItem,
    deleteItem,
    setItemStatus,
  } = useReadingListStore();

  const [listModalOpen, setListModalOpen] = useState(false);
  const [editingList, setEditingList] = useState<ReadingList | null>(null);
  const [itemModalOpen, setItemModalOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<ReadingListItem | null>(null);

  // 首次加载列表
  useEffect(() => {
    if (lists.length === 0) {
      loadLists();
    }
  }, [lists.length, loadLists]);

  // 选中列表时加载条目
  useEffect(() => {
    if (selectedListId) {
      loadItems(selectedListId);
    }
  }, [selectedListId, loadItems]);

  const handleCreateList = useCallback(() => {
    setEditingList(null);
    setListModalOpen(true);
  }, []);

  const handleEditList = useCallback((list: ReadingList) => {
    setEditingList(list);
    setListModalOpen(true);
  }, []);

  const handleDeleteList = useCallback(
    (list: ReadingList) => {
      modal.confirm({
        title: t("readingList.delete"),
        content: t("readingList.deleteConfirm"),
        okText: t("common.confirm"),
        cancelText: t("common.cancel"),
        okButtonProps: { danger: true },
        onOk: async () => {
          try {
            await deleteList(list.id);
            message.success(t("common.success"));
          } catch (e) {
            message.error(String(e));
          }
        },
      });
    },
    [modal, t, deleteList],
  );

  const handleSubmitList = useCallback(
    async (values: { name: string; description?: string }) => {
      try {
        if (editingList) {
          await updateList(editingList.id, {
            name: values.name,
            description: values.description ?? null,
          });
        } else {
          await createList({
            name: values.name,
            description: values.description,
          });
        }
        message.success(t("common.success"));
      } catch (e) {
        message.error(String(e));
      }
    },
    [editingList, createList, updateList, t],
  );

  const handleAddItem = useCallback(() => {
    setEditingItem(null);
    setItemModalOpen(true);
  }, []);

  const handleEditItem = useCallback((item: ReadingListItem) => {
    setEditingItem(item);
    setItemModalOpen(true);
  }, []);

  const handleDeleteItem = useCallback(
    async (id: string) => {
      try {
        await deleteItem(id);
        message.success(t("common.success"));
      } catch (e) {
        message.error(String(e));
      }
    },
    [deleteItem, t],
  );

  const handleSubmitItem = useCallback(
    async (values: {
      title: string;
      notes?: string;
      externalUrl?: string;
      documentId?: string;
      priority?: number;
      status?: string;
    }) => {
      if (!selectedListId) {
        return;
      }
      try {
        if (editingItem) {
          // UpdateReadingListItemInput 契约不含 externalUrl/documentId，
          // 这两个字段在创建时设定后不可更新；状态变更通过列表行 Select 直接走 setItemStatus
          await updateItem(editingItem.id, {
            title: values.title,
            notes: values.notes ?? null,
            priority: values.priority,
            status: values.status,
          });
        } else {
          // CreateReadingListItemInput 契约不含 status，后端默认 unread
          await createItem({
            readingListId: selectedListId,
            title: values.title,
            notes: values.notes,
            externalUrl: values.externalUrl,
            documentId: values.documentId,
            priority: values.priority,
          });
        }
        message.success(t("common.success"));
      } catch (e) {
        message.error(String(e));
      }
    },
    [selectedListId, editingItem, createItem, updateItem, t],
  );

  const handleChangeStatus = useCallback(
    async (id: string, status: string) => {
      try {
        await setItemStatus(id, status);
      } catch (e) {
        message.error(String(e));
      }
    },
    [setItemStatus, t],
  );

  const selectedList = useMemo(
    () => lists.find((l) => l.id === selectedListId) ?? null,
    [lists, selectedListId],
  );

  return (
    <div className="reading-list-settings">
      <SettingsGroup
        title={t("readingList.title")}
        extra={
          <Button size="small" type="primary" onClick={handleCreateList}>
            {t("readingList.create")}
          </Button>
        }
      >
        <div style={{ padding: 4 }}>
          {lists.length === 0
            ? <Empty description={t("readingList.empty")} />
            : (
              <List
                dataSource={lists}
                rowKey={(l) => l.id}
                renderItem={(list) => (
                  <List.Item
                    style={{
                      cursor: "pointer",
                      background: list.id === selectedListId
                        ? "var(--ant-color-fill-quaternary, #fafafa)"
                        : undefined,
                    }}
                    onClick={() => setSelectedList(list.id)}
                    actions={[
                      <Button
                        key="edit"
                        size="small"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleEditList(list);
                        }}
                      >
                        {t("readingList.edit")}
                      </Button>,
                      <Popconfirm
                        key="delete"
                        title={t("readingList.deleteConfirm")}
                        okText={t("common.confirm")}
                        cancelText={t("common.cancel")}
                        onConfirm={(e) => {
                          e?.stopPropagation();
                          handleDeleteList(list);
                        }}
                      >
                        <Button
                          size="small"
                          danger
                          onClick={(e) => e.stopPropagation()}
                        >
                          {t("readingList.delete")}
                        </Button>
                      </Popconfirm>,
                    ]}
                  >
                    <List.Item.Meta
                      title={
                        <Space>
                          <Text strong>{list.name}</Text>
                          <Tag color={list.status === "archived" ? "default" : "green"}>
                            {list.status === "archived"
                              ? t("readingList.statusArchived")
                              : t("readingList.statusActive")}
                          </Tag>
                        </Space>
                      }
                      description={
                        <Space orientation="vertical" size={0}>
                          {list.description && (
                            <Text type="secondary" style={{ fontSize: 12 }}>
                              {list.description}
                            </Text>
                          )}
                          <Text type="secondary" style={{ fontSize: 11 }}>
                            {new Date(list.createdAt).toLocaleString()}
                          </Text>
                        </Space>
                      }
                    />
                  </List.Item>
                )}
              />
            )}
        </div>
      </SettingsGroup>

      <SettingsGroup
        title={t("readingList.items")}
        extra={selectedList
          ? (
            <Button size="small" onClick={handleAddItem}>
              {t("readingList.addItem")}
            </Button>
          )
          : null}
      >
        <div style={{ padding: 4 }}>
          {!selectedListId
            ? <Empty description={t("readingList.empty")} />
            : loading
            ? (
              <div style={{ display: "flex", justifyContent: "center", padding: 24 }}>
                <Spin />
              </div>
            )
            : items.length === 0
            ? <Empty description={t("readingList.emptyItems")} />
            : (
              <Card size="small" bordered={false}>
                <List
                  dataSource={items}
                  rowKey={(it) => it.id}
                  renderItem={(item) => (
                    <ReadingItemRow
                      item={item}
                      onChangeStatus={handleChangeStatus}
                      onEdit={handleEditItem}
                      onDelete={handleDeleteItem}
                    />
                  )}
                />
              </Card>
            )}
        </div>
      </SettingsGroup>

      {selectedList && (
        <Paragraph type="secondary" style={{ fontSize: 12, padding: "0 4px" }}>
          {t("readingList.title")}: {selectedList.name}
        </Paragraph>
      )}

      <ListFormModal
        open={listModalOpen}
        editing={editingList}
        onClose={() => setListModalOpen(false)}
        onSubmit={handleSubmitList}
      />
      <ItemFormModal
        open={itemModalOpen}
        editing={editingItem}
        readingListId={selectedListId}
        onClose={() => setItemModalOpen(false)}
        onSubmit={handleSubmitItem}
      />
    </div>
  );
}
