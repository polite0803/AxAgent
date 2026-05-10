import { MonacoEditor } from "@/components/shared/MonacoEditor";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { Note } from "@/types";
import { DeleteOutlined, EyeOutlined, SaveOutlined } from "@ant-design/icons";
import { Button, message, Modal, Popconfirm, Select, Spin, theme } from "antd";
import { ArrowLeft } from "lucide-react";
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
  const { getNote, updateNote, deleteNote, notes, loadNotes } = useWikiStore();

  const [note, setNote] = useState<Note | null>(null);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [previewMode, setPreviewMode] = useState(false);
  const [wikilinkSelectValue, setWikilinkSelectValue] = useState<string | undefined>(undefined);
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
    if (!hasChanges || saving) { return; }
    if (autoSaveTimerRef.current) { clearTimeout(autoSaveTimerRef.current); }
    autoSaveTimerRef.current = setTimeout(() => {
      handleSave();
    }, 3000);
    return () => {
      if (autoSaveTimerRef.current) { clearTimeout(autoSaveTimerRef.current); }
    };
  }, [content, title]);

  useEffect(() => {
    if (!window.monaco) { return; }
    const provider = window.monaco.languages.registerCompletionItemProvider("markdown", {
      triggerCharacters: ["["],
      provideCompletionItems: (model, position) => {
        const textUntilPosition = model.getValueInRange({
          startLineNumber: position.lineNumber,
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
        });
        const match = textUntilPosition.match(/\[\[([^\]]*)$/);
        if (!match) { return { suggestions: [] }; }

        const search = match[1].toLowerCase();
        const openBracketCol = position.column - match[0].length;
        const currentNotes = useWikiStore.getState().notes;

        const suggestions = currentNotes
          .filter((n) => n.id !== noteId && n.title.toLowerCase().includes(search))
          .map((n) => ({
            kind: window.monaco.languages.CompletionItemKind.Reference,
            label: n.title,
            insertText: `[[${n.title}]]`,
            range: {
              startLineNumber: position.lineNumber,
              startColumn: openBracketCol,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
          }));

        return { suggestions };
      },
    });
    return () => provider.dispose();
  }, [noteId]);

  const handleSave = async () => {
    if (!note || !hasChanges) { return; }
    setSaving(true);
    try {
      const updated = await updateNote(note.id, { title, content });
      if (updated) {
        setNote(updated);
        lastSavedRef.current = content;
        setHasChanges(false);
        message.success(t("wiki.saved", "Saved"));
      }
    } catch (e) {
      message.error(String(e));
    }
    setSaving(false);
  };

  const handleBackWithConfirm = () => {
    if (hasChanges && content !== lastSavedRef.current) {
      Modal.confirm({
        title: t("wiki.unsavedTitle", "Unsaved Changes"),
        content: t("wiki.unsavedContent", "You have unsaved changes. Discard them?"),
        okText: t("wiki.discard", "Discard"),
        cancelText: t("wiki.keepEditing", "Keep Editing"),
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
      <div className="h-full flex items-center justify-center" style={{ backgroundColor: token.colorBgElevated }}>
        <Spin size="large" />
      </div>
    );
  }

  if (!note) {
    return (
      <div className="h-full flex items-center justify-center" style={{ backgroundColor: token.colorBgElevated }}>
        <span>{t("wiki.noteNotFound", "Note not found")}</span>
      </div>
    );
  }

  const noteOptions = notes
    .filter((n) => n.id !== noteId)
    .map((n) => ({ value: n.title, label: n.title }));

  return (
    <div className="h-full flex flex-col" style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}>
      <div className="flex items-center gap-2 p-3 border-b" style={{ borderColor: token.colorBorderSecondary }}>
        <Button icon={<ArrowLeft />} onClick={handleBackWithConfirm} type="text" />
        <input
          type="text"
          value={title}
          onChange={(e) => handleTitleChange(e.target.value)}
          className="flex-1 text-lg font-medium bg-transparent border-none outline-none"
          style={{ color: token.colorText }}
          placeholder={t("wiki.titlePlaceholder", "Note title...")}
        />
        <Button
          icon={<SaveOutlined />}
          type="primary"
          onClick={handleSave}
          loading={saving}
          disabled={!hasChanges}
        >
          {t("wiki.save", "Save")}
        </Button>
        <Popconfirm
          title={t("wiki.confirmDelete", "Delete this note?")}
          onConfirm={async () => {
            await deleteNote(note.id);
            message.success(t("wiki.deleted", "Note deleted"));
            onBack();
          }}
        >
          <Button icon={<DeleteOutlined />} danger type="text" />
        </Popconfirm>
      </div>

      <div className="flex-1 overflow-hidden p-4">
        <div className="h-full flex flex-col">
          <div className="mb-2 flex items-center gap-2">
            <Select
              showSearch
              value={wikilinkSelectValue}
              placeholder={t("wiki.insertLink", "Insert Link")}
              style={{ width: 200 }}
              filterOption={(input, option) => (option?.label as string)?.toLowerCase().includes(input.toLowerCase())}
              options={noteOptions}
              onChange={handleWikilinkInsert}
            />
            <Button
              size="small"
              icon={<EyeOutlined />}
              onClick={() => setPreviewMode(!previewMode)}
            >
              {previewMode ? t("wiki.source", "Source") : t("wiki.preview", "Preview")}
            </Button>
            {note.author === "llm" && (
              <span className="text-xs px-2 py-1 rounded" style={{ backgroundColor: token.colorPrimaryBg }}>
                {t("wiki.llmNote", "LLM Note")}
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
                dangerouslySetInnerHTML={{ __html: markdownToHtml(content) }}
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
    </div>
  );
}
