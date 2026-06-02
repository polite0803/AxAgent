import { invoke } from "@/lib/invoke";
import { useProviderStore, useWorkflowEditorStore } from "@/stores";
import { useCallback, useRef, useState } from "react";

export interface NodeAIAssistContext {
  nodeId: string;
  nodeType: string;
  nodeTitle?: string;
  upstreamVariables?: string[];
  upstreamNodes?: Array<{ id: string; type: string; title: string }>;
  downstreamNodes?: Array<{ id: string; type: string; title: string }>;
}

export interface NodeAIAssistOptions {
  systemPrompt?: string;
  userPrompt: string;
  silentIfNoProvider?: boolean;
  beforeSend?: () => void;
  context?: NodeAIAssistContext;
  transactional?: boolean;
}

export interface NodeAIAssistResult {
  generate: (options: NodeAIAssistOptions) => Promise<string | null>;
  generating: boolean;
  error: string | null;
  lastResult: string | null;
  reset: () => void;
  rollbackLast: () => void;
}

/**
 * 节点级 AI 辅助 hook：复用 send_message 调用模式。
 * 负责 provider 查找、invoke 调用、加载/错误态，调用方拿到 content 后自行决定如何应用。
 * 自动注入 RAG 上下文（上下游节点/变量）以提升生成质量。
 */
export function useNodeAIAssist(): NodeAIAssistResult {
  const { providers } = useProviderStore();
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<string | null>(null);
  const txIdRef = useRef<string | null>(null);

  const generate = useCallback(
    async (options: NodeAIAssistOptions): Promise<string | null> => {
      const { systemPrompt, userPrompt, silentIfNoProvider, beforeSend, context, transactional = true } = options;

      const provider = providers.find((p) => p.enabled && p.models.some((m) => m.enabled));
      const model = provider?.models.find((m) => m.enabled);
      if (!provider || !model) {
        const msg = "no_provider";
        if (!silentIfNoProvider) {
          setError(msg);
        }
        return null;
      }

      const enrichedUserPrompt = context ? buildRagPrompt(userPrompt, context) : userPrompt;

      if (transactional) {
        txIdRef.current = useWorkflowEditorStore.getState().beginAiActionTransaction();
      }

      setError(null);
      setGenerating(true);
      beforeSend?.();
      try {
        const result = await invoke<{ content: string }>("send_message", {
          params: {
            conversationId: "",
            content: enrichedUserPrompt,
            attachments: [],
            options: {
              ...(systemPrompt ? { system_prompt: systemPrompt } : {}),
            },
          },
        });
        const content = result?.content?.trim() ?? "";
        if (content) {
          setLastResult(content);
          if (transactional && txIdRef.current) {
            useWorkflowEditorStore.getState().commitAiActionTransaction(txIdRef.current);
            txIdRef.current = null;
          }
        } else {
          if (transactional && txIdRef.current) {
            useWorkflowEditorStore.getState().rollbackAiActionTransaction(txIdRef.current);
            txIdRef.current = null;
          }
        }
        return content || null;
      } catch (e) {
        setError(String(e));
        if (transactional && txIdRef.current) {
          useWorkflowEditorStore.getState().rollbackAiActionTransaction(txIdRef.current);
          txIdRef.current = null;
        }
        return null;
      } finally {
        setGenerating(false);
      }
    },
    [providers],
  );

  const reset = useCallback(() => {
    setError(null);
    setLastResult(null);
  }, []);

  const rollbackLast = useCallback(() => {
    useWorkflowEditorStore.getState().rollbackLastAiActionTransaction();
  }, []);

  return { generate, generating, error, lastResult, reset, rollbackLast };
}

/**
 * 从当前工作流画布中收集"当前节点"的上下游摘要，拼成 LLM 可用的 RAG 上下文。
 * 放在 hook 内部自动取 store，避免调用方每次都传。
 */
export function useNodeAIContext(nodeId: string, nodeType: string, nodeTitle?: string): NodeAIAssistContext {
  const { nodes, edges } = useWorkflowEditorStore();

  const ctx: NodeAIAssistContext = { nodeId, nodeType, nodeTitle };
  const upstream: Array<{ id: string; type: string; title: string }> = [];
  const downstream: Array<{ id: string; type: string; title: string }> = [];
  const vars = new Set<string>();

  for (const e of edges) {
    if (e.target === nodeId) {
      const u = nodes.find((n) => n.id === e.source);
      if (u) {
        upstream.push({ id: u.id, type: u.type, title: u.title || u.id });
        if (u.title) { vars.add(`\${${u.id}.output}`); }
      }
    }
    if (e.source === nodeId) {
      const d = nodes.find((n) => n.id === e.target);
      if (d) {
        downstream.push({ id: d.id, type: d.type, title: d.title || d.id });
      }
    }
  }
  ctx.upstreamNodes = upstream.slice(0, 8);
  ctx.downstreamNodes = downstream.slice(0, 8);
  ctx.upstreamVariables = Array.from(vars).slice(0, 8);
  return ctx;
}

/**
 * 把 RAG 上下文格式化为前置段落，拼到 userPrompt 顶部。
 * 极简格式以节省 token，但保证 LLM 知道节点位置、上下游链路、可用变量。
 */
function buildRagPrompt(userPrompt: string, ctx: NodeAIAssistContext): string {
  const parts: string[] = [];
  parts.push(`[Workflow Context] node=${ctx.nodeType} "${ctx.nodeTitle ?? ctx.nodeId}" (id=${ctx.nodeId})`);
  if (ctx.upstreamNodes && ctx.upstreamNodes.length > 0) {
    parts.push(`Upstream: ${ctx.upstreamNodes.map((n) => `${n.type}:"${n.title}"#${n.id}`).join(", ")}`);
  }
  if (ctx.downstreamNodes && ctx.downstreamNodes.length > 0) {
    parts.push(`Downstream: ${ctx.downstreamNodes.map((n) => `${n.type}:"${n.title}"#${n.id}`).join(", ")}`);
  }
  if (ctx.upstreamVariables && ctx.upstreamVariables.length > 0) {
    parts.push(`Available vars: ${ctx.upstreamVariables.join(", ")}`);
  }
  parts.push("");
  parts.push(userPrompt);
  return parts.join("\n");
}
