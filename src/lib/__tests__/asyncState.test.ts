// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

import { doneAsync, failAsync, INIT_ASYNC, startAsync, withAsync } from "../asyncState";

describe("asyncState", () => {
  describe("INIT_ASYNC", () => {
    it("初始状态 loading 为 false", () => {
      expect(INIT_ASYNC.loading).toBe(false);
    });

    it("初始状态 error 为 null", () => {
      expect(INIT_ASYNC.error).toBeNull();
    });
  });

  describe("startAsync", () => {
    it("应设置 loading 为 true 并清空 error", () => {
      const set = vi.fn();
      startAsync(set);
      expect(set).toHaveBeenCalledWith({ loading: true, error: null });
    });
  });

  describe("failAsync", () => {
    it("应设置 loading 为 false 并将 error 转为字符串", () => {
      const set = vi.fn();
      failAsync(set, new Error("网络错误"));
      expect(set).toHaveBeenCalledWith({ loading: false, error: "Error: 网络错误" });
    });

    it("应处理非 Error 类型的错误", () => {
      const set = vi.fn();
      failAsync(set, "超时");
      expect(set).toHaveBeenCalledWith({ loading: false, error: "超时" });
    });
  });

  describe("doneAsync", () => {
    it("应设置 loading 为 false", () => {
      const set = vi.fn();
      doneAsync(set);
      expect(set).toHaveBeenCalledWith({ loading: false });
    });
  });

  describe("withAsync", () => {
    it("成功执行时应依次调用 startAsync 和 doneAsync", async () => {
      const set = vi.fn();
      const fn = vi.fn().mockResolvedValue(undefined);

      await withAsync(set, fn);

      expect(set).toHaveBeenNthCalledWith(1, { loading: true, error: null });
      expect(set).toHaveBeenNthCalledWith(2, { loading: false });
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("失败执行时应依次调用 startAsync 和 failAsync", async () => {
      const set = vi.fn();
      const error = new Error("操作失败");
      const fn = vi.fn().mockRejectedValue(error);

      await withAsync(set, fn);

      expect(set).toHaveBeenNthCalledWith(1, { loading: true, error: null });
      expect(set).toHaveBeenNthCalledWith(2, { loading: false, error: "Error: 操作失败" });
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("set 应被调用两次（start + done/fail）", async () => {
      const set = vi.fn();
      await withAsync(set, vi.fn().mockResolvedValue(undefined));
      expect(set).toHaveBeenCalledTimes(2);
    });
  });
});
