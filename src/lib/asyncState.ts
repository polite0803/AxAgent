// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 共享异步状态工具，消除 Zustand store 中重复的 loading/error 模式。
 *
 * 用法（替代手写 try/catch + set({ loading, error })）：
 *   await withAsync(set, async () => {
 *     const data = await invoke("some_command");
 *     set({ data });
 *   });
 */

export interface AsyncState {
  loading: boolean;
  error: string | null;
}

export const INIT_ASYNC: AsyncState = { loading: false, error: null };

type SetFn = (partial: Record<string, unknown>) => void;

export function startAsync(set: SetFn): void {
  set({ loading: true, error: null });
}

export function failAsync(set: SetFn, error: unknown): void {
  set({ loading: false, error: String(error) });
}

export function doneAsync(set: SetFn): void {
  set({ loading: false });
}

export async function withAsync(
  set: SetFn,
  fn: () => Promise<void>,
): Promise<void> {
  startAsync(set);
  try {
    await fn();
    doneAsync(set);
  } catch (e) {
    failAsync(set, e);
  }
}
