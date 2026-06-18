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
  return n as unknown as WorkflowNodeBase;
}

function nodeType(n: WorkflowNode): string {
  return (n as unknown as { type: string }).type;
}

function configOf(n: WorkflowNode): Record<string, unknown> {
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
    title_key: `workflow.diagnostic.issues.${id}.title`,
    message_key: `workflow.diagnostic.issues.${id}.message`,
    node_ids: nodeIds,
    auto_fixable: autoFixable,
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

const RULE_ORPHAN_NODES: Rule = (ctx) => {
  const results: DiagnosticIssue[] = [];
  for (const n of ctx.nodes) {
    const id = baseOf(n).id;
    const type = nodeType(n);
    if (type === "trigger" || CONTAINER_NODE_TYPES.has(type)) { continue; }
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
      const sp = (cfg.system_prompt as string) || "";
      if (!sp.trim()) {
        results.push(issue("agent_empty_prompt", "error", "prompt_quality", [id]));
      } else if (sp.length < 30) {
        results.push(issue("agent_short_prompt", "warning", "prompt_quality", [id]));
      }
      if (!cfg.max_tokens) {
        results.push(issue("agent_no_max_tokens", "info", "cost", [id], true, {
          action_type: "set_node_field",
          node_id: id,
          field: "max_tokens",
          value: 2048,
        }));
      }
      if (cfg.tools && (cfg.tools as unknown[]).length > 0 && !cfg.max_tool_rounds) {
        results.push(issue("agent_no_max_tool_rounds", "info", "cost", [id], true, {
          action_type: "set_node_field",
          node_id: id,
          field: "max_tool_rounds",
          value: 5,
        }));
      }
    }
    if (type === "llm") {
      const prompt = (cfg.prompt as string) || "";
      if (!prompt.trim()) {
        results.push(issue("llm_empty_prompt", "error", "prompt_quality", [id]));
      }
      if (!cfg.max_tokens) {
        results.push(issue("llm_no_max_tokens", "info", "cost", [id], true, {
          action_type: "set_node_field",
          node_id: id,
          field: "max_tokens",
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
      const timeout = cfg.timeout_secs as number | undefined;
      if (!timeout || timeout <= 0) {
        results.push(issue("http_no_timeout", "warning", "performance", [id], true, {
          action_type: "set_node_field",
          node_id: id,
          field: "timeout_secs",
          value: 30,
        }));
      }
      if (!b.retry?.enabled) {
        results.push(issue("http_no_retry", "info", "performance", [id], true, {
          action_type: "enable_retry",
          node_id: id,
          max_retries: 2,
        }));
      }
    }
    if (type === "databaseQuery") {
      const timeout = cfg.timeout_secs as number | undefined;
      if (!timeout || timeout <= 0) {
        results.push(issue("db_no_timeout", "warning", "performance", [id], true, {
          action_type: "set_node_field",
          node_id: id,
          field: "timeout_secs",
          value: 30,
        }));
      }
    }
    if (type === "loop") {
      if (!cfg.max_iterations) {
        results.push(issue("loop_no_max_iter", "warning", "performance", [id], true, {
          action_type: "set_node_field",
          node_id: id,
          field: "max_iterations",
          value: 100,
        }));
      }
      if (!cfg.continue_condition) {
        results.push(issue("loop_no_condition", "warning", "performance", [id]));
      }
    }
    if (type === "documentParser") {
      if (!cfg.parser_type) {
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
      const url = (cfg.webhook_url as string) || "";
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
      if (!cfg.similarity_threshold) {
        results.push(issue("vector_no_threshold", "info", "security", [id]));
      }
      const topK = cfg.top_k as number | undefined;
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
        action_type: "delete_edge",
        edge_id: e.id,
      }));
    }
    if (!knownIds.has(e.target)) {
      results.push(issue("edge_dangling_target", "error", "reference", [e.target], true, {
        action_type: "delete_edge",
        edge_id: e.id,
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
      const debaterSteps = (cfg.debater_steps as string[]) || [];
      if (debaterSteps.length === 0) {
        results.push(issue("debate_no_debaters", "warning", "structure", [id]));
      } else if (debaterSteps.length < 2) {
        results.push(issue("debate_single_debater", "warning", "structure", [id]));
      }
      for (const stepId of debaterSteps) {
        if (!ctx.nodeMap.has(stepId)) {
          results.push(issue("debate_dangling_step", "error", "reference", [id], true, {
            action_type: "remove_debater_step",
            node_id: id,
            step_id: stepId,
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
    const key = `${iss.id}:${iss.node_ids.join(",")}`;
    if (seen.has(key)) { return false; }
    seen.add(key);
    return true;
  });
  const summary = { error: 0, warning: 0, info: 0 };
  for (const iss of deduped) { summary[iss.severity]++; }
  return {
    issues: deduped,
    summary,
    generated_at: Date.now(),
    duration_ms: Math.round(performance.now() - t0),
  };
}
