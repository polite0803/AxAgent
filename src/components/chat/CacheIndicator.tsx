import { Tag, Tooltip } from "antd";
import { AlertTriangle, Clock, Database } from "lucide-react";
import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation();
  if (!cacheValid && !hasPendingChanges) {
    return (
      <Tooltip title={t("cacheIndicator.notEstablished")}>
        <Tag icon={<Clock size={12} />} color="default" style={{ margin: 0 }}>
          Fresh
        </Tag>
      </Tooltip>
    );
  }

  if (hasPendingChanges) {
    return (
      <Tooltip title={t("cacheIndicator.pendingChanges")}>
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
