// SPDX-License-Identifier: AGPL-3.0-only

export function IpcReconnectBanner({ healthy }: { healthy: boolean }) {
  if (healthy) { return null; }
  return null;
}
