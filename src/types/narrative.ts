// SPDX-License-Identifier: AGPL-3.0-only
// 叙事结构类型定义 —— 与后端 axagent-harness::narrative 一一对应

// ── 叙事结构总览 ──

/** 叙事结构总览：包含所有弧线、交汇点和伏笔的完整定义 */
export interface NarrativeStructure {
  arcs: NarrativeArc[];
  confluences: ConfluencePoint[];
  foreshadows: Foreshadow[];
}

// ── 角色/主题弧线 ──

/** 弧线类型 */
export type ArcType =
  | "transformative" // 转换型：角色经历根本性变化
  | "steadfast" // 坚定型：角色坚守信念并获得成长
  | "flat" // 扁平型：角色没有显著变化
  | "tragic" // 悲剧型：角色走向毁灭
  | "comedic"; // 喜剧型：角色走向圆满

/** 弧线阶段 */
export interface ArcStage {
  name: string;
  chapter: number;
  description: string;
}

/** 角色/主题弧线 */
export interface NarrativeArc {
  id: string;
  arcType: ArcType;
  subject: string;
  want: string;
  need: string;
  stages: ArcStage[];
  currentProgress: number; // 0-100
}

// ── 交汇点 ──

/** 交汇点类型 */
export type ConfluenceType =
  | "conflict_burst" // 冲突爆发
  | "reveal_truth" // 真相揭示
  | "shift_perspective"; // 视角转换

/** 交汇点：多条线索/弧线在此汇聚、冲突或转折 */
export interface ConfluencePoint {
  id: string;
  triggerChapter: number;
  confluenceType: ConfluenceType;
  involvedArcs: string[];
  involvedForeshadows: string[];
  impact: string;
}

// ── 伏笔 ──

/** 伏笔状态 */
export type ForeshadowStatus =
  | "setup" // 已埋设
  | "payoff" // 已回收
  | "abandoned"; // 已废弃

/** 伏笔：追踪"埋设"与"回收"的完整生命周期 */
export interface Foreshadow {
  id: string;
  setupChapter: number;
  payoffChapter?: number | null;
  status: ForeshadowStatus;
  description: string;
  payoffDescription?: string | null;
  relatedArcs: string[];
}

// ── 章节结构指令 ──

/** 弧线指令 */
export interface ArcInstruction {
  arcId: string;
  arcType: ArcType;
  stageName: string;
  stageDescription: string;
}

/** 伏笔动作 */
export type ForeshadowAction =
  | "setup" // 埋设伏笔
  | "payoff"; // 回收伏笔

/** 伏笔指令 */
export interface ForeshadowInstruction {
  foreshadowId: string;
  action: ForeshadowAction;
  description: string;
}

/** 章节结构指令：为单章创作提供叙事约束 */
export interface ChapterStructureInstruction {
  chapter: number;
  arcInstructions: ArcInstruction[];
  foreshadowInstructions: ForeshadowInstruction[];
  confluenceTriggers: ConfluencePoint[];
}

// ── 结构合规性检查 ──

/** 结构合规性检查结果 */
export interface StructureComplianceReport {
  chapter: number;
  complianceScore: number; // 0-100
  arcCompliance: number;
  foreshadowCompliance: number;
  confluenceCompliance: number;
  deviations: StructureDeviation[];
  suggestions: string[];
}

/** 结构偏差 */
export interface StructureDeviation {
  deviationType: DeviationType;
  description: string;
  affectedElement: string;
  severity: DeviationSeverity;
}

/** 偏差类型 */
export type DeviationType =
  | "arc_deviation" // 弧线推进偏离
  | "foreshadow_setup_missed" // 伏笔未按时埋设
  | "foreshadow_payoff_missed" // 伏笔未按时回收
  | "confluence_missed" // 交汇点未触发
  | "pacing_issue"; // 叙事节奏问题

/** 偏差严重程度 */
export type DeviationSeverity = "low" | "medium" | "high" | "critical";

// ── 动态调整建议 ──

/** 调整目标类型 */
export type AdjustmentTargetType = "arc" | "foreshadow" | "confluence";

/** 结构调整建议 */
export interface StructureAdjustmentSuggestion {
  id: string;
  adjustmentType: AdjustmentType;
  description: string;
  affectedElements: string[];
  priority: AdjustmentPriority;
  rationale: string;
  /** 调整目标类型 */
  targetType?: AdjustmentTargetType;
  /** 调整目标 ID */
  targetId?: string;
  /** 调整负载数据（如新增的阶段、伏笔等） */
  payload?: unknown;
}

/** 调整类型 */
export type AdjustmentType =
  | "delay_foreshadow_payoff" // 延后伏笔回收
  | "accelerate_foreshadow_payoff" // 提前伏笔回收
  | "add_arc_stage" // 增加弧线阶段
  | "adjust_arc_progress" // 调整弧线推进度
  | "reposition_confluence" // 移动交汇点
  | "add_foreshadow"; // 增加新伏笔

/** 调整优先级 */
export type AdjustmentPriority = "low" | "medium" | "high" | "critical";

// ── KPI 指标（前端视图） ──

/** 叙事结构相关 KPI 指标 */
export interface NarrativeKpiMetrics {
  arcCompletion: number; // 弧线完成度 0-100
  foreshadowRecoveryRate: number; // 伏笔回收率 0-100
  structureCompliance: number; // 结构遵循率 0-100
  totalArcs: number;
  totalForeshadows: number;
  totalConfluences: number;
  completedChapters: number;
  totalChapters: number;
}

// ── 可视化辅助类型 ──

/** 弧线时间线节点 */
export interface ArcTimelineNode {
  id: string;
  arcId: string;
  stageName: string;
  chapter: number;
  description: string;
  isCompleted: boolean;
  isCurrent: boolean;
}

/** 伏笔关系图节点 */
export interface ForeshadowGraphNode {
  id: string;
  label: string;
  type: "setup" | "payoff" | "confluence";
  chapter: number;
  status: ForeshadowStatus;
  description: string;
}

/** 伏笔关系图边 */
export interface ForeshadowGraphEdge {
  source: string;
  target: string;
  type: "setup_to_payoff" | "related_arc" | "confluence_trigger";
}

// ── 章节元信息 ──

/** 章节元数据 */
export interface ChapterMeta {
  number: number;
  title: string;
  summary?: string;
  wordCount?: number;
  status: "draft" | "in_revision" | "final";
}
