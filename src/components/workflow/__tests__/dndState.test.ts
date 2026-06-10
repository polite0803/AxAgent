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

  it("two setDragPayload calls on the same window share state (latest wins)", () => {
    setDragPayload({ type: "agent", label: "A" });
    setDragPayload({ type: "llm", label: "B" });
    expect(getDragPayload()).toEqual({ type: "llm", label: "B" });
  });

  it("setDragPayload on different windowIds are isolated", () => {
    setDragPayloadForWindow("w1", { type: "agent", label: "A" });
    setDragPayloadForWindow("w2", { type: "llm", label: "B" });
    expect(getDragPayloadForWindow("w1")).toEqual({ type: "agent", label: "A" });
    expect(getDragPayloadForWindow("w2")).toEqual({ type: "llm", label: "B" });
  });

  it("get on an uninitialized windowId returns null", () => {
    expect(getDragPayloadForWindow("never-initialized")).toBeNull();
  });

  it("clear on a windowId does not affect other windowIds", () => {
    setDragPayloadForWindow("w1", { type: "agent", label: "A" });
    setDragPayloadForWindow("w2", { type: "llm", label: "B" });
    clearDragPayloadForWindow("w1");
    expect(getDragPayloadForWindow("w1")).toBeNull();
    expect(getDragPayloadForWindow("w2")).toEqual({ type: "llm", label: "B" });
  });
});
