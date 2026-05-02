import { invoke } from "@/lib/invoke";
import { Spin } from "antd";
import NodeRenderer from "markstream-react";
import { useEffect, useMemo, useState } from "react";
import type { BaseNode } from "stream-markdown-parser";
import { getMarkdown, parseMarkdownToStructure } from "stream-markdown-parser";

interface SkillMarkdownPageProps {
  skillName: string;
}

const skillMarkdown = getMarkdown("skill-markdown", {
  customHtmlTags: [],
});

export function SkillMarkdownPage({ skillName }: SkillMarkdownPageProps) {
  const [rawContent, setRawContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    async function loadContent() {
      try {
        const detail = await invoke<{ info: unknown; content: string; files: string[] }>(
          "get_skill",
          { name: skillName },
        );
        if (!cancelled) {
          setRawContent(detail.content || "");
        }
      } catch {
        if (!cancelled) {
          setError(String(Error));
        }
      } finally {
        if (!cancelled) { setLoading(false); }
      }
    }

    loadContent();
    return () => {
      cancelled = true;
    };
  }, [skillName]);

  const nodes = useMemo<BaseNode[] | null>(() => {
    if (!rawContent) { return null; }
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
        Failed to load markdown content: {error || "parse error"}
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
