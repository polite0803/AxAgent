import { estimateMessageTokens } from "@/lib/tokenEstimator";
import { useConversationStore, useStreamStore } from "@/stores";
import { Tag, Tooltip } from "antd";
import { Database, FileText, Layers, Zap } from "lucide-react";
import React, { useMemo } from "react";

interface ContextSummaryPreviewProps {
  conversationId: string;
}

const EMPTY_ARRAY: never[] = [];

const ContextSummaryPreview: React.FC<ContextSummaryPreviewProps> = ({
  conversationId,
}) => {
  const isActive = useConversationStore((s) => s.activeConversationId === conversationId);
  const messages = useConversationStore((s) => isActive ? s.messages : EMPTY_ARRAY);
  const mode = useConversationStore((s) => {
    const conv = s.conversations.find((c) => c.id === conversationId);
    return conv?.mode || null;
  });
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const activeStreams = useStreamStore((s) => s.activeStreams);
  const streaming = activeConversationId ? (activeConversationId in activeStreams) : false;

  const summary = useMemo(() => {
    if (!messages || messages.length === 0) { return null; }

    let totalTokens = 0;
    let userMessages = 0;
    let assistantMessages = 0;
    let toolMessages = 0;
    let systemMessages = 0;

    for (const msg of messages) {
      const tokens = estimateMessageTokens(msg.role, msg.content || "");
      totalTokens += tokens;

      switch (msg.role) {
        case "user":
          userMessages++;
          break;
        case "assistant":
          assistantMessages++;
          break;
        case "tool":
          toolMessages++;
          break;
        case "system":
          systemMessages++;
          break;
      }
    }

    // Estimate context window usage (assuming 128K default)
    const contextWindow = 128_000;
    const usagePercent = Math.min(100, (totalTokens / contextWindow) * 100);
    const isNearLimit = usagePercent > 70;
    const isOverLimit = usagePercent > 90;

    return {
      totalMessages: messages.length,
      userMessages,
      assistantMessages,
      toolMessages,
      systemMessages,
      totalTokens,
      usagePercent,
      isNearLimit,
      isOverLimit,
    };
  }, [messages]);

  if (!summary || mode !== "agent" || streaming) { return null; }

  const formatTokens = (n: number) => {
    if (n >= 1000) { return `${(n / 1000).toFixed(1)}K`; }
    return String(n);
  };

  return (
    <div className="flex items-center gap-2 px-3 py-1 text-xs text-gray-500 dark:text-gray-400 select-none">
      {/* Message count */}
      <Tooltip
        title={`${summary.userMessages} user, ${summary.assistantMessages} assistant, ${summary.toolMessages} tool`}
      >
        <div className="flex items-center gap-1">
          <FileText size={11} />
          <span>{summary.totalMessages} msgs</span>
        </div>
      </Tooltip>

      {/* Token count */}
      <Tooltip title={`Estimated ${formatTokens(summary.totalTokens)} tokens used of ~128K context window`}>
        <div className="flex items-center gap-1">
          <Database size={11} />
          <span>{formatTokens(summary.totalTokens)} tokens</span>
        </div>
      </Tooltip>

      {/* Context usage bar */}
      <Tooltip
        title={`${summary.usagePercent.toFixed(0)}% of context window used${
          summary.isNearLimit ? " — auto-compression will trigger soon" : ""
        }`}
      >
        <div className="flex items-center gap-1">
          <Layers size={11} />
          <div className="w-16 h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
            <div
              className={`h-full rounded-full transition-all duration-300 ${
                summary.isOverLimit
                  ? "bg-red-500"
                  : summary.isNearLimit
                  ? "bg-orange-400"
                  : "bg-blue-400"
              }`}
              style={{ width: `${summary.usagePercent}%` }}
            />
          </div>
          <span className={summary.isOverLimit ? "text-red-500" : summary.isNearLimit ? "text-orange-500" : ""}>
            {summary.usagePercent.toFixed(0)}%
          </span>
        </div>
      </Tooltip>

      {/* Compression warning */}
      {summary.isNearLimit && (
        <Tag color="orange" className="text-xs py-0 leading-tight">
          <Zap size={10} className="inline mr-1" />
          Near limit
        </Tag>
      )}
    </div>
  );
};

export default ContextSummaryPreview;
