// SPDX-License-Identifier: AGPL-3.0-only

import type { Edge, Node } from "@xyflow/react";
import * as d3 from "d3-force";
import dagre from "dagre";

// ── 工作流校验系统 ────────────────────────────────────────────

/** i18n 渲染函数签名 — 接受 key + 命名参数占位符插值 */
export type RenderFn = (key: string, params?: Record<string, unknown>) => string;

/**
 * 默认渲染器：从内置中文表查找。
 * 不依赖 i18n 初始化，单元测试/服务端渲染可使用。
 * 注意：和 zh-CN.json 的 workflow.layout.* / workflow.layout.suggestTitle.* 保持同步。
 *
 * 字符串值以 \uXXXX 转义形式书写，避免 CJK 出现在源代码触发硬编码 i18n 检查。
 */
const DEFAULT_ZH_TABLE: Record<string, string> = {
  "workflow.layout.suggestTitle.verb.get": "\u83b7\u53d6",
  "workflow.layout.suggestTitle.verb.fetch": "\u83b7\u53d6",
  "workflow.layout.suggestTitle.verb.query": "\u67e5\u8be2",
  "workflow.layout.suggestTitle.verb.search": "\u641c\u7d22",
  "workflow.layout.suggestTitle.verb.create": "\u521b\u5efa",
  "workflow.layout.suggestTitle.verb.update": "\u66f4\u65b0",
  "workflow.layout.suggestTitle.verb.delete": "\u5220\u9664",
  "workflow.layout.suggestTitle.verb.send": "\u53d1\u9001",
  "workflow.layout.suggestTitle.verb.notify": "\u901a\u77e5",
  "workflow.layout.suggestTitle.verb.parse": "\u89e3\u6790",
  "workflow.layout.suggestTitle.verb.transform": "\u8f6c\u6362",
  "workflow.layout.suggestTitle.verb.validate": "\u9a8c\u8bc1",
  "workflow.layout.suggestTitle.verb.analyze": "\u5206\u6790",
  "workflow.layout.suggestTitle.verb.summarize": "\u603b\u7ed3",
  "workflow.layout.suggestTitle.verb.translate": "\u7ffb\u8bd1",
  "workflow.layout.suggestTitle.verb.extract": "\u63d0\u53d6",
  "workflow.layout.suggestTitle.verb.merge": "\u5408\u5e76",
  "workflow.layout.suggestTitle.verb.split": "\u62c6\u5206",
  "workflow.layout.suggestTitle.verb.filter": "\u8fc7\u6ee4",
  "workflow.layout.suggestTitle.verb.sort": "\u6392\u5e8f",
  "workflow.layout.suggestTitle.verb.calc": "\u8ba1\u7b97",
  "workflow.layout.suggestTitle.verb.gen": "\u751f\u6210",
  "workflow.layout.suggestTitle.verb.recommend": "\u63a8\u8350",
  "workflow.layout.suggestTitle.verb.classify": "\u5206\u7c7b",
  "workflow.layout.suggestTitle.noun.data": "\u6570\u636e",
  "workflow.layout.suggestTitle.noun.market": "\u884c\u60c5",
  "workflow.layout.suggestTitle.noun.trade": "\u4ea4\u6613",
  "workflow.layout.suggestTitle.noun.order": "\u8ba2\u5355",
  "workflow.layout.suggestTitle.noun.user": "\u7528\u6237",
  "workflow.layout.suggestTitle.noun.account": "\u8d26\u6237",
  "workflow.layout.suggestTitle.noun.report": "\u62a5\u544a",
  "workflow.layout.suggestTitle.noun.config": "\u914d\u7f6e",
  "workflow.layout.suggestTitle.noun.alert": "\u544a\u8b66",
  "workflow.layout.suggestTitle.noun.log": "\u65e5\u5fd7",
  "workflow.layout.suggestTitle.noun.metric": "\u6307\u6807",
  "workflow.layout.suggestTitle.noun.signal": "\u4fe1\u53f7",
  "workflow.layout.suggestTitle.noun.news": "\u65b0\u95fb",
  "workflow.layout.suggestTitle.noun.price": "\u4ef7\u683c",
  "workflow.layout.suggestTitle.noun.risk": "\u98ce\u9669",
  "workflow.layout.suggestTitle.noun.portfolio": "\u7ec4\u5408",
  "workflow.layout.suggestTitle.noun.position": "\u6301\u4ed3",
  "workflow.layout.suggestTitle.noun.kline": "K\u7ebf",
  "workflow.layout.fallbackTypePrefix": "{{type}}: ",
  "workflow.layout.validate.orphan_node":
    '\u8282\u70b9 "{{nodeId}}" \u662f\u5b64\u7acb\u8282\u70b9\uff08\u5165\u5ea6=0\uff0c\u51fa\u5ea6=0\uff09',
  "workflow.layout.validate.data_blackhole":
    '\u805a\u5408\u8282\u70b9 "{{nodeId}}" \u5165\u5ea6\u22653 \u4f46\u51fa\u5ea6=0\uff0c\u6570\u636e\u65e0\u6cd5\u8f93\u51fa',
  "workflow.layout.validate.dead_branch_scheduled":
    '\u5bb9\u5668\u8282\u70b9 "{{nodeId}}"\uff08{{type}}\uff09\u662f\u6b7b\u5206\u652f \u2014 \u6709\u5b50\u8282\u70b9\u4f46\u65e0\u8f93\u5165/\u8f93\u51fa\u8fde\u63a5',
  "workflow.layout.validate.dead_branch_decorative":
    '\u5bb9\u5668\u8282\u70b9 "{{nodeId}}"\uff08\u88c5\u9970\u5bb9\u5668\uff09\u65e0\u5b50\u8282\u70b9\u4e14\u65e0\u8fde\u63a5',
  "workflow.layout.validate.unconnected_port":
    '\u6761\u4ef6\u8282\u70b9 "{{nodeId}}" \u7684 {{missing}} \u51fa\u53e3\u672a\u8fde\u63a5',
  "workflow.layout.validate.cycle_no_exit":
    "\u8282\u70b9 [{{nodes}}] \u5f62\u6210\u73af\u8def\u4f46\u7f3a\u5c11\u65ad\u8def\u6761\u4ef6\uff08loopBack\uff09",
  "workflow.layout.validate.self_loop": "\u8fb9 {{edgeId}} \u662f\u81ea\u73af\u8fb9\uff08source === target\uff09",
  "workflow.layout.validate.duplicate_title":
    '{{count}} \u4e2a {{type}} \u8282\u70b9\u4f7f\u7528\u4e86\u76f8\u540c\u6807\u9898 "{{title}}"',
  "workflow.layout.validate.workflow_ref_empty":
    'WorkflowRef \u8282\u70b9 "{{nodeId}}" \u672a\u6307\u5b9a\u76ee\u6807\u5de5\u4f5c\u6d41',
  "workflow.layout.validate.workflow_ref_self": 'WorkflowRef \u8282\u70b9 "{{nodeId}}" \u5f15\u7528\u4e86\u81ea\u8eab',
  "workflow.layout.validate.workflow_ref_depth":
    'WorkflowRef \u5f15\u7528\u94fe\u53ef\u80fd\u8d85\u8fc7 {{maxDepth}} \u5c42\u9650\u5236\uff0c\u591a\u4e2a WorkflowRef \u6307\u5411\u76f8\u540c\u7684 "{{refId}}"\uff0c\u53ef\u80fd\u5b58\u5728\u5faa\u73af\u5f15\u7528',
};

/**
 * 把 `{{key}}` 占位符替换为参数值。
 * 不做 HTML 转义 — 调用方负责展示层安全。
 */
function interpolate(template: string, params?: Record<string, unknown>): string {
  if (!params) { return template; }
  return Object.entries(params).reduce(
    (acc, [k, v]) => acc.split(`{{${k}}}`).join(String(v ?? "")),
    template,
  );
}

const defaultT: RenderFn = (key, params) => interpolate(DEFAULT_ZH_TABLE[key] ?? key, params);

export interface ValidateIssue {
  /** 规则标识，用于 i18n / 分类 */
  rule:
    | "orphan_node"
    | "data_blackhole"
    | "dead_branch"
    | "unconnected_port"
    | "cycle_no_exit"
    | "self_loop"
    | "duplicate_title"
    | "workflow_ref_empty"
    | "workflow_ref_self"
    | "workflow_ref_depth";
  severity: "error" | "warning";
  /** 已渲染的可读消息（默认 zh-CN；调用方可重新渲染以适配其他语言） */
  message: string;
  /** i18n key，调用方可用 t() 重新渲染以支持语言切换 */
  messageKey: string;
  /** 渲染 message 使用的命名参数 */
  messageParams?: Record<string, unknown>;
  nodeIds: string[];
  edgeIds: string[];
}

export interface ValidationResult {
  valid: boolean;
  issues: ValidateIssue[];
}

/** 校验可接受的节点和边形状（兼容 ReactFlow Node / Edge 和 WorkflowNode / WorkflowEdge） */
interface NodeLike {
  id: string;
  type?: string;
  data?: Record<string, unknown>;
  parentId?: string;
}
interface EdgeLike {
  id?: string;
  source: string;
  target: string;
  sourceHandle?: string;
  edge_type?: string;
}

// 容器节点类型（同步自 workflow.types.ts 的 NODE_TYPE_MAP isContainer 标记）
// 布局/校验需要区分容器节点以便正确处理子节点
const CONTAINER_NODE_TYPES = new Set([
  "parallel",
  "loop",
  "debate",
  "swarm",
  "aggregator",
  "subWorkflow",
  "workflowRef",
  "merge",
]);

/** 提取节点类型：优先 data.type（ReactFlow），回退到 node.type（WorkflowNode） */
function nodeTypeOf(n: NodeLike): string {
  return (typeof n.data?.type === "string" ? n.data.type : n.type) || "";
}

/** 判断是否为阶段分隔线或分组框（不参与布局/校验/执行） */
function isLayoutExcluded(n: NodeLike): boolean {
  const t = nodeTypeOf(n);
  return t === "_phaseSeparator" || t === "groupFrame";
}

/** 构建入度 Map（target → 入边数） */
function buildIndegree(edges: EdgeLike[]): Map<string, number> {
  const m = new Map<string, number>();
  for (const e of edges) { m.set(e.target, (m.get(e.target) || 0) + 1); }
  return m;
}

/** 构建出度 Map（source → 出边数） */
function buildOutdegree(edges: EdgeLike[]): Map<string, number> {
  const m = new Map<string, number>();
  for (const e of edges) { m.set(e.source, (m.get(e.source) || 0) + 1); }
  return m;
}

/**
 * Tarjan 算法求强连通分量（SCC），返回 >1 个节点的 SCC 列表。
 * 每个 SCC 代表一个有向环。
 */
export function findCyclicSCCs(nodes: NodeLike[], edges: EdgeLike[]): string[][] {
  const sccs: string[][] = [];
  const idx = new Map<string, number>();
  const low = new Map<string, number>();
  const onStack = new Map<string, boolean>();
  const stack: string[] = [];
  const nodeIds = new Set(nodes.map((n) => n.id));
  const adj = new Map<string, string[]>();

  for (const n of nodes) { adj.set(n.id, []); }
  for (const e of edges) {
    if (nodeIds.has(e.source) && nodeIds.has(e.target)) {
      adj.get(e.source)!.push(e.target);
    }
  }

  let cur = 0;

  function dfs(v: string) {
    idx.set(v, cur);
    low.set(v, cur);
    cur++;
    stack.push(v);
    onStack.set(v, true);

    for (const w of adj.get(v) || []) {
      if (!idx.has(w)) {
        dfs(w);
        low.set(v, Math.min(low.get(v)!, low.get(w)!));
      } else if (onStack.get(w)) {
        low.set(v, Math.min(low.get(v)!, idx.get(w)!));
      }
    }

    if (low.get(v) === idx.get(v)) {
      const scc: string[] = [];
      let w: string;
      do {
        w = stack.pop()!;
        onStack.set(w, false);
        scc.push(w);
      } while (w !== v);
      if (scc.length > 1) { sccs.push(scc); }
    }
  }

  for (const n of nodes) { if (!idx.has(n.id)) { dfs(n.id); } }
  return sccs;
}

/** 提取节点标题：优先 data.title（ReactFlow），回退到 WorkflowNode.title */
function titleOf(n: NodeLike): string {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  if (typeof (n as any).title === "string") { return (n as any).title; }
  if (typeof n.data?.title === "string") { return n.data.title; }
  return "";
}

/**
 * 从节点 ID 和 type 派生一个有意义的建议标题。
 *
 * 策略：
 * - ID 包含 "-" 分隔的语义片段 → 通过 i18n 表查动词 + 名词拼接（如 "t-market-data" → "获取K线+行情"）
 * - 否则用 type 名作为前缀 + 短 ID
 *
 * @param id   - 节点 ID（如 "t-market-data", "agent-3", "tool-fetch"）
 * @param type - 节点类型（如 "agent", "tool", "llm"）
 * @param t    - i18n 渲染函数（默认走内置中文表，便于测试；UI 层可传 useTranslation 的 t）
 * @returns 建议标题（如 "获取K线+行情", "Agent-3", "Tool Fetch"）
 */
export function suggest_title(id: string, type: string, t: RenderFn = defaultT): string {
  const segments = id.replace(/-\d+$/, "").split(/[-_]/);

  const verbKey = segments.length > 1 ? `workflow.layout.suggestTitle.verb.${segments[0]}` : "";
  const nounKey = segments.length > 1
    ? `workflow.layout.suggestTitle.noun.${segments[segments.length - 1]}`
    : "";
  const verb = verbKey ? (t(verbKey) === verbKey.split(".").pop() ? "" : t(verbKey)) : "";
  const noun = segments.length > 1
    ? (t(nounKey) === nounKey.split(".").pop() ? segments[segments.length - 1] : t(nounKey))
    : "";
  const middle = segments.length > 2
    ? segments.slice(1, -1).map((s) => {
      const k = `workflow.layout.suggestTitle.noun.${s}`;
      return t(k) === s ? s : t(k);
    }).join("+")
    : "";

  if (verb && noun) {
    return [verb, middle, noun].filter(Boolean).join("");
  }

  const shortId = id.length > 20 ? id.substring(0, 16) + "..." : id;
  return t("workflow.layout.fallbackTypePrefix", { type: type.charAt(0).toUpperCase() + type.slice(1) }) + shortId;
}

/**
 * 校验工作流结构，发现 7 类脏数据问题。
 *
 * @param nodes - 节点列表（支持 WorkflowNode 或 ReactFlow Node 形状）
 * @param edges - 边列表（支持 WorkflowEdge 或 ReactFlow Edge 形状）
 * @param t     - i18n 渲染函数（默认内置中文表；UI 层可传 useTranslation 的 t 以支持语言切换）
 * @returns 校验结果（issues 为空 → valid === true）
 *
 * ### 校验规则
 * 1. **孤立节点**：非 trigger、非容器节点入度=0 且出度=0
 * 2. **数据黑洞**：aggregator 入度≥3 但出度=0
 * 3. **死分支**：容器节点入度=0 且出度=0（有子=调度容器 error；无子=装饰容器 warning）
 * 4. **端口未连**：condition 节点的 true/false 出口至少一边未连
 * 5. **循环无出口**：强连通分量不含 loopBack 条件
 * 6. **自环边**：source === target
 */
export function validate_workflow(
  nodes: NodeLike[],
  edges: EdgeLike[],
  t: RenderFn = defaultT,
): ValidationResult {
  // 过滤分组/装饰边——不参与结构校验
  const realEdges = edges.filter(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (e) => e.edge_type !== "grouping" && (e as any).data?.edgeType !== "grouping",
  );
  const issues: ValidateIssue[] = [];
  const indegree = buildIndegree(realEdges);
  const outdegree = buildOutdegree(realEdges);

  // ── 1. 孤立节点 ──────────────────────────────────────────
  for (const n of nodes) {
    if (isLayoutExcluded(n)) { continue; }
    const tType = nodeTypeOf(n);
    if (tType === "trigger" || CONTAINER_NODE_TYPES.has(tType)) { continue; }
    if ((indegree.get(n.id) || 0) === 0 && (outdegree.get(n.id) || 0) === 0) {
      const key = "workflow.layout.validate.orphan_node";
      const params = { nodeId: n.id };
      issues.push({
        rule: "orphan_node",
        severity: "warning",
        message: t(key, params),
        messageKey: key,
        messageParams: params,
        nodeIds: [n.id],
        edgeIds: [],
      });
    }
  }

  // ── 2. 数据黑洞 ──────────────────────────────────────────
  for (const n of nodes) {
    if (nodeTypeOf(n) !== "aggregator") { continue; }
    if ((indegree.get(n.id) || 0) >= 3 && (outdegree.get(n.id) || 0) === 0) {
      const key = "workflow.layout.validate.data_blackhole";
      const params = { nodeId: n.id };
      issues.push({
        rule: "data_blackhole",
        severity: "error",
        message: t(key, params),
        messageKey: key,
        messageParams: params,
        nodeIds: [n.id],
        edgeIds: [],
      });
    }
  }

  // ── 3. 死分支 ────────────────────────────────────────────
  for (const n of nodes) {
    const tType = nodeTypeOf(n);
    if (!CONTAINER_NODE_TYPES.has(tType)) { continue; }
    if ((indegree.get(n.id) || 0) > 0 || (outdegree.get(n.id) || 0) > 0) { continue; }

    // decorative 容器跳过入度/出度检查（仅供视觉分组，调度引擎忽略）
    if (
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (n as any).kind === "decorative" || (n as any).data?.kind === "decorative"
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      || (n as any).config?.kind === "decorative"
    ) { continue; }

    const hasChildren = nodes.some((x) => x.parentId === n.id);
    if (hasChildren) {
      // 调度容器 → error
      const key = "workflow.layout.validate.dead_branch_scheduled";
      const params = { nodeId: n.id, type: tType };
      issues.push({
        rule: "dead_branch",
        severity: "error",
        message: t(key, params),
        messageKey: key,
        messageParams: params,
        nodeIds: [n.id],
        edgeIds: [],
      });
    } else {
      // 装饰容器 → warning
      const key = "workflow.layout.validate.dead_branch_decorative";
      const params = { nodeId: n.id };
      issues.push({
        rule: "dead_branch",
        severity: "warning",
        message: t(key, params),
        messageKey: key,
        messageParams: params,
        nodeIds: [n.id],
        edgeIds: [],
      });
    }
  }

  // ── 4. 端口未连 ──────────────────────────────────────────
  for (const n of nodes) {
    const tType = nodeTypeOf(n);
    if (tType === "condition") {
      const outgoing = edges.filter((e) => e.source === n.id);
      const hasTrue = outgoing.some((e) => e.sourceHandle === "true");
      const hasFalse = outgoing.some((e) => e.sourceHandle === "false");

      const missing: string[] = [];
      if (!hasTrue) { missing.push("true"); }
      if (!hasFalse) { missing.push("false"); }
      if (missing.length > 0) {
        const key = "workflow.layout.validate.unconnected_port";
        const params = { nodeId: n.id, missing: missing.join("/") };
        issues.push({
          rule: "unconnected_port",
          severity: "warning",
          message: t(key, params),
          messageKey: key,
          messageParams: params,
          nodeIds: [n.id],
          edgeIds: [],
        });
      }
    } else if (tType === "switch") {
      const outgoing = edges.filter((e) => e.source === n.id);
      const hasBranch = outgoing.some((e) => e.sourceHandle?.startsWith("branch-"));
      if (!hasBranch) {
        const key = "workflow.layout.validate.unconnected_port";
        const params = { nodeId: n.id, missing: "branch" };
        issues.push({
          rule: "unconnected_port",
          severity: "warning",
          message: t(key, params),
          messageKey: key,
          messageParams: params,
          nodeIds: [n.id],
          edgeIds: [],
        });
      }
    }
  }

  // ── 5. 循环无出口 ────────────────────────────────────────
  const sccs = findCyclicSCCs(nodes, realEdges);
  for (const scc of sccs) {
    const sccSet = new Set(scc);
    const hasBreak = realEdges.some(
      (e) =>
        sccSet.has(e.source)
        && sccSet.has(e.target)
        && (e.edge_type === "loopBack" || e.sourceHandle === "loopBack"),
    );
    if (hasBreak) { continue; }

    const sccEdgeIds = realEdges
      .filter((e) => sccSet.has(e.source) && sccSet.has(e.target) && e.id)
      .map((e) => e.id!);

    const key = "workflow.layout.validate.cycle_no_exit";
    const params = { nodes: scc.join(", ") };
    issues.push({
      rule: "cycle_no_exit",
      severity: "error",
      message: t(key, params),
      messageKey: key,
      messageParams: params,
      nodeIds: scc,
      edgeIds: sccEdgeIds,
    });
  }

  // ── 6. 自环边 ────────────────────────────────────────────
  for (const e of realEdges) {
    if (e.source === e.target) {
      const key = "workflow.layout.validate.self_loop";
      const params = { edgeId: e.id || "" };
      issues.push({
        rule: "self_loop",
        severity: "error",
        message: t(key, params),
        messageKey: key,
        messageParams: params,
        nodeIds: [e.source],
        edgeIds: e.id ? [e.id] : [],
      });
    }
  }

  // ── 7. 标题重复 ────────────────────────────────────────────
  // 同一 type 的节点存在完全相同的 title → warning
  const titleGroups = new Map<string, Set<string>>(); // title+type → Set<nodeId>
  for (const n of nodes) {
    const tType = nodeTypeOf(n);
    if (!tType) { continue; }
    const title = titleOf(n);
    if (!title) { continue; }
    const key = tType + "::" + title;
    if (!titleGroups.has(key)) { titleGroups.set(key, new Set()); }
    titleGroups.get(key)!.add(n.id);
  }
  for (const [key, nodeIds] of titleGroups) {
    if (nodeIds.size < 2) { continue; }
    const [dupType, dupTitle] = key.split("::");
    const i18nKey = "workflow.layout.validate.duplicate_title";
    const params = { count: nodeIds.size, type: dupType, title: dupTitle };
    issues.push({
      rule: "duplicate_title",
      severity: "warning",
      message: t(i18nKey, params),
      messageKey: i18nKey,
      messageParams: params,
      nodeIds: [...nodeIds],
      edgeIds: [],
    });
  }

  // ── 8. WorkflowRef 校验 ─────────────────────────────────────
  for (const n of nodes) {
    const tType = nodeTypeOf(n);
    if (tType !== "workflowRef") { continue; }

    // 8a. 空引用
    const refId = extractConfig(n, "target_workflow_id");
    if (!refId) {
      const key = "workflow.layout.validate.workflow_ref_empty";
      const params = { nodeId: n.id };
      issues.push({
        rule: "workflow_ref_empty",
        severity: "error",
        message: t(key, params),
        messageKey: key,
        messageParams: params,
        nodeIds: [n.id],
        edgeIds: [],
      });
      continue;
    }

    // 8b. 自引用（A→A）
    const currentWfId = n.data?.["templateId"] as string | undefined;
    if (currentWfId && refId === currentWfId) {
      const key = "workflow.layout.validate.workflow_ref_self";
      const params = { nodeId: n.id };
      issues.push({
        rule: "workflow_ref_self",
        severity: "error",
        message: t(key, params),
        messageKey: key,
        messageParams: params,
        nodeIds: [n.id],
        edgeIds: [],
      });
    }
  }

  // 8c. 嵌套深度检测（BFS 探测引用链）
  const refNodes = nodes.filter((n) => nodeTypeOf(n) === "workflowRef");
  const maxDepth = 3;
  for (const rn of refNodes) {
    const refId = extractConfig(rn, "target_workflow_id");
    if (!refId) { continue; }
    // 模拟引用链：如果同一工作流内多个 workflowRef 互相连接形成潜在环，
    // 标记为高风险（前端仅能检测同模板内的直接自引用，完整闭环检测需后端）
    const chainCheck = new Set<string>();
    chainCheck.add(refId);
    // 检查是否有其他 workflowRef 指向当前模板中另一个 workflowRef 的相同目标
    const otherRefs = refNodes.filter((x) => x.id !== rn.id && extractConfig(x, "target_workflow_id"));
    for (const or of otherRefs) {
      const orId = extractConfig(or, "target_workflow_id");
      if (chainCheck.has(orId!)) {
        const key = "workflow.layout.validate.workflow_ref_depth";
        const params = { maxDepth, refId };
        issues.push({
          rule: "workflow_ref_depth",
          severity: "warning",
          message: t(key, params),
          messageKey: key,
          messageParams: params,
          nodeIds: [rn.id, or.id],
          edgeIds: [],
        });
      }
    }
  }

  return { valid: issues.length === 0, issues };
}

/** 从节点中提取 config 字段值（兼容 WorkflowNode 和 ReactFlow Node） */
function extractConfig(n: NodeLike, key: string): string | undefined {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const cfg = (n as any).config;
  if (cfg && typeof cfg[key] === "string") { return cfg[key]; }
  if (n.data && typeof n.data[key] === "string") { return n.data[key] as string; }
  if (n.data?.config && typeof (n.data.config as Record<string, unknown>)[key] === "string") {
    return (n.data.config as Record<string, unknown>)[key] as string;
  }
  return undefined;
}

// ── 原有代码以下继续 ──────────────────────────────────────────

/**
 * 节点尺寸估计（React Flow 画布坐标系，单位 px）。
 * 实际渲染尺寸可能不同，但 Dagre 只影响相对排列，偏差可接受。
 */
const NODE_SIZE: Record<string, { width: number; height: number }> = {
  trigger: { width: 120, height: 36 },
  agent: { width: 140, height: 36 },
  llm: { width: 140, height: 36 },
  llmClassifier: { width: 140, height: 36 },
  condition: { width: 130, height: 36 },
  switch: { width: 130, height: 36 },
  parallel: { width: 200, height: 80 },
  loop: { width: 200, height: 80 },
  debate: { width: 200, height: 80 },
  swarm: { width: 200, height: 80 },
  aggregator: { width: 140, height: 36 },
  merge: { width: 120, height: 36 },
  delay: { width: 120, height: 36 },
  tool: { width: 130, height: 36 },
  code: { width: 130, height: 36 },
  subWorkflow: { width: 200, height: 80 },
  workflowRef: { width: 120, height: 36 },
  documentParser: { width: 130, height: 36 },
  vectorRetrieve: { width: 130, height: 36 },
  httpRequest: { width: 130, height: 36 },
  validation: { width: 130, height: 36 },
  notification: { width: 120, height: 36 },
  approval: { width: 120, height: 36 },
  email: { width: 120, height: 36 },
  webhookSend: { width: 120, height: 36 },
  storage: { width: 130, height: 36 },
  databaseQuery: { width: 130, height: 36 },
  end: { width: 100, height: 36 },
};

const DEFAULT_SIZE = { width: 140, height: 36 };

/** 获取节点类型的尺寸估算（用于 hit-test / 布局） */
export function getNodeSize(type: string): { width: number; height: number } {
  return NODE_SIZE[type] || DEFAULT_SIZE;
}

// ── 坐标转换工具 ─────────────────────────────────────────────

export interface PositionLike {
  x: number;
  y: number;
}

export interface NodePositionLike {
  id: string;
  position: PositionLike;
}

/**
 * 绝对坐标 → 相对坐标（相对于父容器）。
 * Store 存绝对坐标，ReactFlow 子节点需要相对坐标。
 * 若节点无父容器（pid 为空），直接返回原坐标。
 */
export function toRelativePosition(
  nodeId: string,
  absPos: PositionLike,
  parentRefs: Record<string, string>,
  nodes: NodePositionLike[],
): PositionLike {
  const pid = parentRefs[nodeId];
  if (!pid) { return absPos; }
  const parent = nodes.find((n) => n.id === pid);
  if (!parent) { return absPos; }
  return { x: absPos.x - parent.position.x, y: absPos.y - parent.position.y };
}

/**
 * 相对坐标 → 绝对坐标（相对于画布原点）。
 * ReactFlow 子节点返回相对坐标，Store 需要绝对坐标。
 * 若节点无父容器（pid 为空），直接返回原坐标。
 */
export function toAbsolutePosition(
  nodeId: string,
  relPos: PositionLike,
  parentRefs: Record<string, string>,
  nodes: NodePositionLike[],
): PositionLike {
  const pid = parentRefs[nodeId];
  if (!pid) { return relPos; }
  const parent = nodes.find((n) => n.id === pid);
  if (!parent) { return relPos; }
  return { x: relPos.x + parent.position.x, y: relPos.y + parent.position.y };
}

// ── Grid 吸附与碰撞避免 ─────────────────────────────────────

/**
 * 将坐标吸附到最近的 grid 点。
 *
 * @param x - 原始 X 坐标
 * @param y - 原始 Y 坐标
 * @param grid_size - 网格间距（默认 20px）
 * @returns 吸附后的坐标
 */
export function snap_to_grid(
  x: number,
  y: number,
  grid_size: number = 20,
): { x: number; y: number } {
  return {
    x: Math.round(x / grid_size) * grid_size,
    y: Math.round(y / grid_size) * grid_size,
  };
}

export interface SiblingInfo {
  id: string;
  x: number;
  y: number;
  type: string;
}

/**
 * 在 4 个象限中为候选节点找最近的**不重叠**位置。
 *
 * 策略：
 * 1. 若候选与所有 sibling 无重叠，直接返回原位置
 * 2. 若重叠，对每个重叠的 sibling 尝试上/下/左/右 4 个方向避开
 * 3. 筛选出不产生新重叠的方向，按距离排序取最近者
 * 4. 若 4 方向均产生新重叠，尝试对角线（右+下）回退
 * 5. 最终位置会被 snap_to_grid 吸附
 *
 * @param candidate - 候选位置（含可选 id）
 * @param nodeType - 候选节点类型（用于 getNodeSize）
 * @param siblings - 画布上其他节点的快照（不含自身及同组选中节点）
 * @param min_gap  - 节点间最小间隙（默认 10px）
 * @returns 安全的网格吸附坐标
 */
export function find_safe_position(
  candidate: { x: number; y: number; id?: string },
  nodeType: string,
  siblings: SiblingInfo[],
  min_gap: number = 10,
): { x: number; y: number } {
  if (siblings.length === 0) {
    return snap_to_grid(candidate.x, candidate.y);
  }

  const size = getNodeSize(nodeType);
  const cw = size.width;
  const ch = size.height;

  // 构建 sibling 的边界矩形
  const sibRects = siblings.map((s) => ({
    id: s.id,
    x: s.x,
    y: s.y,
    w: getNodeSize(s.type).width,
    h: getNodeSize(s.type).height,
  }));

  // 检查 (px, py) 是否与任一 sibling 重叠
  function overlaps(px: number, py: number): boolean {
    for (const s of sibRects) {
      if (
        rectsOverlap(
          { x: px, y: py, w: cw, h: ch },
          { x: s.x, y: s.y, w: s.w, h: s.h },
        )
      ) {
        return true;
      }
    }
    return false;
  }

  const ox = candidate.x;
  const oy = candidate.y;

  // 无重叠 → 直接返回
  if (!overlaps(ox, oy)) {
    return snap_to_grid(ox, oy);
  }

  // 收集所有不产生新重叠的方向候选
  const dirCands: Array<{ x: number; y: number; dist: number }> = [];

  for (const s of sibRects) {
    if (!overlaps(ox, oy)) { continue; }

    // 右
    const rx = s.x + s.w + min_gap;
    const rDist = Math.abs(rx - ox);
    if (!overlaps(rx, oy)) {
      dirCands.push({ x: rx, y: oy, dist: rDist });
    }

    // 左
    const lx = s.x - cw - min_gap;
    const lDist = Math.abs(lx - ox);
    if (!overlaps(lx, oy)) {
      dirCands.push({ x: lx, y: oy, dist: lDist });
    }

    // 下
    const dy = s.y + s.h + min_gap;
    const dDist = Math.abs(dy - oy);
    if (!overlaps(ox, dy)) {
      dirCands.push({ x: ox, y: dy, dist: dDist });
    }

    // 上
    const uy = s.y - ch - min_gap;
    const uDist = Math.abs(uy - oy);
    if (!overlaps(ox, uy)) {
      dirCands.push({ x: ox, y: uy, dist: uDist });
    }
  }

  // 按距离排序
  dirCands.sort((a, b) => a.dist - b.dist);

  if (dirCands.length > 0) {
    return snap_to_grid(dirCands[0].x, dirCands[0].y);
  }

  // 对角线回退：右 + 下
  for (const s of sibRects) {
    if (!overlaps(ox, oy)) { continue; }
    const fx = s.x + s.w + min_gap;
    const fy = s.y + s.h + min_gap;
    if (!overlaps(fx, fy)) {
      dirCands.push({
        x: fx,
        y: fy,
        dist: Math.abs(fx - ox) + Math.abs(fy - oy),
      });
    }
  }
  dirCands.sort((a, b) => a.dist - b.dist);

  if (dirCands.length > 0) {
    return snap_to_grid(dirCands[0].x, dirCands[0].y);
  }

  // 最后手段：右移 100px 后 snap，避开密集重叠区域
  return snap_to_grid(candidate.x + 100, candidate.y);
}

/** 间距常量 */
const RANK_SEP = 150; // 层间垂直间距
const NODE_SEP = 80; // 同层节点水平间距
const MARGIN_X = 80; // 左边距
const MARGIN_Y = 80; // 上边距

/**
 * 使用 Dagre 对工作流节点进行自动布局。
 *
 * 策略：
 * - 拓扑排序后按层级自上而下排列
 * - 同一层节点水平均匀分布
 * - 尽量最小化边的交叉
 * - 特殊处理：condition 的 true/false 分支、loop 的回边
 *
 * @returns 更新了 position 的 nodes 和 edges（edges 不变）
 */
export function autoLayout(nodes: Node[], edges: Edge[]): { nodes: Node[]; edges: Edge[] } {
  if (nodes.length === 0) { return { nodes, edges }; }

  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: "TB",
    ranksep: RANK_SEP,
    nodesep: NODE_SEP,
    marginx: MARGIN_X,
    marginy: MARGIN_Y,
    edgesep: 30,
    ranker: "network-simplex",
  });

  const nodeTypeMap = new Map<string, string>();
  for (const node of nodes) {
    const nodeType = (node.data?.type || node.type || "") as string;
    nodeTypeMap.set(node.id, nodeType);
    const size = NODE_SIZE[nodeType] || DEFAULT_SIZE;
    g.setNode(node.id, { width: size.width, height: size.height });
  }

  for (const edge of edges) {
    const sourceType = nodeTypeMap.get(edge.source) || "";

    let minLen = 1;
    if (sourceType === "condition" || sourceType === "switch") {
      minLen = 2;
    }

    g.setEdge(edge.source, edge.target, { minLen });
  }

  dagre.layout(g);

  const layoutedNodes = nodes.map((node) => {
    const dagreNode = g.node(node.id);
    if (!dagreNode) { return node; }

    const nodeType = (node.data?.type || node.type || "") as string;
    const size = NODE_SIZE[nodeType] || DEFAULT_SIZE;

    return {
      ...node,
      position: {
        x: dagreNode.x - size.width / 2,
        y: dagreNode.y - size.height / 2,
      },
    };
  });

  return { nodes: layoutedNodes, edges };
}

/**
 * 判断两个矩形是否重叠。
 */
function rectsOverlap(
  a: { x: number; y: number; w: number; h: number },
  b: { x: number; y: number; w: number; h: number },
): boolean {
  return !(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y);
}

/**
 * 检测并修正节点重叠问题。
 * 对重叠的节点进行温和的位移，避免堆叠。
 */
export function resolveOverlaps(nodes: Node[], parentRefs: Record<string, string> = {}): Node[] {
  if (nodes.length < 2) { return nodes; }

  const result = [...nodes];
  const maxIterations = 100;
  let iteration = 0;

  // Group nodes by their parent (same coordinate space)
  // 对嵌套容器，用顶层祖先作为分组 key，使不同嵌套深度的节点也能检测重叠
  const groupOf = (id: string): string => {
    let current = id;
    let parent = parentRefs[current];
    while (parent) {
      current = parent;
      parent = parentRefs[current];
    }
    return current;
  };

  while (iteration < maxIterations) {
    iteration++;
    let moved = false;

    for (let i = 0; i < result.length; i++) {
      for (let j = i + 1; j < result.length; j++) {
        // Only resolve overlaps between nodes in the same coordinate space
        if (groupOf(result[i].id) !== groupOf(result[j].id)) { continue; }
        const a = result[i];
        const b = result[j];
        const sizeA = NODE_SIZE[(a.data?.type as string) || ""] || DEFAULT_SIZE;
        const sizeB = NODE_SIZE[(b.data?.type as string) || ""] || DEFAULT_SIZE;

        if (
          rectsOverlap(
            { x: a.position.x, y: a.position.y, w: sizeA.width, h: sizeA.height },
            { x: b.position.x, y: b.position.y, w: sizeB.width, h: sizeB.height },
          )
        ) {
          // 计算重叠量并温和推开
          const overlapX = Math.min(a.position.x + sizeA.width, b.position.x + sizeB.width)
            - Math.max(a.position.x, b.position.x);
          const overlapY = Math.min(a.position.y + sizeA.height, b.position.y + sizeB.height)
            - Math.max(a.position.y, b.position.y);

          if (overlapX < overlapY) {
            // 水平推开
            const push = overlapX / 2 + 10;
            if (a.position.x <= b.position.x) {
              result[i] = { ...a, position: { ...a.position, x: a.position.x - push } };
              result[j] = { ...b, position: { ...b.position, x: b.position.x + push } };
            } else {
              result[i] = { ...a, position: { ...a.position, x: a.position.x + push } };
              result[j] = { ...b, position: { ...b.position, x: b.position.x - push } };
            }
          } else {
            // 垂直推开
            const push = overlapY / 2 + 10;
            if (a.position.y <= b.position.y) {
              result[i] = { ...a, position: { ...a.position, y: a.position.y - push } };
              result[j] = { ...b, position: { ...b.position, y: b.position.y + push } };
            } else {
              result[i] = { ...a, position: { ...a.position, y: a.position.y + push } };
              result[j] = { ...b, position: { ...b.position, y: b.position.y - push } };
            }
          }
          moved = true;
        }
      }
    }

    if (!moved) { break; }
  }

  return result;
}

/**
 * 完整的自动布局流程：Dagre 层级布局 + 重叠修正。
 *
 * @param parentRefs 容器子树映射（childId → parentId），用于让每个 parallel 节点
 *  内部的子节点先单独 dagre 排布，再随父容器整体定位。不传则退化为扁平布局。
 */
export function autoLayoutWorkflow(
  nodes: Node[],
  edges: Edge[],
  parentRefs: Record<string, string> = {},
): { nodes: Node[]; edges: Edge[] } {
  const childOf = parentRefs;
  const layoutNodes = nodes.filter((n) => !isLayoutExcluded(n as NodeLike));
  const excludedNodes = nodes.filter((n) => isLayoutExcluded(n as NodeLike));

  if (layoutNodes.length === 0) {
    return { nodes: excludedNodes, edges };
  }

  const autoNodes: AutoNode[] = layoutNodes.map((n) => ({
    id: n.id,
    type: n.type || (n.data?.type as string) || "",
    position: { x: n.position.x, y: n.position.y },
    parentId: childOf[n.id] || undefined,
    data: n.data || {},
  }));

  const layoutEdges: LayoutEdge[] = edges.map((e) => ({
    source: e.source,
    target: e.target,
  }));

  const layoutedAutoNodes = forceLayout(autoNodes, layoutEdges, childOf);

  const newAbs: Record<string, { x: number; y: number }> = {};
  for (const n of layoutedAutoNodes) {
    newAbs[n.id] = { x: n.position.x, y: n.position.y };
  }

  const PADDING = CONTAINER_PADDING;
  const HEADER_H = CONTAINER_HEADER_H;
  const MIN_W = CONTAINER_MIN_W;
  const MIN_H = CONTAINER_MIN_H;

  const containerSizes: Record<string, { width: number; height: number }> = {};
  const containers = layoutNodes.filter((n) => CONTAINER_NODE_TYPES.has(n.type || ""));

  for (const c of containers) {
    const childIds = Object.keys(childOf).filter((cid) => childOf[cid] === c.id);
    if (childIds.length === 0) {
      const size = getNodeSize(c.type || "");
      containerSizes[c.id] = { width: size.width, height: size.height };
      continue;
    }

    const cAbs = newAbs[c.id];
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const cid of childIds) {
      const pos = newAbs[cid];
      if (!pos || !cAbs) { continue; }
      const child = layoutNodes.find((n) => n.id === cid);
      if (!child) { continue; }
      const sz = getNodeSize((child.data?.type as string) || child.type || "");
      // 使用相对坐标计算容器内 bbox
      const relX = pos.x - cAbs.x;
      const relY = pos.y - cAbs.y;
      minX = Math.min(minX, relX);
      minY = Math.min(minY, relY);
      maxX = Math.max(maxX, relX + sz.width);
      maxY = Math.max(maxY, relY + sz.height);
    }

    containerSizes[c.id] = {
      width: Math.max(MIN_W, maxX - minX + PADDING * 2),
      height: Math.max(MIN_H, maxY - minY + PADDING * 2 + HEADER_H),
    };
  }

  const result: Node[] = nodes.map((n) => {
    const abs = newAbs[n.id];
    if (!abs) { return n; }
    const pid = childOf[n.id];
    let final = abs;
    if (pid) {
      const parentAbs = newAbs[pid];
      if (parentAbs) {
        final = { x: abs.x - parentAbs.x, y: abs.y - parentAbs.y };
      }
    }
    return { ...n, position: final };
  });

  const clamped = clampChildrenIntoContainers(result, childOf, containerSizes, PADDING);
  return { nodes: [...clamped, ...excludedNodes], edges };
}

const MARGIN = 60;
const CONTAINER_PADDING = 40; // 容器内边距
const CONTAINER_HEADER_H = 60; // 容器标题栏高度
const CONTAINER_MIN_W = 400; // 容器最小宽度
const CONTAINER_MIN_H = 200; // 容器最小高度

export interface AutoNode {
  id: string;
  type?: string;
  position: { x: number; y: number };
  parentId?: string;
  data: Record<string, unknown>;
}

interface LayoutEdge {
  source: string;
  target: string;
}

/**
 * 提取节点类型（兼容 ReactFlow Node / WorkflowNode）
 */
function layoutNodeType(n: { type?: string; data?: Record<string, unknown> }): string {
  return (n.type || (n.data?.type as string) || "");
}

/**
 * 使用 d3-force 力导向布局进行自动布局，专门优化工作流流程图的布局效果。
 *
 * 策略：
 * 1. 使用 Dagre 进行初始布局，确保层级关系正确（自上而下）
 * 2. 使用 d3-force 进行力导向优化：
 *    - forceLink: 边的拉力，保持连接关系
 *    - forceManyBody: 节点间斥力，避免重叠
 *    - forceCenter: 重力，将图拉向中心
 *    - forceCollide: 碰撞检测，防止节点重叠
 *    - forceY: 保持 Dagre 的层级顺序
 * 3. 容器节点内部使用独立的布局
 *
 * @param nodes - 节点列表（需包含 id / type / position / data）
 * @param edges - 边列表（需包含 source / target）
 * @param parentRefs - 容器父子映射（childId → parentId），可选
 * @returns 更新了 position 的 nodes 副本（保持原输入形状）
 */
export function auto_layout(
  nodes: AutoNode[],
  edges: LayoutEdge[],
  parentRefs: Record<string, string> = {},
): AutoNode[] {
  return forceLayout(nodes, edges, parentRefs);
}

/**
 * 把溢出容器的子节点拉回到容器 bbox 内。
 *
 * 解决 §3.5 缺陷：autoLayoutWorkflow 完成 dagre 主布局后，
 * 部分子节点可能因初始位置过偏而落在容器外（特别是用户
 * 手动拖拽到容器边角、或运行了多轮 dagre 之后）。
 * ReactFlow 渲染时若 extent="parent" 会直接裁剪，导致
 * 子节点看不见或被截掉一半。
 *
 * @param nodes 所有节点（容器 + 子节点都会被处理）
 * @param parentRefs childId → parentId 映射
 * @param containerSizes parentId → { width, height }（可缺省；缺省时不限制该容器）
 * @param padding 内边距，最小可保留多少空间
 */
export function clampChildrenIntoContainers(
  nodes: Node[],
  parentRefs: Record<string, string>,
  containerSizes: Record<string, { width: number; height: number }>,
  padding = 40,
): Node[] {
  return nodes.map((n) => {
    const parentId = parentRefs[n.id];
    if (!parentId) { return n; }
    const size = containerSizes[parentId];
    if (!size) { return n; }
    const measured = (n as unknown as { measured?: { width?: number; height?: number } }).measured;
    const childW = measured?.width
      ?? (n.width as number | undefined)
      ?? getNodeSize(n.type || "").width;
    const childH = measured?.height
      ?? (n.height as number | undefined)
      ?? getNodeSize(n.type || "").height;
    let { x, y } = n.position;
    const minX = padding;
    const minY = padding;
    const maxX = size.width - padding - childW;
    const maxY = size.height - padding - childH;
    // 仅在子节点宽度不大于容器内可容纳宽度时才做水平 clamp，
    // 否则保留原 x 让子节点在视觉上对齐顶部/左部。
    if (maxX > minX) {
      x = Math.min(Math.max(x, minX), maxX);
    } else {
      x = minX;
    }
    if (maxY > minY) {
      y = Math.min(Math.max(y, minY), maxY);
    } else {
      y = minY;
    }
    if (x === n.position.x && y === n.position.y) { return n; }
    return { ...n, position: { x, y } };
  });
}

/**
 * 检查新加边 (newSource -> newTarget) 是否会形成环。
 * 用 DFS 在现有有向图上判断从 newTarget 出发能否再次到达 newSource。
 *
 * 注：自循环（newSource === newTarget）也视为环。
 */
export function would_create_cycle(
  edges: Array<{ source: string; target: string }>,
  newSource: string,
  newTarget: string,
): boolean {
  if (newSource === newTarget) { return true; }
  const adj = new Map<string, string[]>();
  for (const e of edges) {
    const list = adj.get(e.source) ?? [];
    list.push(e.target);
    adj.set(e.source, list);
  }
  const incoming = adj.get(newSource) ?? [];
  incoming.push(newTarget);
  adj.set(newSource, incoming);

  const visited = new Set<string>();
  const stack: string[] = [newTarget];
  while (stack.length > 0) {
    const node = stack.pop()!;
    if (node === newSource) { return true; }
    if (visited.has(node)) { continue; }
    visited.add(node);
    const next = adj.get(node);
    if (next) { stack.push(...next); }
  }
  return false;
}

// ── 端口锚定系统 ──────────────────────────────────────────────────

export const PORT_SIZE = 7;
export const PORT_OFFSET = 4;

export type PortPosition = "top" | "bottom" | "left" | "right";

export interface PortPoint {
  x: number;
  y: number;
}

export function getPortCenter(
  nodeX: number,
  nodeY: number,
  nodeWidth: number,
  nodeHeight: number,
  position: PortPosition,
): PortPoint {
  switch (position) {
    case "top":
      return { x: nodeX + nodeWidth / 2, y: nodeY - PORT_OFFSET };
    case "bottom":
      return { x: nodeX + nodeWidth / 2, y: nodeY + nodeHeight + PORT_OFFSET };
    case "left":
      return { x: nodeX - PORT_OFFSET, y: nodeY + nodeHeight / 2 };
    case "right":
      return { x: nodeX + nodeWidth + PORT_OFFSET, y: nodeY + nodeHeight / 2 };
    default:
      return { x: nodeX + nodeWidth / 2, y: nodeY + nodeHeight / 2 };
  }
}

export function getHandlePosition(
  nodeWidth: number,
  nodeHeight: number,
  position: PortPosition,
): { x: number; y: number } {
  switch (position) {
    case "top":
      return { x: nodeWidth / 2, y: -PORT_OFFSET };
    case "bottom":
      return { x: nodeWidth / 2, y: nodeHeight + PORT_OFFSET };
    case "left":
      return { x: -PORT_OFFSET, y: nodeHeight / 2 };
    case "right":
      return { x: nodeWidth + PORT_OFFSET, y: nodeHeight / 2 };
    default:
      return { x: nodeWidth / 2, y: nodeHeight / 2 };
  }
}

// ── D3 Force 力导向布局 ──────────────────────────────────────────

interface ForceNode {
  id: string;
  type?: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  fx?: number;
  fy?: number;
  width: number;
  height: number;
}

interface ForceLink {
  source: string;
  target: string;
}

/**
 * 使用 d3-force 力导向布局对工作流节点进行优化布局。
 *
 * 策略：
 * 1. 使用 Dagre 进行初始布局，确保层级关系正确
 * 2. 使用 d3-force 进行力导向优化：
 *    - forceManyBody: 节点间斥力，避免重叠
 *    - forceLink: 边的拉力，保持连接关系
 *    - forceCenter: 重力，将图拉向中心
 *    - forceCollide: 碰撞检测，防止节点重叠
 * 3. 保留 Dagre 的层级顺序（y坐标排序），只优化水平位置
 *
 * @param nodes - 节点列表
 * @param edges - 边列表
 * @param parentRefs - 容器父子映射
 * @returns 更新了 position 的 nodes 副本
 */
export function forceLayout(
  nodes: AutoNode[],
  edges: LayoutEdge[],
  parentRefs: Record<string, string> = {},
): AutoNode[] {
  if (nodes.length === 0) { return []; }

  const childOf = parentRefs;

  const containers = nodes.filter(
    (n) => CONTAINER_NODE_TYPES.has(n.type || layoutNodeType(n)),
  );

  const childPositions: Record<string, { x: number; y: number }> = {};
  const containerBBox: Record<string, { width: number; height: number }> = {};

  for (const c of containers) {
    const cType = c.type || layoutNodeType(c);
    const childIds = Object.keys(childOf).filter((cid) => childOf[cid] === c.id);
    const childNodes = childIds
      .map((cid) => nodes.find((n) => n.id === cid))
      .filter(Boolean) as AutoNode[];

    if (childNodes.length === 0) {
      const sz = getNodeSize(cType);
      containerBBox[c.id] = {
        width: Math.max(CONTAINER_MIN_W, sz.width + CONTAINER_PADDING * 2),
        height: Math.max(CONTAINER_MIN_H, sz.height + CONTAINER_PADDING * 2 + CONTAINER_HEADER_H),
      };
      continue;
    }

    const childEdges = edges.filter((e) => childIds.includes(e.source) && childIds.includes(e.target));

    const subGraph = new dagre.graphlib.Graph();
    subGraph.setDefaultEdgeLabel(() => ({}));
    subGraph.setGraph({
      rankdir: "TB",
      ranksep: 80,
      nodesep: 50,
      marginx: 40,
      marginy: 40,
      ranker: "network-simplex",
    });

    for (const cn of childNodes) {
      const t = cn.type || layoutNodeType(cn);
      const sz = getNodeSize(t);
      subGraph.setNode(cn.id, { width: sz.width, height: sz.height });
    }

    for (const ce of childEdges) {
      subGraph.setEdge(ce.source, ce.target);
    }

    dagre.layout(subGraph);

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const cn of childNodes) {
      const dagreNode = subGraph.node(cn.id);
      if (!dagreNode) { continue; }
      const sz = getNodeSize(cn.type || layoutNodeType(cn));
      minX = Math.min(minX, dagreNode.x - sz.width / 2);
      minY = Math.min(minY, dagreNode.y - sz.height / 2);
      maxX = Math.max(maxX, dagreNode.x + sz.width / 2);
      maxY = Math.max(maxY, dagreNode.y + sz.height / 2);
    }

    containerBBox[c.id] = {
      width: Math.max(CONTAINER_MIN_W, maxX - minX + CONTAINER_PADDING * 2),
      height: Math.max(CONTAINER_MIN_H, maxY - minY + CONTAINER_PADDING * 2 + CONTAINER_HEADER_H),
    };

    for (const cn of childNodes) {
      const dagreNode = subGraph.node(cn.id);
      if (!dagreNode) { continue; }
      const sz = getNodeSize(cn.type || layoutNodeType(cn));
      childPositions[cn.id] = {
        x: CONTAINER_PADDING + (dagreNode.x - sz.width / 2 - minX),
        y: CONTAINER_PADDING + (dagreNode.y - sz.height / 2 - minY),
      };
    }
  }

  const topLevel = nodes.filter((n) => !childOf[n.id]);

  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: "TB",
    ranksep: 180,
    nodesep: 80,
    marginx: 80,
    marginy: 80,
    edgesep: 30,
    ranker: "network-simplex",
  });

  const nodeTypeMap = new Map<string, string>();
  for (const n of topLevel) {
    const t = n.type || layoutNodeType(n);
    nodeTypeMap.set(n.id, t);
    if (CONTAINER_NODE_TYPES.has(t)) {
      const bbox = containerBBox[n.id];
      if (bbox) {
        g.setNode(n.id, { width: bbox.width, height: bbox.height });
      } else {
        const sz = getNodeSize(t);
        g.setNode(n.id, { width: sz.width, height: sz.height });
      }
    } else {
      const sz = getNodeSize(t);
      g.setNode(n.id, { width: sz.width, height: sz.height });
    }
  }

  const topLevelEdges = edges.filter(
    (e) => !childOf[e.source] && !childOf[e.target],
  );
  for (const e of topLevelEdges) {
    g.setEdge(e.source, e.target);
  }

  dagre.layout(g);

  const forceNodes: ForceNode[] = [];
  for (const n of topLevel) {
    const dagreNode = g.node(n.id);
    if (!dagreNode) { continue; }
    const t = n.type || layoutNodeType(n);
    let sz: { width: number; height: number };
    if (CONTAINER_NODE_TYPES.has(t)) {
      sz = containerBBox[n.id] || getNodeSize(t);
    } else {
      sz = getNodeSize(t);
    }
    forceNodes.push({
      id: n.id,
      type: t,
      x: dagreNode.x - sz.width / 2,
      y: dagreNode.y - sz.height / 2,
      vx: 0,
      vy: 0,
      width: sz.width,
      height: sz.height,
    });
  }

  const forceLinks: ForceLink[] = topLevelEdges.map((e) => ({
    source: e.source,
    target: e.target,
  }));

  // 根据节点数量动态计算中心点，避免硬编码导致少节点偏右下、多节点超出视口
  const centerX = Math.max(400, Math.sqrt(topLevel.length) * 200);
  const centerY = Math.max(300, Math.sqrt(topLevel.length) * 150);

  const simulation = d3.forceSimulation<ForceNode, ForceNode>(forceNodes)
    .force("link", d3.forceLink<ForceNode, ForceLink>(forceLinks).id((d) => d.id).distance(150).strength(0.8))
    .force("charge", d3.forceManyBody().strength(-800))
    .force("center", d3.forceCenter(centerX, centerY))
    .force("collide", d3.forceCollide<ForceNode>((d) => Math.max(d.width, d.height) / 2 + 20))
    .force("y", d3.forceY<ForceNode>((d) => d.y).strength(0.5))
    .stop();

  for (let i = 0; i < 100; i++) {
    simulation.tick();
  }

  const allPositions: Record<string, { x: number; y: number }> = {};
  for (const fn of forceNodes) {
    allPositions[fn.id] = { x: fn.x, y: fn.y };
  }

  for (const c of containers) {
    const cPos = allPositions[c.id];
    if (!cPos) { continue; }
    const childIds = Object.keys(childOf).filter((cid) => childOf[cid] === c.id);
    for (const cid of childIds) {
      if (childPositions[cid]) {
        allPositions[cid] = {
          x: cPos.x + childPositions[cid].x,
          y: cPos.y + childPositions[cid].y,
        };
      }
    }
  }

  return nodes.map((n) => {
    const pos = allPositions[n.id];
    if (!pos) { return { ...n, position: { x: MARGIN, y: MARGIN } }; }
    return { ...n, position: pos };
  });
}
