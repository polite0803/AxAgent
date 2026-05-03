import { SyncOutlined } from "@ant-design/icons";
import Think from "@ant-design/x/es/think";
import { theme, Tooltip, Typography } from "antd";
import {
  Brain,
  Check,
  ChevronDown,
  ChevronRight,
  Code,
  Copy,
  Download,
  FileCode,
  FileText,
  FileType,
  Zap,
} from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";

import NodeRenderer, {
  type InfographicBlockActionContext,
  type MermaidBlockActionContext,
  type NodeComponentProps,
  setCustomComponents,
} from "markstream-react";

import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { CHAT_CUSTOM_HTML_TAGS, type ChatMarkdownNode, parseChatMarkdown } from "@/lib/chatMarkdown";
import { useAgentStore, useSettingsStore } from "@/stores";
import { useTranslation } from "react-i18next";
import { formatDuration } from "../gateway/tokenFormat";
import { CodeBlockHeaderActions } from "./CodeBlockHeaderActions";
import { DiagramModeToggle } from "./DiagramModeToggle";
import { InfographicBlockHeaderActions } from "./InfographicBlockHeaderActions";
import { KnowledgeRetrievalNode } from "./KnowledgeRetrievalNode";
import { McpContainerNode } from "./McpContainerNode";
import { MemoryRetrievalNode } from "./MemoryRetrievalNode";
import { MermaidBlockHeaderActions } from "./MermaidBlockHeaderActions";
import { MermaidZoomControls } from "./MermaidZoomControls";
import { StructuredThinking } from "./StructuredThinking";
import { WebSearchNode } from "./WebSearchNode";

interface CodeBlockActionContext {
  collapsed: boolean;
  toggleCollapse: () => void;
  copied: boolean;
  copy: () => void;
  expanded: boolean;
  toggleExpand: () => void;
  fontSize: number;
  defaultFontSize: number;
  decreaseFontSize: () => void;
  increaseFontSize: () => void;
  resetFontSize: () => void;
  isPreviewable: boolean;
  previewCode: () => void;
}

interface CodeBlockPreviewPayload {
  language: string;
  code: string;
  filename?: string;
  node?: { code?: string; type: string; language?: string; raw: string };
  artifactType?: "text/html" | "image/svg+xml";
  artifactTitle?: string;
  id?: string;
}
// ── markstream-react custom thinking component ──────────────────────────

const THINKING_LOADING_MARKER = "<!--axagent-thinking-loading-->";
const DEFAULT_LIGHT_CODE_BLOCK_THEME = "github-light";
const DEFAULT_DARK_CODE_BLOCK_THEME = "poimandres";
const DANGEROUS_D2_STYLE_PATTERNS = [
  /javascript:/i,
  /expression\s*\(/i,
  /url\s*\(\s*javascript:/i,
  /@import/i,
] as const;
const SAFE_D2_URL_PATTERN = /^(?:https?:|mailto:|tel:|#|\/|data:image\/(?:png|gif|jpe?g|webp);)/i;
const CHAT_D2_DARK_THEME_ID = 200;
const CHAT_RENDER_BATCH_PROPS = {
  viewportPriority: true,
  deferNodesUntilVisible: false,
  initialRenderBatchSize: 24,
  renderBatchSize: 48,
  renderBatchDelay: 24,
  renderBatchBudgetMs: 4,
  maxLiveNodes: Infinity,
  liveNodeBuffer: 24,
} as const;
type CustomNodeAttrs =
  | Record<string, string | boolean>
  | [string, string][]
  | Array<{ name: string; value: string | boolean }>
  | null
  | undefined;

function normalizeCodeTheme(raw?: string) {
  const t = raw?.trim();
  if (t === "vs-code" || t === "vscode") { return "dark-plus"; }
  if (t === "one-dark") { return "one-dark-pro"; }
  return t || undefined;
}

function getChatCodeThemes(selectedDarkTheme?: string, selectedLightTheme?: string) {
  const darkTheme = normalizeCodeTheme(selectedDarkTheme) || DEFAULT_DARK_CODE_BLOCK_THEME;
  const lightTheme = normalizeCodeTheme(selectedLightTheme) || DEFAULT_LIGHT_CODE_BLOCK_THEME;
  const themes = [lightTheme, darkTheme];
  if (lightTheme !== darkTheme) {
    themes.push(darkTheme);
  }
  return {
    darkTheme,
    lightTheme,
    themes,
  };
}

let _codeBlockPreviewHandler: ((payload: CodeBlockPreviewPayload) => void) | null = null;
let _mermaidOpenModalHandler: ((svgString: string | null) => void) | null = null;

function getChatCodeBlockProps(darkTheme: string, lightTheme: string) {
  return {
    darkTheme,
    lightTheme,
    maxHeight: "none",
    renderHeaderActions: (ctx: CodeBlockActionContext) => <CodeBlockHeaderActions ctx={ctx} />,
    onPreviewCode: (payload: CodeBlockPreviewPayload) => {
      _codeBlockPreviewHandler?.(payload);
    },
  };
}

const CHAT_MERMAID_PROPS = {
  renderHeaderActions: (ctx: MermaidBlockActionContext) => <MermaidBlockHeaderActions ctx={ctx} />,
  renderModeToggle: (ctx: MermaidBlockActionContext) => (
    <DiagramModeToggle showSource={ctx.showSource} onSwitchMode={ctx.switchMode} />
  ),
  renderZoomControls: (ctx: MermaidBlockActionContext) => <MermaidZoomControls ctx={ctx} />,
  onOpenModal: (ev: { preventDefault: () => void; svgString?: string | null }) => {
    if (_mermaidOpenModalHandler) {
      ev.preventDefault();
      _mermaidOpenModalHandler(ev.svgString ?? null);
    }
  },
};

const CHAT_INFOGRAPHIC_PROPS = {
  renderHeaderActions: (ctx: InfographicBlockActionContext) => <InfographicBlockHeaderActions ctx={ctx} />,
  renderModeToggle: (ctx: InfographicBlockActionContext) => (
    <DiagramModeToggle showSource={ctx.showSource} onSwitchMode={ctx.switchMode} />
  ),
  renderZoomControls: (ctx: InfographicBlockActionContext) => <MermaidZoomControls ctx={ctx as any} />,
};

function getCustomAttr(attrs: CustomNodeAttrs, name: string): string | undefined {
  if (!attrs) { return undefined; }

  if (Array.isArray(attrs)) {
    for (const attr of attrs) {
      if (Array.isArray(attr)) {
        const [attrName, value] = attr;
        if (attrName === name) { return value; }
        continue;
      }

      if (attr && typeof attr === "object" && "name" in attr && attr.name === name) {
        return typeof attr.value === "string" ? attr.value : undefined;
      }
    }
    return undefined;
  }

  const value = attrs[name];
  return typeof value === "string" ? value : undefined;
}

function isChatD2CodeBlockNode(node: ChatMarkdownNode): node is ChatD2CodeBlockNode {
  return node.type === "code_block"
    && "code" in node
    && typeof node.code === "string"
    && (!("language" in node) || typeof node.language === "string" || typeof node.language === "undefined");
}

function getSingleD2CodeBlockNode(nodes?: ChatMarkdownNode[]) {
  if (!nodes || nodes.length !== 1) { return null; }

  const [firstNode] = nodes;
  if (!isChatD2CodeBlockNode(firstNode) || firstNode.language?.trim().toLowerCase() !== "d2") {
    return null;
  }

  return firstNode;
}

function containsDeferredHeavyNode(nodes?: ChatMarkdownNode[]) {
  if (!nodes) { return false; }

  const stack: unknown[] = [...nodes];
  while (stack.length > 0) {
    const current = stack.pop();
    if (!current || typeof current !== "object") {
      continue;
    }

    if ("type" in current && current.type === "code_block") {
      return true;
    }

    for (const value of Object.values(current)) {
      if (Array.isArray(value)) {
        stack.push(...value);
      }
    }
  }

  return false;
}

function sanitizeD2Url(url: string) {
  const value = url.trim();
  return SAFE_D2_URL_PATTERN.test(value) ? value : "";
}

function sanitizeD2Svg(svg: string) {
  if (typeof document === "undefined" || typeof DOMParser === "undefined") {
    return "";
  }

  const sanitizeTree = (root: Element) => {
    const blockedTags = new Set(["script"]);
    const nodes = [root, ...Array.from(root.querySelectorAll("*"))];

    for (const element of nodes) {
      if (blockedTags.has(element.tagName.toLowerCase())) {
        element.remove();
        continue;
      }

      for (const attr of Array.from(element.attributes)) {
        const name = attr.name;
        if (/^on/i.test(name)) {
          element.removeAttribute(name);
          continue;
        }

        if (name === "style" && attr.value && DANGEROUS_D2_STYLE_PATTERNS.some((pattern) => pattern.test(attr.value))) {
          element.removeAttribute(name);
          continue;
        }

        if ((name === "href" || name === "xlink:href") && attr.value) {
          const safeUrl = sanitizeD2Url(attr.value);
          if (!safeUrl) {
            element.removeAttribute(name);
            continue;
          }
          if (safeUrl !== attr.value) {
            element.setAttribute(name, safeUrl);
          }
        }
      }
    }
  };

  const normalizedSvg = svg
    .replace(/["']\s*javascript:/gi, "#")
    .replace(/\bjavascript:/gi, "#")
    .replace(/["']\s*vbscript:/gi, "#")
    .replace(/\bvbscript:/gi, "#")
    .replace(/\bdata:text\/html/gi, "#");

  const xmlRoot = new DOMParser().parseFromString(normalizedSvg, "image/svg+xml").documentElement;
  if (xmlRoot && xmlRoot.nodeName.toLowerCase() === "svg") {
    sanitizeTree(xmlRoot);
    return xmlRoot.outerHTML;
  }

  const container = document.createElement("div");
  container.innerHTML = normalizedSvg;
  const htmlSvg = container.querySelector("svg");
  if (!htmlSvg) {
    return "";
  }

  sanitizeTree(htmlSvg);
  return htmlSvg.outerHTML;
}

type ChatD2Instance = {
  compile: (source: string) => Promise<unknown>;
  render: (diagram: unknown, options?: unknown) => Promise<unknown>;
};

type ChatD2Constructor = new() => ChatD2Instance;

let chatD2CtorPromise: Promise<ChatD2Constructor> | null = null;

async function loadChatD2Ctor() {
  if (!chatD2CtorPromise) {
    chatD2CtorPromise = import("@terrastruct/d2").then((module) => {
      if (typeof module.D2 !== "function") {
        throw new Error("Failed to resolve D2 constructor from @terrastruct/d2.");
      }

      return module.D2 as ChatD2Constructor;
    });
  }

  return chatD2CtorPromise;
}
function ThinkNode(
  props: NodeComponentProps<{
    type: "think";
    content: string;
    attrs?: CustomNodeAttrs;
  }>,
) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const selectedDarkCodeTheme = useSettingsStore((s) => s.settings.code_theme);
  const selectedLightCodeTheme = useSettingsStore((s) => s.settings.code_theme_light);
  const codeFontFamily = useSettingsStore((s) => s.settings.code_font_family);
  const { node, ctx } = props;
  const thinkingNodesCacheRef = useRef<Map<string, ChatMarkdownNode[]>>(new Map());
  const rawThinkingContent = String(node.content ?? "");
  const isStreaming = rawThinkingContent.includes(THINKING_LOADING_MARKER);
  const totalMsAttr = getCustomAttr(node.attrs, "totalMs") ?? getCustomAttr(node.attrs, "totalms");
  const totalMs = totalMsAttr ? parseInt(totalMsAttr, 10) : null;
  const thinkingContent = rawThinkingContent
    .replace(`${THINKING_LOADING_MARKER}\n`, "")
    .replace(THINKING_LOADING_MARKER, "");
  const [expanded, setExpanded] = useState(isStreaming);
  const [showRawMarkdown, setShowRawMarkdown] = useState(false);
  const prevStreamingRef = useRef(isStreaming);

  useEffect(() => {
    setExpanded(isStreaming);
    prevStreamingRef.current = isStreaming;
  }, [isStreaming]);

  useEffect(() => {
    if (isStreaming) {
      setExpanded(true);
      setShowRawMarkdown(false);
    } else if (prevStreamingRef.current) {
      setExpanded(false);
    }
    prevStreamingRef.current = isStreaming;
  }, [isStreaming]);

  const title = isStreaming
    ? t("chat.thinkingInProgress")
    : totalMs && !isNaN(totalMs)
    ? `${t("chat.thinkingComplete")} ${formatDuration(totalMs)}`
    : t("chat.thinkingComplete");

  const thinkingNodes = useMemo(() => {
    const cache = thinkingNodesCacheRef.current;
    const cached = cache.get(thinkingContent);
    if (cached) { return cached; }

    const parsed = parseChatMarkdown(thinkingContent);
    cache.set(thinkingContent, parsed);
    if (cache.size > 24) {
      const firstKey = cache.keys().next().value;
      if (firstKey) { cache.delete(firstKey); }
    }
    return parsed;
  }, [thinkingContent]);
  const { darkTheme, lightTheme, themes } = useMemo(
    () => getChatCodeThemes(selectedDarkCodeTheme, selectedLightCodeTheme),
    [selectedDarkCodeTheme, selectedLightCodeTheme],
  );
  const codeBlockProps = useMemo(
    () => getChatCodeBlockProps(darkTheme, lightTheme),
    [darkTheme, lightTheme],
  );
  const codeBlockMonacoOptions = useMemo(
    () => codeFontFamily ? { fontFamily: codeFontFamily } : undefined,
    [codeFontFamily],
  );
  const customHtmlTags = useMemo(
    () => CHAT_CUSTOM_HTML_TAGS.filter((t) => t !== "think"),
    [],
  );
  const rendererKey = `${ctx?.customId ?? "default"}:${ctx?.isDark ? "dark" : "light"}:${darkTheme}:${lightTheme}`;

  // 结构化思考：当思考内容较长时显示分阶段视图
  const hasStructuredPhases = thinkingContent.length > 200;

  return (
    <Think
      title={title}
      blink={isStreaming}
      loading={isStreaming
        ? <SyncOutlined style={{ fontSize: 12, animation: "axagent-think-spin 1s linear infinite" }} />
        : false}
      icon={<Brain size={14} />}
      expanded={expanded}
      onExpand={setExpanded}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
        {/* 结构化思考：分阶段展示 */}
        {hasStructuredPhases && (
          <StructuredThinking
            thinking={thinkingContent}
            isStreaming={isStreaming}
            totalMs={totalMs}
          />
        )}
        {/* 原始 Markdown 渲染（可切换） */}
        {(!hasStructuredPhases || showRawMarkdown) && (
          <NodeRenderer
            key={`think:${rendererKey}:${isStreaming ? "s" : "f"}`}
            nodes={thinkingNodes}
            customId={ctx?.customId}
            isDark={ctx?.isDark}
            final={!isStreaming}
            typewriter={false}
            themes={themes}
            codeBlockLightTheme={lightTheme}
            codeBlockDarkTheme={darkTheme}
            codeBlockProps={codeBlockProps}
            codeBlockMonacoOptions={codeBlockMonacoOptions}
            customHtmlTags={customHtmlTags}
            mermaidProps={CHAT_MERMAID_PROPS}
            infographicProps={CHAT_INFOGRAPHIC_PROPS}
            {...CHAT_RENDER_BATCH_PROPS}
          />
        )}
        {/* 切换按钮：在结构化视图和原始 Markdown 之间切换 */}
        {hasStructuredPhases && !isStreaming && (
          <div
            onClick={() => setShowRawMarkdown(!showRawMarkdown)}
            style={{
              textAlign: "center",
              padding: "4px 0",
              fontSize: 11,
              color: token.colorTextQuaternary,
              cursor: "pointer",
              userSelect: "none",
            }}
          >
            {showRawMarkdown ? "收起原始内容" : "查看原始思考"}
          </div>
        )}
      </div>
    </Think>
  );
}
type ChatD2CodeBlockNode = {
  type: "code_block";
  language?: string;
  code: string;
  raw: string;
  loading?: boolean;
};

function ChatD2BlockNode({
  node,
  isDark,
}: {
  node: ChatD2CodeBlockNode;
  isDark?: boolean;
}) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [showSource, setShowSource] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const { copy: copyD2, isCopied: d2Copied } = useCopyToClipboard({ timeout: 1000 });
  const [svgMarkup, setSvgMarkup] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [canRenderPreview, setCanRenderPreview] = useState(false);
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);

  useEffect(() => {
    setCanRenderPreview(false);
    if (showSource) { return; }

    const element = containerRef.current;
    if (!element || typeof window === "undefined" || typeof IntersectionObserver === "undefined") {
      setCanRenderPreview(true);
      return;
    }

    let frameId = 0;
    let timeoutId: number | null = null;
    const win = window as Window & {
      requestIdleCallback?: (callback: () => void, options?: { timeout: number }) => number;
      cancelIdleCallback?: (handle: number) => void;
    };
    const observer = new IntersectionObserver((entries) => {
      if (!entries[0]?.isIntersecting) { return; }
      observer.disconnect();
      frameId = window.requestAnimationFrame(() => {
        if (typeof win.requestIdleCallback === "function") {
          timeoutId = win.requestIdleCallback(() => setCanRenderPreview(true), { timeout: 250 });
          return;
        }
        timeoutId = window.setTimeout(() => setCanRenderPreview(true), 0);
      });
    }, { rootMargin: "160px 0px" });

    observer.observe(element);
    return () => {
      observer.disconnect();
      window.cancelAnimationFrame(frameId);
      if (timeoutId !== null) {
        if (typeof win.cancelIdleCallback === "function") {
          win.cancelIdleCallback(timeoutId);
        } else {
          window.clearTimeout(timeoutId);
        }
      }
    };
  }, [node.code, showSource]);

  useEffect(() => {
    let cancelled = false;
    if (!canRenderPreview || showSource) {
      return () => {
        cancelled = true;
      };
    }

    const renderD2 = async () => {
      const source = String(node.code ?? "");
      if (!source) {
        setSvgMarkup("");
        setError(null);
        return;
      }

      setError(null);

      try {
        const D2Ctor = await loadChatD2Ctor();
        const instance = new D2Ctor();
        const compiled = await instance.compile(source) as {
          diagram?: unknown;
          renderOptions?: Record<string, unknown>;
          options?: Record<string, unknown>;
        } | unknown;
        const diagram = typeof compiled === "object" && compiled !== null && "diagram" in compiled
          ? compiled.diagram
          : compiled;
        const renderOptions = typeof compiled === "object" && compiled !== null
          ? ("renderOptions" in compiled && compiled.renderOptions) || ("options" in compiled && compiled.options) || {}
          : {};
        const nextRenderOptions = typeof renderOptions === "object" && renderOptions !== null
          ? { ...renderOptions as Record<string, unknown> }
          : {};

        if (isDark) {
          nextRenderOptions.themeID = typeof nextRenderOptions.darkThemeID === "number"
            ? nextRenderOptions.darkThemeID
            : CHAT_D2_DARK_THEME_ID;
          nextRenderOptions.darkThemeID = null;
          nextRenderOptions.darkThemeOverrides = null;
          nextRenderOptions.themeOverrides = {
            N1: token.colorText,
            N2: token.colorTextSecondary,
            N3: token.colorTextTertiary,
            N4: token.colorTextQuaternary,
            N5: token.colorBorder,
            N6: token.colorBorderSecondary,
            N7: token.colorBgContainer,
            B1: token.colorText,
            B2: token.colorTextSecondary,
            B3: token.colorTextTertiary,
            B4: token.colorBorder,
            B5: token.colorBorderSecondary,
            B6: token.colorBgElevated,
            AA2: token.colorTextSecondary,
            AA4: token.colorTextTertiary,
            AA5: token.colorBorder,
            AB4: token.colorTextSecondary,
            AB5: token.colorTextTertiary,
            ...(typeof nextRenderOptions.themeOverrides === "object" && nextRenderOptions.themeOverrides !== null
              ? nextRenderOptions.themeOverrides as Record<string, unknown>
              : {}),
          };
        }

        const rendered = await instance.render(diagram, nextRenderOptions);
        const rawSvg = typeof rendered === "string"
          ? rendered
          : typeof rendered === "object" && rendered !== null && "svg" in rendered && typeof rendered.svg === "string"
          ? rendered.svg
          : typeof rendered === "object" && rendered !== null && "data" in rendered && typeof rendered.data === "string"
          ? rendered.data
          : "";

        if (!rawSvg) {
          throw new Error("D2 render returned empty output.");
        }

        const sanitizedSvg = sanitizeD2Svg(rawSvg);
        if (!sanitizedSvg) {
          throw new Error("D2 SVG sanitization failed in the current WebView.");
        }

        if (cancelled) { return; }
        setSvgMarkup(sanitizedSvg);
      } catch (renderError) {
        if (cancelled) { return; }
        setSvgMarkup("");
        setError(renderError instanceof Error ? renderError.message : "D2 render failed.");
      }
    };

    void renderD2();

    return () => {
      cancelled = true;
    };
  }, [
    canRenderPreview,
    isDark,
    node.code,
    showSource,
    token.colorBgContainer,
    token.colorBgElevated,
    token.colorBorder,
    token.colorBorderSecondary,
    token.colorText,
    token.colorTextQuaternary,
    token.colorTextSecondary,
    token.colorTextTertiary,
  ]);

  const handleExport = useCallback(() => {
    if (!svgMarkup) { return; }

    const blob = new Blob([svgMarkup], { type: "image/svg+xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `d2-diagram-${Date.now()}.svg`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }, [svgMarkup]);

  const shellStyle = useMemo(() => ({
    borderColor: isDark ? token.colorBorderSecondary : token.colorBorderSecondary,
    background: isDark ? token.colorBgElevated : token.colorBgContainer,
    color: token.colorText,
  }), [isDark, token.colorBgContainer, token.colorBgElevated, token.colorBorderSecondary, token.colorText]);

  const headerStyle = useMemo(() => ({
    color: token.colorText,
    backgroundColor: isDark ? token.colorBgContainer : token.colorFillAlter,
    borderBottomColor: token.colorBorderSecondary,
  }), [isDark, token.colorBgContainer, token.colorBorderSecondary, token.colorFillAlter, token.colorText]);

  const getD2BtnStyle = useCallback((idx: number): React.CSSProperties => ({
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    width: 28,
    height: 28,
    borderRadius: token.borderRadiusSM,
    border: "none",
    background: hoveredIdx === idx ? (token.colorFillSecondary || "rgba(255,255,255,0.1)") : "transparent",
    color: hoveredIdx === idx ? token.colorText : token.colorTextSecondary,
    cursor: "pointer",
    padding: 0,
    transition: "color 0.2s, background 0.2s",
  }), [hoveredIdx, token]);

  const previewStyle = useMemo(() => ({
    background: isDark ? token.colorBgContainer : token.colorBgElevated,
  }), [isDark, token.colorBgContainer, token.colorBgElevated]);

  return (
    <div ref={containerRef} className="d2-block my-4 rounded-lg border overflow-hidden shadow-sm" style={shellStyle}>
      <div
        className="d2-block-header flex justify-between items-center px-4 py-1.5 border-b border-gray-400/5"
        style={headerStyle}
      >
        <div className="flex items-center gap-x-2">
          <span className="text-sm font-medium font-mono">D2</span>
        </div>
        <div className="flex items-center gap-x-2">
          <DiagramModeToggle
            showSource={showSource}
            onSwitchMode={(mode) =>
              setShowSource(mode === "source")}
          />
          {/* Collapse */}
          <Tooltip title={isCollapsed ? t("common.expand") : t("common.collapse")} mouseEnterDelay={0.4}>
            <button
              type="button"
              style={getD2BtnStyle(0)}
              onClick={() =>
                setIsCollapsed(v =>
                  !v
                )}
              onMouseEnter={() => setHoveredIdx(0)}
              onMouseLeave={() => setHoveredIdx(null)}
            >
              <ChevronRight
                size={14}
                style={{
                  transform: isCollapsed ? "rotate(0deg)" : "rotate(90deg)",
                  transition: "transform 0.2s",
                }}
              />
            </button>
          </Tooltip>
          {/* Copy */}
          <Tooltip title={d2Copied ? t("common.copied") : t("common.copy")} mouseEnterDelay={0.4}>
            <button
              type="button"
              style={getD2BtnStyle(1)}
              onClick={() => void copyD2(node.code)}
              onMouseEnter={() => setHoveredIdx(1)}
              onMouseLeave={() => setHoveredIdx(null)}
            >
              {d2Copied ? <Check size={14} style={{ color: token.colorSuccess }} /> : <Copy size={14} />}
            </button>
          </Tooltip>
          {/* Export */}
          {svgMarkup
            ? (
              <Tooltip title={t("common.export")} mouseEnterDelay={0.4}>
                <button
                  type="button"
                  style={getD2BtnStyle(2)}
                  onClick={handleExport}
                  onMouseEnter={() => setHoveredIdx(2)}
                  onMouseLeave={() => setHoveredIdx(null)}
                >
                  <Download size={14} />
                </button>
              </Tooltip>
            )
            : null}
        </div>
      </div>

      {!isCollapsed && (
        <div className="d2-block-body">
          {showSource || (!svgMarkup && !!error)
            ? (
              <div className="d2-source px-4 py-4">
                <pre className="d2-code"><code>{node.code}</code></pre>
                {error ? <p className="d2-error mt-2 text-xs">{error}</p> : null}
              </div>
            )
            : (
              <div className="d2-render" style={previewStyle}>
                {svgMarkup
                  ? <div className="d2-svg" dangerouslySetInnerHTML={{ __html: svgMarkup }} />
                  : (
                    <div
                      className="flex items-center justify-center px-4 py-10"
                      style={{ color: token.colorTextSecondary, gap: 8 }}
                    >
                      <SyncOutlined spin />
                      <span className="text-sm">
                        {canRenderPreview ? t("chat.renderingChart") : t("chat.chartAboutToRender")}
                      </span>
                    </div>
                  )}
                {error ? <p className="d2-error px-4 pb-3 text-xs">{error}</p> : null}
              </div>
            )}
        </div>
      )}
    </div>
  );
}

function ChatD2Node(props: NodeComponentProps<ChatD2CodeBlockNode>) {
  const { node, ctx } = props;
  return <ChatD2BlockNode node={node} isDark={ctx?.isDark} />;
}

// ── Inline tool-call node (renders inside markdown content flow) ──────

const toolCallIcons: Record<string, React.ReactNode> = {
  bash: <Code size={14} />,
  write: <FileCode size={14} />,
  read: <FileText size={14} />,
  edit: <FileCode size={14} />,
  glob: <FileType size={14} />,
  grep: <FileText size={14} />,
  ls: <FileType size={14} />,
};
function getInlineToolIcon(toolName: string): React.ReactNode {
  const lower = toolName.toLowerCase();
  for (const [key, icon] of Object.entries(toolCallIcons)) {
    if (lower.includes(key)) { return icon; }
  }
  return <Zap size={14} />;
}

const toolCallStatusColors: Record<string, string> = {
  queued: "#faad14",
  running: "#1890ff",
  success: "#52c41a",
  failed: "#ff4d4f",
  cancelled: "#8c8c8c",
};

function ToolCallNode(
  props: NodeComponentProps<{
    type: "tool-call";
    content: string;
    attrs?: CustomNodeAttrs;
  }>,
) {
  const { node } = props;
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const toolCalls = useAgentStore((s) => s.toolCalls);
  const [expanded, setExpanded] = useState(false);

  const execId = getCustomAttr(node.attrs, "id") ?? "";
  const toolName = getCustomAttr(node.attrs, "name") ?? "";
  const summary = String(node.content ?? "");

  const tc = toolCalls[execId];
  const status = tc?.executionStatus ?? "success";
  const statusColor = toolCallStatusColors[status] || token.colorTextSecondary;
  const isLoading = status === "queued" || status === "running";
  const hasDetails = tc && (tc.input || tc.output);

  return (
    <div style={{ margin: "4px 0" }}>
      <div
        onClick={() => hasDetails && setExpanded(!expanded)}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "4px 10px",
          borderRadius: token.borderRadius,
          backgroundColor: token.colorFillQuaternary,
          border: `1px solid ${token.colorBorderSecondary}`,
          fontSize: 13,
          lineHeight: "20px",
          fontFamily: "monospace",
          cursor: hasDetails ? "pointer" : "default",
          userSelect: "none",
        }}
      >
        <span style={{ color: statusColor, display: "flex", alignItems: "center", flexShrink: 0 }}>
          {getInlineToolIcon(toolName)}
        </span>
        <span style={{ fontWeight: 500, flexShrink: 0 }}>{toolName}</span>
        {summary && (
          <>
            <span style={{ color: token.colorTextQuaternary }}>›</span>
            <Typography.Text
              type="secondary"
              ellipsis
              style={{ fontSize: 12, flex: 1, minWidth: 0 }}
            >
              {summary}
            </Typography.Text>
          </>
        )}
        {isLoading ? <SyncOutlined style={{ fontSize: 12, color: statusColor }} spin /> : (
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              backgroundColor: statusColor,
              flexShrink: 0,
            }}
          />
        )}
        {hasDetails && (
          <span
            style={{
              color: token.colorTextSecondary,
              display: "flex",
              alignItems: "center",
              flexShrink: 0,
              transition: "transform 0.2s",
              transform: expanded ? "rotate(180deg)" : "rotate(0deg)",
            }}
          >
            <ChevronDown size={14} />
          </span>
        )}
      </div>
      {expanded && hasDetails && (
        <div
          style={{
            margin: "2px 0 0",
            padding: "6px 10px",
            borderRadius: token.borderRadius,
            backgroundColor: token.colorFillQuaternary,
            border: `1px solid ${token.colorBorderSecondary}`,
            borderTop: "none",
            fontSize: 12,
            display: "flex",
            flexDirection: "column",
            gap: 4,
          }}
        >
          {tc.input && Object.keys(tc.input).length > 0 && (
            <details style={{ margin: 0 }}>
              <summary style={{ fontSize: 12, color: token.colorTextSecondary, cursor: "pointer", userSelect: "none" }}>
                {t("chat.inspector.toolInput", "输入参数")}
              </summary>
              <pre
                style={{
                  margin: "4px 0 0",
                  padding: 8,
                  fontSize: 11,
                  fontFamily: "monospace",
                  backgroundColor: token.colorBgTextHover,
                  borderRadius: token.borderRadius,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-all",
                  maxHeight: 200,
                  overflow: "auto",
                }}
              >
                {JSON.stringify(tc.input, null, 2)}
              </pre>
            </details>
          )}
          {tc.output && (
            <details style={{ margin: 0 }}>
              <summary style={{ fontSize: 12, color: token.colorTextSecondary, cursor: "pointer", userSelect: "none" }}>
                {t("chat.inspector.toolOutput", "执行结果")}
              </summary>
              <pre
                style={{
                  margin: "4px 0 0",
                  padding: 8,
                  fontSize: 11,
                  fontFamily: "monospace",
                  backgroundColor: token.colorBgTextHover,
                  borderRadius: token.borderRadius,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-all",
                  maxHeight: 200,
                  overflow: "auto",
                  color: tc.isError ? token.colorError : undefined,
                }}
              >
                {tc.output}
              </pre>
            </details>
          )}
        </div>
      )}
    </div>
  );
}
setCustomComponents("chat", {
  think: ThinkNode,
  "web-search": WebSearchNode,
  "knowledge-retrieval": KnowledgeRetrievalNode,
  "memory-retrieval": MemoryRetrievalNode,
  "tool-call": ToolCallNode,
  d2: ChatD2Node,
  vmr_container: McpContainerNode,
});

const AssistantMarkdown = React.memo(function AssistantMarkdown({
  content,
  nodes,
  isDarkMode,
  isStreaming,
  codeBlockDarkTheme,
  codeBlockLightTheme,
  codeBlockThemes,
  codeFontFamily,
}: {
  content: string;
  nodes?: ChatMarkdownNode[];
  isDarkMode: boolean;
  isStreaming: boolean;
  codeBlockDarkTheme: string;
  codeBlockLightTheme: string;
  codeBlockThemes: string[];
  codeFontFamily?: string;
}) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const codeBlockProps = useMemo(
    () => getChatCodeBlockProps(codeBlockDarkTheme, codeBlockLightTheme),
    [codeBlockDarkTheme, codeBlockLightTheme],
  );
  const codeBlockMonacoOptions = useMemo(
    () => codeFontFamily ? { fontFamily: codeFontFamily } : undefined,
    [codeFontFamily],
  );
  const singleD2Node = useMemo(() => getSingleD2CodeBlockNode(nodes), [nodes]);
  const hasDeferredHeavyNodes = useMemo(
    () => !isStreaming && (containsDeferredHeavyNode(nodes) || content.includes("```")),
    [content, nodes, isStreaming],
  );
  const [readyToRenderHeavyNodes, setReadyToRenderHeavyNodes] = useState(!hasDeferredHeavyNodes);
  const rendererKey = `${isDarkMode ? "dark" : "light"}:${codeBlockDarkTheme}:${codeBlockLightTheme}`;
  const nodeRendererReseedKey = `${rendererKey}:${isStreaming ? "s" : "f"}`;

  useEffect(() => {
    if (!hasDeferredHeavyNodes) {
      setReadyToRenderHeavyNodes(true);
      return;
    }

    setReadyToRenderHeavyNodes(false);
    const element = containerRef.current;
    if (!element || typeof window === "undefined" || typeof IntersectionObserver === "undefined") {
      setReadyToRenderHeavyNodes(true);
      return;
    }

    let frameId = 0;
    let timeoutId: number | null = null;
    const win = window as Window & {
      requestIdleCallback?: (callback: () => void, options?: { timeout: number }) => number;
      cancelIdleCallback?: (handle: number) => void;
    };
    const observer = new IntersectionObserver((entries) => {
      if (!entries[0]?.isIntersecting) { return; }
      observer.disconnect();
      frameId = window.requestAnimationFrame(() => {
        if (typeof win.requestIdleCallback === "function") {
          timeoutId = win.requestIdleCallback(() => setReadyToRenderHeavyNodes(true), { timeout: 250 });
          return;
        }
        timeoutId = window.setTimeout(() => setReadyToRenderHeavyNodes(true), 0);
      });
    }, { rootMargin: "160px 0px" });

    observer.observe(element);
    return () => {
      observer.disconnect();
      window.cancelAnimationFrame(frameId);
      if (timeoutId !== null) {
        if (typeof win.cancelIdleCallback === "function") {
          win.cancelIdleCallback(timeoutId);
        } else {
          window.clearTimeout(timeoutId);
        }
      }
    };
  }, [content, hasDeferredHeavyNodes]);

  return (
    <>
      {singleD2Node
        ? (
          <ChatD2BlockNode
            key={`d2:${rendererKey}`}
            node={singleD2Node}
            isDark={isDarkMode}
          />
        )
        : hasDeferredHeavyNodes && !readyToRenderHeavyNodes
        ? (
          <div className="axagent-chat-markdown" key={`loading:${rendererKey}`}>
            <div
              ref={containerRef}
              className="my-4 rounded-lg border"
              style={{
                borderColor: token.colorBorderSecondary,
                background: isDarkMode ? token.colorBgContainer : token.colorBgElevated,
              }}
            >
              <div
                className="flex items-center justify-center px-4 py-10"
                style={{ color: token.colorTextSecondary, gap: 8 }}
              >
                <SyncOutlined spin />
                <span className="text-sm">{t("chat.loadingRenderContent")}</span>
              </div>
            </div>
          </div>
        )
        : (
          <div className="axagent-chat-markdown" key={`render:${nodeRendererReseedKey}`}>
            {nodes
              ? (
                <NodeRenderer
                  key={nodeRendererReseedKey}
                  nodes={nodes}
                  isDark={isDarkMode}
                  customId="chat"
                  customHtmlTags={CHAT_CUSTOM_HTML_TAGS}
                  final={!isStreaming}
                  typewriter={isStreaming}
                  themes={codeBlockThemes}
                  codeBlockLightTheme={codeBlockLightTheme}
                  codeBlockDarkTheme={codeBlockDarkTheme}
                  codeBlockProps={codeBlockProps}
                  codeBlockMonacoOptions={codeBlockMonacoOptions}
                  mermaidProps={CHAT_MERMAID_PROPS}
                  infographicProps={CHAT_INFOGRAPHIC_PROPS}
                  {...CHAT_RENDER_BATCH_PROPS}
                />
              )
              : (
                <NodeRenderer
                  key={nodeRendererReseedKey}
                  content={content}
                  isDark={isDarkMode}
                  customId="chat"
                  customHtmlTags={CHAT_CUSTOM_HTML_TAGS}
                  final={!isStreaming}
                  typewriter={isStreaming}
                  themes={codeBlockThemes}
                  codeBlockLightTheme={codeBlockLightTheme}
                  codeBlockDarkTheme={codeBlockDarkTheme}
                  codeBlockProps={codeBlockProps}
                  codeBlockMonacoOptions={codeBlockMonacoOptions}
                  mermaidProps={CHAT_MERMAID_PROPS}
                  infographicProps={CHAT_INFOGRAPHIC_PROPS}
                  {...CHAT_RENDER_BATCH_PROPS}
                />
              )}
          </div>
        )}
    </>
  );
}, (prev, next) => (
  prev.content === next.content
  && prev.nodes === next.nodes
  && prev.isDarkMode === next.isDarkMode
  && prev.isStreaming === next.isStreaming
  && prev.codeBlockDarkTheme === next.codeBlockDarkTheme
  && prev.codeBlockLightTheme === next.codeBlockLightTheme
  && prev.codeBlockThemes === next.codeBlockThemes
  && prev.codeFontFamily === next.codeFontFamily
));

export {
  AssistantMarkdown,
  CHAT_INFOGRAPHIC_PROPS,
  CHAT_MERMAID_PROPS,
  CHAT_RENDER_BATCH_PROPS,
  type CodeBlockActionContext,
  type CodeBlockPreviewPayload,
  DEFAULT_DARK_CODE_BLOCK_THEME,
  DEFAULT_LIGHT_CODE_BLOCK_THEME,
  getChatCodeBlockProps,
  getChatCodeThemes,
  normalizeCodeTheme,
  THINKING_LOADING_MARKER,
};

export function getCodeBlockPreviewHandler() {
  return _codeBlockPreviewHandler;
}
export function setCodeBlockPreviewHandler(h: ((payload: CodeBlockPreviewPayload) => void) | null) {
  _codeBlockPreviewHandler = h;
}
export function getMermaidOpenModalHandler() {
  return _mermaidOpenModalHandler;
}
export function setMermaidOpenModalHandler(h: ((svgString: string | null) => void) | null) {
  _mermaidOpenModalHandler = h;
}
