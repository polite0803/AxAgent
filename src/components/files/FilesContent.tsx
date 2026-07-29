// SPDX-License-Identifier: AGPL-3.0-only

import { showBackendError } from "@/lib/errorI18n";
import { useKnowledgeStore } from "@/stores";
import { Alert, App, Button, Empty, Input, Popconfirm, Space } from "antd";
import { Search, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { FILE_CATEGORIES, type FileCategory } from "./fileCategories";
import { FileList } from "./FileList";

interface FilesContentProps {
  activeCategory: FileCategory;
}

export function FilesContent({ activeCategory }: FilesContentProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const meta = FILE_CATEGORIES.find((c) => c.id === activeCategory);
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

  useEffect(() => {
    setSearch("");
    setSortKey("createdAt");
    void loadCategory(activeCategory);
  }, [activeCategory, loadCategory, setSearch, setSortKey]);

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
      const failedKeys = new Set(
        keysToDelete.filter((_, idx) => results[idx].status === "rejected"),
      );
      setSelectedRowKeys((prev) => prev.filter((k) => !failedKeys.has(k)));
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
      style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
    >
      <div className="flex-1 flex flex-col gap-3 px-4 pt-3 pb-4 overflow-hidden min-w-0">
        {error !== null && (
          <Alert
            data-testid="files-error-alert"
            type="error"
            title={error}
            closable
            onClose={clearError}
          />
        )}

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
    </div>
  );
}
