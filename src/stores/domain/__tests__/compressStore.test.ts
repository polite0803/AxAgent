// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  isTauri: () => false,
  logIpcError: vi.fn(() => vi.fn()),
}));

vi.mock("@/stores/domain/conversationStore", () => ({
  useConversationStore: {
    getState: () => ({
      activeConversationId: "conv-1",
    }),
    setState: vi.fn(),
  },
}));

import { useCompressStore } from "@/stores/domain/compressStore";

const CONV_ID = "conv-1";

function makeSummary(overrides?: Record<string, unknown>) {
  return {
    id: "summary-1",
    conversationId: CONV_ID,
    summary: "This is a compressed summary of the conversation",
    original_message_count: 50,
    compressed_message_count: 5,
    created_at: "2025-01-01T00:00:00Z",
    ...overrides,
  };
}
function makeMessagePage(overrides?: Record<string, unknown>) {
  return {
    messages: [],
    has_older: false,
    total_active_count: 0,
    oldestLoadedMessageId: null,
    ...overrides,
  };
}
describe("compressStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useCompressStore.setState({ compressing: false });
  });

  describe("compressContext", () => {
    it("compresses context and reloads", async () => {
      invokeMock.mockResolvedValueOnce(makeSummary()); // compress_context
      invokeMock.mockResolvedValueOnce(makeMessagePage()); // list_messages_page

      await useCompressStore.getState().compressContext();

      expect(invokeMock).toHaveBeenCalledWith("compress_context", { conversationId: CONV_ID });
      expect(invokeMock).toHaveBeenCalledWith("list_messages_page", {
        conversationId: CONV_ID,
        limit: 100,
        beforeMessageId: null,
      });
      expect(useCompressStore.getState().compressing).toBe(false);
    });

    it("sets compressing to true while in progress", () => {
      invokeMock.mockResolvedValueOnce(makeSummary());
      invokeMock.mockResolvedValueOnce(makeMessagePage());

      const promise = useCompressStore.getState().compressContext();
      expect(useCompressStore.getState().compressing).toBe(true);
      return promise;
    });
  });

  describe("getCompressionSummary", () => {
    it("returns compression summary", async () => {
      const summary = makeSummary();
      invokeMock.mockResolvedValueOnce(summary);

      const result = await useCompressStore.getState().getCompressionSummary(CONV_ID);

      expect(invokeMock).toHaveBeenCalledWith("get_compression_summary", { conversationId: CONV_ID });
      expect(result).toEqual(summary);
    });

    it("returns null on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Not found"));

      const result = await useCompressStore.getState().getCompressionSummary(CONV_ID);

      expect(result).toBeNull();
    });
  });

  describe("deleteCompression", () => {
    it("deletes compression and reloads", async () => {
      invokeMock.mockResolvedValueOnce(undefined); // delete_compression
      invokeMock.mockResolvedValueOnce(makeMessagePage()); // list_messages_page

      await useCompressStore.getState().deleteCompression();

      expect(invokeMock).toHaveBeenCalledWith("delete_compression", { conversationId: CONV_ID });
      expect(invokeMock).toHaveBeenCalledWith("list_messages_page", {
        conversationId: CONV_ID,
        limit: 100,
        beforeMessageId: null,
      });
    });
  });
});
