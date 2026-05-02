import { Tag, Tooltip } from "antd";
import { AlertTriangle, Clock, Database } from "lucide-react";

interface CacheIndicatorProps {
  cacheValid: boolean;
  hasPendingChanges: boolean;
  tokensSaved: number;
  cacheHits: number;
}

export function CacheIndicator({
  cacheValid,
  hasPendingChanges,
  tokensSaved,
  cacheHits,
}: CacheIndicatorProps) {
  if (!cacheValid && !hasPendingChanges) {
    return (
      <Tooltip title="Prompt cache not established. First turn in session.">
        <Tag icon={<Clock size={12} />} color="default" style={{ margin: 0 }}>
          Fresh
        </Tag>
      </Tooltip>
    );
  }

  if (hasPendingChanges) {
    return (
      <Tooltip title="Pending changes detected. Changes apply next session. Use --now to force.">
        <Tag icon={<AlertTriangle size={12} />} color="warning" style={{ margin: 0 }}>
          Pending
        </Tag>
      </Tooltip>
    );
  }

  return (
    <Tooltip title={`Cache active. ${cacheHits} hits, ~${formatTokens(tokensSaved)} tokens saved.`}>
      <Tag icon={<Database size={12} />} color="green" style={{ margin: 0 }}>
        Cached
      </Tag>
    </Tooltip>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) { return `${(n / 1_000_000).toFixed(1)}M`; }
  if (n >= 1_000) { return `${(n / 1_000).toFixed(1)}K`; }
  return n.toString();
}
