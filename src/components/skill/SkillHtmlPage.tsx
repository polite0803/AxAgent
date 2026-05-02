import { invoke } from "@/lib/invoke";
import { Spin } from "antd";
import { useEffect, useMemo, useRef, useState } from "react";

interface SkillHtmlPageProps {
  componentConfig: Record<string, unknown>;
  skillName: string;
}

export function SkillHtmlPage({ componentConfig, skillName }: SkillHtmlPageProps) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const htmlFile = (componentConfig.file as string) || "index.html";

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    async function loadContent() {
      try {
        const result = await invoke<string>("skill_read_asset", {
          name: skillName,
          fileName: htmlFile,
        });
        if (!cancelled) {
          setContent(result);
        }
      } catch (e) {
        if (!cancelled) {
          setError(String(e));
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    loadContent();
    return () => { cancelled = true; };
  }, [htmlFile, skillName]);

  const srcdoc = useMemo(() => {
    if (!content) return "";
    return content.startsWith("<!DOCTYPE") || content.startsWith("<html")
      ? content
      : `<!DOCTYPE html><html><head><meta charset="utf-8"></head><body>${content}</body></html>`;
  }, [content]);

  if (loading) {
    return (
      <div style={{ display: "flex", justifyContent: "center", padding: 48 }}>
        <Spin size="large" />
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ padding: 24, color: "var(--color-error)" }}>
        加载技能页面失败: {error}
      </div>
    );
  }

  return (
    <iframe
      ref={iframeRef}
      srcDoc={srcdoc}
      title="Skill HTML Page"
      sandbox="allow-scripts"
      style={{
        width: "100%",
        height: "100%",
        minHeight: 400,
        border: "none",
        backgroundColor: "#fff",
      }}
    />
  );
}
