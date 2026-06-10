import { Button, Empty, Select, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { Code, Play, Terminal, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

// localStorage 存储键
const STORAGE_KEY = "hook_execution_logs";

// Hook 执行记录
export interface HookExecutionRecord {
  id: string;
  /** 执行时间 ISO 字符串 */
  timestamp: string;
  /** 事件名（如 PreToolUse, PostToolUse） */
  event: string;
  /** 工具名（可选） */
  toolName?: string;
  /** 执行的命令 */
  command: string;
  /** 执行状态 */
  status: "success" | "failed";
  /** 输出摘要（截取前 200 字符） */
  outputSummary?: string;
}

// 读取日志
function readLogs(): HookExecutionRecord[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return [];
    }
    return JSON.parse(raw) as HookExecutionRecord[];
  } catch {
    return [];
  }
}

// 写入日志
function writeLogs(logs: HookExecutionRecord[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(logs));
}

// 添加一条日志（对外暴露，供其他模块调用）
// eslint-disable-next-line react-refresh/only-export-components
export function appendHookLog(record: Omit<HookExecutionRecord, "id">) {
  const logs = readLogs();
  logs.unshift({
    ...record,
    id: crypto.randomUUID(),
  });
  // 最多保留 500 条
  if (logs.length > 500) {
    logs.length = 500;
  }
  writeLogs(logs);
}

interface HookExecutionLogProps {
  /** 限制显示数量，不传则显示全部 */
  maxItems?: number;
}

export function HookExecutionLog({ maxItems }: HookExecutionLogProps) {
  const { t } = useTranslation();
  const [logs, setLogs] = useState<HookExecutionRecord[]>([]);
  const [filterEvent, setFilterEvent] = useState<string | undefined>(undefined);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLogs(readLogs());
  }, []);

  // 所有可用的事件名
  const eventOptions = useMemo(() => {
    const events = new Set(logs.map((l) => l.event));
    return Array.from(events).map((e) => ({ label: e, value: e }));
  }, [logs]);

  // 过滤后的数据
  const filteredLogs = useMemo(() => {
    let result = filterEvent
      ? logs.filter((l) => l.event === filterEvent)
      : logs;
    if (maxItems && result.length > maxItems) {
      result = result.slice(0, maxItems);
    }
    return result;
  }, [logs, filterEvent, maxItems]);

  const handleClearAll = () => {
    writeLogs([]);
    setLogs([]);
  };

  const columns: ColumnsType<HookExecutionRecord> = [
    {
      title: t("hookLog.column.time"),
      dataIndex: "timestamp",
      key: "timestamp",
      width: 170,
      render: (ts: string) => (
        <Text style={{ fontSize: 12 }}>
          {new Date(ts).toLocaleString("zh-CN")}
        </Text>
      ),
    },
    {
      title: t("hookLog.column.event"),
      dataIndex: "event",
      key: "event",
      width: 160,
      render: (event: string) => (
        <div className="flex items-center gap-1">
          <Play size={12} />
          <Tag style={{ margin: 0, fontSize: 12 }}>{event}</Tag>
        </div>
      ),
    },
    {
      title: t("hookLog.column.tool"),
      dataIndex: "toolName",
      key: "toolName",
      width: 130,
      render: (name?: string) =>
        name
          ? (
            <div className="flex items-center gap-1">
              <Code size={12} />
              <Text style={{ fontSize: 12 }}>{name}</Text>
            </div>
          )
          : (
            <Text type="secondary" style={{ fontSize: 12 }}>
              -
            </Text>
          ),
    },
    {
      title: t("hookLog.column.command"),
      dataIndex: "command",
      key: "command",
      ellipsis: true,
      render: (cmd: string) => (
        <div className="flex items-center gap-1">
          <Terminal size={12} />
          <Text code style={{ fontSize: 12 }}>
            {cmd}
          </Text>
        </div>
      ),
    },
    {
      title: t("hookLog.column.status"),
      dataIndex: "status",
      key: "status",
      width: 80,
      render: (status: string) =>
        status === "success"
          ? (
            <Tag color="success" style={{ fontSize: 12 }}>
              {t("hookLog.success")}
            </Tag>
          )
          : (
            <Tag color="error" style={{ fontSize: 12 }}>
              {t("hookLog.failed")}
            </Tag>
          ),
    },
    {
      title: t("hookLog.column.outputSummary"),
      dataIndex: "outputSummary",
      key: "outputSummary",
      ellipsis: true,
      render: (summary?: string) =>
        summary
          ? (
            <Text style={{ fontSize: 12, maxWidth: 200 }} ellipsis>
              {summary}
            </Text>
          )
          : (
            <Text type="secondary" style={{ fontSize: 12 }}>
              -
            </Text>
          ),
    },
  ];

  return (
    <div>
      {/* 工具栏 */}
      <div
        className="flex items-center justify-between"
        style={{ marginBottom: 12 }}
      >
        <Space size={8}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("hookLog.filterLabel")}
          </Text>
          <Select
            id="hook-execution-log-select-65"
            allowClear
            size="small"
            placeholder={t("hookLog.allEvents")}
            value={filterEvent}
            onChange={(val) => setFilterEvent(val)}
            options={eventOptions}
            style={{ width: 180 }}
          />
        </Space>
        <Space size={8}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("hookLog.recordCount", { count: filteredLogs.length })}
          </Text>
          {logs.length > 0 && (
            <Button
              size="small"
              danger
              icon={<Trash2 size={13} />}
              onClick={handleClearAll}
            >
              {t("hookLog.clear")}
            </Button>
          )}
        </Space>
      </div>

      {/* 表格 */}
      {filteredLogs.length === 0
        ? (
          <Empty
            description={logs.length === 0
              ? t("hookLog.emptyStatus")
              : t("hookLog.emptyFiltered")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        )
        : (
          <Table
            size="small"
            columns={columns}
            dataSource={filteredLogs}
            rowKey="id"
            pagination={{ pageSize: 20, size: "small", showSizeChanger: false }}
          />
        )}
    </div>
  );
}
