// SPDX-License-Identifier-Identifier: AGPL-3.0-only

import { ChatPage } from "@/pages/ChatPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { KnowledgeHubPage } from "@/pages/KnowledgeHubPage";
import { TerminalPage } from "@/pages/TerminalPage";
import { WorkflowPage } from "@/pages/WorkflowPage";
import { useWorkspaceTabStore, type WorkspaceTab } from "@/stores";
import { useEffect } from "react";
import { useLocation } from "react-router-dom";

/**
 * 工作台 Hub：/chat 路由的统一渲染器。
 * 根据当前功能 Tab 渲染对应页面组件。
 *
 * 当从旧路由（/dashboard 等）通过 <Navigate state={{ tab }} /> 重定向过来时，
 * 读取 location.state.tab 切换到对应功能 Tab。
 */
export function WorkspaceHub() {
  const activeTab = useWorkspaceTabStore((s) => s.activeTab);
  const setActiveTab = useWorkspaceTabStore((s) => s.setActiveTab);
  const location = useLocation();

  useEffect(() => {
    const state = location.state as { tab?: WorkspaceTab } | null;
    if (state?.tab && state.tab !== activeTab) {
      setActiveTab(state.tab);
      window.history.replaceState({}, "");
    }
  }, [location.state, activeTab, setActiveTab]);

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
