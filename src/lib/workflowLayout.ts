import dagre from "dagre";
import type { Edge, Node } from "reactflow";

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
const CONTAINER_NODE_TYPES = new Set(["parallel", "loop", "debate", "swarm", "aggregator", "subWorkflow"]);

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
function findCyclicSCCs(nodes: NodeLike[], edges: EdgeLike[]): string[][] {
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

/** 提取节点标题：优先 data.title（ReactFlow），回退到 (node as any).title（WorkflowNode） */
function titleOf(n: NodeLike): string {
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
    (e) => (e as any).edge_type !== "grouping" && (e as any).data?.edgeType !== "grouping",
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
      (n as any).kind === "decorative" || (n as any).data?.kind === "decorative"
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
    if (nodeTypeOf(n) !== "condition") { continue; }
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
    const currentWfId = (n as any).data?.templateId || "";
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
  const cfg = (n as any).config;
  if (cfg && typeof cfg[key] === "string") { return cfg[key]; }
  if (n.data && typeof n.data[key] === "string") { return n.data[key] as string; }
  if (n.data?.config && typeof (n.data.config as any)[key] === "string") { return (n.data.config as any)[key]; }
  return undefined;
}

// ── 原有代码以下继续 ──────────────────────────────────────────

/**
 * 节点尺寸估计（React Flow 画布坐标系，单位 px）。
 * 实际渲染尺寸可能不同，但 Dagre 只影响相对排列，偏差可接受。
 */
const NODE_SIZE: Record<string, { width: number; height: number }> = {
  trigger: { width: 200, height: 120 },
  agent: { width: 220, height: 160 },
  llm: { width: 220, height: 180 },
  condition: { width: 200, height: 140 },
  parallel: { width: 500, height: 400 },
  loop: { width: 480, height: 300 },
  debate: { width: 480, height: 260 },
  swarm: { width: 480, height: 300 },
  aggregator: { width: 320, height: 180 },
  merge: { width: 220, height: 120 },
  delay: { width: 180, height: 100 },
  tool: { width: 200, height: 140 },
  code: { width: 200, height: 140 },
  subWorkflow: { width: 480, height: 300 },
  workflowRef: { width: 220, height: 120 },
  documentParser: { width: 200, height: 120 },
  vectorRetrieve: { width: 200, height: 120 },
  validation: { width: 200, height: 120 },
  end: { width: 180, height: 80 },
};

const DEFAULT_SIZE = { width: 200, height: 120 };

/** 获取节点类型的尺寸估算（用于 hit-test / 布局） */
export function getNodeSize(type: string): { width: number; height: number } {
  return NODE_SIZE[type] || DEFAULT_SIZE;
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
const RANK_SEP = 80; // 层间垂直间距
const NODE_SEP = 50; // 同层节点水平间距
const MARGIN_X = 60; // 左边距
const MARGIN_Y = 60; // 上边距

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
    edgesep: 20,
  });

  // 添加节点
  for (const node of nodes) {
    const nodeType = (node.data?.type || node.type || "") as string;
    const size = NODE_SIZE[nodeType] || DEFAULT_SIZE;
    g.setNode(node.id, { width: size.width, height: size.height });
  }

  // 添加边
  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  // 执行布局
  dagre.layout(g);

  // 应用位置
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
export function resolveOverlaps(nodes: Node[]): Node[] {
  if (nodes.length < 2) { return nodes; }

  const result = [...nodes];
  const maxIterations = 100;
  let iteration = 0;

  while (iteration < maxIterations) {
    iteration++;
    let moved = false;

    for (let i = 0; i < result.length; i++) {
      for (let j = i + 1; j < result.length; j++) {
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
  // 排除阶段分隔线和分组框等不参与布局的节点
  const layoutNodes = nodes.filter((n) => !isLayoutExcluded(n as NodeLike));
  const containers = layoutNodes.filter((n) => CONTAINER_NODE_TYPES.has(n.type || "") && !childOf[n.id]);

  if (containers.length === 0 || Object.keys(childOf).length === 0) {
    const dagreResult = autoLayout(layoutNodes, edges);
    const resolvedNodes = resolveOverlaps(dagreResult.nodes);
    // 重新合并被排除的节点（保持其原始位置）
    const excludedNodes = nodes.filter((n) => isLayoutExcluded(n as NodeLike));
    return { nodes: [...resolvedNodes, ...excludedNodes], edges: dagreResult.edges };
  }

  // 1. 反算每个节点的当前绝对坐标（input 是 ReactFlow 坐标系，子节点为相对父）
  const currentAbs: Record<string, { x: number; y: number }> = {};
  for (const n of nodes) {
    const pid = childOf[n.id];
    if (pid && currentAbs[pid]) {
      currentAbs[n.id] = { x: n.position.x + currentAbs[pid].x, y: n.position.y + currentAbs[pid].y };
    } else {
      currentAbs[n.id] = { x: n.position.x, y: n.position.y };
    }
  }

  // 2. 对每个 parallel 容器：单独 dagre 排子节点 + 量 bbox + 归一化到原点
  const PADDING = 40;
  const groupNorm: Record<string, { nodes: Node[]; bboxW: number; bboxH: number }> = {};
  const containerSizes: Record<string, { width: number; height: number }> = {};

  for (const c of containers) {
    const childIds = Object.keys(childOf).filter((cid) => childOf[cid] === c.id);
    const childNodesAbs = childIds
      .map((cid) => nodes.find((n) => n.id === cid))
      .filter((n): n is Node => !!n)
      .map((n) => ({ ...n, position: currentAbs[n.id] || n.position }));

    if (childNodesAbs.length === 0) {
      const size = getNodeSize(c.type || "");
      groupNorm[c.id] = { nodes: [], bboxW: 0, bboxH: 0 };
      containerSizes[c.id] = { width: size.width, height: size.height };
      continue;
    }

    const childEdges = edges.filter((e) => childIds.includes(e.source) && childIds.includes(e.target));
    const sub = autoLayout(childNodesAbs, childEdges);

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of sub.nodes) {
      const sz = getNodeSize((n.data?.type as string) || n.type || "");
      minX = Math.min(minX, n.position.x);
      minY = Math.min(minY, n.position.y);
      maxX = Math.max(maxX, n.position.x + sz.width);
      maxY = Math.max(maxY, n.position.y + sz.height);
    }
    const bboxW = maxX - minX;
    const bboxH = maxY - minY;
    const normalized = sub.nodes.map((n) => ({
      ...n,
      position: { x: n.position.x - minX, y: n.position.y - minY },
    }));
    groupNorm[c.id] = { nodes: normalized, bboxW, bboxH };
    containerSizes[c.id] = { width: bboxW + PADDING * 2, height: bboxH + PADDING * 2 };
  }

  // 3. 主 dagre：只放顶层节点（容器节点 + 无父孤立节点）
  const topLevelIds = new Set<string>();
  for (const n of nodes) {
    if (CONTAINER_NODE_TYPES.has(n.type || "") || !childOf[n.id]) {
      topLevelIds.add(n.id);
    }
  }
  const topLevelNodes = nodes.filter((n) => topLevelIds.has(n.id));
  const topLevelEdges = edges.filter((e) => topLevelIds.has(e.source) && topLevelIds.has(e.target));

  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: "TB",
    ranksep: RANK_SEP,
    nodesep: NODE_SEP,
    marginx: MARGIN_X,
    marginy: MARGIN_Y,
    edgesep: 20,
  });
  for (const n of topLevelNodes) {
    const t = (n.data?.type as string) || n.type || "";
    const size = CONTAINER_NODE_TYPES.has(n.type || "")
      ? (containerSizes[n.id] ?? getNodeSize(t))
      : getNodeSize(t);
    g.setNode(n.id, { width: size.width, height: size.height });
  }
  for (const e of topLevelEdges) {
    g.setEdge(e.source, e.target);
  }
  dagre.layout(g);

  // 4. 写回绝对坐标
  const newAbs: Record<string, { x: number; y: number }> = {};
  for (const n of topLevelNodes) {
    const dagreNode = g.node(n.id);
    if (!dagreNode) { continue; }
    const t = (n.data?.type as string) || n.type || "";
    const size = CONTAINER_NODE_TYPES.has(n.type || "")
      ? (containerSizes[n.id] ?? getNodeSize(t))
      : getNodeSize(t);
    newAbs[n.id] = { x: dagreNode.x - size.width / 2, y: dagreNode.y - size.height / 2 };
  }
  for (const c of containers) {
    const cAbs = newAbs[c.id];
    if (!cAbs) { continue; }
    const group = groupNorm[c.id];
    for (const cn of group.nodes) {
      newAbs[cn.id] = {
        x: cAbs.x + PADDING + cn.position.x,
        y: cAbs.y + PADDING + cn.position.y,
      };
    }
  }

  // 5. 写回 ReactFlow 坐标系（子节点减回父节点位置，转为相对坐标）
  const result: Node[] = nodes.map((n) => {
    const abs = newAbs[n.id];
    if (!abs) { return n; }
    const pid = childOf[n.id];
    let final = abs;
    if (pid) {
      const parentAbs = newAbs[pid] || currentAbs[pid];
      if (parentAbs) {
        final = { x: abs.x - parentAbs.x, y: abs.y - parentAbs.y };
      }
    }
    return { ...n, position: final };
  });

  // 6. 后处理：把子节点位置 clamp 到容器 bbox 内（见 §3.5 修复）
  const clamped = clampChildrenIntoContainers(result, childOf, containerSizes, PADDING);
  return { nodes: clamped, edges };
}

// ── 自动整理（按 type 分层布局） ──────────────────────────────

/**
 * 节点类型 → 层级索引（小 = 在上方）
 */
const LAYER_ORDER: Record<string, number> = {
  trigger: 0,
  tool: 1,
  agent: 2,
  llm: 2,
  debate: 3,
  swarm: 3,
  condition: 3,
  parallel: 3,
  switch: 3,
  llmClassifier: 3,
  loop: 4,
  aggregator: 4,
  delay: 4,
  validation: 4,
  code: 4,
  dataTransformer: 4,
  fileOperation: 4,
  databaseQuery: 4,
  httpRequest: 4,
  webhookSend: 4,
  subWorkflow: 4,
  workflowRef: 4,
  documentParser: 4,
  vectorRetrieve: 4,
  merge: 5,
  notification: 5,
  email: 5,
  approval: 5,
  logging: 5,
  end: 5,
};

const LAYER_Y_SPACING = 200; // 层间垂直间距
const LAYER_X_SPACING = 320; // 层内水平间距
const MARGIN = 60; // 画布边距
const CONTAINER_PADDING = 40; // 容器内边距

interface AutoNode {
  id: string;
  type?: string;
  position: { x: number; y: number };
  parentId?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data: any;
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
 * 按 type 分层 + Barycenter 启发式 + 父容器适配的自动布局。
 *
 * 策略：
 * 1. 顶层节点按 type 固定分层（L0=trigger → L5=end）
 * 2. 同层 Barycenter 排序减少边交叉（mid=邻居平均列号）
 * 3. 容器节点先布局子节点，再按 bbox 大小排入主层
 * 4. 同层水平 320px 间距，层间垂直 200px 间距
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
  if (nodes.length === 0) { return []; }

  const childOf = parentRefs;

  // ── 分离容器与顶层节点 ────────────────────────────────────
  const containers = nodes.filter(
    (n) => CONTAINER_NODE_TYPES.has(n.type || layoutNodeType(n)) && !childOf[n.id],
  );
  const topLevel = nodes.filter((n) => !childOf[n.id]);

  // ── 子容器布局 ────────────────────────────────────────────
  const childPositions: Record<string, { x: number; y: number }> = {}; // 全局绝对坐标
  const containerBBox: Record<string, { w: number; h: number }> = {};

  for (const c of containers) {
    const cType = c.type || layoutNodeType(c);
    const childIds = Object.keys(childOf).filter((cid) => childOf[cid] === c.id);
    const childNodes = childIds
      .map((cid) => nodes.find((n) => n.id === cid))
      .filter(Boolean) as AutoNode[];

    if (childNodes.length === 0) {
      const sz = getNodeSize(cType);
      containerBBox[c.id] = { w: sz.width + CONTAINER_PADDING * 2, h: sz.height + CONTAINER_PADDING * 2 };
      continue;
    }

    // 子节点间边
    const childEdges = edges.filter((e) => childIds.includes(e.source) && childIds.includes(e.target));

    // 对子节点做扁平分层布局
    const subPositions = layerPositions(childNodes, childEdges, {});
    const subNodesPos = childNodes.map((n) => ({ id: n.id, ...subPositions[n.id] }));

    // 计算 bbox
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const sn of subNodesPos) {
      const sz = getNodeSize(childNodes.find((n) => n.id === sn.id)?.type || "");
      minX = Math.min(minX, sn.x);
      minY = Math.min(minY, sn.y);
      maxX = Math.max(maxX, sn.x + sz.width);
      maxY = Math.max(maxY, sn.y + sz.height);
    }
    containerBBox[c.id] = { w: maxX - minX + CONTAINER_PADDING * 2, h: maxY - minY + CONTAINER_PADDING * 2 };

    // 归一化子节点到容器原点（相对于容器内部）
    for (const cn of childNodes) {
      // 子节点相对容器左上角的位置
      childPositions[cn.id] = {
        x: CONTAINER_PADDING + (subPositions[cn.id].x - minX),
        y: CONTAINER_PADDING + (subPositions[cn.id].y - minY),
      };
    }
  }

  // ── 顶层节点分层布局 ──────────────────────────────────────
  const topNodeList = topLevel.filter((n) => !CONTAINER_NODE_TYPES.has(n.type || layoutNodeType(n)));
  const topPositions = layerPositions(topNodeList, edges, containerBBox);

  // ── 容器节点排入主布局 ────────────────────────────────────
  // 找到容器节点所属的层，将容器按已有节点顺序插入
  const allPositions: Record<string, { x: number; y: number }> = { ...topPositions };

  // 先按层分组容器
  const containersByLayer: Record<number, typeof containers> = {};
  for (const c of containers) {
    const cType = c.type || layoutNodeType(c);
    const layer = LAYER_ORDER[cType] ?? 3;
    if (!containersByLayer[layer]) { containersByLayer[layer] = []; }
    containersByLayer[layer].push(c);
  }

  // 对每个容器，插入到对应层的最后一个位置
  for (const [layerStr, conts] of Object.entries(containersByLayer)) {
    const layer = Number(layerStr);
    // 该层已有的顶层节点数
    const existingCount = topNodeList.filter((n) => (LAYER_ORDER[n.type || layoutNodeType(n)] ?? 3) === layer).length;
    for (let i = 0; i < conts.length; i++) {
      const c = conts[i];
      const idx = existingCount + i;
      allPositions[c.id] = {
        x: MARGIN + idx * LAYER_X_SPACING,
        y: MARGIN + layer * LAYER_Y_SPACING,
      };
    }
  }

  // ── 子节点修正：转为相对父容器的坐标 ──────────────────────
  for (const c of containers) {
    const cPos = allPositions[c.id];
    if (!cPos) { continue; }
    const childIds = Object.keys(childOf).filter((cid) => childOf[cid] === c.id);
    for (const cid of childIds) {
      if (childPositions[cid]) {
        // 子节点绝对坐标 = 父容器位置 + 子节点相对位置
        allPositions[cid] = {
          x: cPos.x + childPositions[cid].x,
          y: cPos.y + childPositions[cid].y,
        };
      }
    }
  }

  // ── 写回 ──────────────────────────────────────────────────
  return nodes.map((n) => {
    const pos = allPositions[n.id];
    if (!pos) { return { ...n, position: { x: MARGIN, y: MARGIN } }; }
    // 子节点转为相对父容器的坐标
    const pid = childOf[n.id];
    if (pid && allPositions[pid]) {
      return {
        ...n,
        position: {
          x: pos.x - allPositions[pid].x,
          y: pos.y - allPositions[pid].y,
        },
      };
    }
    return { ...n, position: pos };
  });
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
    const childW = (n.width as number | undefined) ?? getNodeSize(n.type || "").width;
    const childH = (n.height as number | undefined) ?? getNodeSize(n.type || "").height;
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

/**
 * 对一批节点做分层布局（不含容器处理）。
 * 返回 nodeId → { x, y } 的绝对坐标映射。
 */
function layerPositions(
  nodes: AutoNode[],
  edges: LayoutEdge[],
  _containerBBox: Record<string, { w: number; h: number }>,
): Record<string, { x: number; y: number }> {
  if (nodes.length === 0) { return {}; }

  // 1. 按 type 分配到层
  const layers: Record<number, AutoNode[]> = {};
  for (const n of nodes) {
    const l = LAYER_ORDER[n.type || layoutNodeType(n)] ?? 3;
    if (!layers[l]) { layers[l] = []; }
    layers[l].push(n);
  }

  const layerKeys = Object.keys(layers)
    .map(Number)
    .sort((a, b) => a - b);

  // 2. Barycenter 排序：用邻居在上一层的平均位置来排序本层节点
  for (let li = 0; li < layerKeys.length; li++) {
    const layer = layerKeys[li];
    const layerNodes = layers[layer];
    if (li === 0) {
      // 第一层按出度排序（连接到下一层的边数）
      const nextIds = new Set<string>();
      for (const e of edges) {
        const srcInLayer = layerNodes.some((n) => n.id === e.source);
        if (srcInLayer) { nextIds.add(e.source); }
      }
      layerNodes.sort((a, b) => {
        const aCount = edges.filter((e) => e.source === a.id).length;
        const bCount = edges.filter((e) => e.source === b.id).length;
        return bCount - aCount; // 出度多的靠左
      });
      // 无边的节点放最后
      // (already sorted by out-degree)
    } else {
      const prevLayer = layerKeys[li - 1];
      const prevNodes = layers[prevLayer];
      // 构建 prevNodes 的列号索引
      const prevIndex = new Map<string, number>();
      prevNodes.forEach((pn, idx) => prevIndex.set(pn.id, idx));

      // 为每个节点计算 barycenter
      const withBary: Array<{ node: AutoNode; bary: number }> = layerNodes.map((n) => {
        const connected: number[] = [];
        for (const e of edges) {
          if (e.target === n.id && prevIndex.has(e.source)) {
            connected.push(prevIndex.get(e.source)!);
          }
          if (e.source === n.id && prevIndex.has(e.target)) {
            connected.push(prevIndex.get(e.target)!);
          }
        }
        const bary = connected.length > 0
          ? connected.reduce((s, v) => s + v, 0) / connected.length
          : -1; // 无连接的排最后
        return { node: n, bary };
      });

      withBary.sort((a, b) => {
        if (a.bary === -1 && b.bary === -1) { return 0; }
        if (a.bary === -1) { return 1; }
        if (b.bary === -1) { return -1; }
        return a.bary - b.bary;
      });

      layers[layer] = withBary.map((w) => w.node);
    }
  }

  // 3. 分配坐标
  const positions: Record<string, { x: number; y: number }> = {};

  // 先算每层实际宽度（考虑容器节点）
  for (let li = 0; li < layerKeys.length; li++) {
    const layer = layerKeys[li];
    const layerNodes = layers[layer];

    let xOffset = MARGIN;
    for (let ni = 0; ni < layerNodes.length; ni++) {
      const n = layerNodes[ni];
      positions[n.id] = { x: xOffset, y: MARGIN + layer * LAYER_Y_SPACING };
      xOffset += LAYER_X_SPACING;
    }
  }

  return positions;
}
