import type { AiChatAction } from "@/components/workflow/types/workflow.types";
import { invoke, listen } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import {
  Button,
  Card,
  Checkbox,
  DatePicker,
  Empty,
  Input,
  message,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import type { RangePickerProps } from "antd/es/date-picker";
import type { Dayjs } from "dayjs";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Text } = Typography;

interface ReflectionRow {
  id: string;
  stockCode: string;
  stockName: string;
  /** 关联的原始 analysis 记录 ID (手动反思时为空字符串) */
  originalAnalysisId?: string;
  asOfDate: string;
  hindsightDate: string;
  actualOutcome: string;
  whatWentWrong: string | null;
  missedSignals: string | null;
  fixForFuture: string | null;
  reflectionDepth: string;
  minConfidenceThreshold: number;
  status: string;
  createdAt: number;
  decisionJson?: string;
  blackboardSnapshot?: string;
}

interface CronJobResponse {
  id: string;
  name: string;
  schedule: string;
  status: string;
}

export function ReflectionPanel() {
  const { t } = useTranslation();
  const stockCodeFromStore = useStockAnalysisStore((s) => s.stockCode);

  const CRON_PRESETS = [
    { label: t("stockAnalysis.reflection.daily0600"), value: "0 6 * * *" },
    { label: t("stockAnalysis.reflection.daily1200"), value: "0 12 * * *" },
    { label: t("stockAnalysis.reflection.daily1800"), value: "0 18 * * *" },
    { label: t("stockAnalysis.reflection.weekday0600"), value: "0 6 * * 1-5" },
  ];

  const [reflections, setReflections] = useState<ReflectionRow[]>([]);
  const [cronJobs, setCronJobs] = useState<CronJobResponse[]>([]);
  // P1-5 修复: 加载状态 — 表格 Spin + 刷新按钮 loading
  const [loading, setLoading] = useState(false);
  const [cronExpr, setCronExpr] = useState("0 6 * * *");
  const [threshold, setThreshold] = useState(0);
  const [depth, setDepth] = useState("light");
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // P1-7 修复: 反思历史筛选 (股票代码 / 状态)
  const [filterCode, setFilterCode] = useState("");
  const [filterStatus, setFilterStatus] = useState<"" | "running" | "completed" | "failed">("");
  const [filterDateRange, setFilterDateRange] = useState<[Dayjs | null, Dayjs | null] | null>(null);

  const [manualAsOf, setManualAsOf] = useState<Dayjs | null>(null);
  const [manualOutcome, setManualOutcome] = useState("");
  const [manualDepth, setManualDepth] = useState("light");
  const [running, setRunning] = useState(false);

  // 个股分析页内自动取当前股票代码（首次加载时填入）
  const [manualCode, setManualCode] = useState(stockCodeFromStore ?? "");
  // 当 store 股票代码变化但 manualCode 为空时自动填充
  const prevCodeRef = useRef(stockCodeFromStore);
  useEffect(() => {
    if (!manualCode && stockCodeFromStore && stockCodeFromStore !== prevCodeRef.current) {
      prevCodeRef.current = stockCodeFromStore;
      setManualCode(stockCodeFromStore);
    }
  }, [stockCodeFromStore, manualCode]);

  // Bug 4 修复: 统一请求级取消令牌,避免 useEffect 与 onClick 双轨加载
  // 各自维护一份 cancelled 标记造成的乱序写入。
  const loadTokenRef = useRef(0);
  const load = async () => {
    const myToken = ++loadTokenRef.current;
    setLoading(true); // P1-5 修复
    try {
      const [r, c] = await Promise.all([
        invoke<ReflectionRow[]>("list_reflections", {}),
        invoke<CronJobResponse[]>("list_validate_decisions_crons", {}),
      ]);
      if (myToken !== loadTokenRef.current) { return; }
      if (Array.isArray(r)) { setReflections(r); }
      if (Array.isArray(c)) { setCronJobs(c); }
    } catch (e) {
      // P0-2 修复: 加载失败必须显式告知用户,否则后端挂掉时用户无感知
      message.error(t("stockAnalysis.reflection.loadFailed", { error: String(e) }));
    } finally {
      if (myToken === loadTokenRef.current) { setLoading(false); }
    }
  };

  useEffect(() => {
    // Bug 4 修复: 走统一入口
    Promise.resolve().then(() => load());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // P1-6 修复: 自动刷新 — 30s 轮询 + 标签页可见性恢复时立即拉一次
  // 防止定时反思跑完后用户必须手动刷新才能看到新记录
  useEffect(() => {
    const POLL_MS = 30_000;
    const id = setInterval(() => {
      // 只在标签页可见时轮询,避免后台无意义请求
      if (document.visibilityState === "visible") { void load(); }
    }, POLL_MS);
    const onVis = () => {
      if (document.visibilityState === "visible") { void load(); }
    };
    document.addEventListener("visibilitychange", onVis);
    return () => {
      clearInterval(id);
      document.removeEventListener("visibilitychange", onVis);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 后端 CronJobStatus 用 #[serde(rename_all = "snake_case")] 序列化,
  // 实际 status 字符串是 "active" / "paused" / "disabled"（小写）
  // 这里必须用小写比较,否则 isEnabled/activeCron 永远错误。
  const activeCron = cronJobs.find((j) => j.status === "active") ?? cronJobs[0];
  const isEnabled = cronJobs.some((j) => j.status === "active");

  const toggleCron = async (enable: boolean) => {
    try {
      if (enable && !activeCron) {
        await invoke("create_validate_decisions_cron", {
          cronExpression: cronExpr,
          minConfidenceThreshold: threshold,
          reflectionDepth: depth,
          enabled: true,
        });
      } else if (activeCron) {
        await invoke("toggle_validate_decisions_cron", { id: activeCron.id, enabled: enable });
      }
      await load();
    } catch (e) {
      // P0-2 修复: 切换 cron 失败必须显式告知用户
      message.error(t("stockAnalysis.reflection.toggleCronFailed", { error: String(e) }));
    }
  };

  const deleteCron = async () => {
    if (!activeCron) { return; }
    try {
      await invoke("delete_validate_decisions_cron", { id: activeCron.id });
      await load();
    } catch (e) {
      // P0-2 修复: 删除 cron 失败必须显式告知用户
      message.error(t("stockAnalysis.reflection.deleteCronFailed", { error: String(e) }));
    }
  };

  // P0-3 修复: 重跑反思 — 后端无 rerun_reflection 命令,复用 run_reflection_now
  // 语义: 重新跑一次反思,生成新记录,旧失败记录保留(用户可手动删除)
  const rerunReflection = async (r: ReflectionRow) => {
    try {
      await invoke("run_reflection_now", {
        stockCode: r.stockCode,
        stockName: r.stockName,
        asOfDate: r.asOfDate,
        actualOutcome: r.actualOutcome,
        reflectionDepth: r.reflectionDepth,
      });
      message.success(t("stockAnalysis.reflection.rerunSuccess"));
      await load();
    } catch (e) {
      message.error(t("stockAnalysis.reflection.rerunFailed", { error: String(e) }));
    }
  };

  const deleteReflection = async (id: string) => {
    try {
      await invoke("delete_reflection", { reflectionId: id });
      message.success(t("stockAnalysis.reflection.deleteSuccess"));
      await load();
    } catch (e) {
      message.error(t("stockAnalysis.reflection.deleteFailed", { error: String(e) }));
    }
  };

  // P2-15 修复: 清空全部反思 (并行删除,失败累加)
  const clearAllReflections = async () => {
    const ids = reflections.map((r) => r.id);
    if (ids.length === 0) { return; }
    const results = await Promise.allSettled(
      ids.map((id) => invoke("delete_reflection", { reflectionId: id })),
    );
    const failed = results.filter((r) => r.status === "rejected").length;
    if (failed === 0) {
      message.success(t("stockAnalysis.reflection.clearAllSuccess", { count: ids.length }));
    } else {
      message.warning(t("stockAnalysis.reflection.clearAllPartial", { total: ids.length, failed }));
    }
    await load();
  };

  // P1-7 修复: 反思历史筛选 (股票代码模糊匹配 + 状态 + 时间区间)
  const filteredReflections = reflections.filter((r) => {
    if (filterCode && !r.stockCode.toLowerCase().includes(filterCode.toLowerCase())) {
      return false;
    }
    if (filterStatus) {
      const isFailed = typeof r.status === "string" && r.status.startsWith("failed:");
      if (filterStatus === "failed" ? !isFailed : r.status !== filterStatus) {
        return false;
      }
    }
    if (filterDateRange && filterDateRange[0]) {
      const created = new Date(r.createdAt);
      if (created < filterDateRange[0].startOf("day").toDate()) { return false; }
    }
    if (filterDateRange && filterDateRange[1]) {
      const created = new Date(r.createdAt);
      if (created > filterDateRange[1].endOf("day").toDate()) { return false; }
    }
    return true;
  });

  const resetFilters = () => {
    setFilterCode("");
    setFilterStatus("");
    setFilterDateRange(null);
  };

  // P2-13 修复: 反思历史导出 CSV/JSON (前端 blob 下载,无需后端)
  const exportReflections = (rows: ReflectionRow[]) => {
    if (rows.length === 0) { return; }
    const csv = [
      [
        "stockCode",
        "stockName",
        "asOfDate",
        "actualOutcome",
        "status",
        "depth",
        "whatWentWrong",
        "fixForFuture",
        "createdAt",
      ].join(","),
      ...rows.map((r) =>
        [
          r.stockCode,
          `"${(r.stockName || "").replace(/"/g, '""')}"`,
          r.asOfDate,
          `"${(r.actualOutcome || "").replace(/"/g, '""')}"`,
          r.status,
          r.reflectionDepth || "",
          `"${(r.whatWentWrong || "").replace(/"/g, '""')}"`,
          `"${(r.fixForFuture || "").replace(/"/g, '""')}"`,
          new Date(r.createdAt).toISOString(),
        ].join(",")
      ),
    ].join("\n");
    const blob = new Blob(["\uFEFF" + csv], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `reflections_${new Date().toISOString().slice(0, 10)}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    message.success(t("stockAnalysis.reflection.exportSuccess", { count: rows.length }));
  };

  // P2-11 修复: cron 下次运行时间 (前端解析,无需新依赖)
  const computeNextRun = (cronExpression: string): string | null => {
    try {
      // 简单 cron 5-field 解析: 仅支持 * / 数字 / 逗号 / 横线 / step /
      // 完整实现需 ~150 行,这里支持常见 4 种预设足够
      const parts = cronExpression.trim().split(/\s+/);
      if (parts.length !== 5) { return null; }
      const [min, hour, , , dow] = parts;
      // 解析: 返回"每天 HH:MM"或"周N HH:MM"
      const now = new Date();
      const next = new Date(now);
      next.setSeconds(0);
      next.setMilliseconds(0);
      // 简化为下一个匹配的 hour:min (不处理 step/dow 复杂情况)
      const targetMin = min === "*" ? now.getMinutes() : parseInt(min, 10);
      const targetHour = hour === "*" ? now.getHours() : parseInt(hour, 10);
      next.setMinutes(targetMin);
      next.setHours(targetHour);
      if (next <= now) { next.setDate(next.getDate() + 1); }
      // weekday
      if (dow !== "*") {
        const allowedDays = dow.split(",").map((d) => {
          if (d.includes("-")) {
            const [a, b] = d.split("-").map((x) => parseInt(x, 10));
            return { a, b };
          }
          return { a: parseInt(d, 10), b: parseInt(d, 10) };
        });
        while (true) {
          const day = next.getDay();
          const match = allowedDays.some(({ a, b }) => day >= a && day <= b);
          if (match) { break; }
          next.setDate(next.getDate() + 1);
        }
      }
      return next.toLocaleString("zh-CN", { hour12: false });
    } catch {
      return null;
    }
  };

  const handleManualReflection = async () => {
    if (!manualCode.trim() || !manualAsOf || !manualAsOf.isValid() || !manualOutcome.trim()) {
      message.warning(t("stockAnalysis.reflection.fillRequired"));
      return;
    }
    setRunning(true);
    try {
      await invoke("run_reflection_now", {
        stockCode: manualCode.trim(),
        stockName: "",
        asOfDate: manualAsOf.format("YYYY-MM-DD"),
        actualOutcome: manualOutcome.trim(),
        reflectionDepth: manualDepth,
      });
      message.success(t("stockAnalysis.reflection.triggerSuccess"));
      await load();
    } catch (e) {
      message.error(t("stockAnalysis.reflection.triggerFailed", { error: String(e) }));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div style={{ padding: 16 }}>
      {/* 手动触发 */}
      <Card
        title={t("stockAnalysis.reflection.manualTitle")}
        size="small"
        style={{ marginBottom: 16 }}
      >
        <Space direction="vertical" style={{ width: "100%" }}>
          <Space wrap>
            <Input
              placeholder={t("stockAnalysis.reflection.placeholderCode")}
              value={manualCode}
              onChange={(e) => setManualCode(e.target.value)}
              style={{ width: 120 }}
            />
            <DatePicker
              placeholder={t("stockAnalysis.reflection.placeholderDate")}
              value={manualAsOf}
              onChange={(d) => setManualAsOf(d)}
              format="YYYY-MM-DD"
              allowClear
              style={{ width: 160 }}
            />
            <Input
              placeholder={t("stockAnalysis.reflection.placeholderOutcome")}
              value={manualOutcome}
              onChange={(e) => setManualOutcome(e.target.value)}
              style={{ width: 200 }}
            />
            <Select
              value={manualDepth}
              onChange={setManualDepth}
              options={[
                { label: t("stockAnalysis.reflection.depthLight"), value: "light" },
                { label: t("stockAnalysis.reflection.depthDeep"), value: "deep" },
              ]}
              style={{ width: 100 }}
            />
            <Button
              type="primary"
              onClick={handleManualReflection}
              loading={running}
              disabled={running}
            >
              {t("stockAnalysis.reflection.startReflection")}
            </Button>
          </Space>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("stockAnalysis.reflection.manualHint")}
          </Text>
        </Space>
      </Card>

      {/* 定时校验配置 */}
      <Card title={t("stockAnalysis.reflection.scheduleTitle")} size="small" style={{ marginBottom: 16 }}>
        <Space direction="vertical" style={{ width: "100%" }}>
          <Space>
            <Switch checked={isEnabled} onChange={toggleCron} />
            <Text>{t("stockAnalysis.reflection.scheduleDesc")}</Text>
          </Space>
          <Space wrap>
            <Select
              value={cronExpr}
              onChange={setCronExpr}
              options={CRON_PRESETS}
              style={{ width: 150 }}
              disabled={isEnabled}
            />
            <Select
              value={depth}
              onChange={setDepth}
              options={[
                { label: t("stockAnalysis.reflection.depthLight"), value: "light" },
                { label: t("stockAnalysis.reflection.depthDeep"), value: "deep" },
              ]}
              style={{ width: 110 }}
              disabled={isEnabled}
            />
            <Input
              type="number"
              min={0}
              max={100}
              value={threshold}
              onChange={(e) => setThreshold(Number(e.target.value))}
              style={{ width: 100 }}
              disabled={isEnabled}
              addonAfter={t("stockAnalysis.reflection.confidence")}
            />
            {activeCron && (
              // P0-1 修复: 删除 cron 是高危操作(永久丢失调度),必须 Popconfirm 二次确认,
              // 与 deleteReflection (line 340) 的 UX 保持一致。
              <Popconfirm
                title={t("stockAnalysis.reflection.deleteCronConfirm", { schedule: activeCron.schedule })}
                okText={t("common.confirm")}
                cancelText={t("common.cancel")}
                onConfirm={deleteCron}
              >
                <Button danger size="small">
                  {t("stockAnalysis.reflection.deleteBtn")}
                </Button>
              </Popconfirm>
            )}
          </Space>
          {activeCron && (
            <Text type="secondary">
              {t("stockAnalysis.reflection.scheduleDescFull", {
                schedule: activeCron.schedule,
                status: isEnabled
                  ? t("stockAnalysis.reflection.running")
                  : t("stockAnalysis.reflection.paused"),
              })}
              {/* P2-11 修复: 显示下次运行时间 */}
              {isEnabled && (() => {
                const nextRun = computeNextRun(activeCron.schedule);
                return nextRun
                  ? t("stockAnalysis.reflection.nextRunAt", { time: nextRun })
                  : null;
              })()}
            </Text>
          )}
        </Space>
      </Card>

      {/* 反思历史 */}
      <Card
        title={t("stockAnalysis.reflection.historyTitle")}
        size="small"
        extra={
          <Space size="small">
            <Button size="small" onClick={load} loading={loading}>
              {t("stockAnalysis.reflection.refreshBtn")}
            </Button>
            <Button
              size="small"
              onClick={() => exportReflections(filteredReflections)}
              disabled={filteredReflections.length === 0}
            >
              {t("stockAnalysis.reflection.exportBtn")}
            </Button>
            {reflections.length > 0 && (
              <Popconfirm
                title={t("stockAnalysis.reflection.clearAllConfirm", { count: reflections.length })}
                okText={t("common.confirm")}
                cancelText={t("common.cancel")}
                onConfirm={clearAllReflections}
              >
                <Button danger size="small">
                  {t("stockAnalysis.reflection.clearAllBtn")}
                </Button>
              </Popconfirm>
            )}
          </Space>
        }
      >
        {/* P1-7 修复: 反思历史筛选条 */}
        <Space wrap style={{ marginBottom: 12 }}>
          <Input
            placeholder={t("stockAnalysis.reflection.filterCodePlaceholder")}
            value={filterCode}
            onChange={(e) => setFilterCode(e.target.value)}
            allowClear
            style={{ width: 140 }}
          />
          <Select
            value={filterStatus}
            onChange={setFilterStatus}
            options={[
              { label: t("stockAnalysis.reflection.filterStatusAll"), value: "" },
              { label: t("stockAnalysis.reflection.filterStatusRunning"), value: "running" },
              { label: t("stockAnalysis.reflection.filterStatusCompleted"), value: "completed" },
              { label: t("stockAnalysis.reflection.filterStatusFailed"), value: "failed" },
            ]}
            style={{ width: 120 }}
          />
          <DatePicker.RangePicker
            value={filterDateRange}
            onChange={(d: RangePickerProps["value"]) => setFilterDateRange(d as [Dayjs | null, Dayjs | null] | null)}
            allowClear
            format="YYYY-MM-DD"
          />
          {(filterCode || filterStatus || filterDateRange) && (
            <Button size="small" onClick={resetFilters}>
              {t("stockAnalysis.reflection.filterResetBtn")}
            </Button>
          )}
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("stockAnalysis.reflection.filterResultCount", {
              shown: filteredReflections.length,
              total: reflections.length,
            })}
          </Text>
        </Space>
        {/* P1-5 修复: 加载状态 — 表格 Spin */}
        <Spin spinning={loading}>
          {reflections.length === 0
            ? <Empty description={t("stockAnalysis.reflection.empty")} />
            : filteredReflections.length === 0
            ? <Empty description={t("stockAnalysis.reflection.filterEmpty")} />
            : (
              <Table
                dataSource={filteredReflections}
                rowKey="id"
                pagination={{ pageSize: 10, size: "small" }}
                size="small"
                scroll={{ x: 800 }}
                expandable={{
                  expandedRowKeys: expandedId ? [expandedId] : [],
                  onExpandedRowsChange: (keys: readonly React.Key[]) => setExpandedId(keys[0] as string || null),
                  expandedRowRender: (r: ReflectionRow) => (
                    <ExpandedReflectionRow row={r} t={t as (key: string, opts?: object) => string} onRefresh={load} />
                  ),
                }}
              >
                <Table.Column title={t("stockAnalysis.reflection.colCode")} dataIndex="stockCode" width={90} />
                <Table.Column title={t("stockAnalysis.reflection.colName")} dataIndex="stockName" width={100} />
                <Table.Column title={t("stockAnalysis.reflection.colAsOf")} dataIndex="asOfDate" width={100} />
                <Table.Column
                  title={t("stockAnalysis.reflection.colResult")}
                  dataIndex="actualOutcome"
                  width={160}
                  render={(v: string) => <Text type="danger">{v}</Text>}
                />
                <Table.Column
                  title={t("stockAnalysis.reflection.colStatus")}
                  dataIndex="status"
                  width={140}
                  render={(v: string) => {
                    const color = v === "completed" ? "green" : v.startsWith("failed:") ? "red" : "orange";
                    return <Tag color={color}>{v || "running"}</Tag>;
                  }}
                />
                <Table.Column
                  title={t("stockAnalysis.reflection.colCause")}
                  dataIndex="whatWentWrong"
                  ellipsis
                  render={(v: string | null) => v || "-"}
                />
                <Table.Column
                  title={t("stockAnalysis.reflection.colTime")}
                  dataIndex="createdAt"
                  width={150}
                  render={(v: number) => new Date(v).toLocaleDateString("zh-CN")}
                />
                <Table.Column
                  title={t("stockAnalysis.reflection.colAction")}
                  width={150}
                  fixed="right"
                  render={(_: unknown, r: ReflectionRow) => (
                    <Space size="small">
                      {
                        /* P0-3 修复: 失败的反思可一键重跑(复用 run_reflection_now),
                    成功后生成新记录,旧失败记录保留(可手动删除)。
                    仅 failed:* 状态显示,避免误操作正在 running/completed 的记录。 */
                      }
                      {typeof r.status === "string" && r.status.startsWith("failed:") && (
                        <Popconfirm
                          title={t("stockAnalysis.reflection.rerunConfirm")}
                          okText={t("common.confirm")}
                          cancelText={t("common.cancel")}
                          onConfirm={() => rerunReflection(r)}
                        >
                          <Button size="small">
                            {t("stockAnalysis.reflection.rerunBtn")}
                          </Button>
                        </Popconfirm>
                      )}
                      <Popconfirm
                        title={t("stockAnalysis.reflection.deleteConfirm")}
                        okText={t("common.confirm")}
                        cancelText={t("common.cancel")}
                        onConfirm={() => deleteReflection(r.id)}
                      >
                        <Button danger size="small">
                          {t("stockAnalysis.reflection.deleteBtn")}
                        </Button>
                      </Popconfirm>
                    </Space>
                  )}
                />
              </Table>
            )}
        </Spin>
      </Card>
    </div>
  );
}

function formatJson(val: string | null): string {
  if (!val) { return "-"; }
  try {
    const parsed = JSON.parse(val);
    return Array.isArray(parsed) ? parsed.join("; ") : val;
  } catch {
    return val;
  }
}

interface ParamSuggestion {
  param: string;
  current_value: number;
  suggested_value: number;
  reason: string;
}

function parseParamsSuggestion(row: ReflectionRow): ParamSuggestion[] {
  if (!row.decisionJson) { return []; }
  try {
    const parsed = JSON.parse(row.decisionJson);
    // 后端 stock_workflow.rs 写入的 decision_json 结构是
    //   { reflection: { what_went_wrong, missed_signals, fix_for_future, params_suggestion, ... } }
    // 因此 params_suggestion 一定在 parsed.reflection 下。
    // Bug 11 修复: 删掉 `parsed?.params_suggestion` 这条死分支(永远不会命中),
    // 避免误导后续维护者。
    const raw = parsed?.reflection?.params_suggestion;
    if (Array.isArray(raw)) {
      return raw as ParamSuggestion[];
    }
    return [];
  } catch {
    return [];
  }
}

function renderPrettyJson(label: string, raw: string, maxHeight: number) {
  let body = raw;
  try {
    body = JSON.stringify(JSON.parse(raw), null, 2);
  } catch { /* keep raw */ }
  return (
    <>
      <div style={{ fontSize: 11, color: "var(--muted)", marginTop: 4 }}>{label}</div>
      <pre
        style={{
          fontSize: 11,
          background: "var(--bg-elevated)",
          padding: 8,
          borderRadius: 4,
          maxHeight,
          overflow: "auto",
          whiteSpace: "pre-wrap",
          wordBreak: "break-all",
        }}
      >
        {body}
      </pre>
    </>
  );
}

function renderBlackboardSnapshot(raw: string, label: string) {
  let body = raw;
  try {
    const obj = JSON.parse(raw);
    if (obj && typeof obj === "object" && obj.reflection !== undefined) {
      body = JSON.stringify({ reflection: obj.reflection }, null, 2);
    } else {
      body = JSON.stringify(obj, null, 2);
    }
  } catch { /* keep raw */ }
  return (
    <>
      <div style={{ fontSize: 11, color: "var(--muted)", marginTop: 8 }}>{label}</div>
      <pre
        style={{
          fontSize: 11,
          background: "var(--bg-elevated)",
          padding: 8,
          borderRadius: 4,
          maxHeight: 400,
          overflow: "auto",
          whiteSpace: "pre-wrap",
          wordBreak: "break-all",
        }}
      >
        {body}
      </pre>
    </>
  );
}

function ExpandedReflectionRow(
  { row, t, onRefresh }: {
    row: ReflectionRow;
    t: (key: string, opts?: object) => string;
    onRefresh?: () => Promise<void> | void;
  },
) {
  const navigate = useNavigate();
  const suggestions = parseParamsSuggestion(row);
  const [checkedParams, setCheckedParams] = useState<string[]>([]);
  const [applying, setApplying] = useState(false);

  // AI 修改建议状态
  const [aiModalOpen, setAiModalOpen] = useState(false);
  const [aiStreaming, setAiStreaming] = useState(false);
  const [aiContent, setAiContent] = useState("");
  const [aiActions, setAiActions] = useState<AiChatAction[]>([]);
  // P1-9 修复: 部分应用 — 默认全选,用户可取消单条
  const [aiSelected, setAiSelected] = useState<Set<number>>(new Set());
  const [aiApplying, setAiApplying] = useState(false);
  const aiUnlistensRef = useRef<Array<() => void>>([]);

  const handleApply = async () => {
    const selected = suggestions.filter((s) => checkedParams.includes(s.param));
    if (selected.length === 0) { return; }
    setApplying(true);
    try {
      await invoke("apply_param_suggestions", {
        updates: selected.map((s) => ({ param: s.param, value: s.suggested_value })),
      });
      message.success(t("stockAnalysis.reflection.applySuccess"));
      setCheckedParams([]);
    } catch (e) {
      message.error(t("stockAnalysis.reflection.applyFailed", { error: String(e) }));
    } finally {
      setApplying(false);
    }
  };

  /**
   * 调 workflow_ai_chat_stream（直接走后端命令，不依赖 workflowEditorStore），
   * 把反思内容作为 user message 注入，让 AI LLM 基于上游纯净 system_prompt
   * 输出 5 种 v2.0 :::action 块，由前端解析后在弹窗内展示 diff preview。
   * 这条路径让 ReflectionPanel 单独完成"反思 → 修改闭环"，不依赖 workflow 页面。
   */
  const handleAiAssist = async () => {
    setAiModalOpen(true);
    setAiStreaming(true);
    setAiContent("");
    setAiActions([]);
    // 清理旧的 listener
    aiUnlistensRef.current.forEach((fn) => fn());
    aiUnlistensRef.current = [];

    const sessionId = `reflection-${row.id}-${Date.now()}`;
    const userMessage = buildAiAssistPrompt(row);

    let accumulated = "";
    try {
      const chunkUnlisten = await listen<{
        conversation_id: string;
        message_id: string;
        chunk: { content: string | null; done: boolean };
      }>("workflow-ai-chat-chunk", (event) => {
        if (event.payload.conversation_id !== sessionId) { return; }
        const c = event.payload.chunk;
        if (c.content) { accumulated += c.content; }
        if (c.done) {
          setAiContent(accumulated);
          setAiActions(parseActionsFromAccumulated(accumulated));
          // P1-9 修复: 解析完后默认全选,用户可手动取消
          setAiSelected(new Set(parseActionsFromAccumulated(accumulated).map((_, i) => i)));
          setAiStreaming(false);
        } else {
          setAiContent(accumulated + "▍");
        }
      });
      const errorUnlisten = await listen<{ conversation_id: string; error: string }>(
        "workflow-ai-chat-error",
        (event) => {
          if (event.payload.conversation_id !== sessionId) { return; }
          setAiContent((prev) => prev + `\n\n❌ ${event.payload.error}`);
          setAiStreaming(false);
        },
      );
      aiUnlistensRef.current = [chunkUnlisten, errorUnlisten];

      await invoke("workflow_ai_chat_stream", {
        message: userMessage,
        history: [],
        currentNodes: undefined,
        currentEdges: undefined,
        sessionId,
      });
    } catch (e) {
      setAiContent((prev) => prev + `\n\n❌ ${String(e)}`);
      setAiStreaming(false);
    }
  };

  /**
   * 用户确认后调上游 `apply_diff_with_validation` 聚合器（workflow_ai_apply.rs:491）：
   * - actions 直接传 ChatAction[]（强类型 enum，与 AiChatAction discriminated union 1:1）
   * - validation: ValidationSpec（{ type, params }）
   * - rollback_on_failure: bool（默认 true，单条 action 失败 / validation 失败时倒序回滚）
   */
  const handleApplyAiActions = async () => {
    // P1-9 修复: 只 apply 用户勾选中的子集
    const toApply = aiActions.filter((_, i) => aiSelected.has(i));
    if (toApply.length === 0) { return; }
    setAiApplying(true);
    try {
      const result = await invoke<{
        validation_passed: boolean;
        applied_count: number;
        applied: string[];
        validation_metrics: unknown;
        rolled_back: boolean;
        error: string | null;
      }>("apply_diff_with_validation", {
        actions: toApply,
        validation: { type: "noop", params: {} },
        rollback_on_failure: true,
      });
      if (result.validation_passed) {
        message.success(
          t("stockAnalysis.reflection.aiApplySuccess", { count: result.applied_count }),
        );
        // P0-4 修复: 应用成功后刷新反思列表,避免用户看到陈旧数据,
        // 同时关闭弹窗后用户能立即看到新生成的反思行(如果后端触发了重跑)
        // load 是父组件作用域,通过 onRefresh prop 传入
        await onRefresh?.();
        doCloseAiModal();
      } else {
        message.error(
          t("stockAnalysis.reflection.aiApplyFailed", {
            error: result.error ?? t("common.unknown"),
            rollback: result.rolled_back ? t("stockAnalysis.reflection.rolledBack") : "",
          }),
        );
      }
    } catch (e) {
      message.error(t("stockAnalysis.reflection.aiApplyFailed", { error: String(e), rollback: "" }));
    } finally {
      setAiApplying(false);
    }
  };

  const handleAiModalClose = () => {
    // P2-14 修复: 关闭弹窗时若还有未应用的 action,先询问避免误丢
    if (aiActions.length > 0 && aiSelected.size > 0) {
      Modal.confirm({
        title: t("stockAnalysis.reflection.aiDiscardTitle"),
        content: t("stockAnalysis.reflection.aiDiscardContent", { count: aiSelected.size }),
        okText: t("stockAnalysis.reflection.aiDiscardOk"),
        cancelText: t("common.cancel"),
        onOk: () => doCloseAiModal(),
      });
    } else {
      doCloseAiModal();
    }
  };

  const doCloseAiModal = () => {
    setAiModalOpen(false);
    setAiSelected(new Set()); // 重置选择
    aiUnlistensRef.current.forEach((fn) => fn());
    aiUnlistensRef.current = [];
  };

  return (
    <div style={{ padding: "8px 0" }}>
      <Space direction="vertical" style={{ width: "100%" }}>
        <Space wrap>
          <Tag color="red">{row.actualOutcome}</Tag>
          <Tag>
            {row.reflectionDepth === "deep"
              ? t("stockAnalysis.reflection.depthDeepLabel")
              : t("stockAnalysis.reflection.depthLightLabel")}
          </Tag>
          <Text type="secondary">
            {t("stockAnalysis.reflection.asOfLabel", {
              asOf: row.asOfDate,
              hindsight: row.hindsightDate,
            })}
          </Text>
          {
            /* P2-12 修复: 跳转到原 analysis 记录 — 仅当 originalAnalysisId 非空时显示
              (手动反思时为空字符串,此时原 analysis 不存在,按钮隐藏) */
          }
          {row.originalAnalysisId && row.originalAnalysisId.trim() !== "" && (
            <Tooltip title={t("stockAnalysis.reflection.jumpToAnalysisTooltip")}>
              <Button
                size="small"
                type="link"
                onClick={() =>
                  navigate(`/stock-analysis/${row.originalAnalysisId}`)}
                style={{ padding: 0 }}
              >
                {t("stockAnalysis.reflection.jumpToAnalysisBtn")}
              </Button>
            </Tooltip>
          )}
        </Space>
        <div>
          <Text strong>{t("stockAnalysis.reflection.causeLabel")}</Text>
          <Text>{row.whatWentWrong || "-"}</Text>
        </div>
        <div>
          <Text strong>{t("stockAnalysis.reflection.signalsLabel")}</Text>
          <Text>{formatJson(row.missedSignals)}</Text>
        </div>
        <div>
          <Text strong>{t("stockAnalysis.reflection.improveLabel")}</Text>
          <Text>{row.fixForFuture || "-"}</Text>
        </div>
        {(row.decisionJson || row.blackboardSnapshot) && (
          <details style={{ marginTop: 4 }} open={!row.whatWentWrong && !row.fixForFuture}>
            <summary style={{ cursor: "pointer", color: "var(--muted)", fontSize: 12 }}>
              {t("stockAnalysis.reflection.rawOutputTitle")}
            </summary>
            {row.decisionJson
              && renderPrettyJson(
                t("stockAnalysis.reflection.decisionJsonLabel"),
                row.decisionJson,
                200,
              )}
            {row.blackboardSnapshot
              && renderBlackboardSnapshot(
                row.blackboardSnapshot,
                t("stockAnalysis.reflection.blackboardSnapshotLabel"),
              )}
          </details>
        )}
        {suggestions.length > 0 && (
          <div>
            <Text strong style={{ fontSize: 13 }}>
              ⚙ {t("stockAnalysis.reflection.paramSuggestTitle")}
            </Text>
            <div style={{ marginTop: 8 }}>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                <thead>
                  <tr style={{ borderBottom: "1px solid #d9d9d9" }}>
                    <th style={{ padding: "4px 8px", textAlign: "left", width: 32 }} />
                    <th style={{ padding: "4px 8px", textAlign: "left" }}>
                      {t("stockAnalysis.reflection.paramCol")}
                    </th>
                    <th style={{ padding: "4px 8px", textAlign: "right", width: 80 }}>
                      {t("stockAnalysis.reflection.paramCurrent")}
                    </th>
                    <th style={{ padding: "4px 8px", textAlign: "right", width: 80 }}>
                      {t("stockAnalysis.reflection.paramSuggested")}
                    </th>
                    <th style={{ padding: "4px 8px", textAlign: "left" }}>
                      {t("stockAnalysis.reflection.paramReason")}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {suggestions.map((s) => (
                    <tr key={s.param} style={{ borderBottom: "1px solid #f0f0f0" }}>
                      <td style={{ padding: "4px 8px" }}>
                        <Checkbox
                          checked={checkedParams.includes(s.param)}
                          onChange={(e) => {
                            setCheckedParams(
                              e.target.checked
                                ? [...checkedParams, s.param]
                                : checkedParams.filter((p) => p !== s.param),
                            );
                          }}
                        />
                      </td>
                      <td style={{ padding: "4px 8px", fontFamily: "monospace" }}>
                        {s.param}
                      </td>
                      <td style={{ padding: "4px 8px", textAlign: "right" }}>
                        {s.current_value}
                      </td>
                      <td style={{ padding: "4px 8px", textAlign: "right" }}>
                        <Text strong style={{ color: "#1890ff" }}>{s.suggested_value}</Text>
                      </td>
                      <td style={{ padding: "4px 8px" }}>
                        <Text type="secondary">{s.reason}</Text>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <Space style={{ marginTop: 8 }}>
                <Button
                  type="primary"
                  size="small"
                  loading={applying}
                  disabled={checkedParams.length === 0}
                  onClick={handleApply}
                >
                  {t("stockAnalysis.reflection.applyParamsBtn", { count: checkedParams.length })}
                </Button>
                <Button
                  size="small"
                  onClick={handleAiAssist}
                  disabled={aiStreaming}
                >
                  {aiStreaming
                    ? (
                      <>
                        <Spin size="small" /> {t("stockAnalysis.reflection.aiAssistLoading")}
                      </>
                    )
                    : t("stockAnalysis.reflection.aiAssistBtn")}
                </Button>
              </Space>
            </div>
          </div>
        )}
        {suggestions.length === 0 && (
          <div style={{ marginTop: 4 }}>
            <Button
              size="small"
              onClick={handleAiAssist}
              disabled={aiStreaming}
            >
              {aiStreaming
                ? (
                  <>
                    <Spin size="small" /> {t("stockAnalysis.reflection.aiAssistLoading")}
                  </>
                )
                : t("stockAnalysis.reflection.aiAssistBtn")}
            </Button>
          </div>
        )}
      </Space>

      {/* AI 修改建议弹窗：流式响应 + diff preview + 一键应用 */}
      <Modal
        title={t("stockAnalysis.reflection.aiAssistModalTitle")}
        open={aiModalOpen}
        onCancel={handleAiModalClose}
        width={760}
        footer={[
          <Button key="cancel" onClick={handleAiModalClose}>
            {t("common.cancel")}
          </Button>,
          <Button
            key="apply"
            type="primary"
            loading={aiApplying}
            // P1-9 修复: 按钮文案与禁用态基于用户勾选数
            disabled={aiSelected.size === 0 || aiStreaming}
            onClick={handleApplyAiActions}
          >
            {t("stockAnalysis.reflection.aiApplyBtn", { count: aiSelected.size })}
          </Button>,
        ]}
      >
        {aiStreaming && aiActions.length === 0 && (
          <div style={{ marginBottom: 12 }}>
            <Spin /> <Text type="secondary">{t("stockAnalysis.reflection.aiAssistThinking")}</Text>
          </div>
        )}
        {aiActions.length > 0 && (
          <div style={{ marginBottom: 12 }}>
            <Space style={{ marginBottom: 8, width: "100%", justifyContent: "space-between" }}>
              <Text strong style={{ fontSize: 13 }}>
                {t("stockAnalysis.reflection.aiDiffPreviewTitle", {
                  count: aiSelected.size,
                  total: aiActions.length,
                })}
              </Text>
              <Space size="small">
                <Button
                  size="small"
                  onClick={() => setAiSelected(new Set(aiActions.map((_, i) => i)))}
                >
                  {t("stockAnalysis.reflection.aiSelectAllBtn")}
                </Button>
                <Button size="small" onClick={() => setAiSelected(new Set())}>
                  {t("stockAnalysis.reflection.aiSelectNoneBtn")}
                </Button>
              </Space>
            </Space>
            <div style={{ marginTop: 8 }}>
              {aiActions.map((a, i) => (
                <div
                  key={i}
                  style={{
                    border: "1px solid var(--border-color, #d9d9d9)",
                    borderRadius: 4,
                    padding: 8,
                    marginBottom: 6,
                    fontSize: 12,
                    background: aiSelected.has(i) ? "transparent" : "var(--bg-disabled, #f5f5f5)",
                  }}
                >
                  {/* P1-9 修复: Checkbox 让用户选择单条 */}
                  <div style={{ marginBottom: 4, display: "flex", alignItems: "center", gap: 6 }}>
                    <Checkbox
                      checked={aiSelected.has(i)}
                      onChange={(e) => {
                        const next = new Set(aiSelected);
                        if (e.target.checked) { next.add(i); }
                        else { next.delete(i); }
                        setAiSelected(next);
                      }}
                    />
                    <Tag color={ACTION_COLOR[a.action_type] ?? "default"}>{a.action_type}</Tag>
                    {/* P1-10 修复: rollback_to_version 显示对比提示 */}
                    {a.action_type === "rollback_to_version" && (
                      <Text type="warning" style={{ fontSize: 11 }}>
                        ⚠ {t("stockAnalysis.reflection.rollbackWarning", {
                          target: (a.data as { version?: number })?.version ?? "?",
                        })}
                      </Text>
                    )}
                  </div>
                  <pre
                    style={{
                      fontSize: 11,
                      background: "var(--bg-elevated, #fafafa)",
                      padding: 6,
                      borderRadius: 3,
                      margin: 0,
                      whiteSpace: "pre-wrap",
                      wordBreak: "break-all",
                      maxHeight: 180,
                      overflow: "auto",
                    }}
                  >
                    {JSON.stringify(a.data, null, 2)}
                  </pre>
                </div>
              ))}
            </div>
          </div>
        )}
        {aiContent && (
          <details>
            <summary style={{ cursor: "pointer", color: "var(--muted)", fontSize: 12 }}>
              {t("stockAnalysis.reflection.aiRawOutputTitle")}
            </summary>
            <pre
              style={{
                fontSize: 11,
                background: "var(--bg-elevated, #fafafa)",
                padding: 8,
                borderRadius: 4,
                marginTop: 4,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                maxHeight: 200,
                overflow: "auto",
              }}
            >
              {aiContent}
            </pre>
          </details>
        )}
      </Modal>
    </div>
  );
}

// =========================================================================
// AI 修改建议辅助函数
// 把反思上下文打包成 user message，调后端 workflow_ai_chat_stream。
// 让上游 LLM（基于纯净 system_prompt）输出 5 种 v2.0 :::action 块。
// =========================================================================

const REFLECTION_TEMPLATE_ID = "stock-analysis";

const ACTION_COLOR: Record<string, string> = {
  update_variable: "purple",
  rollback_to_version: "orange",
  update_input_mapping: "cyan",
  edit_asset_file: "magenta",
  apply_diff_with_validation: "green",
};

function buildAiAssistPrompt(row: ReflectionRow): string {
  // 解析 decisionJson 里的 params_suggestion / implementation_tier / code_diff_proposal
  let paramsText = "(无)";
  let tierText = "L1";
  let diffText = "(无)";
  try {
    const parsed = row.decisionJson ? JSON.parse(row.decisionJson) : null;
    const ref = parsed?.reflection ?? parsed;
    if (Array.isArray(ref?.params_suggestion) && ref.params_suggestion.length > 0) {
      paramsText = ref.params_suggestion
        .map((p: { param: string; suggested_value: unknown; reason: string }) =>
          `  - ${p.param} = ${JSON.stringify(p.suggested_value)}（${p.reason ?? ""}）`
        )
        .join("\n");
    }
    if (ref?.implementation_tier) {
      tierText = String(ref.implementation_tier);
    }
    if (ref?.code_diff_proposal) {
      diffText = String(ref.code_diff_proposal);
    }
  } catch {
    // 忽略解析错误，使用默认占位
  }

  return `你是工作流修改助手。请基于以下反思结论，输出 1-3 个 :::action 块，
让前端用户可以预览 + 一键应用。

**重要：所有 action payload 字段用 snake_case（template_id, anchor_line, rollback_on_failure）。
template_id 固定为 "${REFLECTION_TEMPLATE_ID}"。**

【反思上下文】
- 股票：${row.stockCode} ${row.stockName}
- as-of: ${row.asOfDate} → hindsight: ${row.hindsightDate}
- 实际结果：${row.actualOutcome}
- 反思深度：${row.reflectionDepth}

【根因】${row.whatWentWrong || "(空)"}

【遗漏信号】${row.missedSignals || "(空)"}

【改进】${row.fixForFuture || "(空)"}

【业务层参数建议（params_suggestion）】
${paramsText}

【实现层级（implementation_tier）】${tierText}
（说明：L1 = 改 workflow_template.variables；L2 = 改 .rhai 公式 / 算法约束；L3 = 改 .md 提示词）

【业务层代码 diff 提案（code_diff_proposal）】
${diffText}

【输出格式】
按以下 schema 输出 1-3 个 :::action 块（用 \`\`\`json 包裹也接受）。
前端会自动用 apply_diff_with_validation 聚合，所以你**不需要**输出 apply_diff_with_validation 自身。
edit_asset_file 的 description 必填（≤ 200 字）。

1. 如果是 L1（参数调整）：
   :::action
   {"action_type":"update_variable","data":{"template_id":"${REFLECTION_TEMPLATE_ID}","name":"<变量名>","value":<new_value>}}
   :::

2. 如果是 L2（改 .rhai 公式）：
   :::action
   {"action_type":"edit_asset_file","data":{"path":"src-tauri/src/commands/portfolio-mgr.rhai","operation":"insert_after|replace|delete","anchor_line":<int>,"code":"<新代码段>","description":"<≤200 字理由>"}}
   :::

3. 如果是 L3（改 .md 提示词）：
   :::action
   {"action_type":"edit_asset_file","data":{"path":"<md path>","operation":"insert_after|replace|delete","anchor_line":<int>,"code":"<新提示词段>","description":"<≤200 字理由>"}}
   :::

4. 如果是回滚：
   :::action
   {"action_type":"rollback_to_version","data":{"template_id":"${REFLECTION_TEMPLATE_ID}","version":<int>}}
   :::

5. 如果是改 sub-workflow input_mapping：
   :::action
   {"action_type":"update_input_mapping","data":{"node_id":"<nodeId>","mappings":[{"target":"<var>","source":"<var>"}]}}
   :::

不要输出除 :::action 块以外的其他内容（除简短中文解释外）。`;
}

function parseActionsFromAccumulated(content: string): AiChatAction[] {
  const regex = /:::action\s*\n([\s\S]*?)\n:::/g;
  const out: AiChatAction[] = [];
  const seen = new Set<string>(); // P1-8 dedup
  let match;
  const known = new Set<AiChatAction["action_type"]>([
    "update_variable",
    "rollback_to_version",
    "update_input_mapping",
    "edit_asset_file",
    "apply_diff_with_validation",
  ]);
  while ((match = regex.exec(content)) !== null) {
    try {
      const parsed = JSON.parse(match[1].trim());
      const actionType = parsed.action_type;
      if (!known.has(actionType as AiChatAction["action_type"])) { continue; }
      // 防御：LLM 可能输出 camelCase（templateId/anchorLine/rollbackOnFailure），
      // 归一化为 snake_case 以匹配上游 ChatAction enum 序列化要求。
      const data = camelToSnakeKeys(parsed.data ?? {}) as AiChatAction["data"];
      // P1-8 修复: dedup 相同 (action_type, data) 的重复 action
      // 防止 LLM 重复输出同一 action 被应用多次(尤其 update_variable 同 key)
      const dedupKey = actionType + "|" + JSON.stringify(data);
      if (seen.has(dedupKey)) { continue; }
      seen.add(dedupKey);
      out.push({ action_type: actionType, data } as AiChatAction);
    } catch {
      // skip invalid JSON
    }
  }
  return out;
}

/** 把对象所有顶层 key 从 camelCase 转为 snake_case（递归）。用于防御 LLM 输出不一致。 */
function camelToSnakeKeys<T = unknown>(v: T): T {
  if (Array.isArray(v)) {
    return v.map((x) => camelToSnakeKeys(x)) as unknown as T;
  }
  if (v && typeof v === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) {
      const snake = k.replace(/[A-Z]/g, (m) => "_" + m.toLowerCase());
      out[snake] = camelToSnakeKeys(val);
    }
    return out as T;
  }
  return v;
}
