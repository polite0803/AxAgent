import { invoke } from "@/lib/invoke";
import {
  Button,
  Card,
  Col,
  Empty,
  Input,
  message,
  Modal,
  Popconfirm,
  Row,
  Select,
  Spin,
  Statistic,
  Switch,
  Table,
  Tag,
  Tooltip,
} from "antd";
import { AlertCircle, AlertTriangle, Briefcase, Calendar, Clock, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface CronJobRow {
  id: string;
  name: string;
  description: string;
  schedule: string;
  status: string;
  runCount: number;
  lastRunAt: number | null;
}

const FAILURE_THRESHOLD = 3;

const CRON_PRESETS = [
  { label: "每个交易日 21:00 (收盘分析)", value: "0 21 * * 1-5" },
  { label: "每个交易日 08:00 (盘前分析)", value: "0 8 * * 1-5" },
  { label: "每周一 21:00", value: "0 21 * * 1" },
  { label: "每两小时", value: "0 */2 * * *" },
];

/** 定时分析管理面板 — 借鉴 TradingAgents scheduled analysis 设计 */
export function ScheduledAnalysisPanel() {
  const { t } = useTranslation();
  const [jobs, setJobs] = useState<CronJobRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [addModalOpen, setAddModalOpen] = useState(false);
  const [newStockCode, setNewStockCode] = useState("");
  const [newStockName, setNewStockName] = useState("");
  const [newCron, setNewCron] = useState(CRON_PRESETS[0].value);
  const [submitting, setSubmitting] = useState(false);

  // 失败计数（模拟：从 description 字段解析）
  // 后端可扩展 consecutive_failures 字段，前端先用 description 后缀标记
  const getFailureCount = (job: CronJobRow): number => {
    if (!job.description) { return 0; }
    const m = job.description.match(/failures:(\d+)/);
    return m ? parseInt(m[1]) : 0;
  };

  const loadJobs = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<CronJobRow[]>("list_stock_crons");
      if (Array.isArray(list)) { setJobs(list); }
    } catch { /* backend not available */ }
    setLoading(false);
  }, []);

  useEffect(() => {
    const init = async () => {
      await loadJobs();
    };
    init();
  }, [loadJobs]);

  const handleCreate = async () => {
    if (!newStockCode.trim()) {
      message.warning(t("stockAnalysis.scheduledAnalysis.stockCodeRequired") ?? "请输入股票代码");
      return;
    }
    setSubmitting(true);
    try {
      await invoke("create_stock_cron", {
        stockCode: newStockCode.trim(),
        stockName: newStockName.trim() || newStockCode.trim(),
        cronExpression: newCron,
      });
      message.success(t("stockAnalysis.scheduledAnalysis.taskCreated"));
      setAddModalOpen(false);
      setNewStockCode("");
      setNewStockName("");
      loadJobs();
    } catch (e) {
      message.error(`${t("common.error")}: ${e}`);
    }
    setSubmitting(false);
  };

  const handleToggle = useCallback(async (id: string, enabled: boolean) => {
    try {
      await invoke("toggle_stock_cron", { id, enabled });
      loadJobs();
    } catch { /* silent */ }
  }, [loadJobs]);

  const handleDelete = useCallback(async (id: string) => {
    try {
      await invoke("delete_stock_cron", { id });
      message.success(t("common.deleted"));
      loadJobs();
    } catch { /* silent */ }
  }, [loadJobs, t]);

  /** P1: 从持仓一键导入 */
  const handleImportFromPortfolio = async () => {
    try {
      const holdings = await invoke<Array<{ stockCode: string; stockName: string }>>("list_portfolio");
      if (!Array.isArray(holdings) || holdings.length === 0) {
        message.info(t("stockAnalysis.scheduledAnalysis.noPortfolioData") ?? "暂无持仓数据");
        return;
      }
      const existing = await invoke<CronJobRow[]>("list_stock_crons");
      const existingCodes = new Set(
        Array.isArray(existing) ? existing.map((j) => j.name) : [],
      );
      let added = 0;
      for (const h of holdings) {
        if (!existingCodes.has(h.stockCode)) {
          try {
            await invoke("create_stock_cron", {
              stockCode: h.stockCode,
              stockName: h.stockName || h.stockCode,
              cronExpression: newCron,
            });
            added++;
          } catch { /* skip duplicates */ }
        }
      }
      message.success(
        t("stockAnalysis.scheduledAnalysis.importedFromPortfolio", { count: added })
          ?? `已从持仓导入 ${added} 个定时分析`,
      );
      loadJobs();
    } catch (e) {
      message.error(`${t("common.error")}: ${e}`);
    }
  };

  const columns = useMemo(() => [
    {
      title: t("stockAnalysis.stockCode") ?? "代码",
      dataIndex: "name",
      width: 100,
      render: (v: string) => <span className="font-mono font-medium text-xs">{v}</span>,
    },
    {
      title: t("stockAnalysis.scheduledAnalysis.column.description"),
      dataIndex: "description",
      ellipsis: true,
      width: 180,
      render: (v: string, record: CronJobRow) => {
        const failures = getFailureCount(record);
        return (
          <div className="flex items-center gap-1">
            <span className="text-xs truncate">{v?.replace(/\s*failures:\d+/, "") || "-"}</span>
            {failures >= FAILURE_THRESHOLD && (
              <Tooltip title={`${t("stockAnalysis.scheduledAnalysis.autoDisabled") ?? "连续失败已自动停用"}`}>
                <AlertTriangle size={12} className="text-orange-500" />
              </Tooltip>
            )}
            {failures > 0 && failures < FAILURE_THRESHOLD && (
              <span className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                ({failures}/{FAILURE_THRESHOLD})
              </span>
            )}
          </div>
        );
      },
    },
    {
      title: "Cron",
      dataIndex: "schedule",
      width: 130,
      render: (v: string) => <Tag className="text-xs m-0 font-mono">{v}</Tag>,
    },
    {
      title: t("stockAnalysis.scheduledAnalysis.column.executed") ?? "执行次数",
      dataIndex: "runCount",
      width: 70,
      align: "center" as const,
      render: (v: number) => <span className="text-xs">{v}</span>,
    },
    {
      title: t("stockAnalysis.scheduledAnalysis.column.lastRun") ?? "上次执行",
      dataIndex: "lastRunAt",
      width: 100,
      render: (v: number | null) => {
        if (!v) { return <span className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>-</span>; }
        const d = new Date(v);
        return (
          <span className="text-xs">
            {d.toLocaleDateString()} {d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
          </span>
        );
      },
    },
    {
      title: t("stockAnalysis.scheduledAnalysis.column.status") ?? "状态",
      dataIndex: "status",
      width: 70,
      render: (v: string, record: CronJobRow) => {
        const failures = getFailureCount(record);
        const disabled = failures >= FAILURE_THRESHOLD;
        return (
          <Switch
            size="small"
            checked={v === "active" && !disabled}
            disabled={disabled}
            onChange={(checked) => handleToggle(record.id, checked)}
          />
        );
      },
    },
    {
      title: "",
      key: "action",
      width: 40,
      render: (_: unknown, record: CronJobRow) => (
        <Popconfirm
          title={t("stockAnalysis.scheduledAnalysis.confirmDeleteTitle") ?? "确认删除？"}
          onConfirm={() => handleDelete(record.id)}
        >
          <Button size="small" type="text" danger icon={<Trash2 size={14} />} />
        </Popconfirm>
      ),
    },
  ], [t, handleToggle, handleDelete]);

  // 统计
  const activeCount = jobs.filter((j) => j.status === "active" && getFailureCount(j) < FAILURE_THRESHOLD).length;
  const failedCount = jobs.filter((j) => getFailureCount(j) >= FAILURE_THRESHOLD).length;
  const totalRunCount = jobs.reduce((s, j) => s + j.runCount, 0);

  return (
    <div className="flex flex-col gap-3">
      {/* 统计卡片 */}
      <Row gutter={12}>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("stockAnalysis.scheduledAnalysis.totalTasks") ?? "定时分析"}
              value={jobs.length}
              prefix={<Clock size={14} />}
              valueStyle={{ fontSize: 18 }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("stockAnalysis.scheduledAnalysis.activeTasks") ?? "运行中"}
              value={activeCount}
              valueStyle={{ fontSize: 18, color: activeCount > 0 ? "var(--color-up)" : undefined }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("stockAnalysis.scheduledAnalysis.failedTasks") ?? "已停用"}
              value={failedCount}
              valueStyle={{ fontSize: 18, color: failedCount > 0 ? "var(--color-down)" : undefined }}
              prefix={failedCount > 0 ? <AlertCircle size={14} /> : undefined}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("stockAnalysis.scheduledAnalysis.totalRuns") ?? "累计执行"}
              value={totalRunCount}
              prefix={<Calendar size={14} />}
              valueStyle={{ fontSize: 18 }}
            />
          </Card>
        </Col>
      </Row>

      {/* 操作栏 */}
      <div className="flex items-center justify-between">
        <p className="text-xs m-0" style={{ color: "var(--color-text-secondary)" }}>
          {t("stockAnalysis.scheduledAnalysis.description")
            ?? "定时分析任务在每日指定时间自动执行，连续失败3次自动停用。"}
        </p>
        <div className="flex gap-2">
          <Button size="small" icon={<Briefcase size={14} />} onClick={handleImportFromPortfolio}>
            {t("stockAnalysis.scheduledAnalysis.importFromPortfolio") ?? "从持仓导入"}
          </Button>
          <Button size="small" icon={<RefreshCw size={14} />} onClick={loadJobs} loading={loading}>
            {t("common.refresh")}
          </Button>
          <Button size="small" type="primary" icon={<Plus size={14} />} onClick={() => setAddModalOpen(true)}>
            {t("stockAnalysis.scheduledAnalysis.addTask") ?? "添加任务"}
          </Button>
        </div>
      </div>

      {/* 表格区 */}
      {loading
        ? (
          <div className="flex justify-center py-8">
            <Spin />
          </div>
        )
        : jobs.length === 0
        ? (
          <Empty
            description={t("stockAnalysis.scheduledAnalysis.empty") ?? "暂无定时分析任务"}
            className="py-8"
          >
            <Button size="small" type="primary" onClick={() => setAddModalOpen(true)}>
              {t("stockAnalysis.scheduledAnalysis.addTask") ?? "添加任务"}
            </Button>
          </Empty>
        )
        : (
          <Table
            dataSource={jobs}
            columns={columns}
            rowKey="id"
            pagination={false}
            size="small"
            className="scheduled-analysis-table"
          />
        )}

      {/* 添加任务弹窗 */}
      <Modal
        title={t("stockAnalysis.scheduledAnalysis.addTask") ?? "添加定时分析任务"}
        open={addModalOpen}
        onCancel={() => setAddModalOpen(false)}
        onOk={handleCreate}
        confirmLoading={submitting}
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
      >
        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium">{t("stockAnalysis.stockCode") ?? "股票代码"}</label>
            <Input
              size="small"
              placeholder="600519"
              value={newStockCode}
              onChange={(e) => setNewStockCode(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium">{t("stockAnalysis.stockName") ?? "股票名称"}</label>
            <Input
              size="small"
              placeholder={t("stockAnalysis.scheduledAnalysis.namePlaceholder") ?? "可选，留空用代码"}
              value={newStockName}
              onChange={(e) => setNewStockName(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium">{t("stockAnalysis.scheduledAnalysis.schedule") ?? "执行时间"}</label>
            <Select
              size="small"
              value={newCron}
              onChange={setNewCron}
              options={CRON_PRESETS}
            />
          </div>
        </div>
      </Modal>
    </div>
  );
}
