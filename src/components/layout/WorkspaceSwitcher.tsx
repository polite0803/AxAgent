// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkspaceStore, type WorkspaceTab } from "@/stores";
import { theme } from "antd";
import { Database, Folder, FolderTree, Grid, MessageSquare, SquareTerminal } from "lucide-react";
import { useTranslation } from "react-i18next";

interface TabConfig {
  key: WorkspaceTab;
  labelKey: string;
  icon: typeof MessageSquare;
}

const TABS: TabConfig[] = [
  { key: "chat", labelKey: "nav.chat", icon: MessageSquare },
  { key: "dashboard", labelKey: "nav.dashboard", icon: Grid },
  { key: "workflow", labelKey: "nav.workflow", icon: FolderTree },
  { key: "terminal", labelKey: "nav.terminal", icon: SquareTerminal },
  { key: "files", labelKey: "nav.files", icon: Folder },
  { key: "knowledge", labelKey: "nav.knowledge", icon: Database },
];

/**
 * 工作台功能切换栏。
 * 紧凑的水平按钮组，位于内容区顶部，在 /chat 路由下显示。
 * 对话页作为核心枢纽，其他功能（仪表盘/工作流/终端/文件/知识源）以 Tab 形式切换。
 */
export function WorkspaceSwitcher() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const activeTab = useWorkspaceStore((s) => s.activeTab);
  const setActiveTab = useWorkspaceStore((s) => s.setActiveTab);

  return (
    <div
      className="ax-workspace-switcher"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 2,
        padding: "4px 12px",
        backgroundColor: token.colorBgContainer,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        flexShrink: 0,
      }}
    >
      {TABS.map(({ key, labelKey, icon: Icon }) => {
        const isActive = activeTab === key;
        return (
          <button
            key={key}
            type="button"
            onClick={() => setActiveTab(key)}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              padding: "4px 10px",
              border: "none",
              borderRadius: 6,
              cursor: "pointer",
              fontSize: 13,
              fontWeight: isActive ? 500 : 400,
              color: isActive ? token.colorPrimary : token.colorTextSecondary,
              backgroundColor: isActive
                ? token.colorPrimaryBg
                : "transparent",
              transition: "all 0.15s",
            }}
            onMouseEnter={(e) => {
              if (!isActive) {
                (e.currentTarget as HTMLElement).style.backgroundColor = token.colorFillQuaternary;
              }
            }}
            onMouseLeave={(e) => {
              if (!isActive) {
                (e.currentTarget as HTMLElement).style.backgroundColor = "transparent";
              }
            }}
          >
            <Icon size={14} />
            {t(labelKey)}
          </button>
        );
      })}
    </div>
  );
}
