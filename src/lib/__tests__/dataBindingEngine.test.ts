// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

import { resolveDataSource, subscribeDataSource } from "../dynamicUI/DataBindingEngine";

const registryGet = vi.fn();

vi.mock("@/lib/storeRegistry", () => ({
  getStoreRegistry: () => ({
    get: registryGet,
  }),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn(),
}));

vi.mock("@/stores/feature/executionStore", () => ({
  useExecutionStore: {
    getState: vi.fn(),
  },
}));

describe("DataBindingEngine", () => {
  describe("resolveDataSource", () => {
    describe("static 类型", () => {
      it("应返回静态值", async () => {
        const result = await resolveDataSource({
          type: "static",
          config: { value: "hello" },
        });
        expect(result).toBe("hello");
      });

      it("应返回数字静态值", async () => {
        const result = await resolveDataSource({
          type: "static",
          config: { value: 42 },
        });
        expect(result).toBe(42);
      });

      it("应返回 null 静态值", async () => {
        const result = await resolveDataSource({
          type: "static",
          config: { value: null },
        });
        expect(result).toBeNull();
      });
    });

    describe("store 类型", () => {
      it("应返回整个 store 状态", async () => {
        const state = { user: { name: "Alice" }, count: 5 };
        registryGet.mockReturnValue({
          get: () => state,
          subscribe: () => vi.fn(),
        });

        const result = await resolveDataSource({
          type: "store",
          config: { storeName: "userStore" },
        });
        expect(result).toEqual(state);
      });

      it("应使用 selector 获取嵌套值", async () => {
        const state = { profile: { settings: { theme: "dark" } } };
        registryGet.mockReturnValue({
          get: () => state,
          subscribe: () => vi.fn(),
        });

        const result = await resolveDataSource({
          type: "store",
          config: { storeName: "userStore", selector: "profile.settings.theme" },
        });
        expect(result).toBe("dark");
      });

      it("store 未注册时应抛出错误", async () => {
        registryGet.mockReturnValue(null);

        await expect(
          resolveDataSource({
            type: "store",
            config: { storeName: "missing" },
          }),
        ).rejects.toThrow();
      });
    });

    describe("api 类型", () => {
      it("invoke 方法应调用 invoke", async () => {
        const { invoke } = await import("@/lib/invoke");
        vi.mocked(invoke).mockResolvedValue({ ok: true });

        const result = await resolveDataSource({
          type: "api",
          config: { endpoint: "get_data", method: "invoke", params: { id: 1 } },
        });
        expect(invoke).toHaveBeenCalledWith("get_data", { id: 1 });
        expect(result).toEqual({ ok: true });
      });

      it("fetch 方法成功时应返回 JSON", async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
          ok: true,
          json: () => Promise.resolve({ data: "fetched" }),
        });

        const result = await resolveDataSource({
          type: "api",
          config: { endpoint: "https://api.example.com", method: "fetch" },
        });
        expect(result).toEqual({ data: "fetched" });
      });

      it("fetch 失败时应抛出错误", async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
          ok: false,
          statusText: "Not Found",
        });

        await expect(
          resolveDataSource({
            type: "api",
            config: { endpoint: "https://api.example.com", method: "fetch" },
          }),
        ).rejects.toThrow();
      });
    });

    describe("agent-generated 类型", () => {
      it("应返回 agent 生成数据", async () => {
        const { useExecutionStore } = await import("@/stores/feature/executionStore");
        const mockState = {
          gen_123: { data: "generated", score: 0.95 },
        };
        vi.mocked(useExecutionStore.getState).mockReturnValue(
          mockState as unknown as ReturnType<typeof useExecutionStore.getState>,
        );

        const result = await resolveDataSource({
          type: "agent-generated",
          config: { generationId: "gen_123" },
        });
        expect(result).toEqual({ data: "generated", score: 0.95 });
      });

      it("应使用 selector 获取嵌套值", async () => {
        const { useExecutionStore } = await import("@/stores/feature/executionStore");
        const mockState = {
          gen_456: { output: { result: "nested" } },
        };
        vi.mocked(useExecutionStore.getState).mockReturnValue(
          mockState as unknown as ReturnType<typeof useExecutionStore.getState>,
        );

        const result = await resolveDataSource({
          type: "agent-generated",
          config: { generationId: "gen_456", selector: "output.result" },
        });
        expect(result).toBe("nested");
      });

      it("generationId 不存在时应抛出错误", async () => {
        const { useExecutionStore } = await import("@/stores/feature/executionStore");
        vi.mocked(useExecutionStore.getState).mockReturnValue(
          {} as unknown as ReturnType<typeof useExecutionStore.getState>,
        );

        await expect(
          resolveDataSource({
            type: "agent-generated",
            config: { generationId: "nonexistent" },
          }),
        ).rejects.toThrow();
      });
    });
  });

  describe("subscribeDataSource", () => {
    it("应返回 unsubscribe 对象", async () => {
      const subscriber = await subscribeDataSource(
        { type: "static", config: { value: "test" } },
        vi.fn(),
      );
      expect(subscriber).toHaveProperty("unsubscribe");
      expect(typeof subscriber.unsubscribe).toBe("function");
    });

    it("初始加载应调用 onData", async () => {
      const onData = vi.fn();
      await subscribeDataSource(
        { type: "static", config: { value: "initial" } },
        onData,
      );
      expect(onData).toHaveBeenCalledWith("initial");
    });

    it("取消订阅后不应再更新", async () => {
      const onData = vi.fn();
      const subscriber = await subscribeDataSource(
        { type: "static", config: { value: "test" } },
        onData,
      );
      subscriber.unsubscribe();
      onData.mockClear();
      await subscribeDataSource(
        { type: "static", config: { value: "after-unsubscribe" } },
        onData,
      );
      expect(onData).toHaveBeenCalledWith("after-unsubscribe");
    });

    it("unsubscribe 可多次调用不被报错", async () => {
      const subscriber = await subscribeDataSource(
        { type: "static", config: { value: "test" } },
        vi.fn(),
      );
      expect(() => {
        subscriber.unsubscribe();
        subscriber.unsubscribe();
      }).not.toThrow();
    });
  });
});
