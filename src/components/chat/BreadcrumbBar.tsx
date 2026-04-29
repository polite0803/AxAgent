import { ChevronRight, Home } from "lucide-react";
import type { Conversation } from "@/types";

interface BreadcrumbBarProps {
  conversations: Conversation[];
  activeConversationId: string;
  setActiveConversation: (id: string) => void;
}

/**
 * Build the breadcrumb path from root → parent → child for sub-agent sessions.
 * Only shown when the active conversation has a parent_conversation_id.
 */
function buildBreadcrumbs(
  conversations: Conversation[],
  activeId: string,
): Conversation[] {
  const path: Conversation[] = [];
  let currentId: string | null = activeId;

  while (currentId) {
    const conv = conversations.find((c) => c.id === currentId);
    if (!conv) break;
    path.unshift(conv);
    currentId = conv.parent_conversation_id;
  }

  return path;
}

export function BreadcrumbBar({
  conversations,
  activeConversationId,
  setActiveConversation,
}: BreadcrumbBarProps) {
  const active = conversations.find((c) => c.id === activeConversationId);
  if (!active?.parent_conversation_id) return null;

  const breadcrumbs = buildBreadcrumbs(conversations, activeConversationId);
  if (breadcrumbs.length <= 1) return null;

  return (
    <div
      className="flex items-center gap-1 px-3 py-1.5 text-xs border-b border-gray-400/10"
      style={{
        backgroundColor: "color-mix(in srgb, var(--background-base, #fff) 96%, transparent)",
        overflow: "hidden",
        whiteSpace: "nowrap",
      }}
    >
      {breadcrumbs.map((conv, index) => {
        const isLast = index === breadcrumbs.length - 1;
        const isClickable = !isLast && conv.id !== activeConversationId;

        return (
          <span key={conv.id} className="flex items-center gap-1 min-w-0">
            {index > 0 && (
              <ChevronRight
                size={10}
                style={{ flexShrink: 0, opacity: 0.4 }}
              />
            )}
            {index === 0 && (
              <Home size={11} style={{ flexShrink: 0, opacity: 0.5 }} />
            )}
            <span
              onClick={isClickable ? () => setActiveConversation(conv.id) : undefined}
              className={isLast ? "truncate font-medium" : "truncate cursor-pointer hover:underline"}
              style={{
                maxWidth: isLast ? "none" : 160,
                color: isLast ? undefined : "var(--text-secondary, #666)",
              }}
              title={conv.title}
            >
              {conv.title || "Untitled"}
            </span>
          </span>
        );
      })}
    </div>
  );
}
