import type { Span, SpanType } from "@/types/tracer";
import { Tag, Tooltip } from "antd";
import dayjs from "dayjs";

interface TimelineProps {
  spans: Span[];
}

function getSpanTypeColor(type_: SpanType): string {
  switch (type_) {
    case "agent":
      return "#1890ff";
    case "tool":
      return "#52c41a";
    case "llm_call":
      return "#722ed1";
    case "task":
      return "#fa8c16";
    case "sub_task":
      return "#13c2c2";
    case "reflection":
      return "#eb2f96";
    case "reasoning":
      return "#faad14";
    default:
      return "#d9d9d9";
  }
}

function formatDuration(ms?: number): string {
  if (!ms) return "-";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

export function Timeline({ spans }: TimelineProps) {
  const sortedSpans = [...spans].sort(
    (a, b) => new Date(a.start_time).getTime() - new Date(b.start_time).getTime()
  );

  const startTime = sortedSpans[0]?.start_time
    ? new Date(sortedSpans[0].start_time).getTime()
    : Date.now();
  const endTime =
    sortedSpans[sortedSpans.length - 1]?.end_time
      ? new Date(sortedSpans[sortedSpans.length - 1].end_time!).getTime()
      : Date.now();
  const totalDuration = endTime - startTime || 1;

  const getDepth = (span: Span): number => {
    let depth = 0;
    let current = span;
    while (current.parent_span_id) {
      depth++;
      current = spans.find((s) => s.id === current.parent_span_id) || current;
      if (depth > 20) break;
    }
    return depth;
  };

  return (
    <div className="p-4">
      <div className="text-sm font-medium text-gray-600 mb-4">
        时间线视图 ({spans.length} spans)
      </div>
      <div className="relative">
        <div className="absolute left-0 top-0 bottom-0 w-px bg-gray-300" />
        {sortedSpans.map((span) => {
          const spanStart = new Date(span.start_time).getTime();
          const spanEnd = span.end_time ? new Date(span.end_time).getTime() : spanStart + (span.duration_ms || 0);
          const left = ((spanStart - startTime) / totalDuration) * 100;
          const width = ((spanEnd - spanStart) / totalDuration) * 100;
          const depth = getDepth(span);

          return (
            <div
              key={span.id}
              className="relative mb-2"
              style={{ paddingLeft: `${depth * 20 + 8}px` }}
            >
              <div className="absolute w-2 h-2 rounded-full bg-gray-400 -left-1 top-2" />
              <Tooltip
                title={
                  <div>
                    <div>{span.name}</div>
                    <div>Start: {dayjs(span.start_time).format("HH:mm:ss.SSS")}</div>
                    <div>Duration: {formatDuration(span.duration_ms)}</div>
                    {span.errors.length > 0 && (
                      <div className="text-red-400">
                        Errors: {span.errors.map((e) => e.message).join(", ")}
                      </div>
                    )}
                  </div>
                }
              >
                <div
                  className="flex items-center h-8 rounded cursor-pointer transition-all hover:opacity-80"
                  style={{
                    marginLeft: `${left}%`,
                    width: `${Math.max(width, 1)}%`,
                    backgroundColor: getSpanTypeColor(span.span_type),
                    opacity: span.status === "error" ? 0.7 : 1,
                  }}
                >
                  <span className="ml-2 text-white text-xs truncate">
                    {span.name}
                  </span>
                  <span className="ml-auto pr-2 text-white text-xs">
                    {formatDuration(span.duration_ms)}
                  </span>
                </div>
              </Tooltip>
              {span.status === "error" && (
                <Tag color="red" className="ml-2 text-xs">
                  error
                </Tag>
              )}
            </div>
          );
        })}
      </div>
      <div className="flex justify-between text-xs text-gray-400 mt-4 px-2">
        <span>{dayjs(startTime).format("HH:mm:ss.SSS")}</span>
        <span>{dayjs(endTime).format("HH:mm:ss.SSS")}</span>
      </div>
    </div>
  );
}
