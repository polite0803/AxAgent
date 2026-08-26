// SPDX-License-Identifier-Identifier: AGPL-3.0-only

import { ChatPage } from "@/pages/ChatPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { DevToolsPage } from "@/pages/DevTools/DevToolsPage";
import { FilesPage } from "@/pages/FilesPage";
import { KnowledgeHubPage } from "@/pages/KnowledgeHubPage";
import { MultiAgentPage } from "@/pages/MultiAgentPage";
import { TerminalPage } from "@/pages/TerminalPage";
import { WorkflowPage } from "@/pages/WorkflowPage";
import { useWorkspaceTabStore, type WorkspaceTab } from "@/stores";
import { useEffect, useRef } from "react";
import { useLocation } from "react-router-dom";

/**
 * 工作台 Hub：/chat 路由的统一渲染器。
 * 内容区根据当前功能 Tab 渲染对应页面组件。
 * 顶部的 WorkspaceSwitcher 在 App.tsx 中渲染（WorkspaceSwitcherBar）。
 *
 * 当从旧路由（/dashboard 等）通过 <Navigate state={{ tab }} /> 重定向过来时，
 * 读取 location.state.tab 切换到对应功能 Tab。
 */
export function WorkspaceHub() {
  const activeTab = useWorkspaceTabStore((s) => s.activeTab);
  const setActiveTab = useWorkspaceTabStore((s) => s.setActiveTab);
  const location = useLocation();

  // 记录已消费的重定向 state 对象引用，只消费一次。
  // 背景：redirectToChat 用 <Navigate replace state={{ tab }} /> 传目标 Tab，
  // 但 window.history.replaceState 不触发 popstate，router 内部 location.state 永不更新、
  // 残留 {tab}。若 effect 依赖 activeTab，手动切 Tab 后 effect 重跑会读到残留 state 导致回弹。
  // 用 ref 记录已消费的 state 对象引用（createLocation 对 state 透传引用、不拷贝）：
  // 残留的同一 state 引用不再消费；新导航必然产生新引用（即使 tab 值相同）也会正常消费。
  const handledStateRef = useRef<unknown>(null);

  useEffect(() => {
    const state = location.state as { tab?: WorkspaceTab } | null;
    console.warn("[WorkspaceHub] useEffect: location.state=", state, "activeTab=", activeTab);
    if (state?.tab && state !== handledStateRef.current) {
      handledStateRef.current = state;
      console.warn("[WorkspaceHub] Setting activeTab to:", state.tab);
      setActiveTab(state.tab);
    }
  }, [location.state, setActiveTab]);

  console.warn("[WorkspaceHub] Rendering with activeTab:", activeTab);
  switch (activeTab) {
    case "chat":
      return <ChatPage />;
    case "dashboard":
      return <DashboardPage />;
    case "workflow":
      return <WorkflowPage />;
    case "terminal":
      return <TerminalPage />;
    case "files":
      return <FilesPage />;
    case "knowledge":
      return <KnowledgeHubPage />;
    case "multiAgent":
      return <MultiAgentPage />;
    case "devtools":
      return <DevToolsPage />;
    default:
      return <ChatPage />;
  }
}
