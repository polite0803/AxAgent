// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { Message } from "@/types";
import { mergeOlderPages, mergePreservedMessages, MESSAGE_PAGE_SIZE } from "../messageUtils";

function makeMessage(
  id: string,
  content: string,
  createdAt: number,
  status: string = "sent",
): Message {
  return {
    id,
    content,
    createdAt,
    status,
    role: "user",
    conversationId: "conv-1",
  } as Message;
}

describe("messageUtils", () => {
  describe("MESSAGE_PAGE_SIZE", () => {
    it("应为 50", () => {
      expect(MESSAGE_PAGE_SIZE).toBe(50);
    });
  });

  describe("mergePreservedMessages", () => {
    it("preserveMessageIds 为空时应返回 pageMessages 原样", () => {
      const pageMessages = [makeMessage("1", "hello", 100)];
      const result = mergePreservedMessages(pageMessages, [], []);

      expect(result).toEqual(pageMessages);
    });

    it("应保留本地消息的 content 和 status", () => {
      const pageMessages = [makeMessage("1", "old content", 100, "sent")];
      const preserveMessageIds = ["1"];
      const currentMessages = [makeMessage("1", "new content", 100, "streaming")];

      const result = mergePreservedMessages(pageMessages, preserveMessageIds, currentMessages);

      expect(result[0].content).toBe("new content");
      expect(result[0].status).toBe("streaming");
    });

    it("应添加本地存在但页面不存在的新消息", () => {
      const pageMessages = [makeMessage("1", "hello", 100)];
      const preserveMessageIds = ["2"];
      const currentMessages = [makeMessage("2", "new message", 200)];

      const result = mergePreservedMessages(pageMessages, preserveMessageIds, currentMessages);

      expect(result).toHaveLength(2);
      expect(result.map((m) => m.id)).toContain("1");
      expect(result.map((m) => m.id)).toContain("2");
    });

    it("应按 created_at 排序", () => {
      const pageMessages = [makeMessage("3", "third", 300)];
      const preserveMessageIds = ["1", "2"];
      const currentMessages = [
        makeMessage("1", "first", 100),
        makeMessage("2", "second", 200),
      ];

      const result = mergePreservedMessages(pageMessages, preserveMessageIds, currentMessages);

      expect(result.map((m) => m.id)).toEqual(["1", "2", "3"]);
    });

    it("created_at 相同时应按 id 字典序排序", () => {
      const pageMessages = [makeMessage("b", "B", 100)];
      const preserveMessageIds = ["a"];
      const currentMessages = [makeMessage("a", "A", 100)];

      const result = mergePreservedMessages(pageMessages, preserveMessageIds, currentMessages);

      expect(result.map((m) => m.id)).toEqual(["a", "b"]);
    });

    it("preserveMessageIds 中不存在的 ID 应被忽略", () => {
      const pageMessages = [makeMessage("1", "hello", 100)];
      const preserveMessageIds = ["nonexistent"];
      const currentMessages: Message[] = [];

      const result = mergePreservedMessages(pageMessages, preserveMessageIds, currentMessages);

      expect(result).toEqual(pageMessages);
    });
  });

  describe("mergeOlderPages", () => {
    it("应以 currentMessages 优先覆盖 olderMessages 中的同 ID 消息", () => {
      const olderMessages = [makeMessage("1", "old", 100)];
      const currentMessages = [makeMessage("1", "new", 100)];

      const result = mergeOlderPages(olderMessages, currentMessages);

      expect(result).toHaveLength(1);
      expect(result[0].content).toBe("new");
    });

    it("应合并两批消息并去重", () => {
      const olderMessages = [makeMessage("1", "first", 100)];
      const currentMessages = [makeMessage("2", "second", 200)];

      const result = mergeOlderPages(olderMessages, currentMessages);

      expect(result).toHaveLength(2);
    });

    it("应按 created_at 排序", () => {
      const olderMessages = [makeMessage("3", "third", 300)];
      const currentMessages = [makeMessage("1", "first", 100)];

      const result = mergeOlderPages(olderMessages, currentMessages);

      expect(result.map((m) => m.id)).toEqual(["1", "3"]);
    });

    it("空数组应返回空数组", () => {
      const result = mergeOlderPages([], []);
      expect(result).toEqual([]);
    });
  });
});
