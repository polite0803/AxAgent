// SPDX-License-Identifier: AGPL-3.0-only

import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App, ConfigProvider } from "antd";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { InputArea } from "../InputArea";

// 所有 mock 状态变量必须放在 vi.hoisted 中，因为 vi.mock 工厂在模块导入阶段
// 就会执行（早于模块顶层代码），此时顶层 const 尚未初始化，引用它们会得到 undefined。
const {
  sendMessage,
  createConversation,
  mockNavigate,
  conversationState,
  providerState,
  settingsState,
  searchState,
  mcpState,
  uiState,
  streamState,
  compressState,
  agentState,
  executionState,
  planState,
  expertState,
  gatewayLinkState,
  knowledgeState,
  memoryState,
  llmWikiState,
  promptTemplateState,
  voicePrefState,
  makeStoreHook,
  gatewayStoreState,
  llmWikiStoreState,
  expertStoreState,
} = vi.hoisted(() => {
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
      defaultProviderId: null,
      defaultModelId: null,
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

  const setQuotedMessageId = vi.fn();

  const uiState = {
    setSettingsSection,
    quotedMessageId: null,
    setQuotedMessageId,
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

  const voicePrefState = {
    ttsVoice: "",
    sttProviderId: null,
    ttsProviderId: null,
  };

  // 组件在 render 阶段会调用 useXxxStore.getState()，因此 mock 的 hook 必须自带
  // getState / setState，否则 render 抛错导致整个组件无法挂载。
  const makeStoreHook = (getState: () => Record<string, unknown>) => {
    const hook = (selector?: (s: Record<string, unknown>) => unknown) =>
      (typeof selector === "function" ? selector(getState()) : getState()) as unknown;
    (hook as unknown as { getState: () => Record<string, unknown> }).getState = getState;
    (hook as unknown as { setState: (p: Record<string, unknown>) => void }).setState = () => {};
    return hook as unknown as
      & ((
        selector?: (s: Record<string, unknown>) => unknown,
      ) => unknown)
      & {
        getState: () => Record<string, unknown>;
        setState: (p: Record<string, unknown>) => void;
      };
  };

  const gatewayStoreState = {
    keys: [] as unknown[],
    fetchKeys: vi.fn(),
    decryptKey: vi.fn(),
  };

  const llmWikiStoreState = {
    wikis: [] as unknown[],
    loadWikis: vi.fn(),
  };

  const expertStoreState = {
    builtinRoles: [] as unknown[],
    agencyRoles: [] as unknown[],
    customRoles: [] as unknown[],
    getRolesByCategory: () => ({}),
    getRoleById: () => null,
  };

  return {
    sendMessage,
    createConversation,
    setSearchEnabled,
    setSearchProviderId,
    loadSearchProviders,
    loadMcpServers,
    toggleMcpServer,
    setThinkingBudget,
    insertContextClear,
    setSettingsSection,
    mockNavigate,
    clearAllMessages,
    updateConversation,
    setActiveConversation,
    setPendingPromptText,
    conversationState,
    providerState,
    settingsState,
    searchState,
    mcpState,
    uiState,
    streamState,
    compressState,
    agentState,
    executionState,
    planState,
    expertState,
    gatewayLinkState,
    knowledgeState,
    memoryState,
    llmWikiState,
    promptTemplateState,
    voicePrefState,
    makeStoreHook,
    gatewayStoreState,
    llmWikiStoreState,
    expertStoreState,
  };
});

vi.mock("react-router-dom", () => ({
  useNavigate: () => mockNavigate,
}));

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
  useConversationStore: makeStoreHook(() => conversationState),
  useProviderStore: makeStoreHook(() => providerState),
  useSettingsStore: makeStoreHook(() => settingsState),
  useSearchStore: makeStoreHook(() => searchState),
  useMcpStore: makeStoreHook(() => mcpState),
  useUIStore: makeStoreHook(() => uiState),
  useStreamStore: makeStoreHook(() => streamState),
  useCompressStore: makeStoreHook(() => compressState),
  useAgentStore: makeStoreHook(() => agentState),
  useExecutionStore: makeStoreHook(() => executionState),
  usePlanStore: makeStoreHook(() => planState),
  useExpertStore: makeStoreHook(() => expertState),
  useGatewayLinkStore: makeStoreHook(() => gatewayLinkState),
  useKnowledgeStore: makeStoreHook(() => knowledgeState),
  useMemoryStore: makeStoreHook(() => memoryState),
  useLlmWikiStore: makeStoreHook(() => llmWikiState),
  usePromptTemplateStore: makeStoreHook(() => promptTemplateState),
  useVoicePreferenceStore: makeStoreHook(() => voicePrefState),
}));

// 组件从子模块路径导入这些 store（绕过上面的 @/stores barrel mock），
// 若不单独 mock，其 mount 期的异步 loader 会调用被 mock 的 invoke 并写入
// undefined，污染真实模块级 store，导致后续测试 case 崩溃。
vi.mock("@/stores/feature/gatewayStore", () => ({
  useGatewayStore: makeStoreHook(() => gatewayStoreState),
}));

vi.mock("@/stores/feature/llmWikiStore", () => ({
  useLlmWikiStore: makeStoreHook(() => llmWikiStoreState),
}));

vi.mock("@/stores/feature/expertStore", () => ({
  useExpertStore: makeStoreHook(() => expertStoreState),
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
  CommandSuggest: () => null,
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

vi.mock("antd", () => {
  const React = require("react");
  const antd = {
    App: Object.assign(
      ({ children }: { children: React.ReactNode }) => React.createElement(React.Fragment, null, children),
      {
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
    ),
    ConfigProvider: ({ children }: { children: React.ReactNode }) =>
      React.createElement(React.Fragment, null, children),
    theme: {
      useToken: () => ({
        token: {
          colorPrimary: "#1890ff",
          colorTextSecondary: "#666",
          colorBorderSecondary: "#ddd",
        },
      }),
    },
    Button: ({ children, ...props }: Record<string, unknown>) => React.createElement("button", props, children),
    Dropdown: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Tooltip: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Space: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Flex: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Typography: {
      Text: ({ children }: Record<string, unknown>) => React.createElement("span", null, children),
      Paragraph: ({ children }: Record<string, unknown>) => React.createElement("p", null, children),
      Title: ({ children }: Record<string, unknown>) => React.createElement("h3", null, children),
    },
    Tag: ({ children }: Record<string, unknown>) => React.createElement("span", null, children),
    Badge: ({ children }: Record<string, unknown>) => React.createElement("span", null, children),
    Avatar: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Input: Object.assign(
      (props: Record<string, unknown>) => React.createElement("input", props),
      {
        TextArea: (props: Record<string, unknown>) => React.createElement("textarea", props),
      },
    ),
    Select: ({ children }: Record<string, unknown>) => React.createElement("select", null, children),
    Switch: (props: Record<string, unknown>) => React.createElement("input", { ...props, type: "checkbox" }),
    Slider: (props: Record<string, unknown>) => React.createElement("input", { ...props, type: "range" }),
    Modal: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Popover: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Popconfirm: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Spin: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Skeleton: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Empty: () => React.createElement("div", null),
    Tabs: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Card: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Collapse: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Divider: () => React.createElement("hr"),
    Alert: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Row: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Col: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Breadcrumb: ({ children }: Record<string, unknown>) => React.createElement("nav", null, children),
    Menu: ({ children }: Record<string, unknown>) => React.createElement("ul", null, children),
    Layout: {
      Header: ({ children }: Record<string, unknown>) => React.createElement("header", null, children),
      Sider: ({ children }: Record<string, unknown>) => React.createElement("aside", null, children),
      Content: ({ children }: Record<string, unknown>) => React.createElement("main", null, children),
      Footer: ({ children }: Record<string, unknown>) => React.createElement("footer", null, children),
    },
    Progress: (props: Record<string, unknown>) => React.createElement("progress", props),
    Result: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    List: ({ children }: Record<string, unknown>) => React.createElement("ul", null, children),
    Segmented: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Upload: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Drawer: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Radio: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Checkbox: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    DatePicker: (props: Record<string, unknown>) => React.createElement("input", { ...props, type: "date" }),
    TimePicker: (props: Record<string, unknown>) => React.createElement("input", { ...props, type: "time" }),
    InputNumber: (props: Record<string, unknown>) => React.createElement("input", { ...props, type: "number" }),
    Transfer: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    TreeSelect: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Cascader: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    AutoComplete: (props: Record<string, unknown>) => React.createElement("input", props),
    Mentions: (props: Record<string, unknown>) => React.createElement("textarea", props),
    Rate: (props: Record<string, unknown>) => React.createElement("div", props),
    ColorPicker: (props: Record<string, unknown>) => React.createElement("input", { ...props, type: "color" }),
    QRCode: () => React.createElement("div", null),
    Watermark: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Tour: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Carousel: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Image: (props: Record<string, unknown>) => React.createElement("img", props),
    Statistic: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Timeline: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Descriptions: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
    Table: ({ children }: Record<string, unknown>) => React.createElement("table", null, children),
    FloatButton: () => React.createElement("button", null),
    notification: {
      info: vi.fn(),
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
    },
    message: {
      info: vi.fn(),
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
    },
    Form: Object.assign(
      ({ children, ...props }: Record<string, unknown>) => React.createElement("form", props, children),
      {
        useForm: () => [
          {
            getFieldsValue: vi.fn(() => ({})),
            getFieldValue: vi.fn(),
            setFieldsValue: vi.fn(),
            setFieldValue: vi.fn(),
            validateFields: vi.fn(async () => ({})),
            resetFields: vi.fn(),
            submit: vi.fn(),
          },
        ],
        Item: ({ children, ...props }: Record<string, unknown>) => React.createElement("div", props, children),
        List: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
        Provider: ({ children }: Record<string, unknown>) => React.createElement(React.Fragment, null, children),
        useWatch: () => undefined,
        useFormInstance: () => ({}),
        ErrorList: ({ children }: Record<string, unknown>) => React.createElement("div", null, children),
      },
    ),
    version: "5.0.0",
  };
  return antd;
});

describe("InputArea", () => {
  let draftSeq = 0;
  beforeEach(() => {
    vi.clearAllMocks();
    // 每次测试用唯一会话 id：组件卸载时会把草稿写入模块级 _draftCache（按
    // activeConversationId 索引），若不隔离会导致输入值跨测试污染。
    draftSeq += 1;
    const id = `conv-${draftSeq}`;
    conversationState.activeConversationId = id;
    conversationState.conversations = [
      { id, title: "Test", provider_id: "provider-1", model_id: "model-1" },
    ];
    conversationState.streaming = false;
    streamState.activeStreams = {};
  });

  const renderInputArea = () =>
    render(
      <ConfigProvider>
        <App>
          <InputArea />
        </App>
      </ConfigProvider>,
    );

  it("renders the textarea with placeholder", () => {
    renderInputArea();
    const textarea = screen.getByPlaceholderText("chat.inputPlaceholder");
    expect(textarea).toBeInTheDocument();
  });

  it("renders the send button", () => {
    renderInputArea();
    const sendButton = screen.getByRole("button", { name: /send/i });
    expect(sendButton).toBeInTheDocument();
  });

  it("allows typing in the textarea", async () => {
    renderInputArea();
    const textarea = screen.getByPlaceholderText(
      "chat.inputPlaceholder",
    ) as HTMLTextAreaElement;

    await userEvent.type(textarea, "Hello world");

    expect(textarea.value).toBe("Hello world");
  });

  it("clears the textarea immediately after sending", async () => {
    let resolveSend!: () => void;
    sendMessage.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          resolveSend = resolve;
        }),
    );

    renderInputArea();

    const textarea = screen.getByPlaceholderText(
      "chat.inputPlaceholder",
    ) as HTMLTextAreaElement;
    await userEvent.type(textarea, "search me");

    expect(textarea.value).toBe("search me");

    fireEvent.keyDown(textarea, { key: "Enter", code: "Enter" });

    expect(textarea.value).toBe("");

    resolveSend();
  });

  it("does not send message on Shift+Enter", async () => {
    renderInputArea();
    const textarea = screen.getByPlaceholderText(
      "chat.inputPlaceholder",
    ) as HTMLTextAreaElement;

    await userEvent.type(textarea, "line1");
    fireEvent.keyDown(textarea, {
      key: "Enter",
      code: "Enter",
      shiftKey: true,
    });

    // Shift+Enter 的换行由浏览器原生处理（jsdom 无法模拟），组件不拦截；
    // 核心回归点：Shift+Enter 不应触发发送，从而与 Enter 区分。
    expect(sendMessage).not.toHaveBeenCalled();
    expect(textarea.value).toBe("line1");
  });

  it("creates a new conversation when no active conversation exists", async () => {
    vi.mocked(createConversation).mockResolvedValueOnce({
      id: "conv-new",
      title: "New Chat",
      provider_id: "provider-1",
      model_id: "model-1",
    });
    conversationState.activeConversationId = "";

    renderInputArea();

    const textarea = screen.getByPlaceholderText(
      "chat.inputPlaceholder",
    ) as HTMLTextAreaElement;
    await userEvent.type(textarea, "Hello");
    fireEvent.keyDown(textarea, { key: "Enter", code: "Enter" });

    expect(createConversation).toHaveBeenCalled();

    conversationState.activeConversationId = "conv-1";
  });

  it("shows stop button (hides send button) when streaming is active", () => {
    // 组件从 useStreamStore.activeStreams 推导 streaming；流式期间发送按钮
    // 会被替换为停止生成按钮（文本框保持可编辑以便续写）。
    streamState.activeStreams = {
      [conversationState.activeConversationId]: { abortController: null },
    };

    renderInputArea();

    expect(screen.getByTestId("stop-generation-btn")).toBeInTheDocument();
    expect(screen.queryByTestId("send-btn")).toBeNull();

    streamState.activeStreams = {};
  });

  it("enables textarea when streaming is inactive", () => {
    conversationState.streaming = false;

    renderInputArea();

    const textarea = screen.getByPlaceholderText(
      "chat.inputPlaceholder",
    ) as HTMLTextAreaElement;
    expect(textarea.disabled).toBe(false);
  });
});
