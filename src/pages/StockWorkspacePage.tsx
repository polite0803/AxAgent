// SPDX-License-Identifier: AGPL-3.0-only

import { StockWorkspaceShell } from "@/components/stock-workspace/StockWorkspaceShell";

/** 股票工作区页面入口 — 薄包装，实际逻辑在 StockWorkspaceShell */
export function StockWorkspacePage() {
  return <StockWorkspaceShell />;
}
