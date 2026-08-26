// SPDX-License-Identifier: AGPL-3.0-only

import type { WorkflowEdge, WorkflowNode, WorkflowNodeBase } from "@/components/workflow/types/workflow.types";
import type {
  DiagnosticCategory,
  DiagnosticFix,
  DiagnosticIssue,
  DiagnosticReport,
  DiagnosticSeverity,
} from "@/components/workflow/types/workflow.types";

interface RuleContext {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  nodeMap: Map<string, WorkflowNode>;
  upstreamOf: Map<string, string[]>;
  downstreamOf: Map<string, string[]>;
}

type Rule = (ctx: RuleContext) => DiagnosticIssue[];

function baseOf(n: WorkflowNode): WorkflowNodeBase {
  // SAFE: all WorkflowNode union variants extend WorkflowNodeBase at runtime
  return n as unknown as WorkflowNodeBase;
}

function nodeType(n: WorkflowNode): string {
  // SAFE: all WorkflowNode union variants have a `type` discriminant field
  return (n as unknown as { type: string }).type;
}

function configOf(n: WorkflowNode): Record<string, unknown> {
  // SAFE: all WorkflowNode union variants have a `config` property at runtime
  return (n as unknown as { config: Record<string, unknown> }).config ?? {};
}

function buildContext(nodes: WorkflowNode[], edges: WorkflowEdge[]): RuleContext {
  const nodeMap = new Map<string, WorkflowNode>();
  for (const n of nodes) { nodeMap.set(baseOf(n).id, n); }
  const upstreamOf = new Map<string, string[]>();
  const downstreamOf = new Map<string, string[]>();
  for (const e of edges) {
    const up = upstreamOf.get(e.target) ?? [];
    up.push(e.source);
    upstreamOf.set(e.target, up);
    const down = downstreamOf.get(e.source) ?? [];
    down.push(e.target);
    downstreamOf.set(e.source, down);
  }
  return { nodes, edges, nodeMap, upstreamOf, downstreamOf };
}

function issue(
  id: string,
  severity: DiagnosticSeverity,
  category: DiagnosticCategory,
  nodeIds: string[],
  autoFixable = false,
  fix?: DiagnosticFix,
): DiagnosticIssue {
  return {
    id,
    severity,
    category,
    titleKey: `workflow.diagnostic.issues.${id}.title`,
    messageKey: `workflow.diagnostic.issues.${id}.message`,
    nodeIds: nodeIds,
    autoFixable: autoFixable,
    fix,
  };
}

const RULE_NO_START_NODE: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  const hasTrigger = ctx.nodes.some((n) => nodeType(n) === "trigger");
  if (!hasTrigger && ctx.nodes.length > 0) {
    results.push(issue("no_trigger", "error", "structure", []));
  }
  return results;
};

const RULE_NO_END_NODE: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  const hasEnd = ctx.nodes.some((n) => nodeType(n) === "end");
  if (!hasEnd && ctx.nodes.length > 0) {
    results.push(issue("no_end", "warning", "structure", []));
  }
  return results;
};

// 容器节点类型：子节点通过 parentId 关联，不经过边（edge），因此孤立节点检查应跳过它们。
// 与 workflowLayout.ts 中的 CONTAINER_NODE_TYPES 保持一致。
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

// 装饰节点类型：阶段分隔线 / 分组框，仅用于编辑器视觉分区，不参与布局/校验/执行，
// 与 workflowLayout.ts 的 isLayoutExcluded 保持一致。它们天然无边，不应被误报为孤立节点。
const DECORATION_NODE_TYPES = new Set(["_phaseSeparator", "groupFrame"]);

const RULE_ORPHAN_NODES: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  const parentByNode = new Map<string, string>();
  for (const n of ctx.nodes) {
    const b = baseOf(n);
    if (b.parentId) { parentByNode.set(b.id, b.parentId); }
  }
  for (const n of ctx.nodes) {
    const id = baseOf(n).id;
    const type = nodeType(n);
    // 触发节点、容器节点、装饰节点均不参与孤立检查
    if (type === "trigger" || CONTAINER_NODE_TYPES.has(type) || DECORATION_NODE_TYPES.has(type)) { continue; }
    // 容器子节点通过 parentId 归属父容器（不经过边），跳过孤立检查
    if (parentByNode.has(id)) { continue; }
    const up = ctx.upstreamOf.get(id);
    const down = ctx.downstreamOf.get(id);
    if ((!up || up.length === 0) && (!down || down.length === 0)) {
      results.push(issue("orphan_node", "warning", "structure", [id]));
    }
  }
  return results;
};

const RULE_PROMPT_QUALITY: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  for (const n of ctx.nodes) {
    const type = nodeType(n);
    const cfg = configOf(n);
    const id = baseOf(n).id;
    if (type === "agent") {
      const sp = (cfg.systemPrompt as string) || "";
      if (!sp.trim()) {
        results.push(issue("agent_empty_prompt", "error", "prompt_quality", [id]));
      } else if (sp.length < 30) {
        results.push(issue("agent_short_prompt", "warning", "prompt_quality", [id]));
      }
      const maxTokens = cfg.maxTokens;
      if (!maxTokens) {
        results.push(issue("agent_no_max_tokens", "info", "cost", [id], true, {
          actionType: "set_node_field",
          nodeId: id,
          field: "maxTokens",
          value: 2048,
        }));
      }
      const maxRounds = cfg.maxToolRounds;
      if (cfg.tools && (cfg.tools as unknown[]).length > 0 && !maxRounds) {
        results.push(issue("agent_no_max_tool_rounds", "info", "cost", [id], true, {
          actionType: "set_node_field",
          nodeId: id,
          field: "maxToolRounds",
          value: 5,
        }));
      }
    }
    if (type === "llm") {
      const prompt = (cfg.prompt as string) || "";
      if (!prompt.trim()) {
        results.push(issue("llm_empty_prompt", "error", "prompt_quality", [id]));
      }
      if (!(cfg.maxTokens)) {
        results.push(issue("llm_no_max_tokens", "info", "cost", [id], true, {
          actionType: "set_node_field",
          nodeId: id,
          field: "maxTokens",
          value: 2048,
        }));
      }
    }
  }
  return results;
};

const RULE_PERFORMANCE: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  for (const n of ctx.nodes) {
    const type = nodeType(n);
    const cfg = configOf(n);
    const id = baseOf(n).id;
    const b = baseOf(n);
    if (type === "httpRequest") {
      const timeout = cfg.timeoutSecs as number | undefined;
      if (!timeout || timeout <= 0) {
        results.push(issue("http_no_timeout", "warning", "performance", [id], true, {
          actionType: "set_node_field",
          nodeId: id,
          field: "timeoutSecs",
          value: 30,
        }));
      }
      if (!b.retry?.enabled) {
        results.push(issue("http_no_retry", "info", "performance", [id], true, {
          actionType: "enable_retry",
          nodeId: id,
          maxRetries: 2,
        }));
      }
    }
    if (type === "databaseQuery") {
      const timeout = cfg.timeoutSecs as number | undefined;
      if (!timeout || timeout <= 0) {
        results.push(issue("db_no_timeout", "warning", "performance", [id], true, {
          actionType: "set_node_field",
          nodeId: id,
          field: "timeoutSecs",
          value: 30,
        }));
      }
    }
    if (type === "loop") {
      if (!cfg.maxIterations) {
        results.push(issue("loop_no_max_iter", "warning", "performance", [id], true, {
          actionType: "set_node_field",
          nodeId: id,
          field: "maxIterations",
          value: 100,
        }));
      }
      if (!cfg.continueCondition) {
        results.push(issue("loop_no_condition", "warning", "performance", [id]));
      }
    }
    if (type === "documentParser") {
      if (!cfg.parserType) {
        results.push(issue("doc_no_parser_type", "info", "performance", [id]));
      }
    }
  }
  return results;
};

const RULE_SECURITY: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  for (const n of ctx.nodes) {
    const type = nodeType(n);
    const cfg = configOf(n);
    const id = baseOf(n).id;
    if (type === "httpRequest" || type === "webhookSend") {
      const url = (cfg.url as string) || "";
      if (url && url.startsWith("http://")) {
        results.push(issue("insecure_http_url", "warning", "security", [id]));
      }
    }
    if (type === "notification") {
      const url = (cfg.webhookUrl as string) || "";
      if (url && url.startsWith("http://")) {
        results.push(issue("insecure_notification_url", "warning", "security", [id]));
      }
    }
    if (type === "approval") {
      if (!cfg.approver || (cfg.approver as string).trim() === "") {
        results.push(issue("approval_no_approver", "error", "security", [id]));
      }
    }
    if (type === "vectorRetrieve") {
      if (!cfg.similarityThreshold) {
        results.push(issue("vector_no_threshold", "info", "security", [id]));
      }
      const topK = cfg.topK as number | undefined;
      if (topK && topK > 20) {
        results.push(issue("vector_high_top_k", "info", "cost", [id]));
      }
    }
  }
  return results;
};

const RULE_BEST_PRACTICE: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  for (const n of ctx.nodes) {
    const type = nodeType(n);
    const cfg = configOf(n);
    const id = baseOf(n).id;
    if (type === "condition") {
      const down = ctx.downstreamOf.get(id) ?? [];
      if (down.length < 2) {
        results.push(issue("condition_single_exit", "warning", "best_practice", [id]));
      }
    }
    if (type === "llmClassifier") {
      const cases = (cfg.cases as unknown[]) || [];
      if (cases.length < 2) {
        results.push(issue("classifier_few_cases", "info", "best_practice", [id]));
      }
    }
    if (type === "validation") {
      const rules = (cfg.rules as unknown[]) || [];
      if (rules.length === 0) {
        results.push(issue("validation_no_rules", "warning", "best_practice", [id]));
      }
    }
  }
  return results;
};

const RULE_REFERENCE: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  const knownIds = new Set(ctx.nodes.map((n) => baseOf(n).id));
  for (const e of ctx.edges) {
    if (!knownIds.has(e.source)) {
      results.push(issue("edge_dangling_source", "error", "reference", [e.source], true, {
        actionType: "delete_edge",
        edgeId: e.id,
      }));
    }
    if (!knownIds.has(e.target)) {
      results.push(issue("edge_dangling_target", "error", "reference", [e.target], true, {
        actionType: "delete_edge",
        edgeId: e.id,
      }));
    }
  }
  return results;
};

const RULE_DEBATE_STRUCTURE: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  for (const n of ctx.nodes) {
    const type = nodeType(n);
    const cfg = configOf(n);
    const id = baseOf(n).id;
    if (type === "debate") {
      const debaterSteps = (cfg.debaterSteps as string[]) || [];
      if (debaterSteps.length === 0) {
        results.push(issue("debate_no_debaters", "warning", "structure", [id]));
      } else if (debaterSteps.length < 2) {
        results.push(issue("debate_single_debater", "warning", "structure", [id]));
      }
      for (const stepId of debaterSteps) {
        if (!ctx.nodeMap.has(stepId)) {
          results.push(issue("debate_dangling_step", "error", "reference", [id], true, {
            actionType: "remove_debater_step",
            nodeId: id,
            stepId: stepId,
          }));
        }
      }
    }
  }
  return results;
};

const ALL_RULES: Rule[] = [
  RULE_NO_START_NODE,
  RULE_NO_END_NODE,
  RULE_ORPHAN_NODES,
  RULE_PROMPT_QUALITY,
  RULE_PERFORMANCE,
  RULE_SECURITY,
  RULE_BEST_PRACTICE,
  RULE_REFERENCE,
  RULE_DEBATE_STRUCTURE,
];

export function runDiagnosticRules(nodes: WorkflowNode[], edges: WorkflowEdge[]): DiagnosticReport {
  const t0 = performance.now();
  const ctx = buildContext(nodes, edges);
  const allIssues: DiagnosticIssue[] = [];
  for (const rule of ALL_RULES) {
    const issues = rule(ctx);
    allIssues.push(...issues);
  }
  const seen = new Set<string>();
  const deduped = allIssues.filter((iss) => {
    const key = `${iss.id}:${iss.nodeIds.join(",")}`;
    if (seen.has(key)) { return false; }
    seen.add(key);
    return true;
  });
  const summary = { error: 0, warning: 0, info: 0 };
  for (const iss of deduped) { summary[iss.severity]++; }
  return {
    issues: deduped,
    summary,
    generatedAt: Date.now(),
    durationMs: Math.round(performance.now() - t0),
  };
}
