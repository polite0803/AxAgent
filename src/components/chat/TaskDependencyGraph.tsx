import { Badge, Card, Collapse, Space, Typography } from 'antd';
import {
  CheckCircle,
  ChevronDown,
  ChevronRight,
  Clock,
  GitBranch,
  Loader2,
  Network,
  SkipForward,
  XCircle,
} from 'lucide-react';
import React, { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

const { Text, Title } = Typography;

export interface TaskNode {
  id: string;
  description: string;
  status: 'pending' | 'blocked' | 'running' | 'completed' | 'failed' | 'skipped';
  dependencies: string[];
  result?: string;
  error?: string;
  retry_count: number;
  max_retries: number;
  phase?: string;
  agent_id?: string;
}

interface TaskDependencyGraphProps {
  tasks: TaskNode[];
  showMermaid?: boolean;
  showDetails?: boolean;
  title?: string;
}

function generateMermaidGraph(tasks: TaskNode[]): string {
  const lines: string[] = ['graph TD'];
  const added = new Set<string>();

  const getNodeShape = (task: TaskNode): string => {
    const truncatedDesc = task.description.length > 40
      ? task.description.slice(0, 40) + '...'
      : task.description;
    return `${task.id}["${truncatedDesc}"]`;
  };

  for (const task of tasks) {
    if (!added.has(task.id)) {
      lines.push(`  ${getNodeShape(task)}`);
      added.add(task.id);
    }

    for (const depId of task.dependencies) {
      if (!added.has(depId)) {
        const depTask = tasks.find((t) => t.id === depId);
        const depDesc = depTask
          ? depTask.description.slice(0, 40) + (depTask.description.length > 40 ? '...' : '')
          : depId;
        lines.push(`  ${depId}["${depDesc}"]`);
        added.add(depId);
      }
      lines.push(`  ${depId} --> ${task.id}`);
    }
  }

  const byStatus: Record<string, string[]> = {};
  for (const task of tasks) {
    (byStatus[task.status] ??= []).push(task.id);
  }

  if (byStatus.running?.length) {
    lines.push(`  style ${byStatus.running.join(',')} fill:#e6f7ff,stroke:#1890ff`);
  }
  if (byStatus.completed?.length) {
    lines.push(`  style ${byStatus.completed.join(',')} fill:#f6ffed,stroke:#52c41a`);
  }
  if (byStatus.failed?.length) {
    lines.push(`  style ${byStatus.failed.join(',')} fill:#fff2f0,stroke:#ff4d4f`);
  }
  if (byStatus.blocked?.length) {
    lines.push(`  style ${byStatus.blocked.join(',')} fill:#fffbe6,stroke:#faad14`);
  }
  if (byStatus.skipped?.length) {
    lines.push(`  style ${byStatus.skipped.join(',')} fill:#f9f9f9,stroke:#8c8c8c`);
  }

  return lines.join('\n');
}

const getStatusIcon = (status: TaskNode['status']) => {
  switch (status) {
    case 'completed':
      return <CheckCircle size={14} className="text-green-500" />;
    case 'running':
      return <Loader2 size={14} className="text-blue-500 animate-spin" />;
    case 'failed':
      return <XCircle size={14} className="text-red-500" />;
    case 'blocked':
      return <Clock size={14} className="text-orange-500" />;
    case 'skipped':
      return <SkipForward size={14} className="text-yellow-500" />;
    default:
      return <Clock size={14} className="text-gray-400" />;
  }
};

const getStatusColor = (status: TaskNode['status']): string => {
  switch (status) {
    case 'completed':
      return '#52c41a';
    case 'running':
      return '#1890ff';
    case 'failed':
      return '#ff4d4f';
    case 'blocked':
      return '#faad14';
    case 'skipped':
      return '#8c8c8c';
    default:
      return '#d9d9d9';
  }
};

interface TaskDetailProps {
  task: TaskNode;
  allTasks: TaskNode[];
}

function TaskDetail({ task, allTasks }: TaskDetailProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = React.useState(false);

  const dependentTasks = useMemo(() => {
    return allTasks.filter((t) => t.dependencies.includes(task.id));
  }, [task.id, allTasks]);

  const dependencyTasks = useMemo(() => {
    return task.dependencies
      .map((id) => allTasks.find((t) => t.id === id))
      .filter(Boolean) as TaskNode[];
  }, [task.dependencies, allTasks]);

  return (
    <div className="border-b border-gray-100 dark:border-gray-800 last:border-b-0">
      <div
        className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50"
        onClick={() => setExpanded(!expanded)}
      >
        <span style={{ color: getStatusColor(task.status), display: 'flex', alignItems: 'center', flexShrink: 0 }}>
          {getStatusIcon(task.status)}
        </span>
        <span className="text-xs font-mono font-medium shrink-0" style={{ color: getStatusColor(task.status) }}>
          {task.id}
        </span>
        <span className="text-xs text-gray-500 dark:text-gray-400 truncate flex-1">
          {task.description}
        </span>
        {task.phase && (
          <Badge color="blue" text={<span className="text-xs">{task.phase}</span>} />
        )}
        {task.retry_count > 0 && (
          <span className="text-xs text-orange-500 shrink-0" title={t('chat.planner.retries', { count: task.retry_count })}>
            <Clock size={12} />
          </span>
        )}
        <span style={{ display: 'flex', alignItems: 'center', flexShrink: 0, color: 'var(--color-text-secondary, #999)' }}>
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </div>
      {expanded && (
        <div className="px-3 pb-2 text-xs space-y-2">
          <div className="flex gap-4">
            <span className="text-gray-500">{t('chat.planner.status')}:</span>
            <span style={{ color: getStatusColor(task.status) }}>{task.status}</span>
          </div>
          {task.agent_id && (
            <div className="flex gap-4">
              <span className="text-gray-500">{t('chat.planner.agent')}:</span>
              <span>{task.agent_id}</span>
            </div>
          )}
          {dependencyTasks.length > 0 && (
            <div className="flex gap-4 flex-wrap">
              <span className="text-gray-500 shrink-0">{t('chat.planner.dependsOn')}:</span>
              <Space size="small">
                {dependencyTasks.map((dep) => (
                  <Badge key={dep.id} color={getStatusColor(dep.status)} text={<span className="text-xs">{dep.id}</span>} />
                ))}
              </Space>
            </div>
          )}
          {dependentTasks.length > 0 && (
            <div className="flex gap-4 flex-wrap">
              <span className="text-gray-500 shrink-0">{t('chat.planner.requiredBy')}:</span>
              <Space size="small">
                {dependentTasks.map((dep) => (
                  <Badge key={dep.id} color={getStatusColor(dep.status)} text={<span className="text-xs">{dep.id}</span>} />
                ))}
              </Space>
            </div>
          )}
          <div className="flex gap-4">
            <span className="text-gray-500">{t('chat.planner.retries')}:</span>
            <span>{task.retry_count}/{task.max_retries}</span>
          </div>
          {task.result && (
            <div>
              <span className="text-gray-500 block mb-1">{t('chat.planner.result')}:</span>
              <pre className="mt-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs max-h-32 overflow-auto whitespace-pre-wrap">
                {task.result.length > 500 ? task.result.slice(0, 500) + '...' : task.result}
              </pre>
            </div>
          )}
          {task.error && (
            <div>
              <span className="text-red-500 block mb-1">{t('chat.planner.error')}:</span>
              <pre className="mt-1 p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded text-xs max-h-32 overflow-auto whitespace-pre-wrap text-red-600 dark:text-red-400">
                {task.error}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function TaskDependencyGraph({
  tasks,
  showMermaid = true,
  showDetails = true,
  title,
}: TaskDependencyGraphProps) {
  const { t } = useTranslation();
  const mermaidGraph = useMemo(() => generateMermaidGraph(tasks), [tasks]);

  const stats = useMemo(() => {
    const total = tasks.length;
    const completed = tasks.filter((t) => t.status === 'completed').length;
    const running = tasks.filter((t) => t.status === 'running').length;
    const failed = tasks.filter((t) => t.status === 'failed').length;
    const blocked = tasks.filter((t) => t.status === 'blocked').length;
    const pending = tasks.filter((t) => t.status === 'pending').length;
    return { total, completed, running, failed, blocked, pending };
  }, [tasks]);

  const tasksByPhase = useMemo(() => {
    const phases: Record<string, TaskNode[]> = {};
    for (const task of tasks) {
      const phase = task.phase || 'default';
      if (!phases[phase]) {
        phases[phase] = [];
      }
      phases[phase].push(task);
    }
    return phases;
  }, [tasks]);

  return (
    <Card size="small" className="task-dependency-graph">
      <div className="flex items-center justify-between mb-3">
        <Space>
          <Network size={18} className="text-blue-500" />
          <Title level={5} className="mb-0">
            {title || t('chat.planner.taskGraph')}
          </Title>
        </Space>
        <Space size="small">
          <Badge status="success" text={<Text type="secondary" className="text-xs">{stats.completed}</Text>} />
          <Badge status="processing" text={<Text type="secondary" className="text-xs">{stats.running}</Text>} />
          <Badge status="error" text={<Text type="secondary" className="text-xs">{stats.failed}</Text>} />
          <Badge status="warning" text={<Text type="secondary" className="text-xs">{stats.blocked}</Text>} />
          <Badge status="default" text={<Text type="secondary" className="text-xs">{stats.pending}</Text>} />
        </Space>
      </div>

      {showMermaid && (
        <div className="mermaid-container mb-3 p-2 bg-gray-50 dark:bg-gray-800/50 rounded border border-gray-200 dark:border-gray-700 overflow-auto max-h-48">
          <pre className="text-xs">{mermaidGraph}</pre>
        </div>
      )}

      {showDetails && Object.keys(tasksByPhase).length > 1 && (
        <Collapse
          size="small"
          items={Object.entries(tasksByPhase).map(([phase, phaseTasks]) => ({
            key: phase,
            label: (
              <Space>
                <GitBranch size={14} />
                <span>{phase}</span>
                <Badge count={phaseTasks.length} size="small" />
              </Space>
            ),
            children: phaseTasks.map((task) => (
              <TaskDetail key={task.id} task={task} allTasks={tasks} />
            )),
          }))}
        />
      )}

      {showDetails && Object.keys(tasksByPhase).length === 1 && (
        <div>
          {tasks.map((task) => (
            <TaskDetail key={task.id} task={task} allTasks={tasks} />
          ))}
        </div>
      )}
    </Card>
  );
}

export function TaskDependencyGraphMini({ tasks }: { tasks: TaskNode[] }) {
  const stats = useMemo(() => {
    const total = tasks.length;
    const completed = tasks.filter((t) => t.status === 'completed').length;
    const percent = total > 0 ? Math.round((completed / total) * 100) : 0;
    return { total, completed, percent };
  }, [tasks]);

  return (
    <div className="flex items-center gap-2">
      <div className="w-24">
        <div
          className="h-2 rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden"
        >
          <div
            className="h-full bg-green-500 transition-all"
            style={{ width: `${stats.percent}%` }}
          />
        </div>
      </div>
      <Text type="secondary" className="text-xs">
        {stats.completed}/{stats.total}
      </Text>
    </div>
  );
}

export default TaskDependencyGraph;
