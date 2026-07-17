// SPDX-License-Identifier: AGPL-3.0-only

import { afterEach, describe, expect, it, vi } from "vitest";

import type { MessageInstance } from "antd/es/message/interface";
import { message, setMessageInstance } from "../toast";

function createMockInstance(): MessageInstance {
  return {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    open: vi.fn(),
    destroy: vi.fn(),
  } as unknown as MessageInstance;
}

describe("toast", () => {
  afterEach(() => {
    (setMessageInstance as (instance: MessageInstance | null) => void)(null);
  });

  it("未初始化时调用应抛出错误", () => {
    expect(() => message.success("test")).toThrow(
      "message instance not initialized",
    );
  });

  it("初始化后应能调用 success", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);

    message.success("操作成功");
    expect(mock.success).toHaveBeenCalledWith("操作成功", undefined, undefined);
  });

  it("初始化后应能调用 error", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);

    message.error("操作失败");
    expect(mock.error).toHaveBeenCalledWith("操作失败", undefined, undefined);
  });

  it("初始化后应能调用 info", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);

    message.info("提示信息");
    expect(mock.info).toHaveBeenCalledWith("提示信息", undefined, undefined);
  });

  it("初始化后应能调用 warning", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);

    message.warning("警告信息");
    expect(mock.warning).toHaveBeenCalledWith("警告信息", undefined, undefined);
  });

  it("初始化后应能调用 loading", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);

    message.loading("加载中");
    expect(mock.loading).toHaveBeenCalledWith("加载中", undefined, undefined);
  });

  it("应传递 duration 和 onClose 参数", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);
    const onClose = vi.fn();

    message.success("done", 3, onClose);
    expect(mock.success).toHaveBeenCalledWith("done", 3, onClose);
  });

  it("初始化后应能调用 open", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);

    message.open({ content: "custom", type: "success" });
    expect(mock.open).toHaveBeenCalledWith({ content: "custom", type: "success" });
  });

  it("初始化后应能调用 destroy", () => {
    const mock = createMockInstance();
    (setMessageInstance as (instance: MessageInstance) => void)(mock);

    message.destroy("my-key");
    expect(mock.destroy).toHaveBeenCalledWith("my-key");
  });
});
