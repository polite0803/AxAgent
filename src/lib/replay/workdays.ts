import type { Dayjs } from "dayjs";

/**
 * 工作日范围(回放 sweep 用)
 */
export interface WorkdayRange {
  start: Dayjs;
  end: Dayjs;
}

/**
 * 把日期范围展开为按工作日(Mon-Fri)排好序的 "YYYY-MM-DD" 列表。
 *
 * - 不含 start/end 之外的日期
 * - 不调用真实交易所日历(避免 30 年国假日表过重)
 *   spec §5 Step 8 注:"可勾选"按工作日""
 */
export function buildAsOfDateRange(range: WorkdayRange): string[] {
  const out: string[] = [];
  if (!range.start.isValid() || !range.end.isValid()) { return out; }
  if (range.end.isBefore(range.start, "day")) { return out; }
  let cursor = range.start.startOf("day");
  const end = range.end.startOf("day");
  // 防御性:最多 366 天,避免意外死循环
  for (let i = 0; i < 366 && !cursor.isAfter(end, "day"); i++) {
    const dow = cursor.day(); // 0 = Sun, 6 = Sat
    if (dow !== 0 && dow !== 6) {
      out.push(cursor.format("YYYY-MM-DD"));
    }
    cursor = cursor.add(1, "day");
  }
  return out;
}
