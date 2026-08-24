// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: 交易执行桥接类型定义（含后端枚举值，如执行模式、方向等中文标识），类型定义非 UI 文案。

/**
 * 执行模式
 * - manual: 手动确认模式
 * - semi_auto: 半自动模式（风控通过后自动执行）
 * - full_auto: 全自动模式（风控通过即执行）
 */
export type ExecutionMode = "manual" | "semi_auto" | "full_auto";

/**
 * 交易方向
 */
export type TradeDirection = "buy" | "sell" | "hold";

/**
 * 风控等级（执行桥接专用）
 */
export type ExecutionRiskLevel = "低" | "中" | "高";

/**
 * 待执行记录（前端展示用）
 */
export interface PendingExecution {
  /** 唯一 ID */
  id: string;
  /** 股票代码 */
  stockCode: string;
  /** 股票名称 */
  stockName: string;
  /** 交易方向 */
  direction: TradeDirection;
  /** 当前价格 */
  price: number;
  /** 数量（待确认时为 0） */
  quantity: number;
  /** 交易原因 */
  reason: string;
  /** 风控等级 */
  riskLevel: ExecutionRiskLevel;
  /** 风控警告信息 */
  riskWarning: string;
  /** 创建时间（时间戳毫秒） */
  createdAt: number;
  /** 状态 */
  status: string;
}

/**
 * 风控检查结果
 */
export interface RiskCheckResult {
  /** 是否通过 */
  passed: boolean;
  /** 风控等级 */
  riskLevel: ExecutionRiskLevel;
  /** 原因 */
  reason: string;
}

// ── Tauri 事件载荷类型 ──

/**
 * execution-pending 事件载荷
 */
export interface ExecutionPendingEvent {
  pendingId: string;
  stockCode: string;
  stockName: string;
  direction: TradeDirection;
  price: number;
  reason: string;
  riskLevel: ExecutionRiskLevel;
  riskWarning: string;
}

/**
 * execution-filled 事件载荷
 */
export interface ExecutionFilledEvent {
  tradeId: string;
  stockCode: string;
  action: TradeDirection;
}

/**
 * execution-confirmed 事件载荷
 */
export interface ExecutionConfirmedEvent {
  pendingId: string;
  tradeId: string;
  quantity: number;
}

/**
 * execution-rejected 事件载荷
 */
export interface ExecutionRejectedEvent {
  pendingId: string;
  reason: string;
}

/**
 * execution-risk-rejected 事件载荷
 */
export interface ExecutionRiskRejectedEvent {
  passed: boolean;
  riskLevel: ExecutionRiskLevel;
  reason: string;
}

// ── invoke 参数类型 ──

/**
 * execution_submit_signal 调用参数
 */
export interface SubmitSignalParams {
  /** 股票代码 */
  signalCode: string;
  /** 交易动作 */
  signalAction: string;
  /** 交易原因 */
  signalReason: string;
  /** 股票名称 */
  stockName: string;
  /** 当前价格 */
  currentPrice: number;
}

/**
 * execution_confirm 调用参数
 */
export interface ConfirmPendingParams {
  /** 待执行记录 ID */
  pendingId: string;
  /** 确认数量 */
  quantity: number;
}

/**
 * execution_reject 调用参数
 */
export interface RejectPendingParams {
  /** 待执行记录 ID */
  pendingId: string;
  /** 驳回原因 */
  reason: string;
}

/**
 * execution_set_mode 调用参数
 */
export interface SetExecutionModeParams {
  /** 执行模式 */
  mode: ExecutionMode;
}
