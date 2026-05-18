import { usePlatformStore } from "@/stores";
import { ALL_PLATFORMS } from "@/types";
import { Card, Empty, Spin, Tag, Typography } from "antd";
import { CheckCircle, Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export function PlatformStatusCard() {
  const { t } = useTranslation();
  const statuses = usePlatformStore((s) => s.statuses);
  const loadStatuses = usePlatformStore((s) => s.loadStatuses);
  const [initialLoading, setInitialLoading] = useState(true);

  useEffect(() => {
    loadStatuses().finally(() => setInitialLoading(false));
    const interval = setInterval(loadStatuses, 30000);
    return () => clearInterval(interval);
  }, [loadStatuses]);

  const metaMap = new Map(ALL_PLATFORMS.map((p) => [p.name, p]));

  if (initialLoading) {
    return (
      <div
        style={{ display: "flex", justifyContent: "center", padding: "48px 0" }}
      >
        <Spin />
      </div>
    );
  }

  if (statuses.length === 0) {
    return (
      <div
        style={{ display: "flex", justifyContent: "center", padding: "48px 0" }}
      >
        <Empty description={t("settings.platform.noStatuses")} />
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {statuses.map((s) => {
        const meta = metaMap.get(s.name);
        return (
          <Card
            key={s.name}
            size="small"
            title={`${meta?.icon ?? "?"} ${meta?.label ?? s.name}`}
          >
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <div className="flex items-center justify-between">
                <Text type="secondary">{t("settings.platform.status")}</Text>
                {!s.enabled
                  ? (
                    <Tag color="default">
                      {t("settings.platform.statusDisabled")}
                    </Tag>
                  )
                  : s.connected
                  ? (
                    <Tag icon={<CheckCircle size={14} />} color="success">
                      {t("settings.platform.statusConnected")}
                    </Tag>
                  )
                  : (
                    <Tag
                      icon={<Loader2 size={14} className="animate-spin" />}
                      color="processing"
                    >
                      {t("settings.platform.statusConnecting")}
                    </Tag>
                  )}
              </div>
              {s.last_activity && (
                <div className="flex items-center justify-between">
                  <Text type="secondary">
                    {t("settings.platform.lastActivity")}
                  </Text>
                  <Text>{new Date(s.last_activity).toLocaleString()}</Text>
                </div>
              )}
              {s.enabled && (
                <div className="flex items-center justify-between">
                  <Text type="secondary">
                    {t("settings.platform.activeSessions")}
                  </Text>
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
