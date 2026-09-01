// SPDX-License-Identifier: AGPL-3.0-only

import { highlightWikilink } from "@/components/wiki/wikilinkHighlight";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { BacklinkInfo } from "@/types";
import { Empty, Spin, theme, Typography } from "antd";
import { ArrowLeftRight, ChevronDown, ChevronRight } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface BacklinkPanelProps {
  noteId: string;
  onNavigateToNote: (noteId: string) => void;
}

export function BacklinkPanel({
  noteId,
  onNavigateToNote,
}: BacklinkPanelProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { getNoteBacklinks, getNote } = useWikiStore();

  const [backlinks, setBacklinks] = useState<BacklinkInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [noteTitle, setNoteTitle] = useState("");
  const [collapsed, setCollapsed] = useState(false);

  const loadBacklinks = useCallback(async () => {
    if (!noteId) {
      return;
    }
    setLoading(true);
    const [bl, note] = await Promise.all([
      getNoteBacklinks(noteId),
      getNote(noteId),
    ]);
    setBacklinks(bl);
    if (note) {
      setNoteTitle(note.title);
    }
    setLoading(false);
  }, [noteId, getNoteBacklinks, getNote]);

  useEffect(() => {
    setTimeout(() => loadBacklinks(), 0);
  }, [loadBacklinks]);

  const totalCount = backlinks.reduce((sum, bl) => sum + bl.snippets.length, 0);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-6">
        <Spin size="small" />
      </div>
    );
  }

  if (backlinks.length === 0) {
    return (
      <div className="px-3 py-2">
        <div
          className="flex items-center gap-1.5 mb-2 text-xs font-medium"
          style={{ color: token.colorTextSecondary }}
        >
          <ArrowLeftRight size={12} />
          {t("wiki.backlinks")}
        </div>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("wiki.noBacklinks")}
          className="my-2"
        />
      </div>
    );
  }

  return (
    <div className="px-3 py-2">
      <div
        className="flex items-center gap-1.5 mb-2 cursor-pointer select-none"
        role="button"
        tabIndex={0}
        style={{ color: token.colorTextSecondary }}
        onClick={() => setCollapsed(!collapsed)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            setCollapsed(!collapsed);
          }
        }}
      >
        {collapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
        <ArrowLeftRight size={12} />
        <span className="text-xs font-medium">{t("wiki.backlinks")}</span>
        <span
          className="text-[10px] px-1.5 py-0.5 rounded-full"
          style={{
            backgroundColor: `${token.colorPrimary}15`,
            color: token.colorPrimary,
          }}
        >
          {totalCount}
        </span>
      </div>

      {!collapsed && (
        <div>
          {backlinks.map((bl) => (
            <div
              key={bl.noteId}
              className="mb-1.5 rounded-lg transition-colors duration-150"
              style={{
                backgroundColor: token.colorBgContainer,
                border: `1px solid ${token.colorBorderSecondary}40`,
              }}
            >
              <div
                className="px-3 py-2 cursor-pointer hover:opacity-80 transition-opacity"
                role="button"
                tabIndex={0}
                onClick={() => onNavigateToNote(bl.noteId)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    onNavigateToNote(bl.noteId);
                  }
                }}
              >
                <Text
                  strong
                  className="text-sm"
                  style={{ color: token.colorPrimary }}
                >
                  {bl.title}
                </Text>
              </div>
              {bl.snippets.length > 0 && (
                <div className="px-3 pb-2">
                  {bl.snippets.map((snippet, si) => (
                    // FIXME: snippets 是字符串数组，无稳定唯一标识
                    <Paragraph
                      key={`snippet-${si}`}
                      className="!mb-1 text-xs leading-relaxed"
                      style={{ color: token.colorTextSecondary }}
                      ellipsis={{ rows: 2, expandable: false }}
                    >
                      {highlightWikilink(snippet, noteTitle, token)}
                    </Paragraph>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
