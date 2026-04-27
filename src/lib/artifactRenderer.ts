import type { ArtifactFormat } from "@/types/artifact";

export type PreviewType = "react" | "diagram" | "html" | "code";

export class ArtifactRenderer {
  static getPreviewType(format: ArtifactFormat): PreviewType {
    switch (format) {
      case "jsx":
      case "tsx":
        return "react";
      case "mermaid":
      case "d2":
      case "svg":
        return "diagram";
      case "html":
        return "html";
      default:
        return "code";
    }
  }

  renderReact(_code: string, container: HTMLElement): void {
    console.warn("[ArtifactRenderer] renderReact not fully implemented");
    container.innerHTML = `<div style="padding:16px;background:#f5f5f5;">React preview not available</div>`;
  }

  renderMermaid(code: string, container: HTMLElement): void {
    console.warn("[ArtifactRenderer] renderMermaid not fully implemented");
    container.innerHTML = `<pre>${code}</pre>`;
  }

  renderD2(code: string, container: HTMLElement): void {
    console.warn("[ArtifactRenderer] renderD2 not fully implemented");
    container.innerHTML = `<pre>${code}</pre>`;
  }

  renderSvg(code: string, container: HTMLElement): void {
    container.innerHTML = code;
  }

  createSandbox(_container: HTMLElement): HTMLIFrameElement {
    const iframe = document.createElement("iframe");
    iframe.sandbox.add("allow-scripts");
    iframe.style.width = "100%";
    iframe.style.height = "100%";
    iframe.style.border = "none";
    return iframe;
  }

  renderHtml(_params: { html: string; css?: string; js?: string }): void {
    console.warn("[ArtifactRenderer] renderHtml not fully implemented");
  }
}
