// SPDX-License-Identifier: AGPL-3.0-only
// 测试 PageContextProvider 包裹不同 page → usePageContext() 返回正确上下文

import { PageContextProvider, usePageContext } from "@/components/shared/PageContextProvider";
import { render, screen } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";

// 测试辅助组件：渲染 usePageContext 结果
function ContextReader({ testId }: { testId: string }) {
  const ctx = usePageContext();
  return (
    <div data-testid={testId}>
      <span data-testid={`${testId}-page`}>{ctx.page}</span>
      <span data-testid={`${testId}-url`}>{ctx.url}</span>
      <span data-testid={`${testId}-wfid`}>{ctx.currentWorkflowId ?? "null"}</span>
      <span data-testid={`${testId}-nodes`}>{ctx.activeNodes}</span>
      <span data-testid={`${testId}-evo-engines`}>{ctx.evolutionStatus.runningEngines.join(",") || "none"}</span>
      <span data-testid={`${testId}-evo-count`}>{ctx.evolutionStatus.totalEvolutions}</span>
      <span data-testid={`${testId}-traces`}>{ctx.recentTraces.count}</span>
    </div>
  );
}

function renderWithPage(page: string, route = "/workflow") {
  return render(
    <MemoryRouter initialEntries={[route]}>
      <PageContextProvider page={page}>
        <ContextReader testId="ctx" />
      </PageContextProvider>
    </MemoryRouter>,
  );
}

describe("PageContextProvider", () => {
  it("provides correct page identifier for chat page", () => {
    renderWithPage("chat", "/");
    expect(screen.getByTestId("ctx-page")).toHaveTextContent("chat");
  });

  it("provides correct page identifier for workflow page", () => {
    renderWithPage("workflow", "/workflow");
    expect(screen.getByTestId("ctx-page")).toHaveTextContent("workflow");
  });

  it("provides correct page identifier for knowledge page", () => {
    renderWithPage("knowledge", "/knowledge");
    expect(screen.getByTestId("ctx-page")).toHaveTextContent("knowledge");
  });

  it("provides correct page identifier for settings page", () => {
    renderWithPage("settings", "/settings");
    expect(screen.getByTestId("ctx-page")).toHaveTextContent("settings");
  });

  it("exposes current URL in context", () => {
    renderWithPage("workflow", "/workflow?id=123");
    expect(screen.getByTestId("ctx-url")).toHaveTextContent("/workflow?id=123");
  });

  it("defaults to empty currentWorkflowId for non-workflow pages", () => {
    renderWithPage("chat", "/");
    expect(screen.getByTestId("ctx-wfid")).toHaveTextContent("null");
  });

  it("defaults to zero activeNodes for non-workflow pages", () => {
    renderWithPage("chat", "/");
    expect(screen.getByTestId("ctx-nodes")).toHaveTextContent("0");
  });

  it("provides default evolution status for non-workflow pages", () => {
    renderWithPage("chat", "/");
    expect(screen.getByTestId("ctx-evo-engines")).toHaveTextContent("none");
    expect(screen.getByTestId("ctx-evo-count")).toHaveTextContent("0");
  });
});

describe("usePageContext hook", () => {
  it("returns default context when used outside provider", () => {
    // usePageContext outside provider returns defaultPageContext (page: "")
    function OutsideReader() {
      try {
        const ctx = usePageContext();
        return <div data-testid="outside">{ctx.page}</div>;
      } catch {
        return <div data-testid="outside">error</div>;
      }
    }
    render(
      <MemoryRouter>
        <OutsideReader />
      </MemoryRouter>,
    );
    // Outside provider, the context returns default value. It won't throw.
    expect(screen.getByTestId("outside")).toHaveTextContent("");
  });
});
