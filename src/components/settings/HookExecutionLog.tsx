import { Button, Empty, Select, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { Code, Play, Terminal, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

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
    if (!raw) { return []; }
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
  const [logs, setLogs] = useState<HookExecutionRecord[]>([]);
  const [filterEvent, setFilterEvent] = useState<string | undefined>(undefined);

  useEffect(() => {
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
      title: "时间",
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
      title: "事件名",
      dataIndex: "event",
      key: "event",
      width: 160,
      render: (event: string) => (
        <div className="flex items-center gap-1">
          <Play size={12} />
          <Tag style={{ margin: 0, fontSize: 11 }}>{event}</Tag>
        </div>
      ),
    },
    {
      title: "工具名",
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
            <Text type="secondary" style={{ fontSize: 11 }}>
              —
            </Text>
          ),
    },
    {
      title: "命令",
      dataIndex: "command",
      key: "command",
      ellipsis: true,
      render: (cmd: string) => (
        <div className="flex items-center gap-1">
          <Terminal size={12} />
          <Text code style={{ fontSize: 11 }}>
            {cmd}
          </Text>
        </div>
      ),
    },
    {
      title: "状态",
      dataIndex: "status",
      key: "status",
      width: 80,
      render: (status: string) =>
        status === "success"
          ? (
            <Tag color="success" style={{ fontSize: 11 }}>
              成功
            </Tag>
          )
          : (
            <Tag color="error" style={{ fontSize: 11 }}>
              失败
            </Tag>
          ),
    },
    {
      title: "输出摘要",
      dataIndex: "outputSummary",
      key: "outputSummary",
      ellipsis: true,
      render: (summary?: string) =>
        summary
          ? (
            <Text style={{ fontSize: 11, maxWidth: 200 }} ellipsis>
              {summary}
            </Text>
          )
          : (
            <Text type="secondary" style={{ fontSize: 11 }}>
              —
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
            按事件过滤：
          </Text>
          <Select
            allowClear
            size="small"
            placeholder="全部事件"
            value={filterEvent}
            onChange={(val) => setFilterEvent(val)}
            options={eventOptions}
            style={{ width: 180 }}
          />
        </Space>
        <Space size={8}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            共 {filteredLogs.length} 条记录
          </Text>
          {logs.length > 0 && (
            <Button
              size="small"
              danger
              icon={<Trash2 size={13} />}
              onClick={handleClearAll}
            >
              清空日志
            </Button>
          )}
        </Space>
      </div>

      {/* 表格 */}
      {filteredLogs.length === 0
        ? (
          <Empty
            description={logs.length === 0 ? "暂无 Hook 执行记录" : "无匹配的记录"}
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

export default HookExecutionLog;
