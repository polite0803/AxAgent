// SPDX-License-Identifier: AGPL-3.0-only

import type { Note } from "@/types";
import { Tag, theme } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

interface TagAggregationPanelProps {
  notes: Note[];
  onTagClick: (tag: string) => void;
  activeTag: string | null;
}

function extractTagsFromFrontmatter(content: string): string[] {
  const tags: string[] = [];
  const fmMatch = content.match(/^---\s*\n([\s\S]*?)\n---/);
  if (!fmMatch) {
    return tags;
  }
  const fm = fmMatch[1];
  for (const line of fm.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.startsWith("tags:")) {
      const rest = trimmed.slice(5).trim();
      if (rest.startsWith("[")) {
        const inner = rest.slice(1, -1);
        for (const t of inner.split(",")) {
          const cleaned = t.trim().replace(/^["']|["']$/g, "");
          if (cleaned) {
            tags.push(cleaned);
          }
        }
      }
      continue;
    }
    if (tags.length > 0 || trimmed.startsWith("- ")) {
      if (trimmed.startsWith("- ")) {
        const val = trimmed
          .slice(2)
          .trim()
          .replace(/^["']|["']$/g, "");
        if (val) {
          tags.push(val);
        }
      }
    }
  }
  return tags;
}

function extractTagsFromContent(content: string): string[] {
  const tags: string[] = [];
  const fmTags = extractTagsFromFrontmatter(content);
  tags.push(...fmTags);
  const bodyMatch = content.match(/^---\s*\n[\s\S]*?\n---\s*\n?([\s\S]*)$/);
  const body = bodyMatch ? bodyMatch[1] : content;
  const hashTagRegex = /(?:^|\s)#([a-zA-Z\u4e00-\u9fff][\w\u4e00-\u9fff-]*)/g;
  let match: RegExpExecArray | null;
  while ((match = hashTagRegex.exec(body)) !== null) {
    tags.push(match[1]);
  }
  return tags;
}

export function TagAggregationPanel({
  notes,
  onTagClick,
  activeTag,
}: TagAggregationPanelProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  const tagData = useMemo(() => {
    const freq = new Map<string, number>();
    for (const note of notes) {
      const tags = extractTagsFromContent(note.content);
      for (const tag of tags) {
        freq.set(tag, (freq.get(tag) || 0) + 1);
      }
    }
    return Array.from(freq.entries()).sort((a, b) => b[1] - a[1]);
  }, [notes]);

  if (tagData.length === 0) {
    return null;
  }

  return (
    <div
      className="px-3 py-2 border-b"
      style={{ borderColor: token.colorBorderSecondary }}
    >
      <div
        className="text-xs font-medium mb-1.5"
        style={{ color: token.colorTextSecondary }}
      >
        {t("wiki.tags")}
      </div>
      <div className="flex flex-wrap gap-1">
        {tagData.map(([tag, count]) => (
          <Tag
            key={tag}
            color={activeTag === tag ? token.colorPrimary : undefined}
            style={activeTag === tag
              ? { cursor: "pointer" }
              : {
                cursor: "pointer",
                borderColor: token.colorBorder,
                background: "transparent",
              }}
            onClick={() => onTagClick(tag)}
          >
            {tag} <span style={{ opacity: 0.6, fontSize: 10 }}>{count}</span>
          </Tag>
        ))}
      </div>
    </div>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export { extractTagsFromContent };
