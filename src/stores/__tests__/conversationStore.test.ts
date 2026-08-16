// SPDX-License-Identifier: AGPL-3.0-only

import type { Message, MessagePage } from "@/types";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
  logIpcError: vi.fn(() => vi.fn()),
}));

import { usePreferenceStore } from "@/stores/domain/preferenceStore";
import { _injectPreferenceStore, useConversationStore } from "../domain/conversationStore";

// 注入 preferenceStore 引用（Vitest 模块加载器下循环依赖不会自动触发注入）
_injectPreferenceStore(usePreferenceStore);

function makeMessage(index: number, conversationId = "conv-1"): Message {
  return {
    id: `msg-${index}`,
    conversationId: conversationId,
    role: index % 2 === 0 ? "assistant" : "user",
    content: `message-${index}`,
    providerId: null,
    modelId: null,
    tokenCount: null,
    attachments: [],
    thinking: null,
    toolCallsJson: null,
    toolCallId: null,
    createdAt: index,
    parentMessageId: null,
    versionIndex: 0,
    isActive: true,
    status: "complete",
  };
}

function makePage(messages: Message[], hasOlder: boolean): MessagePage {
  return {
    messages,
    hasOlder: hasOlder,
    oldestMessageId: messages[0]?.id ?? null,
    totalActiveCount: messages.length,
  };
}

function makeConversation(id: string, overrides: Record<string, unknown> = {}) {
  return {
    id,
    title: `conversation-${id}`,
    modelId: "model-1",
    providerId: "provider-1",
    systemPrompt: null,
    temperature: null,
    maxTokens: null,
    topP: null,
    frequencyPenalty: null,
    searchEnabled: false,
    searchProviderId: null,
    thinkingBudget: null,
    enabledMcpServerIds: [],
    enabledKnowledgeBaseIds: [],
    enabledMemoryNamespaceIds: [],
    enabledWikiIds: [],
    enabledSkillIds: [],
    isPinned: false,
    isArchived: false,
    contextCompression: false,
    categoryId: null,
    parentConversationId: null,
    mode: "chat" as const,
    workStrategy: null,
    messageCount: 0,
    createdAt: 1,
    updatedAt: 1,
    scenario: null,
    workspaceDir: null,
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function flushPromises() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("conversationStore pagination", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useConversationStore.setState({
      conversations: [],
      activeConversationId: null,
      messages: [],
      loading: false,
      loadingOlder: false,
      hasOlderMessages: false,
      oldestLoadedMessageId: null,
      streamingMessageId: null,
      error: null,
      searchEnabled: false,
      searchProviderId: null,
      enabledMcpServerIds: [],
      thinkingBudget: null,
      enabledKnowledgeBaseIds: [],
      activeMemoryNamespaceId: null,
      enabledWikiIds: [],
      archivedConversations: [],
      workspaceSnapshot: null,
    });
  });

  it("loads only the newest 10 messages for the initial conversation page", async () => {
    invokeMock.mockResolvedValueOnce(
      makePage([makeMessage(11), makeMessage(12)], true),
    );
    // useConversationStore imported at module level

    useConversationStore.getState().setActiveConversation("conv-1");
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith("list_messages_page", {
      conversationId: "conv-1",
      limit: 50,
      beforeMessageId: null,
    });
    expect(
      useConversationStore.getState().messages.map((message) => message.id),
    ).toEqual(["msg-11", "msg-12"]);
    expect(useConversationStore.getState().hasOlderMessages).toBe(true);
    expect(useConversationStore.getState().oldestLoadedMessageId).toBe(
      "msg-11",
    );
  });

  it("keeps loading until the newest active conversation request resolves", async () => {
    const pageA = deferred<MessagePage>();
    const pageB = deferred<MessagePage>();
    invokeMock.mockImplementation(
      (cmd: string, args?: Record<string, unknown>) => {
        if (cmd !== "list_messages_page") {
          throw new Error(`unexpected command: ${cmd}`);
        }
        if (args?.conversationId === "conv-a") {
          return pageA.promise;
        }
        if (args?.conversationId === "conv-b") {
          return pageB.promise;
        }
        throw new Error(
          `unexpected conversation: ${String(args?.conversationId)}`,
        );
      },
    );
    // useConversationStore imported at module level

    useConversationStore.getState().setActiveConversation("conv-a");
    useConversationStore.getState().setActiveConversation("conv-b");
    await flushPromises();

    pageA.resolve(makePage([makeMessage(1, "conv-a")], false));
    await flushPromises();

    expect(useConversationStore.getState().activeConversationId).toBe("conv-b");
    expect(useConversationStore.getState().loading).toBe(true);
    expect(useConversationStore.getState().messages).toEqual([]);

    pageB.resolve(makePage([makeMessage(2, "conv-b")], false));
    await flushPromises();

    expect(useConversationStore.getState().loading).toBe(false);
    expect(
      useConversationStore.getState().messages.map((message) => message.id),
    ).toEqual(["msg-2"]);
  });

  it("clears active conversation when the backend reports the conversation is missing", async () => {
    invokeMock.mockImplementation(
      (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === "list_messages_page") {
          if (args?.conversationId === "conv-missing") {
            return Promise.reject(
              new Error("Not found: Conversation conv-missing"),
            );
          }
          if (args?.conversationId === "conv-2") {
            return Promise.resolve(makePage([], false));
          }
        }
        if (cmd === "list_conversations") {
          return Promise.resolve([makeConversation("conv-2")] as never[]);
        }
        throw new Error(`unexpected command: ${cmd}`);
      },
    );
    // useConversationStore imported at module level
    useConversationStore.setState({
      conversations: [makeConversation("conv-missing")] as never[],
    });

    useConversationStore.getState().setActiveConversation("conv-missing");
    await flushPromises();
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith(
      "list_conversations",
      undefined,
      15000,
    );
    expect(useConversationStore.getState().activeConversationId).toBe("conv-2");
    expect(useConversationStore.getState().messages).toEqual([]);
  });

  it("prepends older pages without replacing already loaded messages", async () => {
    invokeMock
      .mockResolvedValueOnce(makePage([makeMessage(11), makeMessage(12)], true))
      .mockResolvedValueOnce(
        makePage([makeMessage(9), makeMessage(10)], false),
      );
    // useConversationStore imported at module level

    useConversationStore.getState().setActiveConversation("conv-1");
    await flushPromises();
    await useConversationStore.getState().loadOlderMessages();

    expect(invokeMock).toHaveBeenLastCalledWith("list_messages_page", {
      conversationId: "conv-1",
      limit: 50,
      beforeMessageId: "msg-11",
    });
    expect(
      useConversationStore.getState().messages.map((message) => message.id),
    ).toEqual(["msg-9", "msg-10", "msg-11", "msg-12"]);
    expect(useConversationStore.getState().hasOlderMessages).toBe(false);
    expect(useConversationStore.getState().loadingOlder).toBe(false);
  });

  it("hydrates persisted conversation preferences when switching active conversations", async () => {
    invokeMock.mockResolvedValue(makePage([], false));
    // useConversationStore imported at module level

    useConversationStore.setState({
      conversations: [
        makeConversation("conv-a", {
          searchEnabled: true,
          searchProviderId: "search-a",
          thinkingBudget: 2048,
          enabledMcpServerIds: ["mcp-a"],
          enabledKnowledgeBaseIds: ["kb-a"],
          enabledMemoryNamespaceIds: ["mem-a"],
        }),
        makeConversation("conv-b", {
          searchEnabled: false,
          searchProviderId: null,
          thinkingBudget: null,
          enabledMcpServerIds: ["mcp-b"],
          enabledKnowledgeBaseIds: [],
          enabledMemoryNamespaceIds: ["mem-b"],
        }),
      ] as never[],
    });

    useConversationStore.getState().setActiveConversation("conv-a");
    await flushPromises();

    expect(useConversationStore.getState().searchEnabled).toBe(true);
    expect(useConversationStore.getState().searchProviderId).toBe("search-a");
    expect(useConversationStore.getState().thinkingBudget).toBe(2048);
    expect(useConversationStore.getState().enabledMcpServerIds).toEqual([
      "mcp-a",
    ]);
    expect(useConversationStore.getState().enabledKnowledgeBaseIds).toEqual([
      "kb-a",
    ]);
    expect(useConversationStore.getState().activeMemoryNamespaceId).toEqual(
      "mem-a",
    );

    useConversationStore.getState().setActiveConversation("conv-b");
    await flushPromises();

    expect(useConversationStore.getState().searchEnabled).toBe(false);
    expect(useConversationStore.getState().searchProviderId).toBeNull();
    expect(useConversationStore.getState().thinkingBudget).toBeNull();
    expect(useConversationStore.getState().enabledMcpServerIds).toEqual([
      "mcp-b",
    ]);
    expect(useConversationStore.getState().enabledKnowledgeBaseIds).toEqual([]);
    expect(useConversationStore.getState().activeMemoryNamespaceId).toEqual(
      "mem-b",
    );
  });

  it("persists search preference changes for the active conversation", async () => {
    invokeMock.mockResolvedValue(makePage([], false));
    invokeMock.mockResolvedValueOnce(makeConversation("conv-1"));
    // useConversationStore imported at module level

    useConversationStore.setState({
      activeConversationId: "conv-1",
      conversations: [makeConversation("conv-1")] as never[],
    });

    useConversationStore.getState().setSearchEnabled(true);
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith("update_conversation", {
      id: "conv-1",
      input: {
        search_enabled: true,
      },
    });
  });

  it("persists MCP changes asynchronously without blocking UI", async () => {
    invokeMock.mockRejectedValueOnce(new Error("save failed"));
    // useConversationStore imported at module level

    useConversationStore.setState({
      activeConversationId: "conv-1",
      conversations: [
        makeConversation("conv-1", { enabledMcpServerIds: ["mcp-a"] }),
      ] as never[],
      enabledMcpServerIds: ["mcp-a"],
    });

    await useConversationStore.getState().toggleMcpServer("mcp-b");
    await flushPromises();
    expect(useConversationStore.getState().enabledMcpServerIds).toEqual([
      "mcp-a",
      "mcp-b",
    ]);
  });

  it("keeps streaming active when a non-final done chunk arrives during a tool loop", async () => {
    const listeners = new Map<string, (event: unknown) => void>();
    listenMock.mockImplementation(
      async (eventName: string, handler: (event: unknown) => void) => {
        listeners.set(eventName, handler);
        return () => {};
      },
    );

    // useConversationStore imported at module level
    const { useStreamStore } = await import("../domain/streamStore");

    useConversationStore.setState({
      activeConversationId: "conv-1",
      messages: [makeMessage(1), makeMessage(2, "conv-1")],
    });
    useStreamStore.setState({
      streaming: true,
      streamingMessageId: "assistant-1",
      streamingConversationId: "conv-1",
    });

    await useConversationStore.getState().startStreamListening();
    const onChunk = listeners.get("chat-stream-chunk");
    expect(onChunk).toBeTypeOf("function");

    onChunk?.({
      payload: {
        conversation_id: "conv-1",
        message_id: "assistant-1",
        chunk: {
          content: null,
          thinking: null,
          tool_calls: null,
          done: true,
          is_final: false,
          usage: null,
        },
      },
    });

    expect(useStreamStore.getState().streaming).toBe(true);
    expect(useStreamStore.getState().streamingMessageId).toBe("assistant-1");
  });

  it("flushes accepted streaming content before stopping the stream", async () => {
    vi.useFakeTimers();

    const listeners = new Map<string, (event: unknown) => void>();
    listenMock.mockImplementation(
      async (eventName: string, handler: (event: unknown) => void) => {
        listeners.set(eventName, handler);
        return () => {};
      },
    );

    // useConversationStore imported at module level
    const { useStreamStore } = await import("../domain/streamStore");

    useConversationStore.setState({
      activeConversationId: "conv-1",
      messages: [
        {
          ...makeMessage(2, "conv-1"),
          id: "assistant-1",
          role: "assistant",
          content: "Hello",
        },
      ],
    });
    useStreamStore.setState({
      streaming: true,
      streamingMessageId: "assistant-1",
      streamingConversationId: "conv-1",
    });

    await useConversationStore.getState().startStreamListening();
    const onChunk = listeners.get("chat-stream-chunk");

    onChunk?.({
      payload: {
        conversation_id: "conv-1",
        message_id: "assistant-1",
        chunk: {
          content: " world",
          thinking: null,
          tool_calls: null,
          done: false,
          usage: null,
        },
      },
    });

    useStreamStore.getState().cancelCurrentStream();

    expect(useConversationStore.getState().messages[0]?.content).toBe(
      "Hello world",
    );

    vi.useRealTimers();
  });

  it("creates a new conversation from a category template when a category id is supplied", async () => {
    invokeMock.mockImplementation(
      (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === "create_conversation") {
          expect(args).toEqual({
            title: "template-conversation",
            modelId: "template-model",
            providerId: "template-provider",
            systemPrompt: "Category prompt",
          });
          return Promise.resolve(
            makeConversation("conv-template", {
              providerId: "template-provider",
              modelId: "template-model",
              systemPrompt: "Category prompt",
            }),
          );
        }

        if (cmd === "update_conversation") {
          expect(args).toEqual({
            id: "conv-template",
            input: {
              category_id: "cat-template",
              system_prompt: "Category prompt",
              temperature: 0.2,
              max_tokens: 8192,
              top_p: 0.95,
              frequency_penalty: 0.4,
              search_enabled: false,
              search_provider_id: null,
              thinking_budget: null,
              enabled_mcp_server_ids: [],
              enabled_knowledge_base_ids: [],
              enabled_memory_namespace_ids: [],
              enabled_wiki_ids: [],
            },
          });

          return Promise.resolve(
            makeConversation("conv-template", {
              providerId: "template-provider",
              modelId: "template-model",
              categoryId: "cat-template",
              systemPrompt: "Category prompt",
              temperature: 0.2,
              maxTokens: 8192,
              topP: 0.95,
              frequencyPenalty: 0.4,
            }),
          );
        }

        if (cmd === "list_messages_page") {
          return Promise.resolve(makePage([], false));
        }

        throw new Error(`unexpected command: ${cmd}`);
      },
    );

    // useConversationStore imported at module level
    const { useCategoryStore } = await import("../feature/categoryStore");

    useCategoryStore.setState({
      categories: [
        {
          id: "cat-template",
          name: "Template",
          iconType: null,
          iconValue: null,
          systemPrompt: "Category prompt",
          defaultProviderId: "template-provider",
          defaultModelId: "template-model",
          defaultTemperature: 0.2,
          defaultMaxTokens: 8192,
          defaultTopP: 0.95,
          defaultFrequencyPenalty: 0.4,
          sortOrder: 0,
          isCollapsed: false,
          createdAt: 1,
          updatedAt: 1,
        },
      ] as never[],
      loading: false,
    });

    const conversation = await useConversationStore
      .getState()
      .createConversation(
        "template-conversation",
        "fallback-model",
        "fallback-provider",
        { categoryId: "cat-template" },
      );

    expect(conversation.categoryId).toBe("cat-template");
    expect(conversation.providerId).toBe("template-provider");
    expect(conversation.modelId).toBe("template-model");
    expect(conversation.temperature).toBe(0.2);
    expect(conversation.maxTokens).toBe(8192);
  });
});
