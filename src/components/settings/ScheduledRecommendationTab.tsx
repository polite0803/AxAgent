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
  { label: "每日 09:00 (开盘前)", value: "0 9 * * *" },
  { label: "每日 15:30 (收盘)", value: "30 15 * * *" },
  { label: "每周一 09:00", value: "0 9 * * 1" },
  { label: "每周一 15:30", value: "30 15 * * 1" },
  { label: "每小时", value: "0 * * * *" },
];

const INTERVAL_PRESETS = [
  { label: "每 30 分钟", value: "*/30 * * * *" },
  { label: "每 1 小时", value: "0 * * * *" },
  { label: "每 2 小时", value: "0 */2 * * *" },
  { label: "每 4 小时", value: "0 */4 * * *" },
];

const ALL_PERIODS: { value: PeriodKey; label: string }[] = [
  { value: "short", label: "短线 (1-2 周)" },
  { value: "mid", label: "中线 (3-8 周)" },
  { value: "long", label: "长线 (3 个月+)" },
];

export function ScheduledRecommendationTab() {
  const { message } = App.useApp();
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
      message.success("定时荐股任务已创建");
    } catch (e: unknown) {
      message.error(`创建失败: ${e instanceof Error ? e.message : String(e)}`);
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
        title: "名称",
        dataIndex: "name",
        width: 140,
        ellipsis: true,
        render: (v: string) => <span className="text-xs font-medium">{v}</span>,
      },
      {
        title: "周期 / 推送",
        key: "config",
        width: 200,
        render: (_: unknown, r: RecoCronRow) => (
          <div className="flex flex-col gap-0.5 text-[11px]">
            <span className="text-gray-600">
              {r.config.periods.map((p) => ALL_PERIODS.find((x) => x.value === p)?.label ?? p).join(" / ")}
            </span>
            <span className="text-gray-400">
              最低置信 {r.config.minConfidence} · 推送 Top {r.config.topN}
            </span>
            {r.lastPicksCount !== null && <span className="text-gray-400">上次推送 {r.lastPicksCount} 只</span>}
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
        title: "已执行",
        dataIndex: "runCount",
        width: 50,
        align: "center" as const,
        render: (v: number) => <span className="text-xs">{v}</span>,
      },
      {
        title: "状态",
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
          <Popconfirm title="确认删除?" onConfirm={() => remove(record.id)}>
            <Button size="small" type="text" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        ),
      },
    ],
    [remove, toggle],
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="text-xs text-gray-500 leading-relaxed">
        按 Cron 表达式定时跑智能荐股扫描，过滤兜底合成后只推"真实"推荐（按置信度排序 Top N）到操作系统通知。
        <Tooltip title="cron 调度需 App 进程运行中；App 关闭时不会触发">
          <Tag color="orange" className="m-0 ml-1 text-[10px]">依赖 App 在线</Tag>
        </Tooltip>
      </div>

      <Card
        size="small"
        title={
          <Space>
            <PlusOutlined />
            <span>新建定时荐股任务</span>
          </Space>
        }
        styles={{ body: { padding: "12px" } }}
        extra={
          <Button size="small" onClick={() => setAdding(!adding)}>
            {adding ? "取消" : "展开"}
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
              name: "智能荐股",
              periods: ["short", "mid"],
              minConfidence: 60,
              topN: 5,
              cronExpression: "30 15 * * *",
            }}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-2">
              <Form.Item
                name="name"
                label="任务名称"
                rules={[{ required: true, max: 64 }]}
                className="!mb-0"
              >
                <Input placeholder="例如：每日收盘荐股" />
              </Form.Item>

              <Form.Item
                name="cronExpression"
                label={
                  <Space size={4}>
                    <Clock size={12} />
                    <span>触发时机</span>
                  </Space>
                }
                rules={[{ required: true }]}
                className="!mb-0"
              >
                <Select
                  options={[
                    { label: "─ 预设时间点 ─", options: CRON_PRESETS, disabled: true },
                    { label: "─ 自定义间隔 ─", options: INTERVAL_PRESETS, disabled: true },
                  ].flatMap((g) => g.options)}
                  placeholder="选择预设或输入 cron"
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>

              <Form.Item
                name="periods"
                label="荐股周期（多选）"
                rules={[{ required: true, type: "array", min: 1 }]}
                className="!mb-0"
              >
                <Select
                  mode="multiple"
                  options={ALL_PERIODS}
                  placeholder="选择至少一个周期"
                  maxTagCount="responsive"
                />
              </Form.Item>

              <Form.Item
                name="minConfidence"
                label={
                  <Space size={4}>
                    <Zap size={12} />
                    <span>最低置信度（0-100）</span>
                  </Space>
                }
                rules={[{ required: true, type: "number", min: 0, max: 100 }]}
                className="!mb-0"
              >
                <Slider min={0} max={100} marks={{ 0: "0", 50: "50", 80: "80", 100: "100" }} />
              </Form.Item>

              <Form.Item
                name="topN"
                label="推送条数（Top N）"
                rules={[{ required: true, type: "number", min: 1, max: 20 }]}
                className="!mb-0"
              >
                <InputNumber min={1} max={20} step={1} className="!w-full" />
              </Form.Item>
            </div>

            <div className="flex justify-end mt-3">
              <Button type="primary" htmlType="submit" size="small">
                创建
              </Button>
            </div>
          </Form>
        )}
      </Card>

      {jobs.length === 0
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="暂无定时荐股任务"
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
