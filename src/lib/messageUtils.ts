// SPDX-License-Identifier: AGPL-3.0-only

import type { Message } from "@/types";

export const MESSAGE_PAGE_SIZE = 50;

export function mergePreservedMessages(
  pageMessages: Message[],
  preserveMessageIds: string[],
  currentMessages: Message[],
): Message[] {
  if (preserveMessageIds.length === 0) {
    return pageMessages;
  }

  const merged = new Map(pageMessages.map((message) => [message.id, message]));
  const currentMap = new Map(
    currentMessages.map((message) => [message.id, message]),
  );
  for (const messageId of preserveMessageIds) {
    const localMessage = currentMap.get(messageId);
    if (localMessage) {
      const dbMessage = merged.get(messageId);
      if (dbMessage) {
        merged.set(messageId, {
          ...dbMessage,
          content: localMessage.content,
          status: localMessage.status,
        });
      } else {
        merged.set(messageId, localMessage);
      }
    }
  }

  return Array.from(merged.values()).sort(
    (left, right) => left.created_at - right.created_at || left.id.localeCompare(right.id),
  );
}

export function mergeOlderPages(
  olderMessages: Message[],
  currentMessages: Message[],
): Message[] {
  const merged = new Map<string, Message>();
  for (const message of olderMessages) {
    merged.set(message.id, message);
  }
  for (const message of currentMessages) {
    merged.set(message.id, message);
  }
  return Array.from(merged.values()).sort(
    (left, right) => left.created_at - right.created_at || left.id.localeCompare(right.id),
  );
}
