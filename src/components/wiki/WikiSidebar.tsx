import type { Note } from "@/types";
import { PlusOutlined } from "@ant-design/icons";
import { Button, Spin, theme } from "antd";
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
  const { token } = theme.useToken();

  return (
    <div
      className="w-64 h-full flex flex-col"
      style={{ backgroundColor: "var(--color-bg-container)" }}
    >
      <div
        className="p-3 border-b flex items-center justify-between"
        style={{ borderColor: "var(--border-color)" }}
      >
        <span className="font-medium">{t("wiki.notes")}</span>
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
                  role="button"
                  tabIndex={0}
                  onClick={() => onSelectNote(note.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      onSelectNote(note.id);
                    }
                  }}
                  className={`p-2 rounded cursor-pointer mb-1 transition-colors`}
                  style={{
                    backgroundColor: selectedNoteId === note.id
                      ? token.colorFillContent
                      : undefined,
                  }}
                  onMouseEnter={(e) => {
                    if (selectedNoteId !== note.id) {
                      e.currentTarget.style.backgroundColor = token.colorFillQuaternary;
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (selectedNoteId !== note.id) {
                      e.currentTarget.style.backgroundColor = "";
                    }
                  }}
                >
                  <div className="font-medium text-sm truncate">{note.title}</div>
                  <div
                    className="text-xs truncate mt-0.5"
                    style={{ color: "var(--color-text-secondary)" }}
                  >
                    {note.filePath}
                  </div>
                  <div className="flex gap-1 mt-1">
                    {note.author === "llm" && (
                      <span
                        className="text-xs px-1.5 py-0.5 rounded"
                        style={{ backgroundColor: token.colorPrimaryBg, color: token.colorPrimary }}
                      >
                        LLM
                      </span>
                    )}
                    {note.pageType && (
                      <span
                        className="text-xs px-1.5 py-0.5 rounded"
                        style={{ backgroundColor: token.colorSuccessBg, color: token.colorSuccess }}
                      >
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
