// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { TFunction } from "i18next";

import type { DashboardData } from "../dynamicUI/dashboardSchema";
import { createDashboardSchema } from "../dynamicUI/dashboardSchema";

const t = ((key: string, _options?: Record<string, unknown>) => key) as unknown as TFunction<"translation">;

function makeData(overrides: Partial<DashboardData> = {}): DashboardData {
  return {
    conversationCount: 10,
    totalMessages: 500,
    totalTokens: 100000,
    gatewayMetrics: {
      totalRequests: 2000,
      totalTokens: 80000,
      todayRequests: 50,
      todayTokens: 2000,
      activeConnections: 3,
    },
    messageTokenSummary: {
      total_tokens: 100000,
      today_tokens: 5000,
    },
    agentStats: {
      totalSessions: 20,
      completedSessions: 15,
      failedSessions: 5,
      totalToolCalls: 100,
    },
    providerCount: 3,
    modelCount: 8,
    ...overrides,
  };
}

describe("dashboardSchema", () => {
  describe("createDashboardSchema", () => {
    it("应返回合法 schema 结构", () => {
      const schema = createDashboardSchema(makeData(), false, t);
      expect(schema.version).toBe("1.0");
      expect(schema.id).toBe("dashboard-page-root");
      expect(schema.type).toBe("Container");
      expect(Array.isArray(schema.children)).toBe(true);
    });

    it("应包含标题块", () => {
      const schema = createDashboardSchema(makeData(), false, t);
      const titleBlock = schema.children!.find(
        (c) => c.id === "dashboard-title-block",
      );
      expect(titleBlock).toBeDefined();
      expect(titleBlock!.type).toBe("Container");
    });

    it("应包含 overview cards", () => {
      const schema = createDashboardSchema(makeData(), false, t);
      const overview = schema.children!.find(
        (c) => c.id === "dashboard-overview-cards",
      );
      expect(overview).toBeDefined();
      expect(overview!.type).toBe("Dashboard");
      const items = overview!.props.items as Record<string, unknown>[];
      expect(items.length).toBe(4);
    });

    it("isLoading=true 时应标记 loading", () => {
      const schema = createDashboardSchema(makeData(), true, t);
      const overview = schema.children!.find(
        (c) => c.id === "dashboard-overview-cards",
      );
      const items = overview!.props.items as Record<string, unknown>[];
      expect(items.every((item) => item.loading === true)).toBe(true);
    });

    it("isLoading=false 时不应标记 loading", () => {
      const schema = createDashboardSchema(makeData(), false, t);
      const overview = schema.children!.find(
        (c) => c.id === "dashboard-overview-cards",
      );
      const items = overview!.props.items as Record<string, unknown>[];
      expect(items.every((item) => !item.loading)).toBe(true);
    });

    it("应包含 Token 和 Gateway 行", () => {
      const schema = createDashboardSchema(makeData(), false, t);
      const row = schema.children!.find(
        (c) => c.id === "dashboard-token-gateway-row",
      );
      expect(row).toBeDefined();
      expect(row!.type).toBe("Row");
      expect(row!.children!.length).toBe(2);
    });

    it("gatewayMetrics 为 null 时应显示占位文本", () => {
      const schema = createDashboardSchema(
        makeData({ gatewayMetrics: null }),
        false,
        t,
      );
      const gatewayCard = findChildById(
        schema,
        "dashboard-gateway-card",
      );
      const emptyText = gatewayCard?.children?.find(
        (c) => c.id === "dashboard-gateway-empty",
      );
      expect(emptyText).toBeDefined();
      expect(emptyText!.type).toBe("Text");
    });

    it("应包含 Agent 和 Model 行", () => {
      const schema = createDashboardSchema(makeData(), false, t);
      const row = schema.children!.find(
        (c) => c.id === "dashboard-agent-model-row",
      );
      expect(row).toBeDefined();
      expect(row!.type).toBe("Row");
      expect(row!.children!.length).toBe(2);
    });

    it("isLoading=true 时 agent stats 应显示 loading", () => {
      const schema = createDashboardSchema(makeData(), true, t);
      const agentCard = findChildById(schema, "dashboard-agent-card");
      const agentStats = agentCard?.children?.find(
        (c) => c.id === "dashboard-agent-stats",
      );
      const items = agentStats?.props.items as Record<string, unknown>[];
      expect(items.every((item) => item.loading === true)).toBe(true);
    });

    it("agentStats.failedSessions > 0 时应标记红色", () => {
      const schema = createDashboardSchema(makeData(), false, t);
      const agentCard = findChildById(schema, "dashboard-agent-card");
      const agentStats = agentCard?.children?.find(
        (c) => c.id === "dashboard-agent-stats",
      );
      const items = agentStats?.props.items as Record<string, unknown>[];
      const failedItem = items.find(
        (item) => item.label === "dashboard.agentFailed",
      );
      expect(failedItem).toBeDefined();
      expect(failedItem!.color).toBe("#ff4d4f");
    });

    it("agentStats.failedSessions = 0 时不应标记颜色", () => {
      const schema = createDashboardSchema(
        makeData({ agentStats: { totalSessions: 10, completedSessions: 10, failedSessions: 0, totalToolCalls: 50 } }),
        false,
        t,
      );
      const agentCard = findChildById(schema, "dashboard-agent-card");
      const agentStats = agentCard?.children?.find(
        (c) => c.id === "dashboard-agent-stats",
      );
      const items = agentStats?.props.items as Record<string, unknown>[];
      const failedItem = items.find(
        (item) => item.label === "dashboard.agentFailed",
      );
      expect(failedItem!.color).toBeUndefined();
    });
  });
});

function findChildById(schema: import("@/types").UISchema, id: string): import("@/types").UISchema | undefined {
  if (schema.id === id) { return schema; }
  for (const child of schema.children ?? []) {
    const found = findChildById(child, id);
    if (found) { return found; }
  }
  return undefined;
}
