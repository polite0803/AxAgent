import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";

/** 时长 0.4s 的面板高亮闪烁,evidence chip 点击跳转时使用 */
const HIGHLIGHT_DURATION_MS = 400;

interface UseRightPanelResult {
  /** 跳转到指定 tab + sheet panel,并触发 0.4s 闪烁高亮 */
  navigateTo: (tabKey: "market" | "analyze" | "execute", panelKey: string, anchor?: string) => void;
  /** 仅高亮指定 panel(不切换 tab) */
  highlightPanel: (panelKey: string) => void;
  /** 当前高亮中的 panel key,用于侧栏面板加 ring 样式 */
  highlightedPanel: string | null;
}

/**
 * useRightPanel — Decision Timeline 证据芯片的桥接 hook
 * 将 evidence 引用翻译为 sheet panel tab 切换 + 0.4s 闪烁高亮。
 * 跨 tab 跳转通过 query param 让 StockAnalysisPage 监听(useEffect)。
 */
export function useRightPanel(): UseRightPanelResult {
  const setHighlightedPanel = useStockAnalysisStore((s) => s.setHighlightedPanel);
  const highlightedPanel = useStockAnalysisStore((s) => s.highlightedPanel);

  const navigateTo: UseRightPanelResult["navigateTo"] = (tabKey, panelKey, _anchor) => {
    // tab 切换通过 query param 走,StockAnalysisPage useEffect 监听
    const url = new URL(window.location.href);
    url.searchParams.set("timelineJump", `${tabKey}:${panelKey}`);
    window.history.replaceState({}, "", url.toString());
    // 触发 React 监听：派发一个 storage 事件（用 query param 变化不够直接）
    window.dispatchEvent(new Event("timeline-jump"));
    setHighlightedPanel(panelKey);
    setTimeout(() => {
      // 仅当当前高亮仍是该 panel 才清除
      const current = useStockAnalysisStore.getState().highlightedPanel;
      if (current === panelKey) { setHighlightedPanel(null); }
    }, HIGHLIGHT_DURATION_MS);
  };

  const highlightPanel: UseRightPanelResult["highlightPanel"] = (panelKey) => {
    setHighlightedPanel(panelKey);
    setTimeout(() => {
      const current = useStockAnalysisStore.getState().highlightedPanel;
      if (current === panelKey) { setHighlightedPanel(null); }
    }, HIGHLIGHT_DURATION_MS);
  };

  return { navigateTo, highlightPanel, highlightedPanel };
}
