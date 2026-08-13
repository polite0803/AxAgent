// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { useWorkflowEditorStore } from "@/stores";
import React, { useCallback, useEffect, useRef } from "react";
import { buildNodesWithParent, getCleanedEdges } from "../utils";

const MAX_AUTO_SAVE_RETRIES = 3;
const AUTO_SAVE_DELAY_MS = 5000;

/**
 * 封装工作流编辑器的自动保存逻辑。
 *
 * 并发安全：
 * 1. 保存前二次检查 `isDirty / isSaving / isDecompositionTemplate` 避免竞态
 *    — 因为 setTimeout 回调在未来执行，期间状态可能被用户操作改变
 * 2. 保存过程中若有新修改，`isSaving` 标志阻止重入
 * 3. 失败重试有上限，防止无限循环阻塞保存
 * 4. 组件卸载时清理 timer，避免对已卸载组件 setState
 */
export interface AutoSaveController {
  /** 手动触发一次保存（带竞态保护） */
  saveNow: () => Promise<void>;
  /** 重置重试计数（保存成功后调用） */
  resetRetryCount: () => void;
  /** 重试计数 ref（供外部访问，如 WorkflowEditor 中显示状态） */
  retryCountRef: React.MutableRefObject<number>;
}

export function useWorkflowAutoSave(): AutoSaveController {
  const retryCountRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resetRetryCount = useCallback(() => {
    retryCountRef.current = 0;
  }, []);

  const doSave = useCallback(async () => {
    const state = useWorkflowEditorStore.getState();
    if (!state.isDirty || state.isSaving || state.isDecompositionTemplate) {
      return;
    }

    const { nodes, edges, parentRefs, currentTemplate } = state;

    // 自动清理引用不存在节点的无效边，防止脏数据持久化
    const cleanedEdges = getCleanedEdges(nodes, edges);
    if (cleanedEdges.length !== edges.length) {
      useWorkflowEditorStore.setState({ edges: cleanedEdges });
    }

    // 装饰节点（_phaseSeparator / groupFrame）仅用于编辑器视觉分区，后端 WorkflowNode
    // 枚举未定义这些变体，反序列化会报 unknown_variant 错误。因此持久化前必须过滤，
    // 否则自动保存必失败。过滤后仍保留在编辑器中显示。
    const PERSISTED_NODE_TYPES = new Set([
      "trigger",
      "agent",
      "llm",
      "condition",
      "parallel",
      "loop",
      "merge",
      "delay",
      "validation",
      "subWorkflow",
      "documentParser",
      "vectorRetrieve",
      "httpRequest",
      "switch",
      "databaseQuery",
      "notification",
      "approval",
      "fileOperation",
      "dataTransformer",
      "webhookSend",
      "logging",
      "llmClassifier",
      "aggregator",
      "email",
      "debate",
      "swarm",
      "multiAgent",
      "storage",
      "workflowRef",
      "end",
      "tool",
      "code",
    ]);

    const nodesWithParent = buildNodesWithParent(nodes, parentRefs)
      .filter((n) => PERSISTED_NODE_TYPES.has(n.type));
    const input = {
      name: currentTemplate?.name || "Unnamed Workflow",
      description: currentTemplate?.description,
      icon: currentTemplate?.icon || "Bot",
      tags: currentTemplate?.tags || [],
      trigger_config: currentTemplate?.trigger_config,
      nodes: nodesWithParent,
      edges: cleanedEdges,
      input_schema: currentTemplate?.input_schema,
      output_schema: currentTemplate?.output_schema,
      variables: currentTemplate?.variables || [],
      error_config: currentTemplate?.error_config,
    };

    try {
      if (currentTemplate?.id) {
        await invoke<boolean>("update_workflow_template", { id: currentTemplate.id, input });
        useWorkflowEditorStore.setState({ isDirty: false, isSaving: false });
      } else {
        const newId = await invoke<string>("create_workflow_template", { input });
        if (newId) {
          useWorkflowEditorStore.setState({ isDirty: false, isSaving: false });
        }
      }
      retryCountRef.current = 0;
    } catch (e) {
      logIpcError("WorkflowEditor: auto-save")(e);
      retryCountRef.current++;
      if (retryCountRef.current >= MAX_AUTO_SAVE_RETRIES) {
        useWorkflowEditorStore.setState({ error: "workflow.autoSave.failed" });
        retryCountRef.current = 0;
      }
    }
  }, []);

  const saveNow = useCallback(async () => {
    await doSave();
  }, [doSave]);

  // 通过选择器订阅 isDirty，仅在脏状态变化（而非每次渲染）时调度自动保存。
  // 之前效果无依赖数组、且用 getState() 读取，导致每次渲染都重建 timer。
  const isDirty = useWorkflowEditorStore((s) => s.isDirty);

  useEffect(() => {
    if (!isDirty) { return; }

    timerRef.current = setTimeout(async () => {
      await doSave();
    }, AUTO_SAVE_DELAY_MS);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [isDirty, doSave]);

  return { saveNow, resetRetryCount, retryCountRef };
}
