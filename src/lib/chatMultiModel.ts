import type { Message } from "@/types";

export function hasMultipleModelVersions(versions: Message[]): boolean {
  const models = new Set<string>();
  for (const v of versions) {
    if (v.model_id) { models.add(v.model_id); }
  }
  return models.size > 1;
}

export function shouldRenderStandaloneAssistantError(message: Message): boolean {
  return message.status === "error" && message.role === "assistant";
}
