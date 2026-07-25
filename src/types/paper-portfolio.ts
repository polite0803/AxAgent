// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G2 模拟观察组合（Paper Trading Portfolio）前端类型定义
 *
 * 与后端 `axagent_entities::paper_portfolios::Model` /
 * `axagent_entities::paper_positions::Model` /
 * `axagent_stock_analysis::paper_portfolio::{PortfolioDetail, PositionWithPnl, PortfolioSummary}` 对齐。
 * 后端使用 #[serde(rename_all = "camelCase")]，前端类型按 camelCase 命名。
 */

/** 模拟组合主表（一个组合 = 一次研究观察） */
export interface PaperPortfolio {
  id: string;
  name: string;
  /** 来源事件描述（如 "英伟达隔夜大跌"） */
  sourceEvent: string;
  /** 关联 news_archive.id（可空） */
  sourceNewsId?: string | null;
  /** 关联 screenshot_diagnoses.id（G6 用，可空） */
  sourceScreenshotDiagnosisId?: string | null;
  /** active / closed / archived */
  status: "active" | "closed" | "archived";
  /** 创建时间戳（ms） */
  createdAt: number;
  /** 关闭时间戳（ms，可空） */
  closedAt?: number | null;
}

/** 模拟组合内的虚拟持仓 */
export interface PaperPosition {
  id: string;
  portfolioId: string;
  symbol: string;
  /** 市场：A / US / HK / ETF */
  market: "A" | "US" | "HK" | "ETF" | string;
  entryPrice: number;
  /** YYYY-MM-DD */
  entryDate: string;
  quantity: number;
  /** 虚拟平仓价（可空） */
  exitPrice?: number | null;
  /** 虚拟平仓日（可空） */
  exitDate?: string | null;
  /** open / closed */
  status: "open" | "closed";
  /** 备注（如 "AI 算力链"） */
  note?: string | null;
  createdAt: number;
  updatedAt: number;
}

/** 单个持仓 + 实时盈亏 */
export interface PositionWithPnl extends PaperPosition {
  /** 最新价（实时拉取，可空表示拉取失败） */
  currentPrice?: number | null;
  /** 浮动盈亏（元）—— 仅 open 持仓计算 */
  unrealizedPnl?: number | null;
  /** 浮动盈亏（百分比） */
  unrealizedPnlPct?: number | null;
  /** 已实现盈亏（元）—— 仅 closed 持仓计算 */
  realizedPnl?: number | null;
  /** 已实现盈亏（百分比） */
  realizedPnlPct?: number | null;
}

/** 组合汇总指标 */
export interface PortfolioSummary {
  positionCount: number;
  openCount: number;
  closedCount: number;
  /** 总成本 */
  totalCost: number;
  /** 当前总市值 */
  totalMarketValue: number;
  /** 总浮动盈亏（元） */
  totalUnrealizedPnl: number;
  /** 总已实现盈亏（元） */
  totalRealizedPnl: number;
  /** 总收益率（百分比，基于 totalCost） */
  totalReturnPct: number;
}

/** 组合详情（含持仓 + 实时盈亏） */
export interface PortfolioDetail extends PaperPortfolio {
  positions: PositionWithPnl[];
  summary: PortfolioSummary;
}

// ── 命令入参 DTO ────────────────────────────────────────────────────────

export interface CreatePortfolioInput {
  name: string;
  sourceEvent: string;
  sourceNewsId?: string | null;
  sourceScreenshotDiagnosisId?: string | null;
}

export interface AddPositionInput {
  portfolioId: string;
  symbol: string;
  /** 默认 "A" */
  market?: string;
  entryPrice: number;
  /** YYYY-MM-DD */
  entryDate: string;
  quantity: number;
  note?: string | null;
}

export interface ClosePositionInput {
  positionId: string;
  exitPrice: number;
  /** YYYY-MM-DD */
  exitDate: string;
}
