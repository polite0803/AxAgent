import { useWikiStore } from "@/stores/feature/wikiStore";
import { theme } from "antd";
import { useEffect, useState } from "react";
import { WikiSidebar } from "@/components/wiki/WikiSidebar";
import { WikiEditorPage } from "./WikiEditorPage";
import { NoteSearchResult } from "@/types";
import { Input, List, Empty, Button, Space } from "antd";
import { useTranslation } from "react-i18next";
import { useSearchParams, useNavigate } from "react-router-dom";
import { BookOpen } from "lucide-react";

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
    setSelectedVaultId,
    setSelectedNoteId,
  } = useWikiStore();

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<NoteSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  // 集成修复：从 URL 参数获取 wikiId 并作为 vault_id 加载笔记
  useEffect(() => {
    const vaultId = wikiIdFromUrl || DEFAULT_VAULT_ID;
    if (vaultId !== selectedVaultId) {
      setSelectedVaultId(vaultId);
    }
  }, [wikiIdFromUrl]);

  useEffect(() => {
    if (selectedVaultId) {
      loadNotes(selectedVaultId);
    }
  }, [selectedVaultId, loadNotes]);

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

  const displayNotes = searchQuery.trim() ? searchResults.map((r) => r.note) : notes;

  const handleSelectNote = (noteId: string) => {
    setSelectedNoteId(noteId);
  };

  const handleCreateNote = () => {
    if (!selectedVaultId) return;
    const now = Date.now();
    createNote({
      vaultId: selectedVaultId,
      title: `Untitled ${new Date(now).toLocaleString()}`,
      filePath: `/untitled-${now}.md`,
      content: "",
      author: "user",
    });
  };

  const handleBack = () => {
    setSelectedNoteId(null);
    setSearchQuery("");
  };

  return (
    <div className="h-full flex" style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}>
      {!selectedNoteId ? (
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
          <div className="flex-1 flex flex-col overflow-hidden border-l" style={{ borderColor: token.colorBorderSecondary }}>
            <div className="p-4 border-b" style={{ borderColor: token.colorBorderSecondary }}>
              <Space className="w-full" direction="vertical" size="small">
                <div className="flex items-center gap-2">
                  <Input.Search
                    placeholder={t("wiki.searchPlaceholder", "Search notes...")}
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    loading={isSearching}
                    allowClear
                    className="flex-1"
                  />
                  {wikiIdFromUrl && wikiIdFromUrl !== DEFAULT_VAULT_ID && (
                    <Button
                      size="small"
                      icon={<BookOpen size={14} />}
                      onClick={() => navigate(`/llm-wiki?wikiId=${wikiIdFromUrl}`)}
                    >
                      {t("wiki.manage", "Manage")}
                    </Button>
                  )}
                </div>
                {wikiIdFromUrl && wikiIdFromUrl !== DEFAULT_VAULT_ID && (
                  <div className="text-xs" style={{ color: token.colorTextSecondary }}>
                    {t("wiki.viewingWiki", "Viewing Wiki: {{id}}", { id: wikiIdFromUrl })}
                  </div>
                )}
              </Space>
            </div>
            <div className="flex-1 overflow-y-auto p-4">
              {displayNotes.length === 0 ? (
                <Empty description={t("wiki.emptyNotes", "No notes yet")} />
              ) : (
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
                            {note.author === "llm" ? t("wiki.llmNote", "LLM") : t("wiki.userNote", "User")} • {note.filePath}
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
      ) : (
        <WikiEditorPage noteId={selectedNoteId} onBack={handleBack} />
      )}
    </div>
  );
}