import { ArtifactRenderer } from "@/lib/artifactRenderer";
import type { ArtifactFormat } from "@/types";
import { useCallback, useEffect, useRef } from "react";
import { MarkdownPreview } from "./MarkdownPreview";
import { ReactPreview } from "./ReactPreview";
import { SplitView } from "./SplitView";

interface ArtifactPreviewProps {
  code: string;
  format: ArtifactFormat;
}

export function ArtifactPreview({ code, format }: ArtifactPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const renderPreview = useCallback(() => {
    if (!containerRef.current) {
      return;
    }

    containerRef.current.innerHTML = "";

    const previewType = ArtifactRenderer.getPreviewType(format);

    switch (previewType) {
      case "react":
        ArtifactRenderer.prototype.renderReact(code, containerRef.current);
        break;
      case "diagram":
        if (format === "mermaid") {
          ArtifactRenderer.prototype.renderMermaid(code, containerRef.current);
        } else if (format === "d2") {
          ArtifactRenderer.prototype.renderD2(code, containerRef.current);
        } else {
          ArtifactRenderer.prototype.renderSvg(code, containerRef.current);
        }
        break;
      case "html":
        iframeRef.current = ArtifactRenderer.prototype.createSandbox(
          containerRef.current,
        );
        ArtifactRenderer.prototype.renderHtml({
          html: code,
          css: "",
          js: "",
        });
        break;
      default:
        containerRef.current.textContent = code;
    }
  }, [code, format]);

  useEffect(() => {
    renderPreview();
    return () => {
      if (iframeRef.current) {
        iframeRef.current.remove();
        iframeRef.current = null;
      }
    };
  }, [renderPreview]);

  // Use ReactPreview for code-based react artifacts
  if (code.startsWith("import React") || code.includes("export default function")) {
    return (
      <div style={{ width: "100%", height: "100%", minHeight: 200 }}>
        <ReactPreview code={code} />
      </div>
    );
  }

  // Use MarkdownPreview for content that looks like markdown
  if (format === "text" && (code.startsWith("#") || code.includes("##") || code.startsWith(">"))) {
    return (
      <div
        style={{
          width: "100%",
          height: "100%",
          minHeight: 200,
          overflow: "auto",
          padding: 16,
        }}
      >
        <MarkdownPreview content={code} />
      </div>
    );
  }

  // Use SplitView for html artifacts with code/preview toggle
  if (format === "html") {
    return (
      <SplitView
        code={code}
        language="html"
        showPreview
      />
    );
  }

  // Fallback: imperative render
  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height: "100%",
        minHeight: "200px",
        backgroundColor: "#fff",
        overflow: "auto",
      }}
    />
  );
}
