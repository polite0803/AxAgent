import { Badge, Card, Collapse, Typography } from "antd";
import { BarChart3, CheckCircle, Loader2, TrendingUp, XCircle } from "lucide-react";
import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface DataPoint {
  label: string;
  value: number;
  series?: string;
}

interface ChartData {
  chart_type: string;
  title: string;
  labels: string[];
  series: string[];
  data_points: DataPoint[];
  insights: string[];
  summary: string;
}

interface ChartInterpreterProps {
  imageUrl?: string;
  chartData: ChartData | null;
  rawAnalysis: string;
  loading?: boolean;
  error?: string | null;
}

const CHART_TYPE_ICONS: Record<string, React.ReactNode> = {
  bar: <BarChart3 size={14} />,
  line: <TrendingUp size={14} />,
  pie: <BarChart3 size={14} />,
  scatter: <TrendingUp size={14} />,
  area: <TrendingUp size={14} />,
};

function ChartInterpreter({
  imageUrl,
  chartData,
  rawAnalysis,
  loading,
  error,
}: ChartInterpreterProps) {
  const { t } = useTranslation();

  const stats = useMemo(() => {
    if (!chartData) { return null; }
    const values = chartData.data_points.map((dp) => dp.value);
    const max = Math.max(...values, 0);
    const min = Math.min(...values, 0);
    const avg = values.length > 0
      ? values.reduce((a, b) => a + b, 0) / values.length
      : 0;
    return { max, min, avg, count: chartData.data_points.length };
  }, [chartData]);

  if (loading) {
    return (
      <Card size="small">
        <div className="flex items-center gap-2 py-4 text-sm text-gray-500">
          <Loader2 size={14} className="animate-spin" />
          <span>{t("chat.chart.analyzing")}</span>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card size="small">
        <div className="flex items-center gap-2 py-2 text-sm text-red-500">
          <XCircle size={14} />
          <span>{error}</span>
        </div>
      </Card>
    );
  }

  return (
    <Card size="small" className="chart-interpreter">
      <div className="flex items-center gap-2 mb-3">
        <BarChart3 size={16} className="text-blue-500" />
        <Title level={5} className="mb-0">{t("chat.chart.analysis")}</Title>
      </div>

      {imageUrl && (
        <div className="mb-3 rounded overflow-hidden border border-gray-200 dark:border-gray-700 max-h-48">
          <img
            src={imageUrl}
            alt={t("chat.chart.chartImage")}
            className="w-full h-full object-contain bg-gray-100 dark:bg-gray-800"
          />
        </div>
      )}

      {chartData && (
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            {CHART_TYPE_ICONS[chartData.chart_type] || <BarChart3 size={14} />}
            <Text strong>{chartData.title}</Text>
            <Badge color="blue" text={<span className="text-xs">{chartData.chart_type}</span>} />
          </div>

          {stats && (
            <div className="grid grid-cols-4 gap-2">
              <Card size="small" className="bg-blue-50 dark:bg-blue-900/10 text-center">
                <Text className="text-lg font-bold text-blue-600 block">{stats.count}</Text>
                <Text type="secondary" className="text-xs">{t("chat.chart.dataPoints")}</Text>
              </Card>
              <Card size="small" className="bg-green-50 dark:bg-green-900/10 text-center">
                <Text className="text-lg font-bold text-green-600 block">{stats.max.toFixed(1)}</Text>
                <Text type="secondary" className="text-xs">{t("chat.chart.max")}</Text>
              </Card>
              <Card size="small" className="bg-orange-50 dark:bg-orange-900/10 text-center">
                <Text className="text-lg font-bold text-orange-600 block">{stats.min.toFixed(1)}</Text>
                <Text type="secondary" className="text-xs">{t("chat.chart.min")}</Text>
              </Card>
              <Card size="small" className="bg-purple-50 dark:bg-purple-900/10 text-center">
                <Text className="text-lg font-bold text-purple-600 block">{stats.avg.toFixed(1)}</Text>
                <Text type="secondary" className="text-xs">{t("chat.chart.avg")}</Text>
              </Card>
            </div>
          )}

          {chartData.insights.length > 0 && (
            <div>
              <Text strong className="text-sm block mb-1">{t("chat.chart.insights")}</Text>
              <ul className="space-y-1">
                {chartData.insights.map((insight, i) => (
                  <li key={i} className="flex items-start gap-2 text-sm">
                    <CheckCircle size={12} className="text-green-500 mt-0.5 shrink-0" />
                    <span>{insight}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {chartData.data_points.length > 0 && (
            <Collapse
              size="small"
              items={[{
                key: "data",
                label: <span>{t("chat.chart.rawData")} ({chartData.data_points.length} points)</span>,
                children: (
                  <div className="max-h-48 overflow-auto">
                    <table className="w-full text-xs">
                      <thead>
                        <tr className="text-gray-500 border-b border-gray-200 dark:border-gray-700">
                          <th className="text-left py-1 pr-2">{t("chat.chart.label")}</th>
                          <th className="text-right py-1 pr-2">{t("chat.chart.value")}</th>
                          {chartData.series.length > 0 && <th className="text-left py-1">{t("chat.chart.series")}</th>}
                        </tr>
                      </thead>
                      <tbody>
                        {chartData.data_points.map((dp, i) => (
                          <tr key={i} className="border-b border-gray-100 dark:border-gray-800">
                            <td className="py-1 pr-2 text-gray-600 dark:text-gray-400">{dp.label}</td>
                            <td className="py-1 pr-2 text-right font-mono">{dp.value}</td>
                            {chartData.series.length > 0 && <td className="py-1 text-gray-500">{dp.series || "-"}</td>}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ),
              }]}
            />
          )}
        </div>
      )}

      {!chartData && rawAnalysis && (
        <div>
          <Text strong className="text-sm block mb-1">{t("chat.chart.summary")}</Text>
          <Text className="text-sm">{rawAnalysis}</Text>
        </div>
      )}

      {!chartData && !rawAnalysis && !loading && !error && (
        <div className="py-4 text-xs text-gray-400 text-center">
          {t("chat.chart.noAnalysis")}
        </div>
      )}
    </Card>
  );
}

export default ChartInterpreter;
