// SPDX-License-Identifier: AGPL-3.0-only

// IPC 健康状态钩子 — 检测 WebSocket/后端连接状态
// 注：完整实现在后续远程同步中补充，当前为桩钩子

export function useIpcHealth(): { ipcHealthy: boolean } {
  return { ipcHealthy: true };
}
