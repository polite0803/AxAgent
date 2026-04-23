function formatCompact(value: number, divisor: number, suffix: string): string {
  const compact = Math.round((value / divisor) * 10) / 10;
  const formatted = Number.isInteger(compact) ? compact.toFixed(0) : compact.toFixed(1);
  return `${formatted}${suffix}`;
}

export function formatTokenCount(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1_000_000) return formatCompact(value, 1_000_000, 'm');
  if (abs >= 1_000) return formatCompact(value, 1_000, 'k');
  return value.toLocaleString();
}

/** Format milliseconds into human-readable duration: 100ms, 1.1s, 1m2s */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60_000) {
    const s = ms / 1000;
    return `${Number.isInteger(s) ? s.toFixed(0) : s.toFixed(1)}s`;
  }
  const m = Math.floor(ms / 60_000);
  const s = Math.round((ms % 60_000) / 1000);
  return s > 0 ? `${m}m${s}s` : `${m}m`;
}

/** Format tokens per second: 28.5 tok/s */
export function formatSpeed(tps: number): string {
  return `${tps < 10 ? tps.toFixed(1) : Math.round(tps)} tok/s`;
}
