// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: NL2Workflow — 自然语言驱动工作流类型定义

/** 节点类型枚举 */
export type NodeType = 'trigger' | 'action' | 'condition' | 'loop' | 'parallel' | 'subflow' | 'output';

/** 单个节点定义 */
export interface WorkflowNode {
  id: string;
  type: NodeType;
  label: string;
  description?: string;
  config: Record<string, unknown>;
  position: { x: number; y: number };
  inputs?: string[];
  outputs?: string[];
}

/** 边 */
export interface WorkflowEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  condition?: string;
}

/** 完整工作流定义 */
export interface WorkflowDefinition {
  id: string;
  name: string;
  description: string;
  version: number;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  variables: Record<string, unknown>;
  createdAt: number;
  updatedAt: number;
  status: 'draft' | 'active' | 'archived';
}

/** 自然语言解析请求 */
export interface NLParseRequest {
  prompt: string;
  context?: string;
  constraints?: string[];
}

/** 自然语言解析结果 */
export interface NLParseResult {
  workflow: WorkflowDefinition;
  confidence: number;
  suggestions: string[];
  alternatives?: WorkflowDefinition[];
}

/** 工作流模板 */
export interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  category: 'data-processing' | 'notification' | 'content-generation' | 'monitoring' | 'integration';
  nodeCount: number;
  tags: string[];
  workflow: WorkflowDefinition;
  isBuiltIn: boolean;
  createdAt: number;
  updatedAt: number;
}

/** 工作流执行状态 */
export type WorkflowExecutionStatus = 'idle' | 'running' | 'paused' | 'completed' | 'failed';

/** 节点执行状态 */
export interface NodeExecutionState {
  nodeId: string;
  status: 'waiting' | 'running' | 'success' | 'failed';
  startedAt?: number;
  finishedAt?: number;
  output?: unknown;
  error?: string;
}

/** 工作流执行记录 */
export interface WorkflowExecution {
  id: string;
  workflowId: string;
  status: WorkflowExecutionStatus;
  startedAt: number;
  finishedAt?: number;
  nodeStates: NodeExecutionState[];
  inputs: Record<string, unknown>;
  outputs?: Record<string, unknown>;
  logs: ExecutionLogEntry[];
}

/** 执行日志条目 */
export interface ExecutionLogEntry {
  timestamp: number;
  nodeId: string;
  nodeName: string;
  level: 'info' | 'warn' | 'error';
  message: string;
}

/** 工作流版本记录 */
export interface WorkflowVersion {
  version: number;
  updatedAt: number;
  summary: string;
  status: 'draft' | 'active' | 'archived';
  snapshot: WorkflowDefinition;
}

/** 版本对比结果 */
export interface VersionDiff {
  addedNodes: WorkflowNode[];
  removedNodes: WorkflowNode[];
  modifiedNodes: { before: WorkflowNode; after: WorkflowNode }[];
  addedEdges: WorkflowEdge[];
  removedEdges: WorkflowEdge[];
  modifiedEdges: { before: WorkflowEdge; after: WorkflowEdge }[];
}

/** 工作流筛选 */
export interface WorkflowFilter {
  search?: string;
  status?: 'all' | 'draft' | 'active' | 'archived';
  category?: WorkflowTemplate['category'] | 'all';
}

// ============================================================
// Phase 4: NL2Skill — 自然语言→技能定义
// ============================================================

/** NL2Skill 解析请求 */
export interface NL2SkillRequest {
  prompt: string;
  context?: string;
  skillType?: "chat" | "tool" | "workflow" | "automation";
}

/** 技能定义 */
export interface SkillDefinition {
  id: string;
  name: string;
  description: string;
  type: "chat" | "tool" | "workflow" | "automation";
  triggers: string[];
  prompt_template: string;
  parameters: SkillParameter[];
  tools: string[];
  icon?: string;
  tags?: string[];
}

export interface SkillParameter {
  name: string;
  type: "string" | "number" | "boolean" | "enum" | "object";
  description: string;
  required: boolean;
  default?: unknown;
  options?: string[];
}

/** NL2Skill 解析阶段 */
export interface NL2SkillPhase {
  phase: string;
  status: "pending" | "running" | "done";
  detail: string;
}

/** NL2Skill 解析结果 */
export interface NL2SkillResult {
  skill: SkillDefinition;
  confidence: number;
  phases: NL2SkillPhase[];
  suggestions: string[];
  alternatives?: SkillDefinition[];
}

// ============================================================
// Phase 4: NL2UI — 自然语言→动态 UI Schema
// ============================================================

/** NL2UI 解析请求 */
export interface NL2UIRequest {
  prompt: string;
  context?: string;
  uiType?: "form" | "dashboard" | "settings" | "report" | "custom";
}

/** NL2UI 解析阶段 */
export interface NL2UIPhase {
  phase: string;
  status: "pending" | "running" | "done";
  detail: string;
}

/** NL2UI 解析结果 */
export interface NL2UIResult {
  schema: UISchema;
  confidence: number;
  phases: NL2UIPhase[];
  suggestions: string[];
  alternatives?: { schema: UISchema; description: string }[];
}

// UISchema 类型从 @/types/dynamicUI 导入，此处仅做 forward-declaration
import type { UISchema } from "./dynamicUI";
