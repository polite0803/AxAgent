import { BaseModal } from "@/components/shared/BaseModal";
import { MonacoEditor } from "@/components/shared/MonacoEditor";
import { BacklinkPanel } from "@/components/wiki/BacklinkPanel";
import { LintReport } from "@/components/wiki/LintReport";
import { OperationTimeline } from "@/components/wiki/OperationTimeline";
import { TagAggregationPanel } from "@/components/wiki/TagAggregationPanel";
import { VersionHistoryPanel } from "@/components/wiki/VersionHistoryPanel";
import { WikiSidebar } from "@/components/wiki/WikiSidebar";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { Note } from "@/types";
import { DeleteOutlined, DownloadOutlined, EyeOutlined, HistoryOutlined, SaveOutlined } from "@ant-design/icons";
import { save } from "@tauri-apps/plugin-dialog";
import { Button, message, Modal, Popconfirm, Select, Spin, theme } from "antd";
import DOMPurify from "dompurify";
import {
  ArrowLeft,
  CheckSquare,
  History,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface WikiEditorPageProps {
  noteId: string;
  onBack: () => void;
}

function markdownToHtml(md: string): string {
  return md
    .replace(/^### (.+)$/gm, "<h3>$1</h3>")
    .replace(/^## (.+)$/gm, "<h2>$1</h2>")
    .replace(/^# (.+)$/gm, "<h1>$1</h1>")
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(.+?)\*/g, "<em>$1</em>")
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\[\[([^\]]+)\]\]/g, '<a class="wikilink">$1</a>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/^- (.+)$/gm, "<li>$1</li>")
    .replace(/^> (.+)$/gm, "<blockquote>$1</blockquote>")
    .replace(/\n/g, "<br/>");
}

export function WikiEditorPage({ noteId, onBack }: WikiEditorPageProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { getNote, updateNote, deleteNote, notes, loadNotes, exportNotePdf } = useWikiStore();

  const [note, setNote] = useState<Note | null>(null);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [previewMode, setPreviewMode] = useState(false);
  const [wikilinkSelectValue, setWikilinkSelectValue] = useState<
    string | undefined
  >(undefined);
  const [versionHistoryOpen, setVersionHistoryOpen] = useState(false);
  const [backlinkPanelOpen, setBacklinkPanelOpen] = useState(true);
  const [leftSidebarOpen, setLeftSidebarOpen] = useState(true);
  const [lintOpen, setLintOpen] = useState(false);
  const [timelineOpen, setTimelineOpen] = useState(false);
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastSavedRef = useRef<string>("");

  const loadNote = useCallback(async () => {
    setLoading(true);
    const loaded = await getNote(noteId);
    if (loaded) {
      setNote(loaded);
      setContent(loaded.content);
      setTitle(loaded.title);
      lastSavedRef.current = loaded.content;
    }
    setLoading(false);
  }, [noteId, getNote]);

  useEffect(() => {
    loadNote();
  }, [loadNote]);

  useEffect(() => {
    if (note?.vaultId) {
      loadNotes(note.vaultId);
    }
  }, [note?.vaultId, loadNotes]);

  useEffect(() => {
    if (note) {
      setHasChanges(content !== note.content || title !== note.title);
    }
  }, [content, title, note]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  useEffect(() => {
    if (!hasChanges || saving) {
      return;
    }
    if (autoSaveTimerRef.current) {
      clearTimeout(autoSaveTimerRef.current);
    }
    autoSaveTimerRef.current = setTimeout(() => {
      handleSave();
    }, 3000);
    return () => {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current);
      }
    };
  }, [content, title]);

  useEffect(() => {
    if (!window.monaco) {
      return;
    }
    const provider = window.monaco.languages.registerCompletionItemProvider(
      "markdown",
      {
        triggerCharacters: ["["],
        provideCompletionItems: (model, position) => {
          const textUntilPosition = model.getValueInRange({
            startLineNumber: position.lineNumber,
            startColumn: 1,
            endLineNumber: position.lineNumber,
            endColumn: position.column,
          });
          const match = textUntilPosition.match(/\[\[([^\]]*)$/);
          if (!match) {
            return { suggestions: [] };
          }

          const search = match[1].toLowerCase();
          const openBracketCol = position.column - match[0].length;
          const currentNotes = useWikiStore.getState().notes;

          const suggestions = currentNotes.flatMap((n) =>
            n.id !== noteId && n.title.toLowerCase().includes(search)
              ? [
                {
                  kind: window.monaco.languages.CompletionItemKind.Reference,
                  label: n.title,
                  insertText: `[[${n.title}]]`,
                  range: {
                    startLineNumber: position.lineNumber,
                    startColumn: openBracketCol,
                    endLineNumber: position.lineNumber,
                    endColumn: position.column,
                  },
                },
              ]
              : []
          );

          return { suggestions };
        },
      },
    );
    return () => provider.dispose();
  }, [noteId]);

  const handleSave = async () => {
    if (!note || !hasChanges) {
      return;
    }
    setSaving(true);
    try {
      const updated = await updateNote(note.id, { title, content });
      if (updated) {
        setNote(updated);
        lastSavedRef.current = content;
        setHasChanges(false);
        message.success(t("wiki.saved"));
      }
    } catch (e) {
      message.error(String(e));
    }
    setSaving(false);
  };

  const handleBackWithConfirm = () => {
    if (hasChanges && content !== lastSavedRef.current) {
      Modal.confirm({
        title: t("wiki.unsavedTitle"),
        content: t("wiki.unsavedContent"),
        okText: t("wiki.discard"),
        cancelText: t("wiki.keepEditing"),
        onOk: onBack,
      });
    } else {
      onBack();
    }
  };

  const handleContentChange = (value: string) => {
    setContent(value);
  };

  const handleTitleChange = (value: string) => {
    setTitle(value);
  };

  const handleWikilinkInsert = (noteTitle: string) => {
    setContent((prev) => prev + `[[${noteTitle}]]`);
    setWikilinkSelectValue(undefined);
  };

  if (loading) {
    return (
      <div
        className="h-full flex items-center justify-center"
        style={{ backgroundColor: token.colorBgElevated }}
      >
        <Spin size="large" />
      </div>
    );
  }

  if (!note) {
    return (
      <div
        className="h-full flex items-center justify-center"
        style={{ backgroundColor: token.colorBgElevated }}
      >
        <span>{t("wiki.noteNotFound")}</span>
      </div>
    );
  }

  const noteOptions = notes.flatMap((n) => n.id !== noteId ? [{ value: n.title, label: n.title }] : []);

  return (
    <div
      className="h-full flex flex-col"
      style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}
    >
      <div
        className="flex items-center gap-2 p-3 border-b"
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <Button
          icon={<ArrowLeft />}
          onClick={handleBackWithConfirm}
          type="text"
        />
        <input
          type="text"
          value={title}
          onChange={(e) => handleTitleChange(e.target.value)}
          className="flex-1 text-lg font-medium bg-transparent border-none outline-none"
          style={{ color: token.colorText }}
          placeholder={t("wiki.titlePlaceholder")}
        />
        <Button
          icon={<SaveOutlined />}
          type="primary"
          onClick={handleSave}
          loading={saving}
          disabled={!hasChanges}
        >
          {t("wiki.save")}
        </Button>
        <Popconfirm
          title={t("wiki.confirmDelete")}
          onConfirm={async () => {
            await deleteNote(note.id);
            message.success(t("wiki.deleted"));
            onBack();
          }}
        >
          <Button icon={<DeleteOutlined />} danger type="text" />
        </Popconfirm>
      </div>

      <div className="flex-1 overflow-hidden flex">
        {leftSidebarOpen && (
          <div className="w-56 shrink-0 overflow-auto border-r" style={{ borderColor: token.colorBorderSecondary }}>
            <TagAggregationPanel notes={notes} onTagClick={() => {}} activeTag={null} />
            <WikiSidebar notes={notes} selectedNoteId={noteId} onSelectNote={() => {}} loading={false} />
          </div>
        )}
        <div className="flex-1 overflow-hidden p-4">
          <div className="h-full flex flex-col">
            <div className="mb-2 flex items-center gap-2">
              <Select
                showSearch
                value={wikilinkSelectValue}
                placeholder={t("wiki.insertLink")}
                style={{ width: 200 }}
                filterOption={(input, option) =>
                  (option?.label as string)
                    ?.toLowerCase()
                    .includes(input.toLowerCase())}
                options={noteOptions}
                onChange={handleWikilinkInsert}
              />
              <Button
                size="small"
                icon={<EyeOutlined />}
                onClick={() => setPreviewMode(!previewMode)}
              >
                {previewMode ? t("wiki.source") : t("wiki.preview")}
              </Button>
              <Button
                size="small"
                icon={<HistoryOutlined />}
                onClick={() => setVersionHistoryOpen(true)}
              >
                {t("wiki.history")}
              </Button>
              <Button
                size="small"
                type="text"
                icon={leftSidebarOpen ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
                onClick={() => setLeftSidebarOpen(!leftSidebarOpen)}
              />
              <Button
                size="small"
                icon={<CheckSquare size={14} />}
                onClick={() => setLintOpen(true)}
              >
                Lint
              </Button>
              <Button
                size="small"
                icon={<History size={14} />}
                onClick={() => setTimelineOpen(true)}
              >
                Timeline
              </Button>
              <Button
                size="small"
                icon={<DownloadOutlined />}
                onClick={async () => {
                  try {
                    const filePath = await save({
                      defaultPath: `${title || "note"}.html`,
                      filters: [{ name: "HTML", extensions: ["html"] }],
                    });
                    if (filePath) {
                      const result = await exportNotePdf(noteId, filePath);
                      if (result) {
                        message.success(
                          t("wiki.exportedPdf", { path: result }),
                        );
                      }
                    }
                  } catch {
                    // User cancelled
                  }
                }}
              >
                {t("wiki.exportPdf")}
              </Button>
              <Button
                size="small"
                type="text"
                icon={backlinkPanelOpen ? <PanelRightClose size={14} /> : <PanelRightOpen size={14} />}
                onClick={() => setBacklinkPanelOpen(!backlinkPanelOpen)}
                title={t("wiki.backlinks")}
              />
              {note.author === "llm" && (
                <span
                  className="text-xs px-2 py-1 rounded"
                  style={{ backgroundColor: token.colorPrimaryBg }}
                >
                  {t("wiki.llmNote")}
                </span>
              )}
            </div>
            {previewMode
              ? (
                <div
                  className="flex-1 overflow-auto p-4 rounded-lg"
                  style={{
                    backgroundColor: token.colorBgContainer,
                    border: `1px solid ${token.colorBorderSecondary}`,
                    color: token.colorText,
                    lineHeight: 1.7,
                  }}
                  dangerouslySetInnerHTML={{
                    __html: DOMPurify.sanitize(markdownToHtml(content)),
                  }}
                />
              )
              : (
                <div
                  className="flex-1 rounded-lg overflow-hidden"
                  style={{ border: `1px solid ${token.colorBorderSecondary}` }}
                >
                  <MonacoEditor
                    value={content}
                    language="markdown"
                    onChange={handleContentChange}
                    height="100%"
                  />
                </div>
              )}
          </div>
        </div>

        {backlinkPanelOpen && (
          <div
            className="shrink-0 overflow-auto border-l"
            style={{
              width: 280,
              borderColor: token.colorBorderSecondary,
              backgroundColor: token.colorBgElevated,
            }}
          >
            <BacklinkPanel
              noteId={noteId}
              onNavigateToNote={(id) => {
                if (id !== noteId) {
                  onBack();
                }
              }}
            />
          </div>
        )}
      </div>

      <VersionHistoryPanel
        noteId={noteId}
        open={versionHistoryOpen}
        onClose={() => setVersionHistoryOpen(false)}
        onRestore={loadNote}
      />

      {note?.vaultId && (
        <BaseModal
          open={lintOpen}
          onCancel={() => setLintOpen(false)}
          title="Lint Report"
        >
          <LintReport wikiId={note.vaultId} />
        </BaseModal>
      )}

      <Modal
        title="Operation Timeline"
        open={timelineOpen}
        onCancel={() => setTimelineOpen(false)}
        footer={null}
        width={600}
      >
        <OperationTimeline operations={[]} />
      </Modal>
    </div>
  );
}
