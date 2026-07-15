// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkspaceStore } from "@/stores";
import { Tooltip } from "antd";
import { useTranslation } from "react-i18next";

/**
 * 用户模式切换：简洁 ↔ 专业。
 * 持久化在 workspaceStore（localStorage）。
 */
export function UserModeToggle() {
  const { t } = useTranslation();
  const userMode = useWorkspaceStore((s) => s.userMode);
  const toggleUserMode = useWorkspaceStore((s) => s.toggleUserMode);

  return (
    <Tooltip title={t("workspace.mode.toggleHint")}>
      <div
        className="flex items-center rounded overflow-hidden"
        style={{ border: "1px solid var(--border)", background: "var(--surface)" }}
      >
        <button
          type="button"
          onClick={() => userMode !== "simple" && toggleUserMode()}
          className="px-2 py-0.5 text-sm transition-colors"
          style={{
            background: userMode === "simple" ? "var(--accent)" : "transparent",
            color: userMode === "simple" ? "white" : "var(--muted)",
          }}
        >
          {t("workspace.mode.simple")}
        </button>
        <button
          type="button"
          onClick={() => userMode !== "professional" && toggleUserMode()}
          className="px-2 py-0.5 text-sm transition-colors"
          style={{
            background: userMode === "professional" ? "var(--accent)" : "transparent",
            color: userMode === "professional" ? "white" : "var(--muted)",
          }}
        >
          {t("workspace.mode.professional")}
        </button>
      </div>
    </Tooltip>
  );
}
