// SPDX-License-Identifier: AGPL-3.0-only

import { showBackendError } from "@/lib/errorI18n";
import { getDocumentsRoot } from "@/lib/fileBrowserApi";
import { useKnowledgeStore } from "@/stores";
import { Alert, App, Button, Empty, Input, Modal, Popconfirm, Space, theme } from "antd";
import { Search, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
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
  // F-P1-2: render 阶段抛错会崩溃整个页面，改为返回 Empty + 控制台告警
  if (!meta) {
    console.warn("[FilesContent] Unhandled file category:", activeCategory);
    return (
      <div className="h-full flex items-center justify-center p-6">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("files.unknownCategory", { cat: activeCategory })}
        />
      </div>
    );
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
  // F-P2-7: 根目录加载失败时显示错误提示，避免左侧栏空白无反馈
  const [treeRootError, setTreeRootError] = useState<string | null>(null);
  // 当前预览的文件路径
  const [previewPath, setPreviewPath] = useState<string | null>(null);

  // F-P2-9: 合并初始化与分类加载为单一 useEffect，避免重复触发 loadCategory
  useEffect(() => {
    setSearch("");
    setSortKey("createdAt");
    void loadCategory(activeCategory);
  }, [activeCategory, loadCategory, setSearch, setSortKey]);

  // 异步获取文件浏览器根目录
  useEffect(() => {
    let cancelled = false;
    setTreeRootError(null);
    getDocumentsRoot()
      .then((root) => {
        if (!cancelled) { setTreeRootPath(root); }
      })
      .catch((e: unknown) => {
        // F-P2-7: 失败时设置错误状态而非静默，UI 上显示 Alert
        if (!cancelled) {
          const msg = e instanceof Error ? e.message : String(e);
          setTreeRootError(msg);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // F-P2-9: 搜索输入加 debounce（300ms），避免每次按键都触发后端调用
  const searchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const handleSearchChange = useCallback(
    (value: string) => {
      setSearch(value);
      if (searchDebounceRef.current) {
        clearTimeout(searchDebounceRef.current);
      }
      searchDebounceRef.current = setTimeout(() => {
        void loadCategory(activeCategory);
      }, 300);
    },
    [activeCategory, loadCategory, setSearch],
  );

  useEffect(() => {
    return () => {
      if (searchDebounceRef.current) {
        clearTimeout(searchDebounceRef.current);
      }
    };
  }, []);

  // F-P1-3: 用 Promise.allSettled 统计部分成功/失败，避免已成功删除不可见
  // eslint-disable-next-line react-hooks/preserve-manual-memoization
  const handleBatchDelete = useCallback(async () => {
    if (selectedRowKeys.length === 0) {
      return;
    }
    const keysToDelete = selectedRowKeys;
    const results = await Promise.allSettled(
      keysToDelete.map((key) => deleteEntry(key)),
    );
    const fulfilled = results.filter((r) => r.status === "fulfilled").length;
    const rejected = results.filter((r) => r.status === "rejected");
    if (fulfilled > 0) {
      // 只保留删除失败的行
      const failedKeys = new Set(
        keysToDelete.filter((_, idx) => results[idx].status === "rejected"),
      );
      setSelectedRowKeys((prev) => prev.filter((k) => failedKeys.has(k)));
      if (rejected.length === 0) {
        message.success(t("files.batchDeleteSuccess", { count: fulfilled }));
      } else {
        message.warning(
          t("files.batchDeletePartial", { ok: fulfilled, fail: rejected.length }),
        );
        const firstError = (rejected[0] as PromiseRejectedResult).reason;
        showBackendError(message, firstError);
      }
    } else if (rejected.length > 0) {
      showBackendError(message, (rejected[0] as PromiseRejectedResult).reason);
    }
    void loadCategory(activeCategory);
  }, [selectedRowKeys, activeCategory, loadCategory, deleteEntry, message, t]);

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
        {treeRootError
          ? (
            <div className="p-3">
              <Alert
                type="error"
                message={t("files.rootLoadFailed")}
                description={treeRootError}
                showIcon
              />
            </div>
          )
          : treeRootPath
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
