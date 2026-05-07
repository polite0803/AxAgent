/**
 * multiModelStore.ts — 多模型并行（Companion Models）状态管理
 *
 * 管理多模型同时响应的状态和方法。
 * 模块级多模型变量（_isMultiModelActive 等）仍在 streamStore.ts 中，
 * 因为它们处于流式性能关键路径上。
 */

import { invoke } from "@/lib/invoke";
import type { AttachmentInput } from "@/types";
import { create } from "zustand";
import { useConversationStore } from "./conversationStore";
import { getEffectiveThinkingBudget, usePreferenceStore } from "./preferenceStore";
import {
  _isMultiModelActive,
  _multiModelDoneResolve,
  _multiModelFirstMessageId,
  _multiModelFirstModelId,
  _multiModelTotalRemaining,
  _userManuallySelectedVersion,
  decrementMultiModelTotalRemaining,
  resetMultiModelState,
  setIsMultiModelActive,
  setMultiModelDoneResolve,
  setMultiModelFirstMessageId,
  setMultiModelFirstModelId,
  setMultiModelTotalRemaining,
  setStreamBuffer,
  setUserManuallySelectedVersion,
  stopConversationStream,
  useStreamStore,
} from "./streamStore";

interface MultiModelState {
  /** Companion models pending or currently streaming */
  pendingCompanionModels: Array<{ providerId: string; model_id: string }>;
  /** User message ID of the current multi-model request */
  multiModelParentId: string | null;
  /** Message IDs of models that have completed their streams */
  multiModelDoneMessageIds: string[];
  /** Pending prompt text from welcome cards */
  pendingPromptText: string | null;
  setPendingPromptText: (text: string | null) => void;

  /** Send a message and generate responses from multiple companion models */
  sendMultiModelMessage: (
    content: string,
    companionModels: Array<{ providerId: string; model_id: string }>,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
  ) => Promise<void>;
}

export const useMultiModelStore = create<MultiModelState>((set, get) => ({
  pendingCompanionModels: [],
  multiModelParentId: null,
  multiModelDoneMessageIds: [],
  pendingPromptText: null,
  setPendingPromptText: (text) => set({ pendingPromptText: text }),

  sendMultiModelMessage: async (content, companionModels, attachments = [], searchProviderId = null) => {
    const convStore = useConversationStore.getState();
    const conversationId = convStore.activeConversationId;
    if (!conversationId || companionModels.length === 0) { return; }

    // Guard: prevent duplicate sends while a stream is already active
    const activeStreams = useStreamStore.getState().activeStreams;
    if (conversationId in activeStreams) {
      console.warn("[sendMultiModelMessage] Ignoring duplicate send — stream already active for", conversationId);
      return;
    }

    // Save original conversation model to restore later
    const conv = convStore.conversations.find((c) => c.id === conversationId);
    const originalProviderId = conv?.provider_id;
    const originalModelId = conv?.model_id;

    // Track ALL models (first + companions) in a unified counter
    setIsMultiModelActive(true);
    setMultiModelTotalRemaining(companionModels.length);
    setMultiModelFirstModelId(companionModels[0].model_id);
    set({ pendingCompanionModels: [...companionModels] });

    // Switch to the first selected model and send
    const firstModel = companionModels[0];
    try {
      await convStore.updateConversation(conversationId, {
        provider_id: firstModel.providerId,
        model_id: firstModel.model_id,
      });
    } catch (e) {
      console.error("[sendMultiModelMessage] failed to switch model:", e);
      resetMultiModelState();
      set({ pendingCompanionModels: [], multiModelParentId: null, multiModelDoneMessageIds: [] });
      return;
    }

    // sendMessage returns after invoke (message created in DB), stream continues in background
    await convStore.sendMessage(content, attachments, searchProviderId);

    // Find the user message that was just created
    const msgs = useConversationStore.getState().messages;
    const lastUserMsg = [...msgs].reverse().find((m) => m.role === "user");
    if (!lastUserMsg) {
      resetMultiModelState();
      set({ pendingCompanionModels: [], multiModelParentId: null, multiModelDoneMessageIds: [] });
      if (originalProviderId && originalModelId) {
        void convStore.updateConversation(conversationId, {
          provider_id: originalProviderId,
          model_id: originalModelId,
        });
      }
      return;
    }

    // Scope loading indicators to this message and set parent_message_id
    set({
      multiModelParentId: lastUserMsg.id,
    });
    useConversationStore.setState((s) => ({
      messages: s.messages.map((m) =>
        m.id === useStreamStore.getState().streamingMessageId && m.role === "assistant"
          ? { ...m, parent_message_id: lastUserMsg.id }
          : m
      ),
    }));

    // Create a unified promise for ALL models
    const allDone = new Promise<void>((resolve) => {
      if (_multiModelTotalRemaining === 0) {
        resolve();
        return;
      }
      setMultiModelDoneResolve(resolve);
    });

    // Fire remaining companions in PARALLEL
    const remaining = companionModels.slice(1);
    if (remaining.length > 0) {
      setStreamBuffer(null);

      const mcpIds = usePreferenceStore.getState().enabledMcpServerIds;
      const thinkingBudget = getEffectiveThinkingBudget(conversationId);
      const kbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
      const memIds = usePreferenceStore.getState().enabledMemoryNamespaceIds;
      const wikiIds = usePreferenceStore.getState().enabledWikiIds;

      const invocations = remaining.map((model) =>
        invoke("regenerate_with_model", {
          conversationId,
          userMessageId: lastUserMsg.id,
          targetProviderId: model.providerId,
          targetModelId: model.model_id,
          enabledMcpServerIds: mcpIds.length > 0 ? mcpIds : undefined,
          thinkingBudget,
          enabledKnowledgeBaseIds: kbIds.length > 0 ? kbIds : undefined,
          enabledMemoryNamespaceIds: memIds.length > 0 ? memIds : undefined,
          enabledWikiIds: wikiIds.length > 0 ? wikiIds : undefined,
          isCompanion: true,
        }).then(async () => {
          if (!_isMultiModelActive) { return; }
          try {
            const versions = await convStore.listMessageVersions(conversationId, lastUserMsg.id);
            if (versions.length > 0 && _isMultiModelActive) {
              useConversationStore.setState((s) => {
                const existingIds = new Set(s.messages.map((m) => m.id));
                const dbVersionMap = new Map(versions.map((v) => [v.id, v]));

                const currentStreamingMessageId = useStreamStore.getState().streamingMessageId;
                let resolvedFirstModelId: string | null = null;
                if (currentStreamingMessageId?.startsWith("temp-") && _multiModelFirstModelId) {
                  const firstDbVersion = versions.find(
                    (v) => v.model_id === _multiModelFirstModelId && !existingIds.has(v.id),
                  );
                  if (firstDbVersion) {
                    resolvedFirstModelId = firstDbVersion.id;
                    existingIds.delete(currentStreamingMessageId);
                    existingIds.add(firstDbVersion.id);
                    useStreamStore.setState({ streamingMessageId: firstDbVersion.id });
                  }
                }

                const newVersions = versions
                  .filter((v) => !existingIds.has(v.id))
                  .map((v) => ({ ...v, is_active: false as const }));
                let enriched = false;
                const updatedMessages = s.messages.map((m) => {
                  if (resolvedFirstModelId && m.id === currentStreamingMessageId) {
                    const dbVersion = dbVersionMap.get(resolvedFirstModelId);
                    enriched = true;
                    return {
                      ...m,
                      id: resolvedFirstModelId,
                      model_id: dbVersion?.model_id ?? m.model_id,
                      provider_id: dbVersion?.provider_id ?? m.provider_id,
                    };
                  }
                  const dbVersion = dbVersionMap.get(m.id);
                  if (dbVersion && (!m.model_id || !m.provider_id)) {
                    enriched = true;
                    return { ...m, model_id: dbVersion.model_id, provider_id: dbVersion.provider_id };
                  }
                  return m;
                });
                if (newVersions.length === 0 && !enriched && resolvedFirstModelId === null) { return {}; }
                return { messages: [...updatedMessages, ...newVersions] };
              });
            }
          } catch (e) {
            console.warn("[sendMultiModelMessage] failed to enrich companion:", e);
          }
        }).catch((e) => {
          console.error(`[sendMultiModelMessage] companion ${model.model_id} invoke failed:`, e);
          decrementMultiModelTotalRemaining();
          if (_multiModelTotalRemaining <= 0 && _multiModelDoneResolve) {
            const r = _multiModelDoneResolve;
            setMultiModelDoneResolve(null);
            useStreamStore.setState((s) => ({
              ...stopConversationStream(s.activeStreams, conversationId),
              streamingStartTimestamps: (() => {
                const t = { ...s.streamingStartTimestamps };
                delete t[conversationId];
                return t;
              })(),
              thinkingActiveMessageIds: new Set<string>(),
            }));
            r();
          }
        })
      );

      void Promise.allSettled(invocations);
    }

    // Wait for ALL streams to complete
    await allDone;

    // All done — cleanup
    setIsMultiModelActive(false);
    setMultiModelFirstModelId(null);
    set({ pendingCompanionModels: [], multiModelDoneMessageIds: [] });

    // Restore original conversation model
    if (originalProviderId && originalModelId) {
      try {
        await convStore.updateConversation(conversationId, {
          provider_id: originalProviderId,
          model_id: originalModelId,
        });
      } catch (e) {
        console.error("[sendMultiModelMessage] failed to restore model:", e);
      }
    }

    // Final fetch for consistency
    if (useConversationStore.getState().activeConversationId === conversationId) {
      const parentId = get().multiModelParentId;

      const userSelectedMessageId = _userManuallySelectedVersion
        ? useConversationStore.getState().messages.find(
          (m) => m.parent_message_id === parentId && m.role === "assistant" && m.is_active,
        )?.id ?? null
        : null;

      if (parentId && !_userManuallySelectedVersion) {
        const firstModelId = companionModels[0].model_id;
        let targetMessageId = _multiModelFirstMessageId;
        if (!targetMessageId) {
          const localMatch = useConversationStore.getState().messages.find(
            (m) => m.parent_message_id === parentId && m.role === "assistant" && m.model_id === firstModelId,
          );
          targetMessageId = localMatch?.id ?? null;
        }
        if (targetMessageId) {
          await invoke("switch_message_version", {
            conversation_id: conversationId,
            parent_message_id: parentId,
            message_id: targetMessageId,
          }).catch((e: unknown) => {
            console.warn("[IPC]", e);
          });
        }
      } else if (parentId && userSelectedMessageId) {
        await invoke("switch_message_version", {
          conversation_id: conversationId,
          parent_message_id: parentId,
          message_id: userSelectedMessageId,
        }).catch((e: unknown) => {
          console.warn("[IPC]", e);
        });
      }

      await convStore.fetchMessages(conversationId);

      if (parentId) {
        const refreshedMsgs = useConversationStore.getState().messages;

        let displayVersion: typeof refreshedMsgs[number] | null = null;
        if (_userManuallySelectedVersion && userSelectedMessageId) {
          displayVersion = refreshedMsgs.find((m) => m.id === userSelectedMessageId) ?? null;
        }
        if (!displayVersion) {
          const firstModelId = companionModels[0].model_id;
          displayVersion = _multiModelFirstMessageId
            ? refreshedMsgs.find((m) => m.id === _multiModelFirstMessageId) ?? null
            : null;
          if (!displayVersion) {
            displayVersion = refreshedMsgs.find(
              (m) => m.parent_message_id === parentId && m.role === "assistant" && m.model_id === firstModelId,
            ) ?? null;
          }
        }

        if (displayVersion) {
          useConversationStore.setState((s) => {
            let kept = false;
            return {
              messages: s.messages.reduce<typeof s.messages>((acc, m) => {
                if (m.parent_message_id === parentId && m.role === "assistant") {
                  if (!kept) {
                    acc.push({ ...displayVersion, is_active: true });
                    kept = true;
                  }
                } else {
                  acc.push(m);
                }
                return acc;
              }, []),
            };
          });
        }
      }
    }

    setMultiModelFirstMessageId(null);
    setUserManuallySelectedVersion(false);
    set({ multiModelParentId: null, multiModelDoneMessageIds: [] });
  },
}));

// ─── 向后兼容：同步多模型状态到 conversationStore ───
// 消费者组件通过 useConversationStore 读取 multiModelParentId 等字段，
// 所以 multiModelStore 状态变化时需要同步写入 conversationStore。
useMultiModelStore.subscribe((state, prev) => {
  const updates: Record<string, unknown> = {};
  if (state.pendingCompanionModels !== prev.pendingCompanionModels) {
    updates.pendingCompanionModels = state.pendingCompanionModels;
  }
  if (state.multiModelParentId !== prev.multiModelParentId) {
    updates.multiModelParentId = state.multiModelParentId;
  }
  if (state.multiModelDoneMessageIds !== prev.multiModelDoneMessageIds) {
    updates.multiModelDoneMessageIds = state.multiModelDoneMessageIds;
  }
  if (state.pendingPromptText !== prev.pendingPromptText) {
    updates.pendingPromptText = state.pendingPromptText;
  }
  if (Object.keys(updates).length > 0) {
    useConversationStore.setState(updates as any);
  }
});
