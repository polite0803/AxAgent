// SPDX-License-Identifier: AGPL-3.0-only

import { showBackendError } from "@/lib/errorI18n";
import { invoke, logIpcError } from "@/lib/invoke";
import type { GatewayMetrics, GatewayRequestLog } from "@/types";
import { ReloadOutlined } from "@ant-design/icons";
import { App, Button, Card, Popconfirm, Spin, Statistic, Table, Tag, theme } from "antd";
import { Activity, BarChart3, DollarSign, Server, TrendingUp, Zap } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

export function GatewayMonitor() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();
  const [metrics, setMetrics] = useState<GatewayMetrics | null>(null);
  const [logs, setLogs] = useState<GatewayRequestLog[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // GW-P0-3/4: 使用后端实际返回的 GatewayMetrics / GatewayRequestLog 类型,
      // 移除内层 .catch 让错误冒泡到外层统一处理
      const [m, l] = await Promise.all([
        invoke<GatewayMetrics>("get_gateway_metrics"),
        invoke<GatewayRequestLog[]>("list_gateway_request_logs", { limit: 50 }),
      ]);
      setMetrics(m);
      setLogs(l);
    } catch (e) {
      logIpcError("GatewayMonitor.loadData")(e);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadDataRef = useRef(loadData);

  useEffect(() => {
    loadDataRef.current = loadData;
  }, [loadData]);

  useEffect(() => {
    loadDataRef.current();
  }, []);

  const handleClearLogs = async () => {
    try {
      await invoke("clear_gateway_request_logs");
      setLogs([]);
      message.success(t("gatewayMonitor.logsCleared"));
    } catch (e) {
      logIpcError("GatewayMonitor.clearLogs")(e);
      showBackendError(message, e);
    }
  };

  const logColumns = [
    {
      title: t("gatewayMonitor.colTime"),
      dataIndex: "createdAt",
      key: "ts",
      width: 160,
      // GW-P0-4: 后端 createdAt 是 number(毫秒时间戳)
      render: (ts: number) => new Date(ts).toLocaleString(),
    },
    {
      title: t("gatewayMonitor.colMethod"),
      dataIndex: "method",
      key: "method",
      width: 70,
      render: (m: string) => <Tag color={m === "POST" ? "blue" : "green"}>{m}</Tag>,
    },
    {
      title: t("gatewayMonitor.colPath"),
      dataIndex: "path",
      key: "path",
      ellipsis: true,
    },
    {
      title: t("gatewayMonitor.colStatus"),
      dataIndex: "statusCode",
      key: "status",
      width: 70,
      render: (s: number) => <Tag color={s < 300 ? "green" : s < 500 ? "orange" : "red"}>{s}</Tag>,
    },
    {
      title: t("gatewayMonitor.colLatency"),
      dataIndex: "durationMs",
      key: "dur",
      width: 70,
      render: (d: number) => `${d}ms`,
    },
    {
      title: t("gatewayMonitor.colToken"),
      key: "tokens",
      width: 110,
      // GW-P0-4: 后端字段是 requestTokens / responseTokens (camelCase)
      render: (_: unknown, r: GatewayRequestLog) => `${r.requestTokens}+${r.responseTokens}`,
    },
  ];

  return (
    <div className="gw-page">
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 16,
        }}
      >
        <h2 style={{ margin: 0, fontSize: 20, fontWeight: 600 }}>
          <Server size={18} style={{ marginRight: 8 }} />
          {t("gatewayMonitor.title")}
        </h2>
        <Button icon={<ReloadOutlined />} onClick={loadData} loading={loading}>
          {t("gatewayMonitor.refresh")}
        </Button>
      </div>

      {/* 指标卡片 */}
      {loading
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin />
          </div>
        )
        : error
        ? (
          <Card>
            <div
              style={{
                textAlign: "center",
                padding: 12,
                color: "var(--color-text-secondary)",
              }}
            >
              {error}
            </div>
          </Card>
        )
        : (
          metrics && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))",
                gap: 12,
                marginBottom: 20,
              }}
            >
              <Card size="small">
                <Statistic
                  title={t("gatewayMonitor.totalRequests")}
                  value={metrics.total_requests}
                  prefix={<Activity size={16} />}
                />
              </Card>
              <Card size="small">
                <Statistic
                  title={t("gatewayMonitor.totalTokens")}
                  value={metrics.total_tokens}
                  prefix={<BarChart3 size={16} />}
                />
              </Card>
              <Card size="small">
                <Statistic
                  title={t("gatewayMonitor.activeConnections")}
                  value={metrics.active_connections}
                  prefix={<Server size={16} />}
                />
              </Card>
              <Card size="small">
                <Statistic
                  title={t("gatewayMonitor.todayRequests")}
                  value={metrics.today_requests}
                  prefix={<TrendingUp size={16} />}
                />
              </Card>
              <Card size="small">
                <Statistic
                  title={t("gatewayMonitor.todayTokens")}
                  value={metrics.today_tokens}
                  prefix={<Zap size={16} />}
                />
              </Card>
              <Card size="small">
                <Statistic
                  title={t("gatewayMonitor.totalCost")}
                  value={metrics.total_cost_usd}
                  precision={4}
                  prefix={<DollarSign size={16} />}
                  styles={{ content: { color: token.colorSuccess } }}
                />
              </Card>
            </div>
          )
        )}

      {/* 请求日志 */}
      <Card
        title={t("gatewayMonitor.requestLogs")}
        extra={
          <Popconfirm
            title={t("gatewayMonitor.clearLogsConfirm")}
            onConfirm={handleClearLogs}
          >
            <Button size="small" danger>
              {t("gatewayMonitor.clear")}
            </Button>
          </Popconfirm>
        }
      >
        <Table
          dataSource={logs}
          columns={logColumns}
          rowKey="id"
          size="small"
          pagination={{ pageSize: 20, size: "small" }}
          scroll={{ x: 700 }}
          locale={{ emptyText: t("gatewayMonitor.noRequestLogs") }}
        />
      </Card>
    </div>
  );
}
