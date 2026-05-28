import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChatPage } from "../ChatPage";

const fetchConversations = vi.fn();
const fetchProviders = vi.fn();
const saveSettings = vi.fn();

const conversationState = {
  conversations: [] as Array<{ id: string }>,
  fetchConversations,
  activeConversationId: null as string | null,
  messages: [],
  setActiveConversation: vi.fn(),
};

const providerState = {
  providers: [] as Array<{ id: string }>,
  fetchProviders,
};

const settingsState = {
  settings: {} as Record<string, unknown>,
  saveSettings,
};

const tabState = {
  tabs: [] as Array<{ id: string; conversationId: string; title: string }>,
  activeTabId: null as string | null,
  openTab: vi.fn(),
  closeTab: vi.fn(),
  updateTabTitle: vi.fn(),
  setActiveTab: vi.fn(),
};

const rightPanelState = {
  setPredictionContext: vi.fn(),
};

const uiState = {
  deviceLayout: "desktop" as string,
};

vi.mock("antd", () => ({
  Typography: {
    Title: ({ children }: any) => children,
    Text: ({ children }: any) => children,
    Paragraph: ({ children }: any) => children,
  },
  Input: Object.assign(
    (props: any) => <input {...props} />,
    { TextArea: (props: any) => <textarea {...props} />, Search: (props: any) => <input {...props} /> },
  ),
  theme: {
    useToken: () => ({
      token: {
        colorBgContainer: "#111",
        colorBgElevated: "#222",
        colorFillQuaternary: "#333",
        colorTextSecondary: "#aaa",
        colorTextQuaternary: "#bbb",
        colorPrimary: "#1890ff",
        colorBorderSecondary: "#444",
        colorBgMask: "rgba(0,0,0,0.5)",
      },
    }),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
  initReactI18next: {
    type: "3rdParty",
    init: () => {},
  },
}));

vi.mock("@/stores", () => ({
  useConversationStore: (
    selector: (state: typeof conversationState) => unknown,
  ) => selector(conversationState),
  useProviderStore: (selector: (state: typeof providerState) => unknown) => selector(providerState),
  useSettingsStore: (selector: (state: typeof settingsState) => unknown) => selector(settingsState),
  useTabStore: (selector: (state: typeof tabState) => unknown) => selector(tabState),
  useRightPanelStore: (selector: (state: typeof rightPanelState) => unknown) => selector(rightPanelState),
  useUIStore: (selector: (state: typeof uiState) => unknown) => selector(uiState),
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

vi.mock("@/components/chat/RightPanelContainer", () => ({
  RightPanelContainer: () => <div data-testid="right-panel-container">right-panel</div>,
}));

vi.mock("@/components/skill/SkillChatCommands", () => ({
  useSkillChatCommands: () => [],
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
