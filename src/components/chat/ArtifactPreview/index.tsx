import { useRef, useEffect, useCallback } from "react";
import type { ArtifactFormat } from "@/types/artifact";
import { ArtifactRenderer } from "@/lib/artifactRenderer";

interface ArtifactPreviewProps {
  code: string;
  format: ArtifactFormat;
}

export function ArtifactPreview({ code, format }: ArtifactPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  const renderPreview = useCallback(() => {
    if (!containerRef.current) return;

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
        iframeRef.current = ArtifactRenderer.prototype.createSandbox(containerRef.current);
        ArtifactRenderer.prototype.renderHtml({
          html: code,
          css: "",
          js: "",
        });
        break;
      default:
        containerRef.current.innerHTML = `<pre style="padding: 16px; background: #f5f5f5;">${code}</pre>`;
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
