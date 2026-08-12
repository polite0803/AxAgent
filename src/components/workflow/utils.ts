// SPDX-License-Identifier: AGPL-3.0-only

import type { WorkflowEdge, WorkflowNode } from "./types";

/**
 * 清理引用了不存在节点的无效边，防止脏数据持久化。
 *
 * 并发安全：纯函数，无副作用，可在 RAF 回调和 useEffect 中安全调用。
 */
export function getCleanedEdges(nodes: WorkflowNode[], edges: WorkflowEdge[]): WorkflowEdge[] {
  const nodeIdSet = new Set(nodes.map((n) => n.id));
  return edges.filter((e) => nodeIdSet.has(e.source) && nodeIdSet.has(e.target));
}

/**
 * 把 `parentRefs` 映射摊平到 nodes 的 `parentId` 字段。
 *
 * 后端 DTO 使用 `nodes[].parentId` 作为父子关系权威来源，
 * 前端运行时使用独立的 `parentRefs: Record<string,string>` 避免污染联合类型，
 * 保存前需要摊平。
 *
 * 并发安全：纯函数，返回新数组，不修改输入。
 */
export function buildNodesWithParent(
  nodes: WorkflowNode[],
  parentRefs: Record<string, string>,
): WorkflowNode[] {
  return nodes.map((n) => {
    const pid = parentRefs[n.id];
    if (pid === undefined) { return n; }
    return { ...n, parentId: pid } as WorkflowNode;
  });
}

/**
 * 从 nodes[].parentId 字段重建父子关系映射。
 *
 * 后端目前不感知 parentRefs，所以老工作流的父子关系以 nodes 字段为准持久化。
 */
export function rebuildParentRefsFromNodes(nodes: WorkflowNode[]): Record<string, string> {
  const refs: Record<string, string> = {};
  for (const n of nodes) {
    const scoped = n as unknown as { parentId?: string };
    const pid = scoped.parentId;
    if (typeof pid === "string" && pid.length > 0) {
      refs[n.id] = pid;
    }
  }
  return refs;
}

/**
 * 深克隆：优先 structuredClone，退到 JSON，再退到空对象。
 * 历史栈只需要基本数据结构，React Flow 节点中的组件引用不需要保留。
 */
export function safeClone<T>(value: T): T {
  try {
    return structuredClone(value);
  } catch (err) {
    try {
      return JSON.parse(JSON.stringify(value)) as T;
    } catch (jsonErr) {
      console.warn("[workflowUtils] history clone failed:", err, jsonErr);
      if (Array.isArray(value)) { return [] as unknown as T; }
      return {} as T;
    }
  }
}

/**
 * localStorage 安全读 — 带 try/catch 和类型守卫。
 */
export function safeLocalStorageGet<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) { return fallback; }
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

/**
 * localStorage 安全写 — 带 try/catch 和 quota 保护。
 */
export function safeLocalStorageSet(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // quota exceeded or private mode — 静默失败
  }
}
