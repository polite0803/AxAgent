import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';
import { handleCommand } from './browserMock';

export type UnlistenFn = () => void;

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    try {
      return await tauriInvoke<T>(cmd, args);
    } catch (e) {
      const msg = String(e);
      // IPC connection refused — backend not ready or shutting down
      if (msg.includes('ERR_CONNECTION_REFUSED') || msg.includes('Failed to fetch')) {
        throw new Error(`IPC connection failed for "${cmd}": backend may not be ready or is shutting down. Please try again.`);
      }
      throw e;
    }
  }
  return handleCommand<T>(cmd, args);
}

export async function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  if (isTauri()) {
    return tauriListen<T>(event, handler);
  }
  // Browser mode: no-op listener
  return () => {};
}
