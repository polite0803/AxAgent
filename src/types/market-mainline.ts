// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: 市场主线类型定义（含后端数据值枚举，如主题大类中文标识），类型定义非 UI 文案。
/**
 * G4 市场主线（Market Mainline）前端类型定义
 *
 * 与后端 `axagent_entities::market_mainlines::Model` /
 * `axagent_stock_analysis::market_mainline::{CreateMainlineInput, UpdateMainlineInput,
 * BatchUpsertInput, BatchUpsertResult}` 对齐。
 * 后端使用 #[serde(rename_all = "camelCase")]，前端类型按 camelCase 命名。
 */

/** 主题大类枚举 */
export type ThemeCategory =
  | "科技"
  | "消费"
  | "周期"
  | "金融"
  | "医药"
  | "政策"
  | "其他"
  | string;

/** 持续性判断 */
export type Persistence = "1d" | "1w" | "1m" | "fading" | "emerging" | string;

/** 主线状态 */
export type MainlineStatus = "active" | "fading" | "archived";

/** 市场主线记录 */
export interface MarketMainline {
  id: string;
  /** 主线日期 YYYY-MM-DD */
  mainlineDate: string;
  /** 主题名（如 "AI 算力"） */
  theme: string;
  /** 主题大类 */
  themeCategory: ThemeCategory;
  /** 主线叙述（1-2 句话故事线） */
  narrative: string;
  /** 代表性标的 JSON 字符串（如 '["600519","000858"]'） */
  representativeSymbols: string;
  /** 强度评分 0-100 */
  strengthScore: number;
  /** 持续性判断 */
  persistence: Persistence;
  /** 证据 JSON 字符串 */
  evidenceJson: string;
  /** 来源工作流执行 ID（可空） */
  sourceWorkflowExecutionId?: string | null;
  /** 主线状态 */
  status: MainlineStatus;
  createdAt: number;
  updatedAt: number;
}

// ── 命令入参 DTO ────────────────────────────────────────────────────────

export interface CreateMainlineInput {
  mainlineDate: string;
  theme: string;
  themeCategory?: ThemeCategory;
  narrative: string;
  representativeSymbols?: string[];
  strengthScore?: number;
  persistence?: Persistence;
  evidence?: Record<string, unknown>;
  sourceWorkflowExecutionId?: string | null;
}

export interface UpdateMainlineInput {
  mainlineId: string;
  status?: MainlineStatus;
  strengthScore?: number;
  persistence?: Persistence;
  narrative?: string;
}

export interface BatchUpsertInput {
  mainlineDate: string;
  mainlines: CreateMainlineInput[];
  /** 是否清除当日已有但本次未提及的主线（true → status=archived） */
  archiveMissing?: boolean;
  sourceWorkflowExecutionId?: string | null;
}

export interface BatchUpsertResult {
  inserted: number;
  updated: number;
  archived: number;
}

// ── 工具函数 ────────────────────────────────────────────────────────────

/** 解析 representativeSymbols JSON 字符串为代码数组 */
export function parseRepresentativeSymbols(json: string): string[] {
  if (!json) { return []; }
  try {
    const arr = JSON.parse(json);
    return Array.isArray(arr) ? arr.map(String) : [];
  } catch {
    return [];
  }
}

/** 解析 evidenceJson 为对象 */
export function parseEvidence(json: string): Record<string, unknown> {
  if (!json) { return {}; }
  try {
    const obj = JSON.parse(json);
    return typeof obj === "object" && obj !== null ? obj : {};
  } catch {
    return {};
  }
}
