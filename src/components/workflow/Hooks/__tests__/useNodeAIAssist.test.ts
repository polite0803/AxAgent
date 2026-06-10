import { describe, expect, it, vi } from "vitest";

import { createLatestWinner } from "../useNodeAIAssist";

describe("createLatestWinner", () => {
  it("第一个 begin 立刻不是 latest", () => {
    const lw = createLatestWinner();
    const id = lw.begin();
    expect(lw.isLatest(id)).toBe(true);
    // 同一个 id 在没有新 begin 之前仍是 latest
    expect(lw.isLatest(id)).toBe(true);
  });

  it("只有最后一个 begin 才是 latest", () => {
    const lw = createLatestWinner();
    const a = lw.begin();
    const b = lw.begin();
    const c = lw.begin();
    expect(lw.isLatest(a)).toBe(false);
    expect(lw.isLatest(b)).toBe(false);
    expect(lw.isLatest(c)).toBe(true);
  });

  it("多次并发调用：只有最后一个被采纳", async () => {
    const lw = createLatestWinner();
    const calls: string[] = [];

    // 模拟三个并发请求
    const makeReq = async (tag: string) => {
      const id = lw.begin();
      // 模拟 invoke 异步延迟
      await new Promise((r) => setTimeout(r, Math.random() * 10));
      if (lw.isLatest(id)) {
        calls.push(tag);
      }
    };

    await Promise.all([makeReq("a"), makeReq("b"), makeReq("c")]);
    // 只有一个最终能通过 isLatest 校验
    expect(calls.length).toBe(1);
  });

  it("begin 编号单调递增", () => {
    const lw = createLatestWinner();
    const a = lw.begin();
    const b = lw.begin();
    const c = lw.begin();
    expect(a).toBeLessThan(b);
    expect(b).toBeLessThan(c);
  });

  it("空 helper 的 isLatest(0) 返回 false", () => {
    const lw = createLatestWinner();
    // 从未 begin 过的 id 一定不是 latest
    expect(lw.isLatest(0)).toBe(false);
  });

  it("可以被注入到 mock 异步场景中", async () => {
    // 真实使用：latest-wins 让陈旧 invoke 静默丢弃
    const lw = createLatestWinner();
    const onResult = vi.fn();
    const fakeInvoke = async (id: number, value: string) => {
      await new Promise((r) => setTimeout(r, 1));
      if (lw.isLatest(id)) { onResult(value); }
    };
    const id1 = lw.begin();
    const id2 = lw.begin();
    await fakeInvoke(id1, "old");
    await fakeInvoke(id2, "new");
    expect(onResult).toHaveBeenCalledTimes(1);
    expect(onResult).toHaveBeenCalledWith("new");
  });
});
