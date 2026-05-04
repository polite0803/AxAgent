import type { Message } from "@/types";

// ─── Constants ───

export const MESSAGE_PAGE_SIZE = 50;

// ─── Helper functions ───

export function mergePreservedMessages(
  pageMessages: Message[],
  preserveMessageIds: string[],
  currentMessages: Message[],
): Message[] {
  if (preserveMessageIds.length === 0) {
    return pageMessages;
  }

  const merged = new Map(pageMessages.map((message) => [message.id, message]));
  for (const messageId of preserveMessageIds) {
    const localMessage = currentMessages.find((message) => message.id === messageId);
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

export function mergeOlderPages(olderMessages: Message[], currentMessages: Message[]): Message[] {
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
