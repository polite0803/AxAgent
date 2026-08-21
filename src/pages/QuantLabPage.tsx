// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// QuantLab 顶层页面 — 与 pages/ 模式一致

import { QuantLab } from "@/components/quant/QuantLab";
import { useAgentContext } from "@/hooks/useAgentContext";

export function QuantLabPage() {
  // ── Agent 上下文注入：告知 Agent 当前页面是量化实验室 ──
  useAgentContext({
    page: "quant-lab",
    url: "/quant-lab",
    quickActions: [
      { id: "list-strategies", description: "列出已注册的量化策略" },
      { id: "run-backtest", description: "运行量化策略回测", requireConfirmation: true },
      { id: "get-quote", description: "查询股票实时行情", params: { stock_code: "string" } },
      { id: "get-industry-ranking", description: "获取行业板块排名" },
    ],
  });

  return <QuantLab />;
}
