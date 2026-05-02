import type { Note } from "@/types";
import { PlusOutlined } from "@ant-design/icons";
import { Button, Spin } from "antd";
import { useTranslation } from "react-i18next";

interface WikiSidebarProps {
  notes: Note[];
  selectedNoteId: string | null;
  onSelectNote: (noteId: string) => void;
  loading: boolean;
  onCreateNote?: () => void;
}

export function WikiSidebar({
  notes,
  selectedNoteId,
  onSelectNote,
  loading,
  onCreateNote,
}: WikiSidebarProps) {
  const { t } = useTranslation();

  return (
    <div className="w-64 h-full flex flex-col" style={{ backgroundColor: "var(--color-bg-container)" }}>
      <div className="p-3 border-b flex items-center justify-between" style={{ borderColor: "var(--border-color)" }}>
        <span className="font-medium">{t("wiki.notes", "Notes")}</span>
        {onCreateNote && <Button icon={<PlusOutlined />} size="small" onClick={onCreateNote} />}
      </div>
      <div className="flex-1 overflow-y-auto">
        {loading
          ? (
            <div className="flex items-center justify-center h-full">
              <Spin size="small" />
            </div>
          )
          : (
            <div className="p-2">
              {notes.map((note) => (
                <div
                  key={note.id}
                  onClick={() => onSelectNote(note.id)}
                  className={`p-2 rounded cursor-pointer mb-1 transition-colors ${
                    selectedNoteId === note.id
                      ? "bg-black/10"
                      : "hover:bg-black/5"
                  }`}
                >
                  <div className="font-medium text-sm truncate">{note.title}</div>
                  <div className="text-xs truncate mt-0.5" style={{ color: "var(--color-text-secondary)" }}>
                    {note.filePath}
                  </div>
                  <div className="flex gap-1 mt-1">
                    {note.author === "llm" && (
                      <span className="text-xs px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-600">
                        LLM
                      </span>
                    )}
                    {note.pageType && (
                      <span className="text-xs px-1.5 py-0.5 rounded bg-green-500/10 text-green-600">
                        {note.pageType}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
      </div>
    </div>
  );
}
