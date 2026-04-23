import React, { useState, useMemo } from 'react';
import { Card, Space, Typography, Progress, theme, Badge, Tooltip } from 'antd';
import { Activity, ChevronDown, ChevronUp, CheckCircle2, XCircle, Loader2, Clock, Bot } from 'lucide-react';
import { useAgentStore } from '@/stores';
import { useTranslation } from 'react-i18next';

const { Text } = Typography;

interface TaskItem {
  id: string;
  toolName: string;
  status: 'queued' | 'running' | 'success' | 'failed' | 'cancelled';
  summary?: string;
}

interface AgentTaskListProps {
  conversationId?: string;
}

function generateTaskSummary(toolName: string, output: string | undefined, t: (key: string, options?: Record<string, unknown>) => string): string {
  if (!output) return '';

  if (output.includes('error') || output.includes('Error') || output.includes('failed')) {
    return t('chat.agentTaskList.summary.failed');
  }

  if (toolName.toLowerCase().includes('write') || toolName.toLowerCase().includes('edit')) {
    const match = output.match(/written|created|modified|(\d+)\s*bytes?/i);
    if (match) return t('chat.agentTaskList.summary.fileWritten');
    return t('chat.agentTaskList.summary.fileSaved');
  }

  if (toolName.toLowerCase().includes('read') || toolName.toLowerCase().includes('grep')) {
    const lines = output.split('\n').length;
    if (lines > 1) return t('chat.agentTaskList.summary.foundLines', { count: lines });
    return t('chat.agentTaskList.summary.readComplete');
  }

  if (toolName.toLowerCase().includes('bash') || toolName.toLowerCase().includes('shell')) {
    return t('chat.agentTaskList.summary.commandComplete');
  }

  if (output.length > 50) {
    return output.slice(0, 50) + '...';
  }
  return output;
}

const AgentTaskList: React.FC<AgentTaskListProps> = ({ conversationId }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [expanded, setExpanded] = useState(false);

  const agentStatus = useAgentStore((s) => conversationId ? s.agentStatus[conversationId] : undefined);
  const toolCalls = useAgentStore((s) => s.toolCalls);

  const tasks: TaskItem[] = useMemo(() => {
    return Object.values(toolCalls)
      .filter((tc) => !conversationId || tc.assistantMessageId === conversationId || tc.assistantMessageId === '')
      .map((tc) => ({
        id: tc.toolUseId,
        toolName: tc.toolName,
        status: tc.executionStatus,
        summary: generateTaskSummary(tc.toolName, tc.output, t),
      }))
      .slice(-10);
  }, [toolCalls, conversationId, t]);

  const stats = useMemo(() => {
    const total = tasks.length;
    const completed = tasks.filter((t) => t.status === 'success').length;
    const failed = tasks.filter((t) => t.status === 'failed').length;
    const running = tasks.filter((t) => t.status === 'running' || t.status === 'queued').length;
    const percent = total > 0 ? Math.round(((completed + failed) / total) * 100) : 0;
    return { total, completed, failed, running, percent };
  }, [tasks]);

  const currentTask = tasks.find((t) => t.status === 'running' || t.status === 'queued');
  const statusText = agentStatus || (stats.running > 0 ? t('chat.agentTaskList.executing') : t('chat.agentTaskList.completed'));

  const isAgentActive = tasks.some((t) => t.status === 'running' || t.status === 'queued');

  const getStatusIcon = (status: TaskItem['status']) => {
    switch (status) {
      case 'success':
        return <CheckCircle2 size={12} style={{ color: token.colorSuccess }} />;
      case 'failed':
        return <XCircle size={12} style={{ color: token.colorError }} />;
      case 'running':
        return <Loader2 size={12} className="spin" style={{ color: token.colorPrimary }} />;
      case 'cancelled':
        return <XCircle size={12} style={{ color: token.colorTextSecondary }} />;
      default:
        return <Clock size={12} style={{ color: token.colorTextSecondary }} />;
    }
  };

  const collapsedContent = (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '4px 8px',
      }}
    >
      {isAgentActive ? (
        <Badge status="processing" />
      ) : stats.failed > 0 ? (
        <Badge status="error" />
      ) : stats.completed > 0 ? (
        <Badge status="success" />
      ) : (
        <Badge status="default" />
      )}
      <Activity size={14} style={{ color: token.colorPrimary }} />
      <Text style={{ fontSize: 12 }}>
        {stats.completed}/{stats.total}
      </Text>
      {isAgentActive && currentTask && (
        <Text type="secondary" style={{ fontSize: 11 }} ellipsis>
          {currentTask.toolName}
        </Text>
      )}
    </div>
  );

  const expandedContent = (
    <div style={{ padding: '8px 12px', minWidth: 280, maxWidth: 320 }}>
      <Space style={{ marginBottom: 8, width: '100%', justifyContent: 'space-between' }}>
        <Space size={4}>
          <Bot size={14} style={{ color: token.colorPrimary }} />
          <Text strong style={{ fontSize: 13 }}>{t('chat.agentTaskList.title')}</Text>
        </Space>
        <Text type="secondary" style={{ fontSize: 11 }}>{statusText}</Text>
      </Space>

      <Progress
        percent={stats.percent}
        size="small"
        strokeColor={stats.failed > 0 ? token.colorError : token.colorPrimary}
        showInfo={false}
        style={{ marginBottom: 8 }}
      />

      <div style={{ display: 'flex', gap: 12, marginBottom: 8 }}>
        <Text style={{ fontSize: 11, color: token.colorSuccess }}>✓ {stats.completed}</Text>
        <Text style={{ fontSize: 11, color: token.colorError }}>✗ {stats.failed}</Text>
        <Text style={{ fontSize: 11, color: token.colorPrimary }}>⏳ {stats.running}</Text>
      </div>

      <div
        style={{
          maxHeight: 200,
          overflow: 'auto',
          borderTop: `1px solid ${token.colorBorder}`,
          paddingTop: 8,
        }}
      >
        {tasks.length === 0 ? (
          <Text type="secondary" style={{ fontSize: 11 }}>{t('chat.agentTaskList.noTasks')}</Text>
        ) : (
          tasks.map((task) => (
            <div
              key={task.id}
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: 6,
                padding: '4px 0',
                opacity: task.status === 'queued' ? 0.6 : 1,
              }}
            >
              {getStatusIcon(task.status)}
              <div style={{ flex: 1, minWidth: 0 }}>
                <Text style={{ fontSize: 12, display: 'block' }} ellipsis>
                  {task.toolName}
                </Text>
                {task.summary && (
                  <Text type="secondary" style={{ fontSize: 10 }} ellipsis>
                    {task.summary}
                  </Text>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );

  if (!isAgentActive && stats.total === 0) {
    return null;
  }

  return (
    <div
      style={{
        position: 'fixed',
        bottom: 16,
        right: 16,
        zIndex: 1000,
        transition: 'all 0.3s ease',
      }}
    >
      <Card
        size="small"
        bodyStyle={{ padding: 0 }}
        style={{
          borderRadius: 12,
          boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
          minWidth: expanded ? 300 : 'auto',
          maxWidth: 360,
        }}
      >
        <div
          onClick={() => setExpanded(!expanded)}
          style={{
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '8px 12px',
            borderBottom: expanded ? `1px solid ${token.colorBorder}` : 'none',
          }}
        >
          {collapsedContent}
          <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            {isAgentActive && (
              <Tooltip title={t('chat.agentTaskList.agentRunning')}>
                <div className="pulse-dot" style={{ width: 8, height: 8, borderRadius: '50%', backgroundColor: token.colorPrimary }} />
              </Tooltip>
            )}
            {expanded ? <ChevronDown size={14} /> : <ChevronUp size={14} />}
          </div>
        </div>

        {expanded && expandedContent}
      </Card>

      <style>{`
        .spin {
          animation: spin 1s linear infinite;
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        .pulse-dot {
          animation: pulse 1.5s ease-in-out infinite;
        }
        @keyframes pulse {
          0%, 100% { opacity: 1; transform: scale(1); }
          50% { opacity: 0.5; transform: scale(0.8); }
        }
      `}</style>
    </div>
  );
};

export default AgentTaskList;