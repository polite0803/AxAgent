import { describe, expect, it, vi } from "vitest";

import type { ArtifactFormat } from "@/types/artifact";
import { ArtifactRenderer } from "../artifactRenderer";

describe("ArtifactRenderer", () => {
  // ═══════════════════════════════════════════════════════════════
  // getPreviewType
  // ═══════════════════════════════════════════════════════════════
  describe("getPreviewType", () => {
    it('jsx 格式应返回 "react"', () => {
      expect(ArtifactRenderer.getPreviewType("jsx")).toBe("react");
    });

    it('tsx 格式应返回 "react"', () => {
      expect(ArtifactRenderer.getPreviewType("tsx")).toBe("react");
    });

    it('mermaid 格式应返回 "diagram"', () => {
      expect(ArtifactRenderer.getPreviewType("mermaid")).toBe("diagram");
    });

    it('d2 格式应返回 "diagram"', () => {
      expect(ArtifactRenderer.getPreviewType("d2")).toBe("diagram");
    });

    it('svg 格式应返回 "diagram"', () => {
      expect(ArtifactRenderer.getPreviewType("svg")).toBe("diagram");
    });

    it('html 格式应返回 "html"', () => {
      expect(ArtifactRenderer.getPreviewType("html")).toBe("html");
    });

    it('未知格式（如 markdown）应返回 "code"', () => {
      expect(ArtifactRenderer.getPreviewType("markdown")).toBe("code");
    });

    it('json 格式应返回 "code"', () => {
      expect(ArtifactRenderer.getPreviewType("json")).toBe("code");
    });

    it('python 格式应返回 "code"', () => {
      expect(ArtifactRenderer.getPreviewType("python")).toBe("code");
    });

    it("应覆盖所有 diagram 类格式", () => {
      const diagramFormats: ArtifactFormat[] = ["mermaid", "d2", "svg"];
      for (const fmt of diagramFormats) {
        expect(ArtifactRenderer.getPreviewType(fmt)).toBe("diagram");
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // renderSvg — 唯一完整实现的方法
  // ═══════════════════════════════════════════════════════════════
  describe("renderSvg", () => {
    it("应将 SVG 代码直接写入容器 innerHTML", () => {
      const container = document.createElement("div");
      const svgCode = '<svg><circle cx="10" cy="10" r="5"/></svg>';

      const renderer = new ArtifactRenderer();
      renderer.renderSvg(svgCode, container);

      expect(container.innerHTML).toBe(svgCode);
    });

    it("空 SVG 代码也应正常工作", () => {
      const container = document.createElement("div");

      const renderer = new ArtifactRenderer();
      renderer.renderSvg("", container);

      expect(container.innerHTML).toBe("");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // renderReact — 占位实现
  // ═══════════════════════════════════════════════════════════════
  describe("renderReact", () => {
    it("应在容器中渲染占位提示", () => {
      const container = document.createElement("div");
      const renderer = new ArtifactRenderer();

      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      renderer.renderReact("code", container);

      expect(container.innerHTML).toContain("not available");
      warnSpy.mockRestore();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // renderMermaid — 占位实现
  // ═══════════════════════════════════════════════════════════════
  describe("renderMermaid", () => {
    it("应使用 pre 标签包裹代码", () => {
      const container = document.createElement("div");
      const code = "graph TD; A-->B;";

      const renderer = new ArtifactRenderer();
      renderer.renderMermaid(code, container);

      expect(container.innerHTML).toContain("<pre>");
      expect(container.innerHTML).toContain("graph TD");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // renderD2 — 占位实现
  // ═══════════════════════════════════════════════════════════════
  describe("renderD2", () => {
    it("应使用 pre 标签包裹代码", () => {
      const container = document.createElement("div");
      const code = "x -> y";

      const renderer = new ArtifactRenderer();
      renderer.renderD2(code, container);

      expect(container.innerHTML).toContain("<pre>");
      expect(container.innerHTML).toContain("x -&gt; y"); // HTML 实体转义
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // createSandbox
  // ═══════════════════════════════════════════════════════════════
  describe("createSandbox", () => {
    it("应创建带 allow-scripts 的 iframe", () => {
      const container = document.createElement("div");
      const renderer = new ArtifactRenderer();

      const iframe = renderer.createSandbox(container);

      expect(iframe.tagName).toBe("IFRAME");
      expect(iframe.sandbox.contains("allow-scripts")).toBe(true);
    });

    it("iframe 应为全宽全高无边框样式", () => {
      const container = document.createElement("div");
      const renderer = new ArtifactRenderer();

      const iframe = renderer.createSandbox(container);

      expect(iframe.style.width).toBe("100%");
      expect(iframe.style.height).toBe("100%");
      expect(iframe.style.border).toBe("none");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // renderHtml — 占位实现
  // ═══════════════════════════════════════════════════════════════
  describe("renderHtml", () => {
    it("调用不应抛出异常", () => {
      const renderer = new ArtifactRenderer();

      expect(() => {
        renderer.renderHtml({ html: "<div>test</div>" });
      }).not.toThrow();
    });
  });
});
