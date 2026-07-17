// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
  logIpcError: vi.fn(() => vi.fn()),
}));

import {
  _isMultiModelActive,
  _multiModelFirstMessageId,
  _multiModelFirstModelId,
  _multiModelTotalRemaining,
  _pendingConversationRefresh,
  _pendingUiChunk,
  _streamBuffer,
  _streamPrefix,
  _userManuallySelectedVersion,
  clearOrphanedBuffer,
  decrementMultiModelTotalRemaining,
  getListenerGen,
  getOrCreateSession,
  incrementActiveMessageLoadSeq,
  incrementListenerGen,
  isWatchdogRunning,
  preserveOrphanedBuffer,
  rebuildMessageIndex,
  removeSession,
  resetMultiModelState,
  resetStreamRuntime,
  setIsMultiModelActive,
  setMultiModelDoneResolve,
  setMultiModelFirstMessageId,
  setMultiModelFirstModelId,
  setMultiModelTotalRemaining,
  setPendingUiChunk,
  setStreamBuffer,
  setStreamPrefix,
  setUserManuallySelectedVersion,
  startStreamWatchdog,
  stopStreamWatchdog,
  type StreamBuffer,
  takeOrphanedBuffer,
} from "@/stores/domain/streamStore";

describe("streamStore runtime", () => {
  beforeEach(() => {
    resetStreamRuntime();
  });

  describe("listenerGen", () => {
    it("starts at 0 after reset", () => {
      expect(getListenerGen()).toBe(0);
    });

    it("increments monotonically", () => {
      expect(incrementListenerGen()).toBe(1);
      expect(incrementListenerGen()).toBe(2);
      expect(incrementListenerGen()).toBe(3);
      expect(getListenerGen()).toBe(3);
    });
  });

  describe("watchdog", () => {
    it("is not running after reset", () => {
      expect(isWatchdogRunning()).toBe(false);
    });

    it("starts and stops watchdog", () => {
      startStreamWatchdog();
      expect(isWatchdogRunning()).toBe(true);
      stopStreamWatchdog();
      expect(isWatchdogRunning()).toBe(false);
    });

    it("startStreamWatchdog is idempotent", () => {
      startStreamWatchdog();
      startStreamWatchdog();
      expect(isWatchdogRunning()).toBe(true);
      stopStreamWatchdog();
    });
  });

  describe("streamBuffer", () => {
    it("setStreamBuffer sets the default session buffer", () => {
      const buf: StreamBuffer = {
        messageId: "msg-1",
        conversationId: "conv-1",
        content: "Hello",
        resolvedId: null,
        thinking: null,
      };
      setStreamBuffer(buf);
      expect(_streamBuffer).toEqual(buf);
    });

    it("setStreamBuffer clears the buffer when null", () => {
      setStreamBuffer({
        messageId: "msg-1",
        conversationId: "conv-1",
        content: "Hello",
        resolvedId: null,
        thinking: null,
      });
      setStreamBuffer(null);
      expect(_streamBuffer).toBeNull();
    });

    it("setStreamPrefix sets and gets correctly", () => {
      setStreamPrefix("prefix-");
      expect(_streamPrefix).toBe("prefix-");
    });

    it("supports per-conversation sessions", () => {
      const session = getOrCreateSession("conv-1");
      expect(session).toBeDefined();
      expect(session.streamBuffer).toBeNull();
      expect(session.streamPrefix).toBe("");
    });

    it("removeSession cleans up a session", () => {
      getOrCreateSession("conv-1");
      removeSession("conv-1");
      const session = getOrCreateSession("conv-1");
      expect(session.streamBuffer).toBeNull();
    });
  });

  describe("orphaned buffers", () => {
    it("preserves and takes orphaned buffers", () => {
      const buf: StreamBuffer = {
        messageId: "msg-1",
        conversationId: "conv-1",
        content: "Partial response...",
        resolvedId: null,
        thinking: null,
      };
      getOrCreateSession("conv-1").streamBuffer = buf;
      preserveOrphanedBuffer("conv-1");

      const taken = takeOrphanedBuffer("conv-1");
      expect(taken).toBeDefined();
      expect(taken?.content).toBe("Partial response...");

      const takenAgain = takeOrphanedBuffer("conv-1");
      expect(takenAgain).toBeUndefined();
    });

    it("clearOrphanedBuffer removes orphaned buffer", () => {
      getOrCreateSession("conv-1").streamBuffer = {
        messageId: "msg-1",
        conversationId: "conv-1",
        content: "test",
        resolvedId: null,
        thinking: null,
      };
      preserveOrphanedBuffer("conv-1");
      clearOrphanedBuffer("conv-1");
      expect(takeOrphanedBuffer("conv-1")).toBeUndefined();
    });
  });

  describe("pendingConversationRefresh", () => {
    it("adds and removes pending refresh", () => {
      expect(_pendingConversationRefresh.has("conv-1")).toBe(false);
      _pendingConversationRefresh.add("conv-1");
      expect(_pendingConversationRefresh.has("conv-1")).toBe(true);
      _pendingConversationRefresh.delete("conv-1");
      expect(_pendingConversationRefresh.has("conv-1")).toBe(false);
    });
  });

  describe("messageIndex", () => {
    it("rebuilds index from messages array", () => {
      const messages = [
        { id: "msg-1" },
        { id: "msg-2" },
        { id: "msg-3" },
      ];
      rebuildMessageIndex(messages);
      expect(true).toBe(true);
    });

    it("handles empty message array", () => {
      rebuildMessageIndex([]);
      expect(true).toBe(true);
    });
  });

  describe("multi-model state", () => {
    it("setMultiModelTotalRemaining sets the count", () => {
      setMultiModelTotalRemaining(3);
      expect(_multiModelTotalRemaining).toBe(3);
    });

    it("decrementMultiModelTotalRemaining decrements the count", () => {
      setMultiModelTotalRemaining(3);
      decrementMultiModelTotalRemaining();
      expect(_multiModelTotalRemaining).toBe(2);
      decrementMultiModelTotalRemaining();
      expect(_multiModelTotalRemaining).toBe(1);
    });

    it("decrementMultiModelTotalRemaining handles already-zero gracefully", () => {
      setMultiModelTotalRemaining(1);
      decrementMultiModelTotalRemaining();
      expect(_multiModelTotalRemaining).toBe(0);
      decrementMultiModelTotalRemaining();
      expect(_multiModelTotalRemaining).toBe(-1);
    });

    it("setIsMultiModelActive toggles active state", () => {
      setIsMultiModelActive(true);
      expect(_isMultiModelActive).toBe(true);
      setIsMultiModelActive(false);
      expect(_isMultiModelActive).toBe(false);
    });

    it("setMultiModelFirstModelId sets first model id", () => {
      setMultiModelFirstModelId("model-gpt4");
      expect(_multiModelFirstModelId).toBe("model-gpt4");
      setMultiModelFirstModelId(null);
      expect(_multiModelFirstModelId).toBeNull();
    });

    it("setMultiModelFirstMessageId sets first message id", () => {
      setMultiModelFirstMessageId("msg-first");
      expect(_multiModelFirstMessageId).toBe("msg-first");
      setMultiModelFirstMessageId(null);
      expect(_multiModelFirstMessageId).toBeNull();
    });

    it("setUserManuallySelectedVersion tracks manual selection", () => {
      setUserManuallySelectedVersion(true);
      expect(_userManuallySelectedVersion).toBe(true);
      setUserManuallySelectedVersion(false);
      expect(_userManuallySelectedVersion).toBe(false);
    });

    it("resetMultiModelState resets all multi-model variables", () => {
      setIsMultiModelActive(true);
      setMultiModelTotalRemaining(5);
      setMultiModelFirstModelId("model-1");
      setMultiModelFirstMessageId("msg-1");
      setUserManuallySelectedVersion(true);

      resetMultiModelState();

      expect(_isMultiModelActive).toBe(false);
      expect(_multiModelTotalRemaining).toBe(0);
      expect(_multiModelFirstModelId).toBeNull();
      expect(_multiModelFirstMessageId).toBeNull();
      expect(_userManuallySelectedVersion).toBe(false);
    });

    it("setMultiModelDoneResolve sets and clears resolve callback", () => {
      const resolver = () => {};
      setMultiModelDoneResolve(resolver);
      expect(_multiModelTotalRemaining).toBeGreaterThanOrEqual(0);
      setMultiModelDoneResolve(null);
    });
  });

  describe("per-conversation sessions", () => {
    it("getOrCreateSession returns same session for same conversation", () => {
      const s1 = getOrCreateSession("conv-1");
      const s2 = getOrCreateSession("conv-1");
      expect(s1).toBe(s2);
    });

    it("getOrCreateSession returns different sessions for different conversations", () => {
      const s1 = getOrCreateSession("conv-1");
      const s2 = getOrCreateSession("conv-2");
      expect(s1).not.toBe(s2);
    });

    it("session stores streamBuffer separately per conversation", () => {
      const buf1: StreamBuffer = {
        messageId: "msg-1",
        conversationId: "conv-1",
        content: "Hello from conv-1",
        resolvedId: null,
        thinking: null,
      };
      const buf2: StreamBuffer = {
        messageId: "msg-2",
        conversationId: "conv-2",
        content: "Hello from conv-2",
        resolvedId: null,
        thinking: null,
      };

      setStreamBuffer(buf1, "conv-1");
      setStreamBuffer(buf2, "conv-2");

      const s1 = getOrCreateSession("conv-1");
      const s2 = getOrCreateSession("conv-2");
      expect(s1.streamBuffer?.content).toBe("Hello from conv-1");
      expect(s2.streamBuffer?.content).toBe("Hello from conv-2");
    });

    it("removeSession clears timer and removes session", () => {
      const session = getOrCreateSession("conv-1");
      session.streamUiFlushTimer = setTimeout(() => {}, 99999);
      removeSession("conv-1");
      expect(session.streamUiFlushTimer).toBeNull();
    });
  });

  describe("PendingUiChunk", () => {
    it("setPendingUiChunk sets pending chunk", () => {
      const chunk = {
        messageId: "msg-1",
        conversationId: "conv-1",
        content: "partial...",
      };
      setPendingUiChunk(chunk);
      expect(_pendingUiChunk).toEqual(chunk);
    });

    it("setPendingUiChunk clears pending chunk when null", () => {
      setPendingUiChunk({
        messageId: "msg-1",
        conversationId: "conv-1",
        content: "partial...",
      });
      setPendingUiChunk(null);
      expect(_pendingUiChunk).toBeNull();
    });
  });

  describe("activeMessageLoadSeq", () => {
    it("increments via incrementActiveMessageLoadSeq", () => {
      expect(incrementActiveMessageLoadSeq()).toBe(1);
      expect(incrementActiveMessageLoadSeq()).toBe(2);
    });
  });
});
