// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
import { TradePage as Page } from "@/components/stock-analysis/TradePage";
import { useAgentContext } from "@/hooks/useAgentContext";

export function TradePage() {
  // ── Agent 上下文注入：告知 Agent 当前页面是交易记录页 ──
  useAgentContext({
    page: "trade",
    url: "/trade",
    quickActions: [
      { id: "record-trade", description: "记录一笔交易（代码/方向/价格/数量）", requireConfirmation: true },
      { id: "list-holdings", description: "列出当前持仓（含盈亏）" },
      { id: "toggle-trading", description: "启用或停用交易功能开关", requireConfirmation: true },
      { id: "get-quote", description: "查询股票实时行情", params: { stock_code: "string" } },
    ],
  });

  return <Page />;
}
