// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
import { StockAnalysisPage as Page } from "@/components/stock-analysis/StockAnalysisPage";
import { useAgentContext } from "@/hooks/useAgentContext";

export function StockAnalysisPage() {
  // ── Agent 上下文注入：告知 Agent 当前页面是股票分析页 ──
  useAgentContext({
    page: "stock-analysis",
    url: "/stock-analysis",
    quickActions: [
      { id: "analyze-stock", description: "对指定股票执行完整分析", params: { stock_code: "string" } },
      { id: "get-quote", description: "查询股票实时行情", params: { stock_code: "string" } },
      { id: "search-stock", description: "搜索股票", params: { keyword: "string" } },
      { id: "list-analyses", description: "列出最近的分析记录" },
      { id: "search-news", description: "搜索财经新闻", params: { keyword: "string" } },
    ],
  });

  return <Page />;
}
