import { getDualView } from "@/lib/dualView";
import { Maximize2, Minimize2 } from "lucide-react";
import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelCollapseButton } from "./PanelCollapseButton";

interface DualViewRendererProps {
  id: string;
  data: unknown;
  /** 初始模式:panel 完整显示,bubble 紧凑显示(带展开/折叠切换) */
  defaultMode?: "panel" | "bubble";
  /** 强制模式:不显示切换按钮 */
  forceMode?: "panel" | "bubble";
}

/**
 * DualViewRenderer — 同一个 dual view 的两种渲染出口
 * - panel: 完整 panel 内容,不可折叠
 * - bubble: 紧凑 1-2 行 + 折叠/展开切换
 */
export function DualViewRenderer({
  id,
  data,
  defaultMode = "panel",
  forceMode,
}: DualViewRendererProps): ReactNode {
  const { t } = useTranslation();
  const view = getDualView(id);
  const [mode, setMode] = useState<"panel" | "bubble">(defaultMode);
  const [expanded, setExpanded] = useState(false);

  if (!view) {
    return (
      <div className="text-[11px] italic" style={{ color: "var(--muted)" }}>
        {t("dualView.notRegistered", { id })}
      </div>
    );
  }

  if (view.noDualView) {
    return <>{view.panel(data)}</>;
  }

  // 强制模式
  if (forceMode === "panel") {
    const collapseContent = typeof data === "string" ? data : JSON.stringify(data ?? {});
    return (
      <div className="dual-panel">
        <div className="flex justify-end mb-1">
          <PanelCollapseButton content={collapseContent} />
        </div>
        {view.panel(data)}
      </div>
    );
  }
  if (forceMode === "bubble") {
    return (
      <div className="dual-bubble border rounded p-2 text-[12px]">
        <div className="flex items-center justify-between mb-1">
          <span className="font-semibold">{view.title}</span>
        </div>
        {view.compact(data)}
      </div>
    );
  }

  // 自适应模式
  if (mode === "panel") {
    const collapseContent = typeof data === "string" ? data : JSON.stringify(data ?? {});
    return (
      <div className="dual-panel">
        <div className="flex justify-end mb-1 gap-1">
          <button
            type="button"
            className="sa-header-back"
            onClick={() => setMode("bubble")}
            title={t("dualView.collapseToBubble")}
          >
            <Minimize2 size={12} /> {t("dualView.collapseToBubble")}
          </button>
          <PanelCollapseButton content={collapseContent} />
        </div>
        {view.panel(data)}
      </div>
    );
  }

  return (
    <div className="dual-bubble border rounded p-2 text-[12px]">
      <div className="flex items-center justify-between mb-1">
        <span className="font-semibold">{view.title}</span>
        <button
          type="button"
          className="sa-header-back"
          onClick={() => setExpanded(!expanded)}
          title={expanded ? t("dualView.collapse") : t("dualView.expand")}
        >
          {expanded ? <Minimize2 size={12} /> : <Maximize2 size={12} />}
        </button>
      </div>
      {view.compact(data)}
      {expanded && (
        <div className="mt-2 border-t pt-2">
          <button
            type="button"
            className="sa-header-back"
            onClick={() => setMode("panel")}
            title={t("dualView.expandToPanel")}
          >
            <Maximize2 size={12} /> {t("dualView.expandToPanel")}
          </button>
          <div className="mt-2">{view.panel(data)}</div>
        </div>
      )}
    </div>
  );
}
