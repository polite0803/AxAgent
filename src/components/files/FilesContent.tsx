// SPDX-License-Identifier: AGPL-3.0-only

import { showBackendError } from "@/lib/errorI18n";
import { getDocumentsRoot } from "@/lib/fileBrowserApi";
import { useKnowledgeStore } from "@/stores";
import { Alert, App, Button, Input, Modal, Popconfirm, Space, theme } from "antd";
import { Search, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FILE_CATEGORIES, type FileCategory } from "./fileCategories";
import { FileList } from "./FileList";
import { FilePreview } from "./FilePreview";
import { FileTreeView } from "./FileTreeView";

interface FilesContentProps {
  activeCategory: FileCategory;
}

export function FilesContent({ activeCategory }: FilesContentProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const { token } = theme.useToken();
  const meta = FILE_CATEGORIES.find((c) => c.id === activeCategory);
  if (!meta) {
    throw new Error(`Unhandled file category: ${activeCategory}`);
  }

  const {
    rows,
    search,
    error,
    loadCategory,
    setSearch,
    setSortKey,
    clearError,
    revealEntry,
    deleteEntry,
  } = useKnowledgeStore();

  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  // 文件浏览器根目录（异步加载 documents_root）
  const [treeRootPath, setTreeRootPath] = useState<string>("");
  // 当前预览的文件路径
  const [previewPath, setPreviewPath] = useState<string | null>(null);

  // 组件挂载时初始化外部 store 状态（避免在 effect 内 setState 触发级联渲染）
  useEffect(() => {
    setSearch("");
    setSortKey("createdAt");
  }, [setSearch, setSortKey]);

  useEffect(() => {
    void loadCategory(activeCategory);
  }, [activeCategory, loadCategory]);

  // 异步获取文件浏览器根目录
  useEffect(() => {
    let cancelled = false;
    getDocumentsRoot()
      .then((root) => {
        if (!cancelled) { setTreeRootPath(root); }
      })
      .catch((e: unknown) => {
        // 获取根目录失败不致命，静默处理
        console.warn("[FilesContent] getDocumentsRoot failed:", e);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleSearchChange = (value: string) => {
    setSearch(value);
    void loadCategory(activeCategory);
  };

  // eslint-disable-next-line react-hooks/preserve-manual-memoization
  const handleBatchDelete = useCallback(async () => {
    if (selectedRowKeys.length === 0) {
      return;
    }
    try {
      await Promise.all(selectedRowKeys.map((key) => deleteEntry(key)));
      setSelectedRowKeys([]);
      message.success(
        t("files.batchDeleteSuccess", { count: selectedRowKeys.length }),
      );
      void loadCategory(activeCategory);
    } catch (e) {
      showBackendError(message, e);
    }
  }, [
    selectedRowKeys,
    activeCategory,
    loadCategory,
    deleteEntry,
    message,
    t,
  ]);

  const handleDeleteEntry = useCallback(
    // eslint-disable-next-line react-hooks/preserve-manual-memoization
    async (id: string) => {
      try {
        await deleteEntry(id);
        setSelectedRowKeys((prev) => prev.filter((k) => k !== id));
        message.success(t("files.deleteSuccess"));
        void loadCategory(activeCategory);
      } catch (e) {
        showBackendError(message, e);
      }
    },
    [activeCategory, loadCategory, deleteEntry, message, t],
  );

  return (
    <div
      data-testid="files-content"
      data-category={activeCategory}
      className="h-full flex gap-0"
    >
      {/* 左侧：文件浏览器目录树 */}
      <div
        className="shrink-0 h-full"
        style={{
          width: 280,
          borderRight: `1px solid ${token.colorBorderSecondary}`,
        }}
      >
        {treeRootPath
          ? (
            <FileTreeView
              rootPath={treeRootPath}
              onSelectFile={(p) => setPreviewPath(p)}
            />
          )
          : null}
      </div>

      {/* 右侧：原附件列表 */}
      <div className="flex-1 flex flex-col gap-3 px-2 pt-0 pb-4 overflow-hidden min-w-0">
        {error !== null && (
          <Alert
            data-testid="files-error-alert"
            type="error"
            message={error}
            closable
            onClose={clearError}
          />
        )}

        {/* Toolbar: batch delete (left) + search (right) */}
        <div className="flex items-center justify-between gap-4">
          <Space>
            <Popconfirm
              title={t("files.batchDeleteConfirm", {
                count: selectedRowKeys.length,
              })}
              onConfirm={() => void handleBatchDelete()}
              okText={t("files.confirmYes")}
              cancelText={t("files.confirmNo")}
              disabled={selectedRowKeys.length === 0}
            >
              <Button
                danger
                icon={<Trash2 size={14} />}
                disabled={selectedRowKeys.length === 0}
              >
                {t("files.batchDelete", { count: selectedRowKeys.length })}
              </Button>
            </Popconfirm>
          </Space>
          <div
            data-testid="category-search"
            data-category={activeCategory}
            style={{ maxWidth: 300 }}
          >
            <Input
              id="files-content-input-39"
              prefix={<Search size={14} />}
              placeholder={t("files.searchPlaceholder", {
                category: t(meta.labelKey),
              })}
              value={search}
              onChange={(e) => {
                handleSearchChange(e.target.value);
              }}
              allowClear
            />
          </div>
        </div>

        <div className="flex-1 overflow-hidden min-h-0">
          <FileList
            rows={rows}
            category={activeCategory}
            selectedRowKeys={selectedRowKeys}
            onSelectionChange={setSelectedRowKeys}
            onReveal={(path) => void revealEntry(path)}
            onDelete={(id) => void handleDeleteEntry(id)}
          />
        </div>
      </div>

      {/* 文件预览 Modal */}
      <Modal
        title={t("files.previewTitle")}
        open={previewPath !== null}
        onCancel={() => setPreviewPath(null)}
        footer={null}
        width={720}
        destroyOnHidden
        styles={{ body: { maxHeight: "70vh", overflow: "auto" } }}
      >
        <FilePreview path={previewPath} />
      </Modal>
    </div>
  );
}
