import { useWikiStore } from "@/stores/feature/wikiStore";
import type { Note } from "@/types";
import { DeleteOutlined, SaveOutlined } from "@ant-design/icons";
import { Button, message, Modal, Popconfirm, Spin, theme } from "antd";
import { ArrowLeft } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface WikiEditorPageProps {
  noteId: string;
  onBack: () => void;
}

export function WikiEditorPage({ noteId, onBack }: WikiEditorPageProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { getNote, updateNote, deleteNote } = useWikiStore();

  const [note, setNote] = useState<Note | null>(null);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
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
    if (note) {
      setHasChanges(content !== note.content || title !== note.title);
    }
  }, [content, title, note]);

  // #15: Ctrl+S 快捷键
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

  // #15: 自动保存（3 秒空闲后触发）
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

  // #15: 离开确认 — 未保存时弹窗确认
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

  const insertWikiLink = () => {
    const linkText = "[[New Note]]";
    setContent((prev) => prev + linkText);
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
          <div className="mb-2 flex gap-2">
            <Button size="small" onClick={insertWikiLink}>
              {t("wiki.insertLink", "Insert Link")}
            </Button>
            {note.author === "llm" && (
              <span className="text-xs px-2 py-1 rounded" style={{ backgroundColor: token.colorPrimaryBg }}>
                {t("wiki.llmNote", "LLM Note")}
              </span>
            )}
          </div>
          <textarea
            value={content}
            onChange={(e) => handleContentChange(e.target.value)}
            className="flex-1 w-full p-4 resize-none rounded-lg outline-none font-mono text-sm"
            style={{
              backgroundColor: token.colorBgContainer,
              border: `1px solid ${token.colorBorderSecondary}`,
              color: token.colorText,
            }}
            placeholder={t("wiki.contentPlaceholder", "Start writing...")}
          />
        </div>
      </div>
    </div>
  );
}
