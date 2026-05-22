import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App, ConfigProvider } from "antd";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { InputArea } from "../InputArea";

const sendMessage = vi.fn();
const createConversation = vi.fn();
const setSearchEnabled = vi.fn();
const setSearchProviderId = vi.fn();
const loadSearchProviders = vi.fn();
const loadMcpServers = vi.fn();
const toggleMcpServer = vi.fn();
const setThinkingBudget = vi.fn();
const insertContextClear = vi.fn();
const setSettingsSection = vi.fn();
const mockNavigate = vi.fn();
const clearAllMessages = vi.fn();
const updateConversation = vi.fn();
const setActiveConversation = vi.fn();
const setPendingPromptText = vi.fn();

vi.mock("react-router-dom", () => ({
  useNavigate: () => mockNavigate,
}));

const conversationState = {
  streaming: false,
  activeConversationId: "conv-1",
  sendMessage,
  createConversation,
  messages: [],
  conversations: [
    {
      id: "conv-1",
      title: "Test",
      provider_id: "provider-1",
      model_id: "model-1",
    },
  ],
  searchEnabled: true,
  searchProviderId: "search-1",
  setSearchEnabled,
  setSearchProviderId,
  enabledMcpServerIds: [] as string[],
  toggleMcpServer,
  thinkingBudget: null as number | null,
  setThinkingBudget,
  insertContextClear,
  clearAllMessages,
  updateConversation,
  setActiveConversation,
  setPendingPromptText,
  pendingPromptText: null,
  hasOlderMessages: false,
  totalActiveCount: 0,
  mcpMode: "auto",
  setMcpMode: vi.fn(),
  enabledKnowledgeBaseIds: [] as string[],
  toggleKnowledgeBase: vi.fn(),
  activeMemoryNamespaceId: null,
  setActiveMemoryNamespace: vi.fn(),
  enabledWikiIds: [] as string[],
  toggleWiki: vi.fn(),
  sendMultiModelMessage: vi.fn(),
};

const providerState = {
  providers: [
    {
      id: "provider-1",
      enabled: true,
      models: [
        {
          model_id: "model-1",
          enabled: true,
          capabilities: [],
        },
      ],
    },
  ],
  loading: false,
};

const settingsState = {
  settings: {
    default_provider_id: null,
    default_model_id: null,
  },
};

const searchState = {
  providers: [
    {
      id: "search-1",
      name: "Test Search",
      providerType: "tavily",
    },
  ],
  loadProviders: loadSearchProviders,
};

const mcpState = {
  servers: [],
  loadServers: loadMcpServers,
};

const uiState = {
  setSettingsSection,
};

const streamState = {
  activeStreams: {},
  cancelCurrentStream: vi.fn(),
};

const compressState = {
  compressing: false,
  getCompressionSummary: vi.fn(),
};

const agentState = {
  clearConversation: vi.fn(),
};

const executionState = {
  clearConversation: vi.fn(),
};

const planState = {
  clearActivePlan: vi.fn(),
};

const expertState = {
  getRolesByCategory: () => ({}),
  getRoleById: () => null,
};

const gatewayLinkState = {
  links: [],
  fetchLinks: vi.fn(),
  createGatewayConversation: vi.fn(),
};

const knowledgeState = {
  bases: [],
  loadBases: vi.fn(),
};

const memoryState = {
  namespaces: [],
  loadNamespaces: vi.fn(),
};

const llmWikiState = {
  wikis: [],
  loadWikis: vi.fn(),
};

const promptTemplateState = {
  incrementUsage: vi.fn(),
};

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
  initReactI18next: {
    type: "3rdParty",
    init: () => {},
  },
}));

vi.mock("@/stores", () => ({
  useConversationStore: (
    selector: (state: typeof conversationState) => unknown,
  ) => {
    if (typeof selector === "function") {
      return selector(conversationState);
    }
    return conversationState;
  },
  useProviderStore: (selector: (state: typeof providerState) => unknown) => {
    if (typeof selector === "function") {
      return selector(providerState);
    }
    return providerState;
  },
  useSettingsStore: (selector: (state: typeof settingsState) => unknown) => {
    if (typeof selector === "function") {
      return selector(settingsState);
    }
    return settingsState;
  },
  useSearchStore: (selector: (state: typeof searchState) => unknown) => {
    if (typeof selector === "function") {
      return selector(searchState);
    }
    return searchState;
  },
  useMcpStore: (selector: (state: typeof mcpState) => unknown) => {
    if (typeof selector === "function") {
      return selector(mcpState);
    }
    return mcpState;
  },
  useUIStore: (selector: (state: typeof uiState) => unknown) => {
    if (typeof selector === "function") {
      return selector(uiState);
    }
    return uiState;
  },
  useStreamStore: (selector: (state: typeof streamState) => unknown) => {
    if (typeof selector === "function") {
      return selector(streamState);
    }
    return streamState;
  },
  useCompressStore: (selector: (state: typeof compressState) => unknown) => {
    if (typeof selector === "function") {
      return selector(compressState);
    }
    return compressState;
  },
  useAgentStore: (selector: (state: typeof agentState) => unknown) => {
    if (typeof selector === "function") {
      return selector(agentState);
    }
    return agentState;
  },
  useExecutionStore: (selector: (state: typeof executionState) => unknown) => {
    if (typeof selector === "function") {
      return selector(executionState);
    }
    return executionState;
  },
  usePlanStore: (selector: (state: typeof planState) => unknown) => {
    if (typeof selector === "function") {
      return selector(planState);
    }
    return planState;
  },
  useExpertStore: (selector: (state: typeof expertState) => unknown) => {
    if (typeof selector === "function") {
      return selector(expertState);
    }
    return expertState;
  },
  useGatewayLinkStore: (
    selector: (state: typeof gatewayLinkState) => unknown,
  ) => {
    if (typeof selector === "function") {
      return selector(gatewayLinkState);
    }
    return gatewayLinkState;
  },
  useKnowledgeStore: (selector: (state: typeof knowledgeState) => unknown) => {
    if (typeof selector === "function") {
      return selector(knowledgeState);
    }
    return knowledgeState;
  },
  useMemoryStore: (selector: (state: typeof memoryState) => unknown) => {
    if (typeof selector === "function") {
      return selector(memoryState);
    }
    return memoryState;
  },
  useLlmWikiStore: (selector: (state: typeof llmWikiState) => unknown) => {
    if (typeof selector === "function") {
      return selector(llmWikiState);
    }
    return llmWikiState;
  },
  usePromptTemplateStore: (
    selector: (state: typeof promptTemplateState) => unknown,
  ) => {
    if (typeof selector === "function") {
      return selector(promptTemplateState);
    }
    return promptTemplateState;
  },
}));

vi.mock("@/lib/modelCapabilities", () => ({
  findModelByIds: () => ({
    model_id: "model-1",
    capabilities: [],
    max_tokens: 4096,
  }),
  supportsReasoning: () => false,
  modelHasCapability: () => false,
}));

vi.mock("@/lib/shortcuts", () => ({
  getShortcutBinding: () => "",
  formatShortcutForDisplay: () => "",
}));

vi.mock("@/lib/tokenEstimator", () => ({
  estimateMessageTokens: () => 10,
  estimateTokens: () => 10,
}));

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn(),
  isTauri: false,
}));

vi.mock("@lobehub/icons", () => ({
  ModelIcon: () => null,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

vi.mock("@/components/shared/SearchProviderIcon", () => ({
  SearchProviderTypeIcon: () => null,
  PROVIDER_TYPE_LABELS: {
    tavily: "Tavily",
  },
}));

vi.mock("@/components/shared/KnowledgeBaseIcon", () => ({
  KnowledgeBaseIcon: () => null,
}));

vi.mock("@/components/shared/NamespaceIcon", () => ({
  NamespaceIcon: () => null,
}));

vi.mock("@/components/shared/McpServerIcon", () => ({
  McpServerIcon: () => null,
}));

vi.mock("@/components/skill/SkillToolbar", () => ({
  SkillToolbar: () => null,
}));

vi.mock("@/components/chat/CommandSuggest", () => ({
  default: () => null,
}));

vi.mock("../VoiceCall", () => ({
  VoiceCall: () => null,
}));

vi.mock("../ConversationSettingsModal", () => ({
  ConversationSettingsModal: () => null,
}));

vi.mock("../ModelSelector", () => ({
  ModelSelector: () => null,
}));

vi.mock("../PromptTemplateSelector", () => ({
  PromptTemplateSelector: () => null,
}));

vi.mock("../ModelRoutingConfigPanel", () => ({
  default: () => null,
}));

vi.mock("../PlanHistoryPanel", () => ({
  PlanHistoryPanel: () => null,
}));

vi.mock("antd", async () => {
  const actual = await vi.importActual("antd");
  return {
    ...actual,
    App: {
      ...(actual.App as Record<string, unknown>),
      useApp: () => ({
        message: {
          info: vi.fn(),
          success: vi.fn(),
          error: vi.fn(),
          warning: vi.fn(),
        },
        modal: {
          confirm: vi.fn(),
        },
      }),
    },
    theme: {
      useToken: () => ({
        token: {
          colorPrimary: "#1890ff",
          colorTextSecondary: "#666",
          colorBorderSecondary: "#ddd",
        },
      }),
    },
  };
});

describe("InputArea", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it.skip("clears the textarea immediately after sending even while search-backed send is still pending", async () => {
    let resolveSend!: () => void;
    sendMessage.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          resolveSend = resolve;
        }),
    );

    render(
      <ConfigProvider>
        <App>
          <InputArea />
        </App>
      </ConfigProvider>,
    );

    const textarea = screen.getByPlaceholderText(
      "chat.inputPlaceholder",
    ) as HTMLTextAreaElement;
    await userEvent.type(textarea, "search me");

    expect(textarea.value).toBe("search me");

    fireEvent.keyDown(textarea, { key: "Enter", code: "Enter" });

    expect(textarea.value).toBe("");

    resolveSend();
  });
});
