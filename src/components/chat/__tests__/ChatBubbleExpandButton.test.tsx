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
  conversation_id: "c1",
  role: "assistant",
  content: "x",
  provider_id: null,
  model_id: null,
  token_count: null,
  attachments: [],
  thinking: null,
  tool_calls_json: null,
  tool_call_id: null,
  created_at: 0,
  parent_message_id: null,
  version_index: 0,
  is_active: true,
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
