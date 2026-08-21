import { invoke } from "@/lib/invoke";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import {
  App,
  Button,
  Card,
  Empty,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Select,
  Slider,
  Space,
  Switch,
  Table,
  Tag,
  Tooltip,
} from "antd";
import { Clock, Zap } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

type PeriodKey = "short" | "mid" | "long";

interface RecoCronConfig {
  periods: PeriodKey[];
  minConfidence: number;
  topN: number;
}

interface RecoCronRow {
  id: string;
  name: string;
  description: string;
  schedule: string;
  status: string;
  runCount: number;
  lastRunAt: number | null;
  nextRunAt: number | null;
  config: RecoCronConfig;
  lastPicksCount: number | null;
}

const CRON_PRESETS = [
  { labelKey: "settings.scheduled-recommendation.preset-daily-open", value: "0 9 * * *" },
  { labelKey: "settings.scheduled-recommendation.preset-daily-close", value: "30 15 * * *" },
  { labelKey: "settings.scheduled-recommendation.preset-monday-open", value: "0 9 * * 1" },
  { labelKey: "settings.scheduled-recommendation.preset-monday-close", value: "30 15 * * 1" },
  { labelKey: "settings.scheduled-recommendation.preset-hourly", value: "0 * * * *" },
];

const INTERVAL_PRESETS = [
  { labelKey: "settings.scheduled-recommendation.interval-30min", value: "*/30 * * * *" },
  { labelKey: "settings.scheduled-recommendation.interval-1h", value: "0 * * * *" },
  { labelKey: "settings.scheduled-recommendation.interval-2h", value: "0 */2 * * *" },
  { labelKey: "settings.scheduled-recommendation.interval-4h", value: "0 */4 * * *" },
];

const ALL_PERIODS: { value: PeriodKey; labelKey: string }[] = [
  { value: "short", labelKey: "settings.scheduled-recommendation.period-short" },
  { value: "mid", labelKey: "settings.scheduled-recommendation.period-mid" },
  { value: "long", labelKey: "settings.scheduled-recommendation.period-long" },
];

export function ScheduledRecommendationTab() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [jobs, setJobs] = useState<RecoCronRow[]>([]);
  const [adding, setAdding] = useState(false);
  const [, setLoading] = useState(true);
  const [form] = Form.useForm();

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return invoke<RecoCronRow[]>("list_recommendation_crons");
    })
      .then((list) => {
        if (cancelled) { return; }
        if (Array.isArray(list)) { setJobs(list); }
      })
      .catch((e) => {
        console.warn("[ScheduledRecommendation] load failed:", e);
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const loadTasks = useCallback(async () => {
    try {
      const list = await invoke<RecoCronRow[]>("list_recommendation_crons");
      if (Array.isArray(list)) { setJobs(list); }
    } catch { /* 静默 */ }
  }, []);

  const create = async (values: Record<string, unknown>) => {
    try {
      await invoke("create_recommendation_cron", {
        name: values.name,
        cronExpression: values.cronExpression,
        periods: values.periods,
        minConfidence: values.minConfidence,
        topN: values.topN,
      });
      form.resetFields();
      setAdding(false);
      loadTasks();
      message.success(t("settings.scheduled-recommendation.task-created"));
    } catch (e: unknown) {
      message.error(
        t("settings.scheduled-recommendation.create-failed", { error: e instanceof Error ? e.message : String(e) }),
      );
    }
  };

  const toggle = useCallback(async (id: string, active: boolean) => {
    try {
      await invoke("toggle_recommendation_cron", { id, enabled: active });
      loadTasks();
    } catch { /* 静默 */ }
  }, [loadTasks]);

  const remove = useCallback(async (id: string) => {
    try {
      await invoke("delete_recommendation_cron", { id });
      loadTasks();
    } catch { /* 静默 */ }
  }, [loadTasks]);

  const columns = useMemo(
    () => [
      {
        title: t("settings.scheduled-recommendation.column-name"),
        dataIndex: "name",
        width: 140,
        ellipsis: true,
        render: (v: string) => <span className="text-xs font-medium">{v}</span>,
      },
      {
        title: t("settings.scheduled-recommendation.column-period"),
        key: "config",
        width: 200,
        render: (_: unknown, r: RecoCronRow) => (
          <div className="flex flex-col gap-0.5 text-[11px]">
            <span className="text-gray-600">
              {r.config.periods.map((p) => t(ALL_PERIODS.find((x) => x.value === p)?.labelKey ?? p)).join(" / ")}
            </span>
            <span className="text-gray-400">
              {t("settings.scheduled-recommendation.min-confidence-and-top-n", {
                confidence: r.config.minConfidence,
                topN: r.config.topN,
              })}
            </span>
            {r.lastPicksCount !== null && (
              <span className="text-gray-400">
                {t("settings.scheduled-recommendation.last-push-count", { count: r.lastPicksCount })}
              </span>
            )}
          </div>
        ),
      },
      {
        title: "Cron",
        dataIndex: "schedule",
        width: 130,
        render: (v: string) => <Tag className="text-[10px] m-0 font-mono">{v}</Tag>,
      },
      {
        title: t("settings.scheduled-recommendation.column-executed"),
        dataIndex: "runCount",
        width: 50,
        align: "center" as const,
        render: (v: number) => <span className="text-xs">{v}</span>,
      },
      {
        title: t("settings.scheduled-recommendation.column-status"),
        dataIndex: "status",
        width: 70,
        render: (v: string, record: RecoCronRow) => (
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
        render: (_: unknown, record: RecoCronRow) => (
          <Popconfirm
            title={t("settings.scheduled-recommendation.confirm-delete")}
            onConfirm={() => remove(record.id)}
          >
            <Button size="small" type="text" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        ),
      },
    ],
    [remove, toggle, t],
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="text-xs text-gray-500 leading-relaxed">
        {t("settings.scheduled-recommendation.description")}
        <Tooltip title={t("settings.scheduled-recommendation.tooltip-cron-dependency")}>
          <Tag color="orange" className="m-0 ml-1 text-[10px]">
            {t("settings.scheduled-recommendation.tag-app-online")}
          </Tag>
        </Tooltip>
      </div>

      <Card
        size="small"
        title={
          <Space>
            <PlusOutlined />
            <span>{t("settings.scheduled-recommendation.new-task")}</span>
          </Space>
        }
        styles={{ body: { padding: "12px" } }}
        extra={
          <Button size="small" onClick={() => setAdding(!adding)}>
            {adding ? t("settings.scheduled-recommendation.cancel") : t("settings.scheduled-recommendation.expand")}
          </Button>
        }
      >
        {adding && (
          <Form
            form={form}
            size="small"
            layout="vertical"
            onFinish={create}
            initialValues={{
              name: t("settings.scheduled-recommendation.default-name"),
              periods: ["short", "mid"],
              minConfidence: 60,
              topN: 5,
              cronExpression: "30 15 * * *",
            }}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-2">
              <Form.Item
                name="name"
                label={t("settings.scheduled-recommendation.label-task-name")}
                rules={[{ required: true, max: 64 }]}
                className="!mb-0"
              >
                <Input placeholder={t("settings.scheduled-recommendation.placeholder-task-name")} />
              </Form.Item>

              <Form.Item
                name="cronExpression"
                label={
                  <Space size={4}>
                    <Clock size={12} />
                    <span>{t("settings.scheduled-recommendation.label-trigger-time")}</span>
                  </Space>
                }
                rules={[{ required: true }]}
                className="!mb-0"
              >
                <Select
                  options={[
                    {
                      label: t("settings.scheduled-recommendation.group-presets"),
                      options: CRON_PRESETS.map((p) => ({ ...p, label: t(p.labelKey) })),
                      disabled: true,
                    },
                    {
                      label: t("settings.scheduled-recommendation.group-custom-interval"),
                      options: INTERVAL_PRESETS.map((p) => ({ ...p, label: t(p.labelKey) })),
                      disabled: true,
                    },
                  ].flatMap((g) => g.options)}
                  placeholder={t("settings.scheduled-recommendation.placeholder-cron-preset")}
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>

              <Form.Item
                name="periods"
                label={t("settings.scheduled-recommendation.label-periods")}
                rules={[{ required: true, type: "array", min: 1 }]}
                className="!mb-0"
              >
                <Select
                  mode="multiple"
                  options={ALL_PERIODS.map((p) => ({ ...p, label: t(p.labelKey) }))}
                  placeholder={t("settings.scheduled-recommendation.placeholder-periods")}
                  maxTagCount="responsive"
                />
              </Form.Item>

              <Form.Item
                name="minConfidence"
                label={
                  <Space size={4}>
                    <Zap size={12} />
                    <span>{t("settings.scheduled-recommendation.label-min-confidence")}</span>
                  </Space>
                }
                rules={[{ required: true, type: "number", min: 0, max: 100 }]}
                className="!mb-0"
              >
                <Slider min={0} max={100} marks={{ 0: "0", 50: "50", 80: "80", 100: "100" }} />
              </Form.Item>

              <Form.Item
                name="topN"
                label={t("settings.scheduled-recommendation.label-top-n")}
                rules={[{ required: true, type: "number", min: 1, max: 20 }]}
                className="!mb-0"
              >
                <InputNumber min={1} max={20} step={1} className="!w-full" />
              </Form.Item>
            </div>

            <div className="flex justify-end mt-3">
              <Button type="primary" htmlType="submit" size="small">
                {t("settings.scheduled-recommendation.create-task")}
              </Button>
            </div>
          </Form>
        )}
      </Card>

      {jobs.length === 0
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("settings.scheduled-recommendation.empty-description")}
          />
        )
        : (
          <Table
            size="small"
            dataSource={jobs}
            columns={columns}
            rowKey="id"
            pagination={false}
          />
        )}
    </div>
  );
}
