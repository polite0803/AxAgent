import { useGatewayLinkStore } from "@/stores";
import type { GatewayLink } from "@/types";
import { Button, Card, Col, Empty, Row, Statistic, Tag, theme } from "antd";
import { Clock, PlayCircle, Power, RefreshCw, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";

interface LinkOverviewProps {
  link: GatewayLink;
}

export function LinkOverview({ link }: LinkOverviewProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const connectLink = useGatewayLinkStore((s) => s.connectLink);
  const disconnectLink = useGatewayLinkStore((s) => s.disconnectLink);
  const fetchLinks = useGatewayLinkStore((s) => s.fetchLinks);
  const activities = useGatewayLinkStore((s) => s.activities);
  const fetchActivities = useGatewayLinkStore((s) => s.fetchActivities);
  const modelSyncs = useGatewayLinkStore((s) => s.modelSyncs);
  const skillSyncs = useGatewayLinkStore((s) => s.skillSyncs);

  const syncedModelCount = modelSyncs.filter((m) => m.sync_status === "synced").length;
  const syncedSkillCount = skillSyncs.filter((s) => s.sync_status === "synced").length;

  const handleToggle = async () => {
    try {
      if (link.status === "connected") {
        await disconnectLink(link.id);
      } else {
        await connectLink(link.id);
      }
      void fetchLinks();
    } catch {
      // error handled in store
    }
  };

  const isConnecting = link.status === "connecting";

  const handleRefresh = () => {
    void fetchActivities(link.id);
  };

  const statusColor = link.status === "connected"
    ? "green"
    : link.status === "connecting"
    ? "orange"
    : link.status === "error"
    ? "red"
    : "default";

  const statusLabel = link.status === "connected"
    ? t("link.statusConnected")
    : link.status === "connecting"
    ? t("link.statusConnecting")
    : link.status === "error"
    ? t("link.statusError")
    : t("link.statusDisconnected");

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <Card size="small">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div>
              <Tag color={statusColor}>{statusLabel}</Tag>
              {link.latency_ms != null && link.status === "connected" && (
                <span style={{ fontSize: 12, color: token.colorTextSecondary, marginLeft: 8 }}>
                  {t("link.latency")}: {link.latency_ms}ms
                </span>
              )}
              {link.version && (
                <span style={{ fontSize: 12, color: token.colorTextSecondary, marginLeft: 8 }}>
                  v{link.version}
                </span>
              )}
              {link.status === "error" && link.error_message && (
                <div style={{ fontSize: 12, color: token.colorError, marginTop: 4 }}>
                  {link.error_message}
                </div>
              )}
            </div>
          </div>
          <Button
            type={link.status === "connected" ? "default" : "primary"}
            danger={link.status === "connected"}
            icon={link.status === "connected" ? <Power size={16} /> : <PlayCircle size={16} />}
            onClick={handleToggle}
            loading={isConnecting}
            disabled={isConnecting}
          >
            {isConnecting
              ? t("link.statusConnecting")
              : link.status === "connected"
              ? t("link.disconnect")
              : t("link.connect")}
          </Button>
        </div>
      </Card>

      <Row gutter={16}>
        <Col span={8}>
          <Card size="small">
            <Statistic
              title={t("link.syncedModels")}
              value={syncedModelCount}
              prefix={<Zap size={14} />}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small">
            <Statistic
              title={t("link.syncedSkills")}
              value={syncedSkillCount}
              prefix={<Zap size={14} />}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small">
            <Statistic
              title={t("link.lastSync")}
              value={link.last_sync_at ? new Date(link.last_sync_at * 1000).toLocaleString() : "-"}
              valueStyle={{ fontSize: 14 }}
              prefix={<Clock size={14} />}
            />
          </Card>
        </Col>
      </Row>

      <Card
        size="small"
        title={t("link.recentActivity")}
        extra={
          <Button size="small" icon={<RefreshCw size={14} />} onClick={handleRefresh}>
            {t("common.refresh")}
          </Button>
        }
      >
        {activities.length === 0
          ? <Empty description={t("link.noActivity")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
          : (
            <div className="flex flex-col gap-2">
              {activities.slice(0, 10).map((activity) => (
                <div key={activity.id} className="flex items-start gap-2" style={{ fontSize: 13 }}>
                  <span style={{ color: token.colorTextTertiary, flexShrink: 0, fontSize: 12 }}>
                    {new Date(activity.created_at * 1000).toLocaleTimeString()}
                  </span>
                  <span>{activity.detail}</span>
                </div>
              ))}
            </div>
          )}
      </Card>
    </div>
  );
}
