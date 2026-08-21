// i18n-exempt: 业务逻辑/格式化/日志字符串，非 UI 展示文本
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { ScheduledAnalysisPanel } from "@/components/stock-analysis/ScheduledAnalysisPanel";
import { invoke } from "@/lib/invoke";
import { App, Button, Card, Col, Modal, Row, Spin, Statistic, Table, Tabs, Tag } from "antd";
import { BarChart3, Calendar, Plus, RefreshCw, Trash2, TrendingDown, TrendingUp, Upload, Wallet } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

interface Holding {
  id: string;
  stockCode: string;
  stockName: string;
  shares: number;
  avgCost: number;
  currentPrice: number;
  marketValue: number;
  pnl: number;
  pnlPct: number;
  notes?: string;
  createdAt: number;
}

/** 组合跟踪看板 — 借鉴 TradingAgents tracking board 设计 */
export function PortfolioDashboard() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"holdings" | "scheduled">("holdings");

  return (
    <PageErrorBoundary title={t("stockAnalysis.page.portfolioDashboard")}>
      <div className="flex h-full flex-col">
        <Tabs
          activeKey={activeTab}
          onChange={(k) => setActiveTab(k as "holdings" | "scheduled")}
          size="small"
          className="portfolio-tabs"
          items={[
            {
              key: "holdings",
              label: (
                <span className="flex items-center gap-1">
                  <Wallet size={14} />
                  {t("portfolio.title")}
                </span>
              ),
              children: <HoldingsTab />,
            },
            {
              key: "scheduled",
              label: (
                <span className="flex items-center gap-1">
                  <Calendar size={14} />
                  {t("stockAnalysis.scheduledAnalysis.title")}
                </span>
              ),
              children: <ScheduledAnalysisPanel />,
            },
          ]}
        />
      </div>
    </PageErrorBoundary>
  );
}

/** 持仓视图 — 原 PortfolioDashboard 主体逻辑 */
function HoldingsTab() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { message: messageApi } = App.useApp();
  const [holdings, setHoldings] = useState<Holding[]>([]);
  const [loading, setLoading] = useState(false);
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [importText, setImportText] = useState("");
  const [importLoading, setImportLoading] = useState(false);
  const [vlmModalOpen, setVlmModalOpen] = useState(false);
  const [vlmResult, setVlmResult] = useState<Holding[]>([]);
  const [vlmRaw, setVlmRaw] = useState("");
  const refreshInterval = useRef<ReturnType<typeof setInterval> | null>(null);

  const loadHoldings = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<Holding[]>("list_portfolio");
      setHoldings(result.sort((a, b) => b.marketValue - a.marketValue));
    } catch (_e) {
      console.error("[portfolio] load failed:", _e);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    const initLoad = async () => {
      await loadHoldings();
    };
    initLoad();
    refreshInterval.current = setInterval(loadHoldings, 30000); // 30s auto-refresh
    return () => {
      if (refreshInterval.current) { clearInterval(refreshInterval.current); }
    };
  }, [loadHoldings]);

  // 汇总统计
  const totalMarketValue = holdings.reduce((s, h) => s + h.marketValue, 0);
  const totalPnl = holdings.reduce((s, h) => s + h.pnl, 0);
  const totalCost = holdings.reduce((s, h) => s + h.avgCost * h.shares, 0);
  const totalPnlPct = totalCost > 0 ? (totalPnl / totalCost) * 100 : 0;

  const handleDelete = async (id: string) => {
    try {
      await invoke("remove_portfolio_holding", { id });
      messageApi.success(t("common.deleted"));
      loadHoldings();
    } catch {
      messageApi.error(t("common.error"));
    }
  };

  const handleVlmImport = async () => {
    if (!vlmRaw.trim()) { return; }
    setImportLoading(true);
    try {
      const result = await invoke<{ success: boolean; holdings: Holding[] }>(
        "parse_vlm_portfolio_screenshot",
        { rawVlmOutput: vlmRaw },
      );
      if (result.success && result.holdings.length > 0) {
        setVlmResult(result.holdings);
        // 批量导入：逐条执行并收集成功/失败明细，避免中途失败后静默部分写入且无提示
        const ok: string[] = [];
        const failed: { code: string; reason: string }[] = [];
        for (const h of result.holdings) {
          try {
            await invoke("add_portfolio_holding", {
              stockCode: h.stockCode,
              stockName: h.stockName,
              shares: h.shares,
              avgCost: h.avgCost,
            });
            ok.push(h.stockCode);
          } catch (err) {
            failed.push({ code: h.stockCode, reason: String(err) });
          }
        }
        if (failed.length === 0) {
          messageApi.success(t("portfolio.importSuccess", { count: ok.length }));
        } else {
          // 部分失败时给出明细，让用户知道哪些成功、哪些失败
          messageApi.warning(
            t("stockAnalysis.portfolio.importPartial", { ok: ok.length, failed: failed.length }),
          );
          console.warn("[PortfolioDashboard] VLM 导入部分失败:", failed);
        }
        setVlmModalOpen(false);
        setVlmRaw("");
        loadHoldings();
      } else {
        messageApi.error(t("portfolio.importFailed"));
      }
    } catch (e) {
      messageApi.error(`${t("common.error")}: ${e}`);
    }
    setImportLoading(false);
  };

  // 手动快速导入
  const handleManualImport = async () => {
    if (!importText.trim()) { return; }
    setImportLoading(true);
    try {
      const lines = importText.trim().split("\n");
      let count = 0;
      for (const line of lines) {
        const parts = line.trim().split(/[,\t\s]+/);
        if (parts.length >= 3) {
          const stockCode = parts[0].replace(/\.(SH|SZ|BJ)$/i, "");
          const shares = parseFloat(parts[1]);
          const avgCost = parseFloat(parts[2]);
          const stockName = parts.length > 3 ? parts[3] : stockCode;
          if (!isNaN(shares) && !isNaN(avgCost) && shares > 0) {
            await invoke("add_portfolio_holding", {
              stockCode,
              stockName,
              shares,
              avgCost,
            });
            count++;
          }
        }
      }
      messageApi.success(t("portfolio.importSuccess", { count }));
      setImportModalOpen(false);
      setImportText("");
      loadHoldings();
    } catch (e) {
      messageApi.error(`${t("common.error")}: ${e}`);
    }
    setImportLoading(false);
  };

  const columns = [
    {
      title: t("portfolio.stock"),
      dataIndex: "stockName",
      key: "stockName",
      render: (_: string, r: Holding) => (
        <a onClick={() => navigate(`/stock-analysis?code=${r.stockCode}`)}>
          <span className="font-medium">{r.stockName}</span>
          <span className="text-xs ml-1" style={{ color: "var(--color-text-tertiary)" }}>
            {r.stockCode}
          </span>
        </a>
      ),
    },
    {
      title: t("portfolio.shares"),
      dataIndex: "shares",
      key: "shares",
      width: 100,
      render: (v: number) => v.toLocaleString(),
    },
    {
      title: t("portfolio.avgCost"),
      dataIndex: "avgCost",
      key: "avgCost",
      width: 110,
      render: (v: number) => `¥${v.toFixed(2)}`,
    },
    {
      title: t("portfolio.currentPrice"),
      dataIndex: "currentPrice",
      key: "currentPrice",
      width: 110,
      render: (v: number) => <span className="font-mono">¥{v.toFixed(2)}</span>,
    },
    {
      title: t("portfolio.marketValue"),
      dataIndex: "marketValue",
      key: "marketValue",
      width: 130,
      render: (v: number) => `¥${v.toLocaleString(undefined, { minimumFractionDigits: 2 })}`,
    },
    {
      title: t("portfolio.pnl"),
      key: "pnl",
      width: 140,
      render: (_: unknown, r: Holding) => (
        <span className={`font-mono ${r.pnl >= 0 ? "text-red-500" : "text-green-500"}`}>
          {r.pnl >= 0 ? "+" : ""}¥{r.pnl.toFixed(2)}
          <span className="text-xs ml-1">
            ({r.pnlPct >= 0 ? "+" : ""}
            {r.pnlPct.toFixed(2)}%)
          </span>
        </span>
      ),
    },
    {
      title: "",
      key: "action",
      width: 50,
      render: (_: unknown, r: Holding) => (
        <Button
          type="text"
          size="small"
          danger
          icon={<Trash2 size={14} />}
          onClick={() => handleDelete(r.id)}
        />
      ),
    },
  ];

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: "var(--color-border)" }}>
        <div className="flex items-center gap-3">
          <Wallet size={20} />
          <h2 className="text-base font-semibold m-0">{t("portfolio.title")}</h2>
          <Tag>{holdings.length} {t("portfolio.holdings")}</Tag>
        </div>
        <div className="flex gap-2">
          <Button icon={<Upload size={14} />} size="small" onClick={() => setVlmModalOpen(true)}>
            {t("portfolio.vlmImport")}
          </Button>
          <Button icon={<Plus size={14} />} size="small" onClick={() => setImportModalOpen(true)}>
            {t("portfolio.manualImport")}
          </Button>
          <Button icon={<RefreshCw size={14} />} size="small" loading={loading} onClick={loadHoldings}>
            {t("common.refresh")}
          </Button>
        </div>
      </div>

      {/* Summary */}
      <div className="p-4">
        <Row gutter={16}>
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("portfolio.totalMarketValue")}
                value={totalMarketValue}
                precision={2}
                prefix="¥"
                styles={{ content: { fontSize: 18 } }}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("portfolio.totalPnl")}
                value={totalPnl}
                precision={2}
                prefix={totalPnl >= 0
                  ? <TrendingUp size={16} className="text-red-500" />
                  : <TrendingDown size={16} className="text-green-500" />}
                suffix={`(${totalPnlPct >= 0 ? "+" : ""}${totalPnlPct.toFixed(2)}%)`}
                styles={{ content: { color: totalPnl >= 0 ? "var(--color-up)" : "var(--color-down)", fontSize: 18 } }}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("portfolio.totalCost")}
                value={totalCost}
                precision={2}
                prefix="¥"
                styles={{ content: { fontSize: 18 } }}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("portfolio.holdings")}
                value={holdings.length}
                prefix={<BarChart3 size={16} />}
                styles={{ content: { fontSize: 18 } }}
              />
            </Card>
          </Col>
        </Row>
      </div>

      {/* Holdings table */}
      <div className="flex-1 px-4 overflow-auto pb-4">
        <Spin spinning={loading}>
          <Table
            dataSource={holdings}
            columns={columns}
            rowKey="id"
            pagination={false}
            size="small"
            className="portfolio-table"
          />
        </Spin>
      </div>

      {/* Manual import modal */}
      <Modal
        title={t("portfolio.manualImport")}
        open={importModalOpen}
        onCancel={() => setImportModalOpen(false)}
        onOk={handleManualImport}
        confirmLoading={importLoading}
        okText={t("common.import")}
      >
        <p className="text-xs mb-2" style={{ color: "var(--color-text-secondary)" }}>
          {t("portfolio.importHint")}
        </p>
        <textarea
          className="w-full border rounded p-2 text-sm font-mono"
          rows={8}
          placeholder={t("portfolio.importPlaceholder")}
          value={importText}
          onChange={(e) => setImportText(e.target.value)}
          style={{ background: "var(--color-bg)", color: "var(--color-text)", borderColor: "var(--color-border)" }}
        />
      </Modal>

      {/* VLM import modal */}
      <Modal
        title={t("portfolio.vlmImport")}
        open={vlmModalOpen}
        onCancel={() => setVlmModalOpen(false)}
        onOk={handleVlmImport}
        confirmLoading={importLoading}
        okText={t("common.import")}
      >
        <p className="text-xs mb-2" style={{ color: "var(--color-text-secondary)" }}>
          {t("portfolio.vlmHint")}
        </p>
        <textarea
          className="w-full border rounded p-2 text-sm"
          rows={8}
          placeholder={t("portfolio.vlmPlaceholder")}
          value={vlmRaw}
          onChange={(e) => setVlmRaw(e.target.value)}
          style={{ background: "var(--color-bg)", color: "var(--color-text)", borderColor: "var(--color-border)" }}
        />
        {vlmResult.length > 0 && (
          <div className="mt-2">
            <p className="text-sm font-medium">{t("portfolio.preview")}:</p>
            {vlmResult.map((h, i) => (
              <div key={i} className="text-xs py-1">
                {h.stockName} ({h.stockCode}) — {h.shares.toLocaleString()}
                {t("stockAnalysis.sharesUnit")} @ ¥{h.avgCost.toFixed(2)}
              </div>
            ))}
          </div>
        )}
      </Modal>
    </div>
  );
}
