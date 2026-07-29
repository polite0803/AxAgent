// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import type { SkillDetail } from "@/types";
import { Spin } from "antd";
import NodeRenderer from "markstream-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { BaseNode } from "stream-markdown-parser";
import { getMarkdown, parseMarkdownToStructure } from "stream-markdown-parser";

interface SkillMarkdownPageProps {
  skillName: string;
}

const skillMarkdown = getMarkdown("skill-markdown", {
  customHtmlTags: [],
});

export function SkillMarkdownPage({ skillName }: SkillMarkdownPageProps) {
  const { t } = useTranslation();
  const [rawContent, setRawContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    /* eslint-disable react-hooks/set-state-in-effect */
    setLoading(true);
    setError(null);
    /* eslint-enable react-hooks/set-state-in-effect */

    async function loadContent() {
      try {
        // SK-P1: 复用 @/types 中的 SkillDetail 类型,而非本地重定义
        const detail = await invoke<SkillDetail>("get_skill", { name: skillName });
        if (!cancelled) {
          setRawContent(detail.content || "");
        }
      } catch (e) {
        logIpcError("get_skill")(e);
        if (!cancelled) {
          setError(String(e));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    loadContent();
    return () => {
      cancelled = true;
    };
  }, [skillName]);

  const nodes = useMemo<BaseNode[] | null>(() => {
    if (!rawContent) {
      return null;
    }
    try {
      return parseMarkdownToStructure(rawContent, skillMarkdown, {
        customHtmlTags: [],
      });
    } catch {
      return null;
    }
  }, [rawContent]);

  if (loading) {
    return (
      <div style={{ display: "flex", justifyContent: "center", padding: 48 }}>
        <Spin size="large" />
      </div>
    );
  }

  if (error || !nodes) {
    return (
      <div style={{ padding: 24, color: "var(--color-error)" }}>
        {t("skill.markdown.loadFailed")}: {error || t("skill.markdown.parseError")}
      </div>
    );
  }

  return (
    <div
      className="markstream-react"
      style={{
        padding: "24px 32px",
        maxWidth: 900,
        margin: "0 auto",
      }}
    >
      <NodeRenderer nodes={nodes} />
    </div>
  );
}
