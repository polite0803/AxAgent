// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * G1 跨市场数据接入 Dashboard
 *
 * 功能：
 * - 输入美股/港股代码（AAPL / TSLA / 00700.HK / BABA.US）→ 拉取实时行情
 * - 多基准指数 K 线对比（SPX / IXIC / HSI / 000001.SH / 399006）
 * - 外汇 K 线（USD/CNY、HKD/CNY）
 * - 显示最新价、涨跌幅、市值
 *
 * 数据来自后端 Tauri 命令：
 * - get_international_stock_quote / get_international_stock_kline
 * - get_benchmark_kline
 * - get_forex_kline
 */

import { KLineChart } from "@/components/stock-analysis/KLineChart";
import { useCrossMarketStore } from "@/stores";
import { App, Button, Empty, Input, Select, Space, Statistic, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Paragraph } = Typography;

interface IntlQuoteRow {
  key: string;
  code: string;
  name: string;
  price: number;
  changePct: number;
  currency?: string;
  totalMv?: number | null;
}

const DEFAULT_BENCHMARKS = ["SPX", "IXIC", "HSI", "000001.SH", "399006"];
const DEFAULT_FOREX = ["USD/CNY", "HKD/CNY"];

export function CrossMarketDashboard() {
  const { message: messageApi } = App.useApp();
  const { t } = useTranslation();
  const {
    intlQuotes,
    intlKlines,
    benchmarkKlines,
    forexKlines,
    loadingQuote,
    loadingKline,
    loadingBenchmark,
    loadingForex,
    error,
    fetchIntlQuote,
    fetchIntlKline,
    fetchBenchmarkKline,
    fetchForexKline,
    clearError,
  } = useCrossMarketStore();

  const [codeInput, setCodeInput] = useState("");
  const [benchmarkSel, setBenchmarkSel] = useState<string>("SPX");
  const [forexSel, setForexSel] = useState<string>("USD/CNY");
  const [activeIntlCode, setActiveIntlCode] = useState<string | null>(null);

  // 启动时预加载默认基准指数和外汇 K 线
  useEffect(() => {
    fetchBenchmarkKline("SPX");
    fetchForexKline("USD/CNY");
  }, [fetchBenchmarkKline, fetchForexKline]);

  // 错误统一 toast
  useEffect(() => {
    if (error) {
      messageApi.error(error);
      clearError();
    }
  }, [error, clearError]);

  const handleAddCode = async () => {
    const code = codeInput.trim();
    if (!code) {
      messageApi.warning(t("crossMarket.codeRequired"));
      return;
    }
    const q = await fetchIntlQuote(code, true);
    if (q) {
      messageApi.success(t("crossMarket.quoteLoaded", { code }));
      setActiveIntlCode(code);
      await fetchIntlKline(code, "daily", 120);
    }
    setCodeInput("");
  };

  const handleBenchmarkChange = async (val: string) => {
    setBenchmarkSel(val);
    await fetchBenchmarkKline(val);
  };

  const handleForexChange = async (val: string) => {
    setForexSel(val);
    await fetchForexKline(val);
  };

  const quoteRows: IntlQuoteRow[] = Object.values(intlQuotes).map((q) => ({
    key: q.code,
    code: q.code,
    name: q.name,
    price: q.price,
    changePct: q.changePct,
    totalMv: q.totalMv ?? null,
  }));

  const quoteColumns: ColumnsType<IntlQuoteRow> = [
    { title: t("crossMarket.code"), dataIndex: "code", key: "code" },
    { title: t("crossMarket.name"), dataIndex: "name", key: "name" },
    {
      title: t("crossMarket.price"),
      dataIndex: "price",
      key: "price",
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("crossMarket.changePct"),
      dataIndex: "changePct",
      key: "changePct",
      render: (v: number) => (
        <Tag color={v >= 0 ? "green" : "red"}>
          {v >= 0 ? "+" : ""}
          {v.toFixed(2)}%
        </Tag>
      ),
    },
    {
      title: t("crossMarket.totalMv"),
      dataIndex: "totalMv",
      key: "totalMv",
      render: (v: number | null) => (v != null ? `${(v / 1_0000_0000).toFixed(2)} 亿` : "-"),
    },
  ];

  const intlKline = activeIntlCode
    ? intlKlines[`${activeIntlCode}:daily:120`]
    : undefined;
  const benchmarkKey = `${benchmarkSel}:daily:120`;
  const forexKey = `${forexSel}:daily:120`;

  return (
    <div style={{ padding: 16 }}>
      <Space orientation="vertical" size="large" style={{ width: "100%" }}>
        <div>
          <Title level={4}>{t("crossMarket.title")}</Title>
          <Paragraph type="secondary">{t("crossMarket.subtitle")}</Paragraph>
        </div>

        {/* 美股/港股查询 */}
        <div>
          <Space style={{ marginBottom: 12 }}>
            <Input
              placeholder={t("crossMarket.codePlaceholder")}
              value={codeInput}
              onChange={(e) => setCodeInput(e.target.value)}
              onPressEnter={handleAddCode}
              style={{ width: 280 }}
            />
            <Button type="primary" loading={loadingQuote} onClick={handleAddCode}>
              {t("crossMarket.fetchQuote")}
            </Button>
          </Space>

          {quoteRows.length === 0 ? <Empty description={t("crossMarket.noQuotes")} /> : (
            <Table
              columns={quoteColumns}
              dataSource={quoteRows}
              rowKey="key"
              size="small"
              pagination={false}
              onRow={(row) => ({
                onClick: () => {
                  setActiveIntlCode(row.code);
                  fetchIntlKline(row.code, "daily", 120);
                },
                style: { cursor: "pointer" },
              })}
            />
          )}
        </div>

        {/* 国际股票 K 线 */}
        {activeIntlCode && (
          <div>
            <Title level={5}>
              {t("crossMarket.klineTitle", { code: activeIntlCode })}
            </Title>
            {loadingKline
              ? <Paragraph>{t("crossMarket.loading")}</Paragraph>
              : intlKline && intlKline.length > 0
              ? <KLineChart klines={intlKline} height={320} />
              : <Empty description={t("crossMarket.noKline")} />}
          </div>
        )}

        {/* 基准指数对比 */}
        <div>
          <Title level={5}>{t("crossMarket.benchmarkTitle")}</Title>
          <Space style={{ marginBottom: 12 }}>
            <Select
              value={benchmarkSel}
              onChange={handleBenchmarkChange}
              style={{ width: 200 }}
              options={DEFAULT_BENCHMARKS.map((b) => ({ value: b, label: b }))}
            />
            <Button
              loading={loadingBenchmark}
              onClick={() => fetchBenchmarkKline(benchmarkSel, "daily", 120)}
            >
              {t("crossMarket.refresh")}
            </Button>
          </Space>
          {loadingBenchmark
            ? <Paragraph>{t("crossMarket.loading")}</Paragraph>
            : benchmarkKlines[benchmarkKey] && benchmarkKlines[benchmarkKey].length > 0
            ? (
              <>
                <Space size="large" style={{ marginBottom: 12 }}>
                  <Statistic
                    title={t("crossMarket.latestClose")}
                    value={benchmarkKlines[benchmarkKey].slice(-1)[0]?.close ?? 0}
                    precision={2}
                  />
                  <Statistic
                    title={t("crossMarket.klineCount")}
                    value={benchmarkKlines[benchmarkKey].length}
                  />
                </Space>
                <KLineChart klines={benchmarkKlines[benchmarkKey]} height={320} />
              </>
            )
            : <Empty description={t("crossMarket.noBenchmark")} />}
        </div>

        {/* 外汇 K 线 */}
        <div>
          <Title level={5}>{t("crossMarket.forexTitle")}</Title>
          <Space style={{ marginBottom: 12 }}>
            <Select
              value={forexSel}
              onChange={handleForexChange}
              style={{ width: 200 }}
              options={DEFAULT_FOREX.map((f) => ({ value: f, label: f }))}
            />
            <Button loading={loadingForex} onClick={() => fetchForexKline(forexSel, "daily", 120)}>
              {t("crossMarket.refresh")}
            </Button>
          </Space>
          {loadingForex
            ? <Paragraph>{t("crossMarket.loading")}</Paragraph>
            : forexKlines[forexKey] && forexKlines[forexKey].length > 0
            ? (
              <>
                <Space size="large" style={{ marginBottom: 12 }}>
                  <Statistic
                    title={t("crossMarket.latestClose")}
                    value={forexKlines[forexKey].slice(-1)[0]?.close ?? 0}
                    precision={4}
                  />
                  <Statistic
                    title={t("crossMarket.klineCount")}
                    value={forexKlines[forexKey].length}
                  />
                </Space>
                <KLineChart klines={forexKlines[forexKey]} height={320} />
              </>
            )
            : <Empty description={t("crossMarket.noForex")} />}
        </div>
      </Space>
    </div>
  );
}
