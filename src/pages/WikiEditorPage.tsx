// SPDX-License-Identifier: AGPL-3.0-only

import { BaseModal } from "@/components/shared/BaseModal";
import { MonacoEditor } from "@/components/shared/MonacoEditor";
import { BacklinkPanel } from "@/components/wiki/BacklinkPanel";
import { LintReport } from "@/components/wiki/LintReport";
import { OperationTimeline } from "@/components/wiki/OperationTimeline";
import { extractTagsFromContent, TagAggregationPanel } from "@/components/wiki/TagAggregationPanel";
import { VersionHistoryPanel } from "@/components/wiki/VersionHistoryPanel";
import { WikiSidebar } from "@/components/wiki/WikiSidebar";
import { useWikiAutoSave } from "@/hooks/useWikiAutoSave";
import { showBackendError } from "@/lib/errorI18n";
import { loadMonaco } from "@/lib/monaco";
import { message } from "@/lib/toast";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { Note } from "@/types";
import {
  DeleteOutlined,
  DownloadOutlined,
  EllipsisOutlined,
  EyeOutlined,
  HistoryOutlined,
  SaveOutlined,
} from "@ant-design/icons";
import { save } from "@tauri-apps/plugin-dialog";
import { Button, Divider, Dropdown, Modal, Popconfirm, Select, Spin, theme } from "antd";
import type { MenuProps } from "antd";
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
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

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
  const navigate = useNavigate();
  const { getNote, updateNote, deleteNote, notes, loadNotes, exportNoteHtml } = useWikiStore();
  const { operations, loadOperations } = useLlmWikiStore();

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
  // F4: 标签聚合面板点击过滤（再点一次取消）
  const [activeTag, setActiveTag] = useState<string | null>(null);
  const filteredNotes = useMemo(
    () =>
      activeTag
        ? notes.filter((n) => extractTagsFromContent(n.content).includes(activeTag))
        : notes,
    [notes, activeTag],
  );
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

  const handleSave = useCallback(async () => {
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
      showBackendError(message, e);
    }
    setSaving(false);
  }, [note, hasChanges, content, title, updateNote, t]);

  useEffect(() => {
    setTimeout(() => loadNote(), 0);
  }, [loadNote]);

  useEffect(() => {
    if (note?.vaultId) {
      setTimeout(() => loadNotes(note.vaultId), 0);
    }
  }, [note?.vaultId, loadNotes]);

  useEffect(() => {
    if (note) {
      setTimeout(() => setHasChanges(content !== note.content || title !== note.title), 0);
    }
  }, [content, title, note, setHasChanges]);

  // Ctrl+S 立即保存 + 3 秒空闲自动保存（F5：共享 hook，与详情面板行为一致）
  useWikiAutoSave({
    content,
    title,
    autoSaveEnabled: hasChanges && !saving,
    handleSave,
  });

  useEffect(() => {
    let disposed = false;
    let provider: import("monaco-editor").IDisposable | null = null;

    loadMonaco()
      .then((monaco) => {
        if (disposed) {
          return;
        }
        provider = monaco.languages.registerCompletionItemProvider(
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
                      kind: monaco.languages.CompletionItemKind.Reference,
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
      })
      .catch((e) => {
        console.error("[WikiEditorPage] Failed to load monaco-editor:", e);
      });

    return () => {
      disposed = true;
      provider?.dispose();
    };
  }, [noteId]);

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
      {/* 标题栏 — 紧凑 */}
      <div
        className="flex items-center gap-1.5 px-3 py-1.5 border-b shrink-0"
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <Button icon={<ArrowLeft size={15} />} onClick={handleBackWithConfirm} type="text" size="small" />
        <input
          type="text"
          value={title}
          onChange={(e) => handleTitleChange(e.target.value)}
          className="flex-1 text-base font-medium bg-transparent border-none outline-none min-w-0"
          style={{ color: token.colorText }}
          placeholder={t("wiki.titlePlaceholder")}
        />
        <Select
          showSearch
          size="small"
          value={wikilinkSelectValue}
          placeholder={t("wiki.insertLink")}
          style={{ width: 160 }}
          filterOption={(input, option) => (option?.label as string)?.toLowerCase().includes(input.toLowerCase())}
          options={noteOptions}
          onChange={handleWikilinkInsert}
        />
        <Divider type="vertical" style={{ height: 16 }} />
        <Button
          size="small"
          icon={<EyeOutlined />}
          type={previewMode ? "primary" : "default"}
          onClick={() => setPreviewMode(!previewMode)}
        />
        <Button
          size="small"
          icon={<SaveOutlined />}
          type={hasChanges ? "primary" : "default"}
          onClick={handleSave}
          loading={saving}
          disabled={!hasChanges}
        />
        <Popconfirm
          title={t("wiki.confirmDelete")}
          onConfirm={async () => {
            await deleteNote(note.id);
            message.success(t("wiki.deleted"));
            onBack();
          }}
        >
          <Button size="small" icon={<DeleteOutlined />} danger type="text" />
        </Popconfirm>
        <Divider type="vertical" style={{ height: 16 }} />
        <Button
          size="small"
          type="text"
          icon={leftSidebarOpen ? <PanelLeftClose size={13} /> : <PanelLeftOpen size={13} />}
          onClick={() => setLeftSidebarOpen(!leftSidebarOpen)}
          title={t("wiki.toggleSidebar")}
        />
        <Button
          size="small"
          type="text"
          icon={backlinkPanelOpen ? <PanelRightClose size={13} /> : <PanelRightOpen size={13} />}
          onClick={() => setBacklinkPanelOpen(!backlinkPanelOpen)}
          title={t("wiki.backlinks")}
        />
        <Dropdown
          menu={{
            items: [
              {
                key: "history",
                icon: <HistoryOutlined />,
                label: t("wiki.history"),
                onClick: () => setVersionHistoryOpen(true),
              },
              {
                key: "lint",
                icon: <CheckSquare size={14} />,
                label: t("wiki.lint"),
                onClick: () => setLintOpen(true),
              },
              {
                key: "timeline",
                icon: <History size={14} />,
                label: t("wiki.timeline"),
                onClick: async () => {
                  if (note?.vaultId) {
                    await loadOperations(note.vaultId);
                  }
                  setTimelineOpen(true);
                },
              },
              { type: "divider" },
              {
                key: "export",
                icon: <DownloadOutlined />,
                label: t("wiki.exportPdf"),
                onClick: async () => {
                  try {
                    const filePath = await save({
                      defaultPath: `${title || "note"}.html`,
                      filters: [{ name: "HTML", extensions: ["html"] }],
                    });
                    if (filePath) {
                      const result = await exportNoteHtml(noteId, filePath);
                      if (result) {
                        message.success(t("wiki.exportedPdf", { path: result }));
                      }
                    }
                  } catch { /* user cancelled */ }
                },
              },
            ] as MenuProps["items"],
          }}
          trigger={["click"]}
        >
          <Button size="small" type="text" icon={<EllipsisOutlined />} />
        </Dropdown>
        {note.author === "llm" && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded shrink-0"
            style={{ backgroundColor: token.colorPrimaryBg, color: token.colorPrimary }}
          >
            LLM
          </span>
        )}
      </div>

      <div className="flex-1 overflow-hidden flex">
        {leftSidebarOpen && (
          <div className="w-52 shrink-0 overflow-auto border-r" style={{ borderColor: token.colorBorderSecondary }}>
            <TagAggregationPanel
              notes={notes}
              onTagClick={(tag) =>
                setActiveTag((prev) => (prev === tag ? null : tag))}
              activeTag={activeTag}
            />
            <WikiSidebar
              notes={filteredNotes}
              selectedNoteId={noteId}
              onSelectNote={(id) => {
                if (id !== noteId && note?.vaultId) {
                  onBack();
                  navigate(`/llm-wiki/${note.vaultId}/edit/${id}`);
                }
              }}
              loading={false}
            />
          </div>
        )}
        <div className="flex-1 overflow-hidden p-3">
          <div className="h-full flex flex-col">
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
          title={t("wiki.lintReport")}
        >
          <LintReport wikiId={note.vaultId} />
        </BaseModal>
      )}

      <Modal
        title={t("wiki.operationTimeline")}
        open={timelineOpen}
        onCancel={() => setTimelineOpen(false)}
        footer={null}
        width={600}
      >
        <OperationTimeline operations={operations} />
      </Modal>
    </div>
  );
}
