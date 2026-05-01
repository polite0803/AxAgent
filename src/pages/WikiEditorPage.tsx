import { useWikiStore } from "@/stores/feature/wikiStore";
import { useCallback, useEffect, useState } from "react";
import { Button, message, Spin, theme, Popconfirm } from "antd";
import { ArrowLeft } from "lucide-react";
import { SaveOutlined, DeleteOutlined } from "@ant-design/icons";
import { useTranslation } from "react-i18next";
import type { Note } from "@/types";

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

  const loadNote = useCallback(async () => {
    setLoading(true);
    const loaded = await getNote(noteId);
    if (loaded) {
      setNote(loaded);
      setContent(loaded.content);
      setTitle(loaded.title);
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

  const handleSave = async () => {
    if (!note) return;
    setSaving(true);
    try {
      const updated = await updateNote(note.id, { title, content });
      if (updated) {
        setNote(updated);
        message.success(t("wiki.saved", "Saved"));
      }
    } catch (e) {
      message.error(String(e));
    }
    setSaving(false);
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
        <Button icon={<ArrowLeft />} onClick={onBack} type="text" />
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