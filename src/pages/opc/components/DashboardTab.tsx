// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { DollarOutlined, FileTextOutlined, ProjectOutlined, TeamOutlined } from "@ant-design/icons";
import { Button, Card, Col, Empty, Row, Space, Spin, Statistic, Tag, Timeline } from "antd";
import * as echarts from "echarts";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface DashboardSummary {
  total_revenue: number;
  total_invoices: number;
  active_projects: number;
  total_customers: number;
  recent_kpis: Array<{ name: string; value: number; unit: string; period: string }>;
  // P1-6：与后端 RevenueRecord { id, amount, currency, category, description, recorded_at } 对齐
  revenue_trend?: Array<{ amount: number; recorded_at: number }>;
}

const formatTs = (ts: number): string => {
  const d = new Date(ts * 1000);
  return `${d.getMonth() + 1}/${d.getDate()}`;
};

export function DashboardTab() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);

  const revenueChartRef = useRef<HTMLDivElement>(null);
  const revenueChartInstance = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    invoke<DashboardSummary>("opc_get_dashboard_summary")
      .then(setSummary)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  // 初始化图表
  useEffect(() => {
    if (!summary) { return; }

    // 收入趋势图
    if (revenueChartRef.current && summary.revenue_trend) {
      if (!revenueChartInstance.current) {
        revenueChartInstance.current = echarts.init(revenueChartRef.current);
      }
      revenueChartInstance.current.setOption({
        tooltip: { trigger: "axis" },
        grid: { left: 50, right: 20, top: 20, bottom: 30 },
        xAxis: {
          type: "category",
          data: summary.revenue_trend.map((item) => formatTs(item.recorded_at)),
          axisLabel: { color: "#999" },
        },
        yAxis: {
          type: "value",
          axisLabel: { color: "#999", formatter: "¥{value}" },
        },
        series: [
          {
            name: t("opc.dashboard.totalRevenue"),
            type: "line",
            data: summary.revenue_trend.map((item) => item.amount),
            smooth: true,
            areaStyle: { opacity: 0.3 },
            lineStyle: { color: "#3f8600" },
            itemStyle: { color: "#3f8600" },
          },
        ],
      });
    }

    // 响应式
    const handleResize = () => {
      revenueChartInstance.current?.resize();
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [summary, t]);

  // 清理
  useEffect(() => {
    return () => {
      revenueChartInstance.current?.dispose();
    };
  }, []);

  if (loading) {
    return <Spin size="large" style={{ display: "block", margin: "80px auto" }} />;
  }
  if (!summary) {
    return <Empty description={t("opc.dashboard.loadFailed")} />;
  }

  return (
    <div className="space-y-4">
      {/* KPI 卡片 */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} md={6}>
          <Card size="small" className="h-full">
            <Statistic
              title={t("opc.dashboard.totalRevenue")}
              value={summary.total_revenue}
              prefix="¥"
              precision={2}
              valueStyle={{ color: "#3f8600" }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card size="small" className="h-full">
            <Statistic
              title={t("opc.dashboard.totalInvoices")}
              value={summary.total_invoices}
              prefix={<FileTextOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card size="small" className="h-full">
            <Statistic
              title={t("opc.dashboard.activeCustomers")}
              value={summary.total_customers}
              prefix={<TeamOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card size="small" className="h-full">
            <Statistic
              title={t("opc.dashboard.activeProjects")}
              value={summary.active_projects}
              prefix={<ProjectOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* 图表区域 */}
      {summary.revenue_trend && (
        <Row gutter={[16, 16]}>
          <Col xs={24}>
            <Card title={t("opc.dashboard.revenueTrend")} size="small">
              <div ref={revenueChartRef} style={{ height: 240 }} />
            </Card>
          </Col>
        </Row>
      )}

      {/* KPI 时间线 + 快捷操作 */}
      <Row gutter={16}>
        <Col xs={24} lg={12}>
          <Card title={t("opc.dashboard.kpiTitle")} size="small">
            {summary.recent_kpis.length === 0 ? <Empty description={t("opc.dashboard.noKpi")} /> : (
              <Timeline
                items={summary.recent_kpis.slice(0, 5).map((kpi) => ({
                  color: "blue",
                  children: (
                    <>
                      <strong>{kpi.name}</strong>: {kpi.value} {kpi.unit} <Tag>{kpi.period}</Tag>
                    </>
                  ),
                }))}
              />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title={t("opc.dashboard.quickActionsTitle")} size="small">
            <Space direction="vertical" style={{ width: "100%" }}>
              <Button
                type="primary"
                block
                icon={<DollarOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "invoices" }))}
              >
                {t("opc.dashboard.manageInvoices")}
              </Button>
              <Button
                block
                icon={<TeamOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "customers" }))}
              >
                {t("opc.dashboard.manageCustomers")}
              </Button>
              <Button
                block
                icon={<ProjectOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "projects" }))}
              >
                {t("opc.dashboard.manageProjects")}
              </Button>
            </Space>
          </Card>
        </Col>
      </Row>
    </div>
  );
}
