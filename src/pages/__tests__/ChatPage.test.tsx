import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChatPage } from "../ChatPage";

const fetchConversations = vi.fn();
const fetchProviders = vi.fn();
const saveSettings = vi.fn();

const conversationState = {
  conversations: [] as Array<{ id: string }>,
  fetchConversations,
  activeConversationId: null,
  messages: [],
};

const providerState = {
  providers: [] as Array<{ id: string }>,
  fetchProviders,
};

const settingsState = {
  settings: {},
  saveSettings,
};

const tabState = {
  tabs: [],
  activeTabId: null,
  openTab: vi.fn(),
  updateTabTitle: vi.fn(),
  setActiveTab: vi.fn(),
};

vi.mock("antd", () => ({
  theme: {
    useToken: () => ({
      token: {
        colorBgContainer: "#111",
        colorBgElevated: "#222",
        colorFillQuaternary: "#333",
        colorTextSecondary: "#aaa",
        colorPrimary: "#1890ff",
      },
    }),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/stores", () => ({
  useConversationStore: (selector: (state: typeof conversationState) => unknown) => selector(conversationState),
  useProviderStore: (selector: (state: typeof providerState) => unknown) => selector(providerState),
  useSettingsStore: (selector: (state: typeof settingsState) => unknown) => selector(settingsState),
  useTabStore: (selector: (state: typeof tabState) => unknown) => selector(tabState),
}));

vi.mock("@/components/chat/ChatSidebar", () => ({
  ChatSidebar: () => <div data-testid="chat-sidebar">sidebar</div>,
}));

vi.mock("@/components/chat/ChatView", () => ({
  ChatView: () => <div data-testid="chat-view">chat-view</div>,
}));

vi.mock("@/components/chat/TabBar", () => ({
  TabBar: () => <div data-testid="tab-bar">tab-bar</div>,
}));

vi.mock("@/components/chat/AgentExecutionPanel", () => ({
  AgentExecutionPanel: () => <div data-testid="agent-execution-panel">agent-panel</div>,
}));

vi.mock("@/components/chat/ScrollToMessageContext", () => ({
  ScrollToMessageProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

describe("ChatPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    conversationState.conversations = [];
    providerState.providers = [];
  });

  it("fetches conversations and providers only when the stores are empty", () => {
    render(<ChatPage />);

    expect(fetchConversations).toHaveBeenCalledTimes(1);
    expect(fetchProviders).toHaveBeenCalledTimes(1);
  });

  it("skips refetching when conversations and providers are already loaded", () => {
    conversationState.conversations = [{ id: "conv-1" }];
    providerState.providers = [{ id: "provider-1" }];

    render(<ChatPage />);

    expect(fetchConversations).not.toHaveBeenCalled();
    expect(fetchProviders).not.toHaveBeenCalled();
  });
});
