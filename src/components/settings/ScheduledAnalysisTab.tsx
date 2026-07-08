import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { App, Button, Card, Empty, Form, Input, Popconfirm, Select, Spin, Switch, Table, Tag } from "antd";
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

function getCronPresets(t: (k: string) => string) {
  return [
    { label: t("stockAnalysis.scheduledAnalysis.cron.daily9am"), value: "0 9 * * *" },
    { label: t("stockAnalysis.scheduledAnalysis.cron.daily1530"), value: "30 15 * * *" },
    { label: t("stockAnalysis.scheduledAnalysis.cron.weeklyMon"), value: "0 9 * * 1" },
    { label: t("stockAnalysis.scheduledAnalysis.cron.hourly"), value: "0 * * * *" },
  ];
}

export function ScheduledAnalysisTab() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  const [jobs, setJobs] = useState<CronJobRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState(false);
  const [form] = Form.useForm();

  // 自选股自动扫描
  const [wlScanJobs, setWlScanJobs] = useState<CronJobRow[]>([]);
  const [wlCron, setWlCron] = useState("0 21 * * 1-5");

  const loadWlScan = async () => {
    try {
      const list = await invoke<CronJobRow[]>("list_watchlist_scan_crons");
      if (Array.isArray(list)) { setWlScanJobs(list); }
    } catch { /* backend not running */ }
  };

  const loadTasks = useCallback(async () => {
    try {
      const list = await invoke<CronJobRow[]>("list_stock_crons");
      if (Array.isArray(list)) { setJobs(list); }
    } catch { /* backend not running */ }
  }, []);

  const toggleWlScan = async (job: CronJobRow | null, enable: boolean) => {
    try {
      if (enable && !job) {
        await invoke("create_watchlist_scan_cron", {
          cronExpression: wlCron,
          enabled: true,
        });
        message.success(t("stockAnalysis.scheduledAnalysis.scanStarted"));
      } else if (!enable && job) {
        await invoke("toggle_watchlist_scan_cron", { id: job.id, enabled: false });
        message.success(t("stockAnalysis.scheduledAnalysis.scanPaused"));
      }
      loadWlScan();
    } catch {
      message.error(t("stockAnalysis.scheduledAnalysis.operationFailed"));
    }
  };

  const deleteWlScan = async (id: string) => {
    try {
      await invoke("delete_watchlist_scan_cron", { id });
      loadWlScan();
    } catch { /* silent */ }
  };

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return invoke<CronJobRow[]>("list_stock_crons");
    })
      .then((list) => {
        if (cancelled) { return; }
        if (Array.isArray(list)) { setJobs(list); }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    invoke<CronJobRow[]>("list_watchlist_scan_crons")
      .then((list) => {
        if (cancelled) { return; }
        if (Array.isArray(list)) { setWlScanJobs(list); }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const create = async (values: Record<string, unknown>) => {
    try {
      await invoke("create_stock_cron", {
        stockCode: values.stockCode,
        stockName: values.stockName,
        cronExpression: values.cronExpression,
      });
      form.resetFields();
      setAdding(false);
      loadTasks();
      message.success(t("stockAnalysis.scheduledAnalysis.taskCreated"));
    } catch {
      message.error(t("stockAnalysis.scheduledAnalysis.createFailed"));
    }
  };

  const toggle = useCallback(async (id: string, active: boolean) => {
    try {
      await invoke("toggle_stock_cron", { id, enabled: active });
      loadTasks();
    } catch { /* silent */ }
  }, [loadTasks]);

  const remove = useCallback(async (id: string) => {
    try {
      await invoke("delete_stock_cron", { id });
      loadTasks();
    } catch { /* silent */ }
  }, [loadTasks]);

  const columns = useMemo(() => [
    {
      title: t("stockAnalysis.scheduledAnalysis.column.id"),
      dataIndex: "name",
      ellipsis: true,
      width: 100,
      render: (v: string) => <span className="text-xs font-mono">{v}</span>,
    },
    { title: t("stockAnalysis.scheduledAnalysis.column.description"), dataIndex: "description", ellipsis: true },
    {
      title: "Cron",
      dataIndex: "schedule",
      width: 120,
      render: (v: string) => <Tag className="text-xs m-0 font-mono">{v}</Tag>,
    },
    {
      title: t("stockAnalysis.scheduledAnalysis.column.executed"),
      dataIndex: "runCount",
      width: 50,
      align: "center" as const,
      render: (v: number) => <span className="text-xs">{v}</span>,
    },
    {
      title: t("stockAnalysis.scheduledAnalysis.column.status"),
      dataIndex: "status",
      width: 60,
      render: (v: string, record: CronJobRow) => (
        <Switch
          size="small"
          checked={v === "active"}
          onChange={(checked) => toggle(record.id, checked)}
        />
      ),
    },
    {
      title: "",
      key: "action",
      width: 30,
      render: (_: unknown, record: CronJobRow) => (
        <Popconfirm
          title={t("stockAnalysis.scheduledAnalysis.confirmDeleteTitle")}
          onConfirm={() => remove(record.id)}
        >
          <Button size="small" type="text" danger icon={<DeleteOutlined />} />
        </Popconfirm>
      ),
    },
  ], [remove, toggle, t]);

  if (loading) { return <Spin size="small" />; }

  return (
    <div className="flex flex-col gap-3">
      <div className="text-sm text-gray-500">
        {t("stockAnalysis.scheduledAnalysis.description")}
      </div>

      {/* watchlist auto scan */}
      <Card
        size="small"
        title={t("stockAnalysis.scheduledAnalysis.watchlistScan")}
        styles={{ body: { padding: "8px 12px" } }}
      >
        <div className="flex items-center gap-3">
          <Switch
            checked={wlScanJobs.some((j) => j.status === "active")}
            onChange={(checked) => toggleWlScan(wlScanJobs.length > 0 ? wlScanJobs[0] : null, checked)}
          />
          <span className="text-xs text-gray-400">
            {wlScanJobs.some((j) => j.status === "active")
              ? t("stockAnalysis.scheduledAnalysis.enabled")
              : t("stockAnalysis.scheduledAnalysis.disabled")}
          </span>
          <Select
            size="small"
            style={{ width: 180 }}
            value={wlCron}
            onChange={setWlCron}
            options={[
              { label: t("stockAnalysis.scheduledAnalysis.cron.tradeDayClose"), value: "0 21 * * 1-5" },
              { label: t("stockAnalysis.scheduledAnalysis.cron.dailyOpen"), value: "0 9 * * *" },
              { label: t("stockAnalysis.scheduledAnalysis.cron.dailyClose"), value: "30 15 * * *" },
              { label: t("stockAnalysis.scheduledAnalysis.cron.weeklyMon"), value: "0 9 * * 1" },
            ]}
          />
          {wlScanJobs.length > 0 && (
            <Popconfirm
              title={t("stockAnalysis.scheduledAnalysis.confirmDelete")}
              onConfirm={() => deleteWlScan(wlScanJobs[0].id)}
            >
              <Button size="small" type="text" danger icon={<DeleteOutlined />} />
            </Popconfirm>
          )}
        </div>
      </Card>

      <Card
        size="small"
        title={t("stockAnalysis.scheduledAnalysis.newTask")}
        styles={{ body: { padding: "8px 12px" } }}
        extra={
          <Button size="small" icon={<PlusOutlined />} onClick={() => setAdding(!adding)}>
            {adding ? t("stockAnalysis.scheduledAnalysis.cancel") : t("stockAnalysis.scheduledAnalysis.new")}
          </Button>
        }
      >
        {adding && (
          <Form form={form} size="small" layout="inline" onFinish={create}>
            <Form.Item name="stockCode" rules={[{ required: true }]} initialValue={stockCode}>
              <Input placeholder={t("stockAnalysis.scheduledAnalysis.placeholder.stockCode")} style={{ width: 100 }} />
            </Form.Item>
            <Form.Item name="stockName" rules={[{ required: true }]} initialValue={stockName}>
              <Input placeholder={t("stockAnalysis.scheduledAnalysis.placeholder.stockName")} style={{ width: 100 }} />
            </Form.Item>
            <Form.Item name="cronExpression" rules={[{ required: true }]} initialValue="0 9 * * *">
              <Select
                style={{ width: 160 }}
                options={getCronPresets(t)}
                placeholder={t("stockAnalysis.scheduledAnalysis.placeholder.cron")}
              />
            </Form.Item>
            <Form.Item>
              <Button type="primary" htmlType="submit">{t("stockAnalysis.scheduledAnalysis.create")}</Button>
            </Form.Item>
          </Form>
        )}
      </Card>

      {jobs.length === 0
        ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("stockAnalysis.scheduledAnalysis.empty")} />
        : <Table size="small" dataSource={jobs} columns={columns} rowKey="id" pagination={false} />}
    </div>
  );
}
