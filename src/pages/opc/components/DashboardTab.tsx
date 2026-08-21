// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import {
  DollarOutlined,
  FileTextOutlined,
  ProjectOutlined,
  TeamOutlined,
  VideoCameraOutlined,
} from "@ant-design/icons";
import { Button, Card, Col, Empty, message, Row, Space, Spin, Statistic, Tag, Timeline } from "antd";
import * as echarts from "echarts";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface DashboardSummary {
  total_revenue: number;
  total_invoices: number;
  active_projects: number;
  total_customers: number;
  recent_kpis: Array<{ name: string; value: number; unit: string; period: string }>;
  revenue_trend?: Array<{ amount: number; recorded_at: number }>;
}

interface ContentMediaKpis {
  content_assets_count: number;
  blog_posts_count: number;
  landing_pages_count: number;
  publish_schedules_pending: number;
  publish_schedules_published: number;
}

interface ContentAsset {
  id: string;
  title: string;
  content_type: string;
  status: string;
}

interface PublishSchedule {
  id: string;
  status: string;
}

interface BlogPost {
  id: string;
  published: boolean;
}

interface LandingPage {
  id: string;
}

const formatTs = (ts: number): string => {
  const d = new Date(ts * 1000);
  return `${d.getMonth() + 1}/${d.getDate()}`;
};

export function DashboardTab() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [cmKpis, setCmKpis] = useState<ContentMediaKpis | null>(null);
  const [cmLoading, setCmLoading] = useState(true);

  const revenueChartRef = useRef<HTMLDivElement>(null);
  const revenueChartInstance = useRef<echarts.ECharts | null>(null);
  const contentTypeChartRef = useRef<HTMLDivElement>(null);
  const contentTypeChartInstance = useRef<echarts.ECharts | null>(null);
  const scheduleStatusChartRef = useRef<HTMLDivElement>(null);
  const scheduleStatusChartInstance = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const data = await invoke<DashboardSummary>("opc_get_dashboard_summary");
        setSummary(data);
      } catch (e) {
        message.error(t("opc.common.loadFailed", { error: String(e) }));
        setSummary(null);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [assets, schedules, posts, pages] = await Promise.all([
          invoke<ContentAsset[]>("opc_list_content_assets").catch(() => []),
          invoke<PublishSchedule[]>("opc_list_publish_schedules").catch(() => []),
          invoke<BlogPost[]>("opc_list_blog_posts").catch(() => []),
          invoke<LandingPage[]>("opc_list_landing_pages").catch(() => []),
        ]);
        if (cancelled) { return; }
        const typeCounts: Record<string, number> = {};
        for (const a of assets) {
          typeCounts[a.content_type] = (typeCounts[a.content_type] || 0) + 1;
        }
        const pending = schedules.filter((s) => s.status === "pending").length;
        const published = schedules.filter((s) => s.status === "published").length;

        setCmKpis({
          content_assets_count: assets.length,
          blog_posts_count: posts.length,
          landing_pages_count: pages.length,
          publish_schedules_pending: pending,
          publish_schedules_published: published,
        });

        if (contentTypeChartRef.current) {
          if (!contentTypeChartInstance.current) {
            contentTypeChartInstance.current = echarts.init(contentTypeChartRef.current);
          }
          contentTypeChartInstance.current.setOption({
            tooltip: { trigger: "item" },
            series: [
              {
                type: "pie",
                radius: ["40%", "70%"],
                data: Object.entries(typeCounts).map(([type, count]) => ({
                  name: type,
                  value: count,
                })),
                label: { show: false },
              },
            ],
          });
        }

        if (scheduleStatusChartRef.current) {
          if (!scheduleStatusChartInstance.current) {
            scheduleStatusChartInstance.current = echarts.init(scheduleStatusChartRef.current);
          }
          scheduleStatusChartInstance.current.setOption({
            tooltip: { trigger: "item" },
            series: [
              {
                type: "pie",
                radius: ["40%", "70%"],
                data: [
                  { name: t("opc.contentMedia.schedule_pending"), value: pending },
                  { name: t("opc.contentMedia.schedule_published"), value: published },
                  {
                    name: t("opc.contentMedia.schedule_failed"),
                    value: schedules.filter((s) => s.status === "failed").length,
                  },
                ].filter((d) => d.value > 0),
                label: { show: false },
              },
            ],
          });
        }
      } catch (e) {
        if (!cancelled) {
          message.error(t("opc.common.loadFailed", { error: String(e) }));
        }
      } finally {
        if (!cancelled) { setCmLoading(false); }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [t]);

  useEffect(() => {
    if (!summary) { return; }

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

    const handleResize = () => {
      revenueChartInstance.current?.resize();
      contentTypeChartInstance.current?.resize();
      scheduleStatusChartInstance.current?.resize();
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [summary, t]);

  useEffect(() => {
    return () => {
      revenueChartInstance.current?.dispose();
      contentTypeChartInstance.current?.dispose();
      scheduleStatusChartInstance.current?.dispose();
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
      {/* 核心 KPI 卡片 */}
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

      {/* 收入趋势图 */}
      {summary.revenue_trend && (
        <Row gutter={[16, 16]}>
          <Col xs={24}>
            <Card title={t("opc.dashboard.revenueTrend")} size="small">
              <div ref={revenueChartRef} style={{ height: 240 }} />
            </Card>
          </Col>
        </Row>
      )}

      {/* 内容媒体 KPI */}
      <Row gutter={[16, 16]}>
        <Col xs={24}>
          <Card
            title={
              <Space>
                <VideoCameraOutlined />
                {t("opc.contentMedia.dashboardTitle")}
              </Space>
            }
            size="small"
          >
            {cmLoading ? <Spin /> : cmKpis
              ? (
                <>
                  <Row gutter={[12, 12]} style={{ marginBottom: 16 }}>
                    <Col xs={12} sm={8} md={4}>
                      <Card size="small">
                        <Statistic
                          title={t("opc.contentMedia.kpiAssets")}
                          value={cmKpis.content_assets_count}
                          valueStyle={{ color: "#1677ff" }}
                        />
                      </Card>
                    </Col>
                    <Col xs={12} sm={8} md={4}>
                      <Card size="small">
                        <Statistic
                          title={t("opc.contentMedia.kpiBlogPosts")}
                          value={cmKpis.blog_posts_count}
                          valueStyle={{ color: "#722ed1" }}
                        />
                      </Card>
                    </Col>
                    <Col xs={12} sm={8} md={4}>
                      <Card size="small">
                        <Statistic
                          title={t("opc.contentMedia.kpiLandingPages")}
                          value={cmKpis.landing_pages_count}
                          valueStyle={{ color: "#13c2c2" }}
                        />
                      </Card>
                    </Col>
                    <Col xs={12} sm={8} md={4}>
                      <Card size="small">
                        <Statistic
                          title={t("opc.contentMedia.kpiSchedulesPending")}
                          value={cmKpis.publish_schedules_pending}
                          valueStyle={{ color: "#fa8c16" }}
                        />
                      </Card>
                    </Col>
                    <Col xs={12} sm={8} md={4}>
                      <Card size="small">
                        <Statistic
                          title={t("opc.contentMedia.kpiSchedulesPublished")}
                          value={cmKpis.publish_schedules_published}
                          valueStyle={{ color: "#52c41a" }}
                        />
                      </Card>
                    </Col>
                  </Row>
                  <Row gutter={[16, 16]}>
                    <Col xs={24} md={12}>
                      <Card size="small" title={t("opc.contentMedia.chartContentType")}>
                        <div ref={contentTypeChartRef} style={{ height: 200 }} />
                      </Card>
                    </Col>
                    <Col xs={24} md={12}>
                      <Card size="small" title={t("opc.contentMedia.chartScheduleStatus")}>
                        <div ref={scheduleStatusChartRef} style={{ height: 200 }} />
                      </Card>
                    </Col>
                  </Row>
                </>
              )
              : <Empty description={t("opc.contentMedia.noData")} />}
          </Card>
        </Col>
      </Row>

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
              <Button
                block
                icon={<VideoCameraOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "content_media" }))}
              >
                {t("opc.contentMedia.manageContent")}
              </Button>
            </Space>
          </Card>
        </Col>
      </Row>
    </div>
  );
}
