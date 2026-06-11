// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

import { createLatestWinner } from "../useNodeAIAssist";

describe("createLatestWinner", () => {
  it("first begin is immediately latest", () => {
    const lw = createLatestWinner();
    const id = lw.begin();
    expect(lw.isLatest(id)).toBe(true);
    // 同一个 id 在没有新 begin 之前仍是 latest
    expect(lw.isLatest(id)).toBe(true);
  });

  it("only the last begin is latest", () => {
    const lw = createLatestWinner();
    const a = lw.begin();
    const b = lw.begin();
    const c = lw.begin();
    expect(lw.isLatest(a)).toBe(false);
    expect(lw.isLatest(b)).toBe(false);
    expect(lw.isLatest(c)).toBe(true);
  });

  it("concurrent calls: only the last is adopted", async () => {
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

  it("begin ids increase monotonically", () => {
    const lw = createLatestWinner();
    const a = lw.begin();
    const b = lw.begin();
    const c = lw.begin();
    expect(a).toBeLessThan(b);
    expect(b).toBeLessThan(c);
  });

  it("isLatest(0) returns false on a fresh helper", () => {
    const lw = createLatestWinner();
    // 从未 begin 过的 id 一定不是 latest
    expect(lw.isLatest(0)).toBe(false);
  });

  it("can be plugged into mocked async scenarios", async () => {
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
