import { invoke } from "@/lib/invoke";
import { ReloadOutlined } from "@ant-design/icons";
import { Button, Card, Popconfirm, Statistic, Table, Tag } from "antd";
import { Activity, BarChart3, Clock, Server } from "lucide-react";
import { useEffect, useState } from "react";

interface GatewayMetrics {
  total_requests: number;
  total_tokens: number;
  active_connections: number;
  avg_latency_ms: number;
  error_count: number;
  uptime_seconds: number;
}

interface RequestLog {
  id: string;
  timestamp: string;
  method: string;
  path: string;
  status: number;
  duration_ms: number;
  tokens_in: number;
  tokens_out: number;
  client_ip: string;
}

export function GatewayMonitor() {
  const [metrics, setMetrics] = useState<GatewayMetrics | null>(null);
  const [logs, setLogs] = useState<RequestLog[]>([]);
  const [loading, setLoading] = useState(false);

  const loadData = async () => {
    setLoading(true);
    try {
      const [m, l] = await Promise.all([
        invoke<GatewayMetrics>("get_gateway_metrics").catch(() => null),
        invoke<RequestLog[]>("list_gateway_request_logs", { limit: 50 }).catch(() => []),
      ]);
      setMetrics(m);
      setLogs(l);
    } catch (e) {
      console.warn("Gateway metrics load failed:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleClearLogs = async () => {
    try {
      await invoke("clear_gateway_request_logs");
      setLogs([]);
    } catch (e) {
      console.error(e);
    }
  };

  const formatUptime = (s: number) => {
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    return `${h}h ${m}m`;
  };

  const logColumns = [
    {
      title: "时间",
      dataIndex: "timestamp",
      key: "ts",
      width: 160,
      render: (t: string) => new Date(t).toLocaleString(),
    },
    {
      title: "方法",
      dataIndex: "method",
      key: "method",
      width: 70,
      render: (m: string) => <Tag color={m === "POST" ? "blue" : "green"}>{m}</Tag>,
    },
    { title: "路径", dataIndex: "path", key: "path", ellipsis: true },
    {
      title: "状态",
      dataIndex: "status",
      key: "status",
      width: 70,
      render: (s: number) => <Tag color={s < 300 ? "green" : s < 500 ? "orange" : "red"}>{s}</Tag>,
    },
    { title: "延迟", dataIndex: "duration_ms", key: "dur", width: 70, render: (d: number) => `${d}ms` },
    {
      title: "Token",
      key: "tokens",
      width: 100,
      render: (_: unknown, r: RequestLog) => `${r.tokens_in}+${r.tokens_out}`,
    },
  ];

  return (
    <div style={{ padding: "16px 24px", maxWidth: 1200, margin: "0 auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <h2 style={{ margin: 0, fontSize: 20, fontWeight: 600 }}>
          <Server size={18} style={{ marginRight: 8 }} />
          Gateway 监控
        </h2>
        <Button icon={<ReloadOutlined />} onClick={loadData} loading={loading}>刷新</Button>
      </div>

      {/* 指标卡片 */}
      {metrics && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))",
            gap: 12,
            marginBottom: 20,
          }}
        >
          <Card size="small">
            <Statistic title="总请求" value={metrics.total_requests} prefix={<Activity size={16} />} />
          </Card>
          <Card size="small">
            <Statistic title="总 Token" value={metrics.total_tokens} prefix={<BarChart3 size={16} />} />
          </Card>
          <Card size="small">
            <Statistic title="活跃连接" value={metrics.active_connections} prefix={<Server size={16} />} />
          </Card>
          <Card size="small">
            <Statistic title="平均延迟" value={metrics.avg_latency_ms} suffix="ms" precision={0} />
          </Card>
          <Card size="small">
            <Statistic
              title="错误数"
              value={metrics.error_count}
              valueStyle={{ color: metrics.error_count > 0 ? "#ff4d4f" : undefined }}
            />
          </Card>
          <Card size="small">
            <Statistic title="运行时间" value={formatUptime(metrics.uptime_seconds)} prefix={<Clock size={16} />} />
          </Card>
        </div>
      )}

      {/* 请求日志 */}
      <Card
        title="请求日志"
        extra={
          <Popconfirm title="清空所有日志?" onConfirm={handleClearLogs}>
            <Button size="small" danger>清空</Button>
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
          locale={{ emptyText: "暂无请求日志" }}
        />
      </Card>
    </div>
  );
}
