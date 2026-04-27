import { composeHtml, parseHtmlContent } from "@/lib/htmlParser";
import type { ArtifactLanguage } from "@/types/artifact";
import { memo, useCallback, useEffect, useRef, useState } from "react";

interface HtmlPreviewProps {
  html: string;
  css?: string;
  js?: string;
  language?: ArtifactLanguage;
  previewMode?: "split" | "preview" | "code";
  sandbox?: "allow-scripts" | "allow-same-origin";
  onError?: (error: string) => void;
}

export const HtmlPreview = memo(function HtmlPreview({
  html,
  css,
  js,
  previewMode,
  sandbox = "allow-scripts",
  onError,
}: HtmlPreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [error, setError] = useState<string | null>(null);

  const buildSrcDoc = useCallback(() => {
    const parsed = parseHtmlContent(html);
    return composeHtml({
      html: parsed.html,
      css: css || parsed.css,
      js: js || parsed.js,
    });
  }, [html, css, js]);

  useEffect(() => {
    if (previewMode === "preview" && iframeRef.current) {
      try {
        const srcDoc = buildSrcDoc();
        iframeRef.current.srcdoc = srcDoc;
        setError(null);
      } catch (e) {
        const errMsg = e instanceof Error ? e.message : String(e);
        setError(errMsg);
        onError?.(errMsg);
      }
    }
  }, [previewMode, buildSrcDoc, onError]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      if (event.data?.type === "html-preview-error") {
        setError(event.data.message);
        onError?.(event.data.message);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [onError]);

  if (previewMode === "code") {
    return (
      <pre style={{ padding: 16, overflow: "auto" }}>
        <code>{html}</code>
      </pre>
    );
  }

  if (error) {
    return (
      <div
        style={{
          padding: 16,
          background: "#fff",
          color: "#ff4d4f",
          borderRadius: 8,
        }}
      >
        <div style={{ fontWeight: 500, marginBottom: 8 }}>Error:</div>
        <pre style={{ margin: 0 }}>{error}</pre>
      </div>
    );
  }

  return (
    <iframe
      ref={iframeRef}
      sandbox={sandbox}
      style={{
        width: "100%",
        height: "100%",
        border: "none",
        background: "#fff",
        borderRadius: 8,
      }}
    />
  );
});
