// SPDX-License-Identifier: AGPL-3.0-only

import { act, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, useNavigate } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

// ── 页面组件 mock：仅验证 WorkspaceHub 的 Tab 渲染分发 ──
vi.mock("@/pages/ChatPage", () => ({ ChatPage: () => <div data-testid="page-chat" /> }));
vi.mock("@/pages/DashboardPage", () => ({ DashboardPage: () => <div data-testid="page-dashboard" /> }));
vi.mock("@/pages/DevTools/DevToolsPage", () => ({ DevToolsPage: () => <div data-testid="page-devtools" /> }));
vi.mock("@/pages/FilesPage", () => ({ FilesPage: () => <div data-testid="page-files" /> }));
vi.mock("@/pages/KnowledgeHubPage", () => ({ KnowledgeHubPage: () => <div data-testid="page-knowledge" /> }));
vi.mock("@/pages/MultiAgentPage", () => ({ MultiAgentPage: () => <div data-testid="page-multiagent" /> }));
vi.mock("@/pages/TerminalPage", () => ({ TerminalPage: () => <div data-testid="page-terminal" /> }));
vi.mock("@/pages/WorkflowPage", () => ({ WorkflowPage: () => <div data-testid="page-workflow" /> }));

import { WorkspaceHub } from "@/components/layout/WorkspaceHub";
import { useWorkspaceTabStore } from "@/stores";

/** 模拟旧路由重定向：navigate("/chat", { state: { tab } }) 产生新的 state 对象引用 */
function RedirectProbe({ tab }: { tab: string }) {
  const navigate = useNavigate();
  return (
    <button onClick={() => navigate("/chat", { state: { tab } })}>
      redirect-{tab}
    </button>
  );
}

function renderWithState(state?: { tab: string }) {
  return render(
    <MemoryRouter initialEntries={[{ pathname: "/chat", state }]}>
      <WorkspaceHub />
    </MemoryRouter>,
  );
}

describe("WorkspaceHub 重定向 Tab 消费", () => {
  beforeEach(() => {
    // 重置全局 store，避免用例间泄漏
    useWorkspaceTabStore.setState({ activeTab: "chat" });
  });

  it("从旧路由重定向进入时消费 state.tab 并切到对应 Tab", async () => {
    renderWithState({ tab: "workflow" });
    expect(await screen.findByTestId("page-workflow")).toBeInTheDocument();
  });

  it("消费重定向后手动切换 Tab 不再回弹", async () => {
    renderWithState({ tab: "workflow" });
    expect(await screen.findByTestId("page-workflow")).toBeInTheDocument();

    // 手动切换 Tab（顶部 WorkspaceSwitcher 走 setActiveTab，不触发路由变化）
    act(() => {
      useWorkspaceTabStore.getState().setActiveTab("chat");
    });

    // 修复前：effect 依赖 activeTab 重跑，读到残留 state.tab="workflow" 回弹 → 仍显示 workflow
    // 修复后：保持手动切换的结果
    expect(screen.getByTestId("page-chat")).toBeInTheDocument();
    expect(screen.queryByTestId("page-workflow")).not.toBeInTheDocument();
  });

  it("同值二次导航（state 为新对象）仍会消费，不会因 ref 值相同漏消费", async () => {
    render(
      <MemoryRouter initialEntries={[{ pathname: "/chat", state: { tab: "workflow" } }]}>
        <RedirectProbe tab="workflow" />
        <WorkspaceHub />
      </MemoryRouter>,
    );
    expect(await screen.findByTestId("page-workflow")).toBeInTheDocument();

    // 手动切走
    act(() => {
      useWorkspaceTabStore.getState().setActiveTab("chat");
    });
    expect(screen.getByTestId("page-chat")).toBeInTheDocument();

    // 再次通过旧路由重定向进入（tab 值相同但 state 对象引用全新）
    fireEvent.click(screen.getByRole("button"));
    expect(await screen.findByTestId("page-workflow")).toBeInTheDocument();
  });

  it("直接访问 /chat（无 state）保持默认 Tab", async () => {
    renderWithState();
    expect(await screen.findByTestId("page-chat")).toBeInTheDocument();
  });
});
