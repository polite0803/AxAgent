import { useTracerStore } from "@/stores/devtools/tracerStore";
import type { SpanTreeNode, SpanType } from "@/types/tracer";
import { Tag, Tooltip, Typography } from "antd";
import { useState } from "react";

const { Text } = Typography;

interface SpanTreeProps {
  spans: SpanTreeNode[];
  level?: number;
}

function getSpanTypeColor(type_: SpanType): string {
  switch (type_) {
    case "agent":
      return "blue";
    case "tool":
      return "green";
    case "llm_call":
      return "purple";
    case "task":
      return "orange";
    case "sub_task":
      return "cyan";
    case "reflection":
      return "magenta";
    case "reasoning":
      return "gold";
    default:
      return "default";
  }
}

function formatDuration(ms?: number): string {
  if (!ms) { return "-"; }
  if (ms < 1000) { return `${ms}ms`; }
  return `${(ms / 1000).toFixed(2)}s`;
}

interface SpanNodeProps {
  node: SpanTreeNode;
  level: number;
  isSelected: boolean;
  onSelect: () => void;
}

function SpanNode({ node, level, isSelected, onSelect }: SpanNodeProps) {
  const [isExpanded, setIsExpanded] = useState(level < 3);
  const hasChildren = node.children.length > 0;

  return (
    <div className="mb-1">
      <div
        className={`flex items-center p-2 rounded cursor-pointer transition-colors ${
          isSelected ? "bg-blue-100" : "hover:bg-gray-50"
        }`}
        style={{ paddingLeft: `${level * 20 + 8}px` }}
        onClick={onSelect}
      >
        {hasChildren
          ? (
            <span
              className="mr-1 cursor-pointer text-gray-400 hover:text-gray-600"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
            >
              {isExpanded ? "▼" : "▶"}
            </span>
          )
          : <span className="mr-1 w-3 inline-block" />}
        <Tag
          color={getSpanTypeColor(node.span_type)}
          className="mr-2 text-xs"
        >
          {node.span_type.replace("_", " ")}
        </Tag>
        <Text className="flex-1 truncate">{node.name}</Text>
        <Tooltip title={`Duration: ${formatDuration(node.duration_ms)}`}>
          <Text type="secondary" className="text-xs ml-2">
            {formatDuration(node.duration_ms)}
          </Text>
        </Tooltip>
        {node.errors.length > 0 && (
          <Tag color="red" className="ml-2 text-xs">
            {node.errors.length} errors
          </Tag>
        )}
      </div>
      {isExpanded
        && hasChildren
        && node.children.map((child) => (
          <SpanNode
            key={child.id}
            node={child}
            level={level + 1}
            isSelected={false}
            onSelect={() => {}}
          />
        ))}
    </div>
  );
}

export function SpanTree({ spans }: SpanTreeProps) {
  const { selectedSpan, selectSpan } = useTracerStore();

  return (
    <div className="py-2">
      <div className="text-sm font-medium text-gray-600 mb-2 px-2">
        调用链 ({spans.length} spans)
      </div>
      {spans.map((span) => (
        <SpanNode
          key={span.id}
          node={span}
          level={0}
          isSelected={selectedSpan?.id === span.id}
          onSelect={() => selectSpan(span.id)}
        />
      ))}
    </div>
  );
}
