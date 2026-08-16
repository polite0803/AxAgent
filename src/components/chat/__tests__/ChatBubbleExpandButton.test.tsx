import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string, fallback?: string) => fallback ?? key }),
  initReactI18next: { type: "3rdParty", init: () => {} },
}));

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn().mockResolvedValue([]),
  listen: vi.fn().mockResolvedValue(() => {}),
  isTauri: () => false,
}));

import type { Message } from "@/types";
import { ChatBubbleExpandButton } from "../ChatBubbleExpandButton";

const baseMsg: Message = {
  id: "m1",
  conversationId: "c1",
  role: "assistant",
  content: "x",
  providerId: null,
  modelId: null,
  tokenCount: null,
  attachments: [],
  thinking: null,
  toolCallsJson: null,
  toolCallId: null,
  createdAt: 0,
  parentMessageId: null,
  versionIndex: 0,
  isActive: true,
  status: "complete",
};

describe("ChatBubbleExpandButton", () => {
  it("无 meta 时不渲染", () => {
    const { container } = render(
      <MemoryRouter>
        <ChatBubbleExpandButton message={baseMsg} />
      </MemoryRouter>,
    );
    expect(container.querySelector("button")).toBeNull();
  });

  it("未注册的 dual view id 不渲染", () => {
    const msg: Message = { ...baseMsg, meta: { bubbleMeta: { dualViewId: "nonexistent" } } };
    const { container } = render(
      <MemoryRouter>
        <ChatBubbleExpandButton message={msg} />
      </MemoryRouter>,
    );
    expect(container.querySelector("button")).toBeNull();
  });
});
