import type { SkillReplacementAction } from "@/components/workflow/types";

/**
 * 节点级操作选择（同一节点可对应多个 existing skill 匹配）
 * 第一层 key: 节点 id；第二层 key: existing skill id
 */
export type SemanticActionMap = Record<string, Record<string, SkillReplacementAction>>;

export function setSemanticAction(
  prev: SemanticActionMap,
  nodeId: string,
  skillId: string,
  action: SkillReplacementAction,
): SemanticActionMap {
  const nodeActions = { ...(prev[nodeId] ?? {}) };
  nodeActions[skillId] = action;
  return { ...prev, [nodeId]: nodeActions };
}

export function clearSemanticAction(
  prev: SemanticActionMap,
  nodeId: string,
  skillId: string,
): SemanticActionMap {
  const nodeActions = { ...(prev[nodeId] ?? {}) };
  delete nodeActions[skillId];
  if (Object.keys(nodeActions).length === 0) {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const { [nodeId]: _removed, ...rest } = prev;
    return rest;
  }
  return { ...prev, [nodeId]: nodeActions };
}

export function isActionSelected(
  map: SemanticActionMap,
  nodeId: string,
  skillId: string,
  action: SkillReplacementAction,
): boolean {
  return map[nodeId]?.[skillId] === action;
}

/** 把 SemanticActionMap 摊平为 [nodeId, skillId, action] 三元组列表 */
export function flattenSemanticActions(
  map: SemanticActionMap,
): Array<{ nodeId: string; skillId: string; action: SkillReplacementAction }> {
  const out: Array<{ nodeId: string; skillId: string; action: SkillReplacementAction }> = [];
  for (const [nodeId, nodeMap] of Object.entries(map)) {
    for (const [skillId, action] of Object.entries(nodeMap)) {
      out.push({ nodeId, skillId, action });
    }
  }
  return out;
}
