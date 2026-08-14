// SPDX-License-Identifier: AGPL-3.0-only

// 统一的大小 / 时长 / 时间格式化工具。
// 替换各组件中重复实现且口径不一致的 formatBytes / formatDuration / formatTime。

/** 字节数 → 人类可读大小（如 "0 B"、"1.5 MB"）。 */
export function formatBytes(n: number | null | undefined): string {
  if (n == null || n === 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (Math.abs(v) >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  const digits = i === 0 ? 0 : Math.abs(v) >= 100 ? 0 : 1;
  return `${v.toFixed(digits)} ${units[i]}`;
}

/** 毫秒 → 人类可读时长（如 "0ms"、"12.3s"、"3m 5s"、"1h 20m"）。 */
export function formatDuration(ms: number | null | undefined): string {
  if (ms == null || ms < 1) {
    return "0ms";
  }
  if (ms < 1000) {
    return `${Math.round(ms)}ms`;
  }
  const s = ms / 1000;
  if (s < 60) {
    return `${s.toFixed(s < 10 ? 1 : 0)}s`;
  }
  const m = Math.floor(s / 60);
  const rs = Math.round(s % 60);
  if (m < 60) {
    return rs === 0 ? `${m}m` : `${m}m ${rs}s`;
  }
  const h = Math.floor(m / 60);
  const rm = Math.round(m % 60);
  return `${h}h ${rm}m`;
}

/** 时间戳 / 日期 → "HH:mm"。非法输入返回 "-"。 */
export function formatTime(ts: number | string | Date | null | undefined): string {
  if (ts == null) {
    return "-";
  }
  const d = ts instanceof Date ? ts : new Date(ts);
  if (Number.isNaN(d.getTime())) {
    return "-";
  }
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  return `${hh}:${mm}`;
}
