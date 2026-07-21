// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { BacktestPage } from "@/components/stock-analysis/BacktestPage";
import { BUILTIN_PAGE_PATH } from "@/lib/pageRegistry";
import { Clock } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

/**
 * 回测视图 — 工作区中栏的"回测"视图。
 *
 * 直接复用现有 BacktestPage 组件；顶部追加"时间旅行回放"入口按钮，
 * 承载原 /replay-workbench 路由的能力（不在 Tab 内嵌入以避免双重 PageHeader）。
 */
export function BacktestView() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <div className="relative h-full">
      <Tooltip title={t("replayWorkbench.title")} placement="left">
        <button
          type="button"
          onClick={() => navigate(BUILTIN_PAGE_PATH["replay-workbench"])}
          className="absolute top-2 right-3 z-10 flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors hover:opacity-80"
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            color: "var(--color-text-secondary)",
          }}
          aria-label={t("replayWorkbench.title")}
        >
          <Clock size={12} />
          {t("replayWorkbench.title")}
        </button>
      </Tooltip>
      <BacktestPage />
    </div>
  );
}
