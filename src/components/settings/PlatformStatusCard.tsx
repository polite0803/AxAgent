import { usePlatformStore } from "@/stores";
import { ALL_PLATFORMS } from "@/types/platform";
import { Card, Tag, Typography } from "antd";
import { CheckCircle, Loader2 } from "lucide-react";
import { useEffect } from "react";

const { Text } = Typography;

export function PlatformStatusCard() {
  const statuses = usePlatformStore((s) => s.statuses);
  const loadStatuses = usePlatformStore((s) => s.loadStatuses);

  useEffect(() => {
    loadStatuses();
    const interval = setInterval(loadStatuses, 30000);
    return () => clearInterval(interval);
  }, [loadStatuses]);

  const metaMap = new Map(ALL_PLATFORMS.map(p => [p.name, p]));

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {statuses.map((s) => {
        const meta = metaMap.get(s.name);
        return (
          <Card key={s.name} size="small" title={`${meta?.icon ?? "?"} ${meta?.label ?? s.name}`}>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <div className="flex items-center justify-between">
                <Text type="secondary">Status</Text>
                {!s.enabled ? (
                  <Tag color="default">Disabled</Tag>
                ) : s.connected ? (
                  <Tag icon={<CheckCircle size={14} />} color="success">
                    Connected
                  </Tag>
                ) : (
                  <Tag icon={<Loader2 size={14} className="animate-spin" />} color="processing">
                    Connecting
                  </Tag>
                )}
              </div>
              {s.last_activity && (
                <div className="flex items-center justify-between">
                  <Text type="secondary">Last Activity</Text>
                  <Text>{new Date(s.last_activity).toLocaleString()}</Text>
                </div>
              )}
              {s.enabled && (
                <div className="flex items-center justify-between">
                  <Text type="secondary">Active Sessions</Text>
                  <Text>{s.active_sessions}</Text>
                </div>
              )}
            </div>
          </Card>
        );
      })}
    </div>
  );
}
