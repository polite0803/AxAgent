import { invoke } from "@/lib/invoke";
import { buildAsOfDateRange, type WorkdayRange } from "@/lib/replay/workdays";
import {
  Alert,
  Button,
  Card,
  Collapse,
  DatePicker,
  Form,
  Input,
  InputNumber,
  Progress,
  Select,
  Space,
  Statistic,
  Table,
  Tag,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import dayjs, { type Dayjs } from "dayjs";
import { CalendarRange, Download, Play, ShieldAlert } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { RangePicker } = DatePicker;

// ── 共享类型(与后端 `run_replay_backtest` 返回的形状对齐) ──
export interface ReplaySweepItem {
  stock_code: string;
  as_of_date: string;
  decision_action: string;
  decision_confidence: number;
}

export interface ReplaySweepInvalid {
  stock_code: string;
  as_of_date: string;
  reason: string;
}

export interface ReplaySweepStats {
  total: number;
  correct: number;
  wrong: number;
  accuracy: number;
  avg_return: number;
  alpha: number;
  avg_confidence: number;
  total_pnl: number;
  sharpe: number;
  max_drawdown: number;
}

export interface ReplaySweepResult {
  total: number;
  valid: number;
  invalid: number;
  results: Array<{
    stock_code: string;
    analysis_date: string;
    decision_action: string;
    return_rate: number;
    is_correct: boolean;
    confidence: number;
  }>;
  invalid_details: ReplaySweepInvalid[];
  stats: ReplaySweepStats;
}

const ACTIONS = ["买入", "持有", "卖出", "BUY", "HOLD", "SELL"] as const;

export interface ReplaySweepFormValues {
  codes: string;
  range: [Dayjs, Dayjs];
  holdingDays: number;
  action: string;
  confidence: number;
}

export function ReplaySweep() {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState({ done: 0, total: 0 });
  const [result, setResult] = useState<ReplaySweepResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const parseCodes = useCallback((raw: string): string[] => {
    return raw
      .split(/[\s,;]+/g)
      .map((c) => c.trim())
      .filter((c) => c.length > 0);
  }, []);

  const expandRange = useCallback((range: [Dayjs, Dayjs]): string[] => {
    const wd: WorkdayRange = { start: range[0], end: range[1] };
    return buildAsOfDateRange(wd);
  }, []);

  const onRun = useCallback(
    async (values: ReplaySweepFormValues) => {
      setError(null);
      setResult(null);
      const codes = parseCodes(values.codes);
      if (codes.length === 0) {
        setError(t("replayWorkbench.sweep.codesLabel") + " — required");
        return;
      }
      const dates = expandRange(values.range);
      if (dates.length === 0) {
        setError(t("replayWorkbench.sweep.dateRangeLabel") + " — no workdays");
        return;
      }
      const items: ReplaySweepItem[] = [];
      for (const code of codes) {
        for (const d of dates) {
          items.push({
            stock_code: code,
            as_of_date: d,
            decision_action: values.action,
            decision_confidence: values.confidence,
          });
        }
      }
      const total = items.length;
      setProgress({ done: 0, total });
      setBusy(true);
      try {
        // 后端一次性返回;UI 进度条只在等待期间显示"running"文案
        const res = await invoke<ReplaySweepResult>("run_replay_backtest", {
          items,
          holdingDays: values.holdingDays,
        });
        setResult(res);
        setProgress({ done: total, total });
      } catch (e) {
        setError(typeof e === "string" ? e : (e as Error).message);
      } finally {
        setBusy(false);
      }
    },
    [expandRange, parseCodes, t],
  );

  const validColumns: ColumnsType<ReplaySweepResult["results"][number]> = useMemo(
    () => [
      { title: "Code", dataIndex: "stock_code", key: "stock_code", width: 100 },
      { title: "Date", dataIndex: "analysis_date", key: "analysis_date", width: 120 },
      { title: "Action", dataIndex: "decision_action", key: "decision_action", width: 100 },
      {
        title: "Return",
        dataIndex: "return_rate",
        key: "return_rate",
        width: 100,
        render: (v: number) => (
          <span style={{ color: v >= 0 ? "var(--sa-green, #10b981)" : "var(--sa-red, #ef4444)" }}>
            {(v * 100).toFixed(2)}%
          </span>
        ),
      },
      {
        title: "Correct",
        dataIndex: "is_correct",
        key: "is_correct",
        width: 90,
        render: (v: boolean) => v ? <Tag color="green">✓</Tag> : <Tag color="red">✗</Tag>,
      },
      {
        title: "Confidence",
        dataIndex: "confidence",
        key: "confidence",
        width: 100,
        render: (v: number) => v.toFixed(2),
      },
    ],
    [],
  );

  const invalidColumns: ColumnsType<ReplaySweepInvalid> = useMemo(
    () => [
      { title: "Code", dataIndex: "stock_code", key: "stock_code", width: 100 },
      { title: "Date", dataIndex: "as_of_date", key: "as_of_date", width: 120 },
      { title: "Reason", dataIndex: "reason", key: "reason" },
    ],
    [],
  );

  const exportCsv = useCallback(() => {
    if (!result) { return; }
    const rows = [
      ["code", "date", "action", "return_rate", "is_correct", "confidence"].join(","),
      ...result.results.map((r) =>
        [
          r.stock_code,
          r.analysis_date,
          r.decision_action,
          r.return_rate.toFixed(4),
          r.is_correct,
          r.confidence.toFixed(2),
        ].join(",")
      ),
    ];
    const blob = new Blob([rows.join("\n")], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `replay-sweep-${Date.now()}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }, [result]);

  const exportJson = useCallback(() => {
    if (!result) { return; }
    const blob = new Blob([JSON.stringify(result, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `replay-sweep-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [result]);

  return (
    <Card
      title={
        <span>
          <CalendarRange size={14} style={{ marginRight: 6, verticalAlign: -2 }} />
          {t("replayWorkbench.sweep.title")}
        </span>
      }
      size="small"
      style={{ marginBottom: 16 }}
    >
      <Alert
        type="info"
        showIcon
        message={t("replayWorkbench.sweep.intro")}
        style={{ marginBottom: 12 }}
      />

      <Form<ReplaySweepFormValues>
        layout="vertical"
        size="small"
        onFinish={onRun}
        initialValues={{
          codes: "",
          range: [dayjs().subtract(30, "day"), dayjs().subtract(1, "day")],
          holdingDays: 5,
          action: "持有",
          confidence: 0.5,
        }}
        disabled={busy}
      >
        <Form.Item
          name="codes"
          label={t("replayWorkbench.sweep.codesLabel")}
          rules={[{ required: true, message: "Required" }]}
        >
          <Input.TextArea
            rows={3}
            placeholder={t("replayWorkbench.sweep.codesPlaceholder") ?? ""}
            data-testid="sweep-codes-input"
          />
        </Form.Item>

        <Space wrap>
          <Form.Item
            name="range"
            label={t("replayWorkbench.sweep.dateRangeLabel")}
            rules={[{ required: true, message: "Required" }]}
          >
            <RangePicker
              disabledDate={(d) => d.isAfter(dayjs().subtract(1, "day"))}
            />
          </Form.Item>
          <Form.Item
            name="holdingDays"
            label={t("replayWorkbench.sweep.holdingDaysLabel")}
            rules={[{ required: true }]}
          >
            <InputNumber min={1} max={120} />
          </Form.Item>
          <Form.Item
            name="action"
            label={t("replayWorkbench.sweep.actionLabel")}
            rules={[{ required: true }]}
          >
            <Select
              style={{ width: 120 }}
              options={ACTIONS.map((a) => ({ label: a, value: a }))}
            />
          </Form.Item>
          <Form.Item
            name="confidence"
            label={t("replayWorkbench.sweep.confidenceLabel")}
            rules={[{ required: true }]}
          >
            <InputNumber min={0} max={1} step={0.05} />
          </Form.Item>
          <Form.Item label=" ">
            <Button
              type="primary"
              htmlType="submit"
              icon={<Play size={12} />}
              loading={busy}
              data-testid="sweep-run-btn"
            >
              {t("replayWorkbench.sweep.runBtn")}
            </Button>
          </Form.Item>
        </Space>
      </Form>

      {busy && (
        <div style={{ marginTop: 8 }}>
          <Progress
            percent={progress.total === 0
              ? 0
              : Math.round((progress.done / progress.total) * 100)}
            status="active"
            showInfo
          />
          <Typography.Text type="secondary">
            {t("replayWorkbench.sweep.running", {
              done: progress.done,
              total: progress.total,
            })}
          </Typography.Text>
        </div>
      )}

      {error && (
        <Alert
          type="error"
          showIcon
          message={error}
          style={{ marginTop: 12 }}
        />
      )}

      {result && (
        <div style={{ marginTop: 16 }}>
          <Space size="large" wrap>
            <Statistic
              title={t("replayWorkbench.sweep.summary", {
                total: result.total,
                valid: result.valid,
                invalid: result.invalid,
              })}
              value={`${result.valid} / ${result.invalid}`}
            />
            <Statistic title="Total" value={result.total} />
            <Statistic
              title="Accuracy"
              value={(result.stats.accuracy * 100).toFixed(1) + "%"}
            />
            <Statistic
              title="Alpha"
              value={(result.stats.alpha * 100).toFixed(2) + "%"}
            />
            <Statistic
              title="Sharpe"
              value={result.stats.sharpe.toFixed(2)}
            />
            <Space>
              <Button
                icon={<Download size={12} />}
                onClick={exportCsv}
                data-testid="sweep-export-csv"
              >
                {t("replayWorkbench.sweep.exportCsv")}
              </Button>
              <Button
                icon={<Download size={12} />}
                onClick={exportJson}
                data-testid="sweep-export-json"
              >
                {t("replayWorkbench.sweep.exportJson")}
              </Button>
            </Space>
          </Space>

          {result.invalid > 0 && (
            <Alert
              type="warning"
              showIcon
              icon={<ShieldAlert size={14} />}
              message={t("replayWorkbench.sweep.violationNotice", {
                count: result.invalid,
              })}
              style={{ marginTop: 12 }}
            />
          )}

          <Collapse
            ghost
            style={{ marginTop: 12 }}
            items={[
              {
                key: "valid",
                label: `Valid samples (${result.valid})`,
                children: (
                  <Table<ReplaySweepResult["results"][number]>
                    size="small"
                    rowKey={(r) => `${r.stock_code}__${r.analysis_date}`}
                    columns={validColumns}
                    dataSource={result.results}
                    pagination={{ pageSize: 10 }}
                    data-testid="sweep-valid-table"
                  />
                ),
              },
              {
                key: "invalid",
                label: `Invalid samples (${result.invalid})`,
                children: result.invalid === 0
                  ? (
                    <Typography.Text type="secondary">
                      {t("replayWorkbench.sweep.noInvalid")}
                    </Typography.Text>
                  )
                  : (
                    <Table<ReplaySweepInvalid>
                      size="small"
                      rowKey={(r) => `${r.stock_code}__${r.as_of_date}`}
                      columns={invalidColumns}
                      dataSource={result.invalid_details}
                      pagination={{ pageSize: 10 }}
                      data-testid="sweep-invalid-table"
                    />
                  ),
              },
            ]}
          />
        </div>
      )}
    </Card>
  );
}
