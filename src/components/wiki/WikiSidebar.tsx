// SPDX-License-Identifier: AGPL-3.0-only

import type { Note } from "@/types";
import { PlusOutlined } from "@ant-design/icons";
import { Button, Spin, theme } from "antd";
// eslint-disable-next-line react-hooks/incompatible-library
import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import { useTranslation } from "react-i18next";

interface WikiSidebarProps {
  notes: Note[];
  selectedNoteId: string | null;
  onSelectNote: (noteId: string) => void;
  loading: boolean;
  onCreateNote?: () => void;
}

// F8：笔记列表虚拟化（@tanstack/react-virtual，与 ModelSelector / ProviderDetail 同款模式）。
// 大 vault（数千笔记）下避免全量渲染卡顿；行高不固定（badge 可选），用 measureElement 动态测量。
export function WikiSidebar({
  notes,
  selectedNoteId,
  onSelectNote,
  loading,
  onCreateNote,
}: WikiSidebarProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const scrollRef = useRef<HTMLDivElement>(null);

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: notes.length,
    getScrollElement: () => scrollRef.current,
    // 估算：标题(20) + 路径(16) + badge 行(0~22) + 内边距(16) + 间距(4)
    estimateSize: () => 68,
    getItemKey: (index) => notes[index].id,
    overscan: 8,
  });

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
      <div ref={scrollRef} className="flex-1 overflow-y-auto">
        {loading
          ? (
            <div className="flex items-center justify-center h-full">
              <Spin size="small" />
            </div>
          )
          : (
            <div className="px-2 py-2">
              <div
                style={{ height: virtualizer.getTotalSize(), position: "relative" }}
              >
                {virtualizer.getVirtualItems().map((virtualRow) => {
                  const note = notes[virtualRow.index];
                  const selected = selectedNoteId === note.id;
                  return (
                    <div
                      key={virtualRow.key}
                      data-index={virtualRow.index}
                      ref={virtualizer.measureElement}
                      role="button"
                      tabIndex={0}
                      onClick={() => onSelectNote(note.id)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          onSelectNote(note.id);
                        }
                      }}
                      className="p-2 rounded cursor-pointer mb-1 transition-colors"
                      style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        width: "100%",
                        transform: `translateY(${virtualRow.start}px)`,
                        backgroundColor: selected ? token.colorFillContent : undefined,
                      }}
                      onMouseEnter={(e) => {
                        if (!selected) {
                          e.currentTarget.style.backgroundColor = token.colorFillQuaternary;
                        }
                      }}
                      onMouseLeave={(e) => {
                        if (!selected) {
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
                            style={{
                              backgroundColor: token.colorPrimaryBg,
                              color: token.colorPrimary,
                            }}
                          >
                            LLM
                          </span>
                        )}
                        {note.pageType && (
                          <span
                            className="text-xs px-1.5 py-0.5 rounded"
                            style={{
                              backgroundColor: token.colorSuccessBg,
                              color: token.colorSuccess,
                            }}
                          >
                            {note.pageType}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
      </div>
    </div>
  );
}
