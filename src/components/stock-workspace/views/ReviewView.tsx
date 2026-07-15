// SPDX-License-Identifier: AGPL-3.0-only

import { EvolutionDriftPanel } from "@/components/stock-analysis/EvolutionDriftPanel";
import { ReflectionPanel } from "@/components/stock-analysis/ReflectionPanel";
import { useWorkspaceStore } from "@/stores";
import { RotateCcw, Sparkles } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * 复盘视图 — 工作区中栏的"复盘"视图。
 *
 * 组合反思反馈 + 进化漂移，通过子 Tab 切换。
 * 阶段 5 会增加决策生命周期条（分析→决策→持仓→反思→进化）。
 */
export function ReviewView() {
  const { t } = useTranslation();
  const currentStockCode = useWorkspaceStore((s) => s.currentStockCode);
  const [subTab, setSubTab] = useState<"reflection" | "evolution">("reflection");

  return (
    <div className="flex flex-col h-full">
      {/* 子 Tab 切换器 */}
      <div
        className="flex items-center gap-1 px-3 py-1.5"
        style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}
      >
        <button
          type="button"
          onClick={() => setSubTab("reflection")}
          className="flex items-center gap-1 px-3 py-1 rounded text-sm transition-colors"
          style={{
            background: subTab === "reflection" ? "var(--accent)" : "transparent",
            color: subTab === "reflection" ? "white" : "var(--muted)",
          }}
        >
          <RotateCcw size={14} />
          {t("workspace.view.reflection")}
        </button>
        <button
          type="button"
          onClick={() => setSubTab("evolution")}
          className="flex items-center gap-1 px-3 py-1 rounded text-sm transition-colors"
          style={{
            background: subTab === "evolution" ? "var(--accent)" : "transparent",
            color: subTab === "evolution" ? "white" : "var(--muted)",
          }}
        >
          <Sparkles size={14} />
          {t("workspace.view.evolution")}
        </button>
        {currentStockCode && (
          <span className="ml-auto text-sm" style={{ color: "var(--muted)" }}>
            {t("workspace.view.currentStock")}: {currentStockCode}
          </span>
        )}
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto">
        {subTab === "reflection" ? <ReflectionPanel /> : <EvolutionDriftPanel />}
      </div>
    </div>
  );
}
