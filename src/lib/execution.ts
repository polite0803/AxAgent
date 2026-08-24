// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: 交易执行桥接 API 封装（含后端命令名/事件名等技术常量），API 层非 UI 文案。

import type {
  ConfirmPendingParams,
  ExecutionConfirmedEvent,
  ExecutionFilledEvent,
  ExecutionMode,
  ExecutionPendingEvent,
  ExecutionRejectedEvent,
  ExecutionRiskRejectedEvent,
  PendingExecution,
  RejectPendingParams,
  SetExecutionModeParams,
  SubmitSignalParams,
} from "@/types";
import { invoke, listen } from "./invoke";

// ── Tauri 事件名 ──

export const EXECUTION_EVENTS = {
  PENDING: "execution-pending",
  FILLED: "execution-filled",
  CONFIRMED: "execution-confirmed",
  REJECTED: "execution-rejected",
  RISK_REJECTED: "execution-risk-rejected",
} as const;

// ── 命令调用封装 ──

/**
 * 提交交易信号到执行管道
 */
export async function submitSignal(params: SubmitSignalParams): Promise<string> {
  return invoke<string>("execution_submit_signal", {
    signal_code: params.signalCode,
    signal_action: params.signalAction,
    signal_reason: params.signalReason,
    stock_name: params.stockName,
    current_price: params.currentPrice,
  });
}

/**
 * 确认待执行交易
 */
export async function confirmPending(
  params: ConfirmPendingParams,
): Promise<string> {
  return invoke<string>("execution_confirm", {
    pending_id: params.pendingId,
    quantity: params.quantity,
  });
}

/**
 * 驳回待执行交易
 */
export async function rejectPending(params: RejectPendingParams): Promise<void> {
  return invoke<void>("execution_reject", {
    pending_id: params.pendingId,
    reason: params.reason,
  });
}

/**
 * 列出所有待执行记录
 */
export async function listPending(): Promise<PendingExecution[]> {
  return invoke<PendingExecution[]>("execution_list_pending");
}

/**
 * 设置执行模式
 */
export async function setExecutionMode(
  params: SetExecutionModeParams,
): Promise<void> {
  return invoke<void>("execution_set_mode", {
    mode: params.mode,
  });
}

/**
 * 获取当前执行模式
 */
export async function getExecutionMode(): Promise<ExecutionMode> {
  return invoke<ExecutionMode>("execution_get_mode");
}

// ── 事件监听辅助函数 ──

/**
 * 监听 execution-pending 事件（信号进入待确认队列）
 */
export async function onExecutionPending(
  handler: (payload: ExecutionPendingEvent) => void,
) {
  return listen<ExecutionPendingEvent>(EXECUTION_EVENTS.PENDING, (event) => {
    handler(event.payload);
  });
}

/**
 * 监听 execution-filled 事件（交易已执行）
 */
export async function onExecutionFilled(
  handler: (payload: ExecutionFilledEvent) => void,
) {
  return listen<ExecutionFilledEvent>(EXECUTION_EVENTS.FILLED, (event) => {
    handler(event.payload);
  });
}

/**
 * 监听 execution-confirmed 事件（待执行交易已确认）
 */
export async function onExecutionConfirmed(
  handler: (payload: ExecutionConfirmedEvent) => void,
) {
  return listen<ExecutionConfirmedEvent>(EXECUTION_EVENTS.CONFIRMED, (event) => {
    handler(event.payload);
  });
}

/**
 * 监听 execution-rejected 事件（待执行交易已驳回）
 */
export async function onExecutionRejected(
  handler: (payload: ExecutionRejectedEvent) => void,
) {
  return listen<ExecutionRejectedEvent>(EXECUTION_EVENTS.REJECTED, (event) => {
    handler(event.payload);
  });
}

/**
 * 监听 execution-risk-rejected 事件（风控检查未通过）
 */
export async function onExecutionRiskRejected(
  handler: (payload: ExecutionRiskRejectedEvent) => void,
) {
  return listen<ExecutionRiskRejectedEvent>(
    EXECUTION_EVENTS.RISK_REJECTED,
    (event) => {
      handler(event.payload);
    },
  );
}
