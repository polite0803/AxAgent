// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import { Tooltip } from "antd";
import { Bot, Maximize2, Minimize2, PanelRightClose } from "lucide-react";
import { useTranslation } from "react-i18next";

export function AgentPanelHeader() {
  const { t } = useTranslation();
  const agentContext = useAgentPanelStore((s) => s.agentContext);
  const isMiniMode = useAgentPanelStore((s) => s.isMiniMode);
  const toggleMiniMode = useAgentPanelStore((s) => s.toggleMiniMode);
  const close = useAgentPanelStore((s) => s.close);

  const contextLabel = agentContext
    ? agentContext.page
    : null;

  return (
    <div className="flex items-center justify-between px-3 py-2 border-b border-[var(--border-color)] shrink-0">
      <div className="flex items-center gap-2 min-w-0">
        <Bot size={18} className="text-[var(--color-primary)] shrink-0" />
        <span className="font-semibold text-sm truncate">{t("app.name")}</span>
      </div>

      <div className="flex-1 mx-3 min-w-0">
        {contextLabel
          ? (
            <div className="text-xs px-2 py-0.5 rounded-full bg-[var(--color-primary)]/10 text-[var(--color-primary)] truncate text-center">
              {contextLabel}
              {agentContext?.selection && (
                <span className="ml-1 opacity-70">
                  · {agentContext.selection.label}
                </span>
              )}
            </div>
          )
          : (
            <div className="text-xs text-[var(--color-text-secondary)] text-center">
              {t("agentPanel.globalMode")}
            </div>
          )}
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <Tooltip title={isMiniMode ? t("agentPanel.expandFullPanel") : t("agentPanel.miniMode")}>
          <button
            type="button"
            className="p-1 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
            onClick={toggleMiniMode}
          >
            {isMiniMode ? <Maximize2 size={15} /> : <Minimize2 size={15} />}
          </button>
        </Tooltip>
        <Tooltip title={t("agentPanel.closePanel")}>
          <button
            type="button"
            className="p-1 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
            onClick={close}
          >
            <PanelRightClose size={15} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
