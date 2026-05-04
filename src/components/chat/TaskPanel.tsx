import { invoke, listen } from "@/lib/invoke";
import { Badge, Button, Empty, message, Popconfirm, Spin, Tag, Typography } from "antd";
import { Circle, LoaderCircle, StopCircle } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

const { Text, Paragraph } = Typography;

interface BackgroundTask {
  id: string;
  title: string;
  description: string;
  task_type: string;
  command?: string;
  prompt?: string;
  status: string;
  output: string;
  exit_code?: number;
  conversation_id?: string;
  created_at: number;
  updated_at: number;
  finished_at?: number;
}

const STATUS_CONFIG: Record<string, { color: string; icon: React.ReactNode; label: string }> = {
  pending: { color: "default", icon: <Circle size={10} />, label: "等待中" },
  running: {
    color: "processing",
    icon: <LoaderCircle size={10} style={{ animation: "spin 1s linear infinite" }} />,
    label: "运行中",
  },
  completed: { color: "success", icon: <Circle size={10} fill="currentColor" />, label: "已完成" },
  failed: { color: "error", icon: <Circle size={10} fill="currentColor" />, label: "失败" },
  stopped: { color: "warning", icon: <StopCircle size={10} />, label: "已停止" },
};

export function TaskPanel() {
  const [tasks, setTasks] = useState<BackgroundTask[]>([]);
  const [loading, setLoading] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const fetchTasks = useCallback(async () => {
    try {
      const result = await invoke<BackgroundTask[]>("list_background_tasks");
      setTasks(result);
    } catch (e) {
      console.error("Failed to fetch tasks:", e);
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    fetchTasks().finally(() => setLoading(false));

    // 监听实时更新
    let unlistenCreated: (() => void) | undefined;
    let unlistenUpdated: (() => void) | undefined;

    listen<{ task_id: string }>("background-task:created", () => {
      fetchTasks();
    }).then((fn) => {
      unlistenCreated = fn;
    });

    listen<{ task_id: string }>("background-task:updated", () => {
      fetchTasks();
    }).then((fn) => {
      unlistenUpdated = fn;
    });

    const interval = setInterval(fetchTasks, 5000);

    return () => {
      clearInterval(interval);
      unlistenCreated?.();
      unlistenUpdated?.();
    };
  }, [fetchTasks]);

  const handleStop = useCallback(async (taskId: string) => {
    try {
      await invoke("stop_background_task", { taskId });
      message.success("任务已停止");
      fetchTasks();
    } catch (e) {
      message.error(`停止失败: ${String(e)}`);
    }
  }, [fetchTasks]);

  const runningCount = tasks.filter((t) => t.status === "running").length;
  const pendingCount = tasks.filter((t) => t.status === "pending").length;

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <div
        style={{
          padding: "12px 16px",
          borderBottom: "1px solid var(--color-border)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <span style={{ fontWeight: 600, fontSize: 14 }}>
          后台任务
          {(runningCount > 0 || pendingCount > 0) && (
            <Badge count={runningCount + pendingCount} size="small" style={{ marginLeft: 8 }} />
          )}
        </span>
        <Button size="small" onClick={fetchTasks} loading={loading}>
          刷新
        </Button>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: "8px 12px" }}>
        {loading && tasks.length === 0 && <Spin style={{ display: "block", margin: "24px auto" }} />}

        {!loading && tasks.length === 0 && (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无后台任务" style={{ marginTop: 32 }} />
        )}

        {tasks.map((task) => {
          const cfg = STATUS_CONFIG[task.status] || STATUS_CONFIG.pending;
          const isExpanded = expandedId === task.id;

          return (
            <div
              key={task.id}
              style={{
                marginBottom: 8,
                border: "1px solid var(--color-border)",
                borderRadius: 8,
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  padding: "8px 12px",
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  cursor: "pointer",
                  backgroundColor: isExpanded ? "var(--color-fill-alter)" : "transparent",
                }}
                onClick={() => setExpandedId(isExpanded ? null : task.id)}
              >
                <span style={{ color: `var(--color-${cfg.color})`, display: "flex" }}>
                  {cfg.icon}
                </span>
                <Text strong style={{ flex: 1, fontSize: 13 }}>{task.title}</Text>
                <Tag color={cfg.color} style={{ margin: 0 }}>{cfg.label}</Tag>
                <Tag style={{ margin: 0, fontSize: 11 }}>{task.task_type}</Tag>
                {task.status === "running" && (
                  <Popconfirm
                    title="确定停止此任务？"
                    onConfirm={(e) => {
                      e?.stopPropagation();
                      handleStop(task.id);
                    }}
                    onCancel={(e) => e?.stopPropagation()}
                    okText="停止"
                    cancelText="取消"
                  >
                    <Button
                      type="text"
                      size="small"
                      danger
                      icon={<StopCircle size={14} />}
                      onClick={(e) => e.stopPropagation()}
                    />
                  </Popconfirm>
                )}
              </div>

              {isExpanded && (
                <div style={{ padding: "8px 12px 12px", borderTop: "1px solid var(--color-border-secondary)" }}>
                  {task.description && (
                    <Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 8 }}>
                      {task.description}
                    </Paragraph>
                  )}
                  {task.command && (
                    <Text code style={{ fontSize: 11, display: "block", marginBottom: 8 }}>
                      $ {task.command}
                    </Text>
                  )}
                  {task.exit_code != null && (
                    <Text type="secondary" style={{ fontSize: 11 }}>
                      退出码: {task.exit_code}
                    </Text>
                  )}
                  <div
                    style={{
                      marginTop: 8,
                      background: "var(--color-fill-secondary)",
                      borderRadius: 6,
                      padding: "8px 12px",
                      maxHeight: 300,
                      overflow: "auto",
                      fontFamily: "monospace",
                      fontSize: 11,
                      whiteSpace: "pre-wrap",
                      wordBreak: "break-all",
                    }}
                  >
                    {task.output || "(无输出)"}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
