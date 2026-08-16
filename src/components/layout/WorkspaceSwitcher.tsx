// SPDX-License-Identifier: AGPL-3.0-only

import { useSettingsStore, useWorkspaceStore, type WorkspaceTab } from "@/stores";
import { theme } from "antd";
import { Database, Folder, FolderTree, Grid, MessageSquare, SquareTerminal, Users, Wrench } from "lucide-react";
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
  { key: "multiAgent", labelKey: "nav.multiAgent", icon: Users },
  { key: "devtools", labelKey: "nav.devTools", icon: Wrench },
];

/**
 * 工作台功能切换栏。
 * 紧凑的水平按钮组，位于内容区顶部，在 /chat 路由下显示。
 * 对话页作为核心枢纽，其他功能（仪表盘/工作流/终端/文件/知识源/开发工具）以 Tab 形式切换。
 * 开发工具 Tab 由设置 show_developer_tools 门控（默认开启）。
 */
export function WorkspaceSwitcher() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const activeTab = useWorkspaceStore((s) => s.activeTab);
  const setActiveTab = useWorkspaceStore((s) => s.setActiveTab);
  const showDevTools = useSettingsStore((s) => s.settings.showDeveloperTools !== false);
  const visibleTabs = showDevTools ? TABS : TABS.filter((x) => x.key !== "devtools");

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
        overflowX: "auto",
        whiteSpace: "nowrap",
      }}
    >
      {visibleTabs.map(({ key, labelKey, icon: Icon }) => {
        const isActive = activeTab === key;
        return (
          <button
            key={key}
            type="button"
            title={t(labelKey)}
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
              flexShrink: 0,
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
            <span className="ws-label">{t(labelKey)}</span>
          </button>
        );
      })}
    </div>
  );
}
