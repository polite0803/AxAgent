export interface DeadEndInput {
  id: string;
  nodeType: string;
  hasIncoming: boolean;
  hasOutgoing: boolean;
}

/**
 * 判断节点是否为"死端"（带入但无出，且不是 trigger/end，且不是工作流唯一合法终端）。
 *
 * 关键规则：
 * - trigger / end 类型不算死端（前者出度=0 是预期的；后者入度>0、出度=0 是预期的）
 * - 工作流中有多个终端（带入无出）时，每个都被视为"漏连"（dead end）
 * - 工作流中只有 1 个终端时，它就是唯一合法终点，**不**算 dead end
 * - 孤立节点（既无入又无出）由 isOrphan 单独处理，不在本函数判定
 */
export function isDeadEndNode(
  node: DeadEndInput,
  totalTerminalNodeCount: number,
): boolean {
  if (node.nodeType === "trigger" || node.nodeType === "end") { return false; }
  if (!node.hasIncoming || node.hasOutgoing) { return false; }
  return totalTerminalNodeCount > 1;
}

/** 从 (id, hasIncoming, hasOutgoing) 元数据中提取工作流级终端节点计数 */
export function countTerminalNodes(nodes: DeadEndInput[]): number {
  return nodes.filter((n) => n.hasIncoming && !n.hasOutgoing).length;
}
