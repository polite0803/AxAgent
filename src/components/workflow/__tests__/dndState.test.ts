import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  __resetDragStateForTest,
  clearDragPayload,
  clearDragPayloadForWindow,
  type DragPayload,
  getDragPayload,
  getDragPayloadForWindow,
  setDragPayload,
  setDragPayloadForWindow,
} from "../dndState";

describe("dndState", () => {
  beforeEach(() => {
    __resetDragStateForTest();
  });
  afterEach(() => {
    __resetDragStateForTest();
  });

  it("getDragPayload returns null initially", () => {
    expect(getDragPayload()).toBeNull();
  });

  it("set/get roundtrip for the same window", () => {
    const p: DragPayload = { type: "agent", label: "Agent" };
    setDragPayload(p);
    expect(getDragPayload()).toEqual(p);
  });

  it("clearDragPayload resets to null", () => {
    setDragPayload({ type: "llm", label: "LLM" });
    clearDragPayload();
    expect(getDragPayload()).toBeNull();
  });

  it("两个 setDragPayload 之间共享同一 window 的 state（latest 覆盖）", () => {
    setDragPayload({ type: "agent", label: "A" });
    setDragPayload({ type: "llm", label: "B" });
    expect(getDragPayload()).toEqual({ type: "llm", label: "B" });
  });

  it("不同 windowId 调用 setDragPayload 互不干扰（隔离）", () => {
    setDragPayloadForWindow("w1", { type: "agent", label: "A" });
    setDragPayloadForWindow("w2", { type: "llm", label: "B" });
    expect(getDragPayloadForWindow("w1")).toEqual({ type: "agent", label: "A" });
    expect(getDragPayloadForWindow("w2")).toEqual({ type: "llm", label: "B" });
  });

  it("对未初始化的 windowId 调用 get 返回 null", () => {
    expect(getDragPayloadForWindow("never-initialized")).toBeNull();
  });

  it("对指定 windowId 调用 clear 不影响其他 windowId", () => {
    setDragPayloadForWindow("w1", { type: "agent", label: "A" });
    setDragPayloadForWindow("w2", { type: "llm", label: "B" });
    clearDragPayloadForWindow("w1");
    expect(getDragPayloadForWindow("w1")).toBeNull();
    expect(getDragPayloadForWindow("w2")).toEqual({ type: "llm", label: "B" });
  });
});
