import { extractTagsFromContent, TagAggregationPanel } from "@/components/wiki/TagAggregationPanel";
import { WikiSidebar } from "@/components/wiki/WikiSidebar";
import { useWikiStore } from "@/stores/feature/wikiStore";
import { NoteSearchResult } from "@/types";
import { CalendarOutlined, DownloadOutlined, ImportOutlined, PlusOutlined } from "@ant-design/icons";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Dropdown, message, Modal, theme } from "antd";
import type { MenuProps } from "antd";
import { Button, Empty, Input, List, Space } from "antd";
import { BookOpen, FolderOpen } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { WikiEditorPage } from "./WikiEditorPage";

const DEFAULT_VAULT_ID = "default";

export function WikiPage() {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const wikiIdFromUrl = searchParams.get("wikiId");

  const {
    notes,
    selectedNoteId,
    selectedVaultId,
    loading,
    error,
    loadNotes,
    searchNotes,
    createNote,
    createDailyNote,
    createNoteFromTemplate,
    loadTemplates,
    templates,
    setSelectedVaultId,
    setSelectedNoteId,
    importObsidianVault,
    exportMarkdown,
    exportHtml,
  } = useWikiStore();

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<NoteSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [activeTag, setActiveTag] = useState<string | null>(null);
  const [quickCapture, setQuickCapture] = useState("");
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [importPath, setImportPath] = useState("");
  const [importing, setImporting] = useState(false);

  useEffect(() => {
    const vaultId = wikiIdFromUrl || DEFAULT_VAULT_ID;
    if (vaultId !== selectedVaultId) {
      setSelectedVaultId(vaultId);
    }
  }, [wikiIdFromUrl]);

  useEffect(() => {
    if (selectedVaultId) {
      loadNotes(selectedVaultId);
      loadTemplates(selectedVaultId);
    }
  }, [selectedVaultId, loadNotes, loadTemplates]);

  useEffect(() => {
    if (searchQuery.trim() && selectedVaultId) {
      setIsSearching(true);
      const timer = setTimeout(async () => {
        const results = await searchNotes(selectedVaultId, searchQuery);
        setSearchResults(results);
        setIsSearching(false);
      }, 300);
      return () => clearTimeout(timer);
    } else {
      setSearchResults([]);
      setIsSearching(false);
    }
  }, [searchQuery, selectedVaultId, searchNotes]);

  const filteredByTag = useCallback(
    (noteList: typeof notes) => {
      if (!activeTag) { return noteList; }
      return noteList.filter((note) => {
        const tags = extractTagsFromContent(note.content);
        return tags.includes(activeTag);
      });
    },
    [activeTag],
  );

  const displayNotes = filteredByTag(
    searchQuery.trim() ? searchResults.map((r) => r.note) : notes,
  );

  const handleSelectNote = (noteId: string) => {
    setSelectedNoteId(noteId);
  };

  const handleCreateNote = () => {
    if (!selectedVaultId) { return; }
    const now = Date.now();
    createNote({
      vaultId: selectedVaultId,
      title: `Untitled ${new Date(now).toLocaleString()}`,
      filePath: `/untitled-${now}.md`,
      content: "",
      author: "user",
    });
  };

  const handleDailyNote = async () => {
    if (!selectedVaultId) { return; }
    const note = await createDailyNote(selectedVaultId);
    if (note) {
      setSelectedNoteId(note.id);
    }
  };

  const handleCreateFromTemplate = async (templateId: string) => {
    if (!selectedVaultId) { return; }
    const note = await createNoteFromTemplate(selectedVaultId, templateId);
    if (note) {
      setSelectedNoteId(note.id);
    }
  };

  const handleQuickCapture = async () => {
    if (!quickCapture.trim() || !selectedVaultId) { return; }
    const text = quickCapture.trim();
    const title = text.length > 50 ? text.slice(0, 50) + "..." : text;
    const now = Date.now();
    await createNote({
      vaultId: selectedVaultId,
      title,
      filePath: `/inbox/${now}.md`,
      content: text,
      author: "user",
      pageType: "inbox",
    });
    setQuickCapture("");
  };

  const handleBack = () => {
    setSelectedNoteId(null);
    setSearchQuery("");
  };

  const handleTagClick = (tag: string) => {
    setActiveTag((prev) => (prev === tag ? null : tag));
  };

  const handleBrowseVaultPath = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        setImportPath(selected as string);
      }
    } catch {
      // User cancelled
    }
  };

  const handleImport = async () => {
    if (!selectedVaultId || !importPath.trim()) { return; }
    setImporting(true);
    const stats = await importObsidianVault(selectedVaultId, importPath);
    setImporting(false);
    if (stats) {
      message.success(
        t("wiki.importResult", {
          imported: stats.imported,
          skipped: stats.skipped,
          failed: stats.failed,
        }),
      );
      setImportModalOpen(false);
      setImportPath("");
      loadNotes(selectedVaultId);
    }
  };

  const handleExportMarkdown = async () => {
    if (!selectedVaultId) { return; }
    try {
      const filePath = await save({
        defaultPath: "wiki-export",
      });
      if (filePath) {
        const stats = await exportMarkdown(selectedVaultId, filePath);
        if (stats) {
          message.success(
            t("wiki.exportResult", {
              exported: stats.exported,
              failed: stats.failed,
            }),
          );
        }
      }
    } catch {
      // User cancelled
    }
  };

  const handleExportHtml = async () => {
    if (!selectedVaultId) { return; }
    try {
      const filePath = await save({
        defaultPath: "wiki-html-export",
      });
      if (filePath) {
        const stats = await exportHtml(selectedVaultId, filePath);
        if (stats) {
          message.success(
            t("wiki.exportResult", {
              exported: stats.exported,
              failed: stats.failed,
            }),
          );
        }
      }
    } catch {
      // User cancelled
    }
  };

  const exportMenuItems: MenuProps["items"] = [
    {
      key: "markdown",
      label: t("wiki.exportMarkdown"),
      onClick: handleExportMarkdown,
    },
    {
      key: "html",
      label: t("wiki.exportHtml"),
      onClick: handleExportHtml,
    },
  ];

  return (
    <div className="h-full flex" style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}>
      {!selectedNoteId
        ? (
          <>
            <WikiSidebar
              notes={displayNotes}
              selectedNoteId={selectedNoteId}
              onSelectNote={handleSelectNote}
              onCreateNote={handleCreateNote}
              loading={loading}
            />
            {error && (
              <div className="px-3 py-2 text-xs text-red-500 bg-red-50 border-b border-red-200">
                {error}
              </div>
            )}
            <div
              className="flex-1 flex flex-col overflow-hidden border-l"
              style={{ borderColor: token.colorBorderSecondary }}
            >
              <div className="p-4 border-b" style={{ borderColor: token.colorBorderSecondary }}>
                <Space className="w-full" direction="vertical" size="small">
                  <div className="flex items-center gap-2">
                    <Input.Search
                      placeholder={t("wiki.searchPlaceholder")}
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      loading={isSearching}
                      allowClear
                      className="flex-1"
                    />
                    <Input
                      id="wiki-page-input-134"
                      placeholder={t("wiki.quickCapture")}
                      value={quickCapture}
                      onChange={(e) => setQuickCapture(e.target.value)}
                      onPressEnter={handleQuickCapture}
                      style={{ width: 200 }}
                      size="small"
                    />
                    <Button
                      size="small"
                      icon={<CalendarOutlined />}
                      onClick={handleDailyNote}
                    >
                      {t("wiki.dailyNote")}
                    </Button>
                    <Button
                      size="small"
                      icon={<ImportOutlined />}
                      onClick={() => setImportModalOpen(true)}
                    >
                      {t("wiki.import")}
                    </Button>
                    <Dropdown menu={{ items: exportMenuItems }}>
                      <Button size="small" icon={<DownloadOutlined />}>
                        {t("wiki.export")}
                      </Button>
                    </Dropdown>
                    {wikiIdFromUrl && wikiIdFromUrl !== DEFAULT_VAULT_ID && (
                      <Button
                        size="small"
                        icon={<BookOpen size={14} />}
                        onClick={() => navigate(`/llm-wiki?wikiId=${wikiIdFromUrl}`)}
                      >
                        {t("wiki.manage")}
                      </Button>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    {templates.length > 0 && (
                      <Button
                        size="small"
                        icon={<PlusOutlined />}
                        onClick={() => {
                          const first = templates[0];
                          if (first) { handleCreateFromTemplate(first.id); }
                        }}
                      >
                        {t("wiki.fromTemplate")}
                      </Button>
                    )}
                    {activeTag && (
                      <span className="text-xs" style={{ color: token.colorPrimary }}>
                        {t("wiki.filteredByTag", { tag: activeTag })}
                        <Button type="link" size="small" onClick={() => setActiveTag(null)}>
                          ✕
                        </Button>
                      </span>
                    )}
                  </div>
                  {wikiIdFromUrl && wikiIdFromUrl !== DEFAULT_VAULT_ID && (
                    <div className="text-xs" style={{ color: token.colorTextSecondary }}>
                      {t("wiki.viewingWiki", { id: wikiIdFromUrl })}
                    </div>
                  )}
                </Space>
              </div>
              <TagAggregationPanel
                notes={notes}
                onTagClick={handleTagClick}
                activeTag={activeTag}
              />
              <div className="flex-1 overflow-y-auto p-4">
                {displayNotes.length === 0 ? <Empty description={t("wiki.emptyNotes")} /> : (
                  <List
                    dataSource={displayNotes}
                    renderItem={(note) => (
                      <List.Item
                        onClick={() => handleSelectNote(note.id)}
                        className="cursor-pointer hover:bg-black/5 px-3 py-2 rounded"
                        style={{ borderRadius: token.borderRadius }}
                      >
                        <List.Item.Meta
                          title={note.title}
                          description={
                            <span className="text-xs" style={{ color: token.colorTextSecondary }}>
                              {note.author === "llm" ? t("wiki.llmNote") : t("wiki.userNote")} • {note.filePath}
                            </span>
                          }
                        />
                      </List.Item>
                    )}
                  />
                )}
              </div>
            </div>
          </>
        )
        : <WikiEditorPage noteId={selectedNoteId} onBack={handleBack} />}
      <Modal
        title={t("wiki.importObsidian")}
        open={importModalOpen}
        onOk={handleImport}
        onCancel={() => {
          setImportModalOpen(false);
          setImportPath("");
        }}
        okText={t("wiki.startImport")}
        okButtonProps={{ loading: importing, disabled: !importPath.trim() }}
      >
        <div className="flex flex-col gap-3">
          <p className="text-sm" style={{ color: token.colorTextSecondary }}>
            {t("wiki.importObsidianDesc")}
          </p>
          <div className="flex items-center gap-2">
            <Input
              id="wiki-page-input-135"
              value={importPath}
              onChange={(e) => setImportPath(e.target.value)}
              placeholder={t("wiki.vaultPath")}
              className="flex-1"
            />
            <Button
              icon={<FolderOpen size={14} />}
              onClick={handleBrowseVaultPath}
            >
              {t("wiki.browse")}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
