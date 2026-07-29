// SPDX-License-Identifier: AGPL-3.0-only

import { ChatPage } from "@/pages/ChatPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { KnowledgeHubPage } from "@/pages/KnowledgeHubPage";
import { TerminalPage } from "@/pages/TerminalPage";
import { WorkflowPage } from "@/pages/WorkflowPage";
import { useWorkspaceTabStore } from "@/stores";

/**
 * 工作台 Hub：/chat 路由的统一渲染器。
 * 根据当前功能 Tab 渲染对应页面组件。
 * WorkspaceSwitcher（功能切换栏）由 App.tsx 在 content-col 顶部渲染。
 */
export function WorkspaceHub() {
  const activeTab = useWorkspaceTabStore((s) => s.activeTab);

  switch (activeTab) {
    case "chat":
      return <ChatPage />;
    case "dashboard":
      return <DashboardPage />;
    case "workflow":
      return <WorkflowPage />;
    case "terminal":
      return <TerminalPage />;
    case "knowledge":
      return <KnowledgeHubPage />;
    default:
      return <ChatPage />;
  }
}
