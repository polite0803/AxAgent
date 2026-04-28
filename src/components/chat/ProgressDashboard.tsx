import { Badge, Card, Progress, Space, Typography } from 'antd';
import {
  CheckCircle,
  Clock,
  Loader2,
  SkipForward,
  XCircle,
  GitBranch,
  Bot,
  FileText,
} from 'lucide-react';
import React from 'react';
import { useTranslation } from 'react-i18next';

const { Text, Title } = Typography;

interface Phase {
  id: string;
  name: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'skipped';
  tasks: Task[];
}

interface Task {
  id: string;
  description: string;
  status: 'pending' | 'blocked' | 'running' | 'completed' | 'failed' | 'skipped';
  result?: string;
  retry_count: number;
  max_retries: number;
}

interface Plan {
  id: string;
  goal: string;
  phases: Phase[];
  status: 'draft' | 'executing' | 'paused' | 'completed' | 'failed';
  created_at: number;
  updated_at: number;
}

interface ProgressDashboardProps {
  plan: Plan;
  showDetails?: boolean;
  compact?: boolean;
}

const getPhaseIcon = (status: Phase['status']) => {
  switch (status) {
    case 'completed':
      return <CheckCircle size={14} className="text-green-500" />;
    case 'in_progress':
      return <Loader2 size={14} className="text-blue-500 animate-spin" />;
    case 'failed':
      return <XCircle size={14} className="text-red-500" />;
    case 'skipped':
      return <SkipForward size={14} className="text-yellow-500" />;
    default:
      return <Clock size={14} className="text-gray-400" />;
  }
};

const getTaskIcon = (status: Task['status']) => {
  switch (status) {
    case 'completed':
      return <CheckCircle size={12} className="text-green-500" />;
    case 'running':
      return <Loader2 size={12} className="text-blue-500 animate-spin" />;
    case 'failed':
      return <XCircle size={12} className="text-red-500" />;
    case 'blocked':
      return <Clock size={12} className="text-orange-500" />;
    case 'skipped':
      return <SkipForward size={12} className="text-yellow-500" />;
    default:
      return <Clock size={12} className="text-gray-400" />;
  }
};

const getStatusColor = (status: string): string => {
  switch (status) {
    case 'completed':
    case 'success':
      return '#52c41a';
    case 'running':
    case 'executing':
    case 'in_progress':
      return '#1890ff';
    case 'failed':
    case 'error':
      return '#ff4d4f';
    case 'skipped':
    case 'paused':
      return '#faad14';
    default:
      return '#8c8c8c';
  }
};

function PhaseCard({ phase, showTasks }: { phase: Phase; showTasks: boolean }) {
  const { t } = useTranslation();
  const completedTasks = phase.tasks.filter((t) => t.status === 'completed').length;
  const totalTasks = phase.tasks.length;
  const percent = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  return (
    <Card size="small" className="phase-card">
      <div className="flex items-center justify-between mb-2">
        <Space>
          {getPhaseIcon(phase.status)}
          <Text strong>{phase.name}</Text>
        </Space>
        <Badge
          status={phase.status === 'completed' ? 'success' : phase.status === 'in_progress' ? 'processing' : phase.status === 'failed' ? 'error' : 'default'}
          text={<Text type="secondary" className="text-xs">{t(`chat.planner.phaseStatus.${phase.status}`, phase.status)}</Text>}
        />
      </div>
      <Progress percent={percent} size="small" showInfo={false} strokeColor={getStatusColor(phase.status)} />

      {showTasks && (
        <div className="mt-2 pl-4 border-l-2 border-gray-200">
          {phase.tasks.map((task) => (
            <div key={task.id} className="flex items-center justify-between py-1">
              <Space size="small">
                {getTaskIcon(task.status)}
                <Text className="text-sm" ellipsis={{ tooltip: task.description }}>
                  {task.description.length > 50 ? task.description.slice(0, 50) + '...' : task.description}
                </Text>
              </Space>
              {task.retry_count > 0 && (
                <Text type="warning" className="text-xs">
                  {t('chat.planner.retry', { count: task.retry_count })}
                </Text>
              )}
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

export function ProgressDashboard({ plan, showDetails = true, compact = false }: ProgressDashboardProps) {
  const { t } = useTranslation();

  const completedPhases = plan.phases.filter((p) => p.status === 'completed').length;
  const totalPhases = plan.phases.length;
  const overallPercent = totalPhases > 0 ? Math.round((completedPhases / totalPhases) * 100) : 0;

  const totalTasks = plan.phases.reduce((sum, p) => sum + p.tasks.length, 0);
  const completedTasks = plan.phases.reduce(
    (sum, p) => sum + p.tasks.filter((t) => t.status === 'completed').length,
    0
  );
  const taskPercent = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  if (compact) {
    return (
      <div className="flex items-center gap-2">
        <Progress type="circle" percent={overallPercent} size={40} strokeColor={getStatusColor(plan.status)} />
        <div>
          <Text className="text-sm">{plan.goal}</Text>
          <br />
          <Text type="secondary" className="text-xs">
            {completedTasks}/{totalTasks} {t('chat.planner.tasksCompleted')}
          </Text>
        </div>
      </div>
    );
  }

  return (
    <div className="progress-dashboard">
      <Card>
        <div className="flex items-center justify-between mb-4">
          <Space>
            <GitBranch size={20} className="text-blue-500" />
            <Title level={5} className="mb-0">{t('chat.planner.planProgress')}</Title>
          </Space>
          <Badge
            status={plan.status === 'completed' ? 'success' : plan.status === 'executing' ? 'processing' : plan.status === 'failed' ? 'error' : 'default'}
            text={<Text type="secondary">{t(`chat.planner.planStatus.${plan.status}`, plan.status)}</Text>}
          />
        </div>

        <Text className="text-lg font-medium block mb-4">{plan.goal}</Text>

        <div className="grid grid-cols-2 gap-4 mb-4">
          <Card size="small" className="bg-gray-50">
            <div className="flex items-center gap-2">
              <Bot size={16} className="text-blue-500" />
              <Text type="secondary">{t('chat.planner.phases')}</Text>
            </div>
            <div className="flex items-end gap-2 mt-2">
              <Progress percent={overallPercent} size="small" showInfo={false} className="mb-0" />
              <Text className="text-sm">{completedPhases}/{totalPhases}</Text>
            </div>
          </Card>

          <Card size="small" className="bg-gray-50">
            <div className="flex items-center gap-2">
              <FileText size={16} className="text-green-500" />
              <Text type="secondary">{t('chat.planner.tasks')}</Text>
            </div>
            <div className="flex items-end gap-2 mt-2">
              <Progress percent={taskPercent} size="small" showInfo={false} className="mb-0" strokeColor="#52c41a" />
              <Text className="text-sm">{completedTasks}/{totalTasks}</Text>
            </div>
          </Card>
        </div>

        {showDetails && (
          <div className="space-y-2">
            {plan.phases.map((phase) => (
              <PhaseCard key={phase.id} phase={phase} showTasks={showDetails} />
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}

export function ProgressDashboardMini({ plan }: { plan: Plan }) {
  const completedPhases = plan.phases.filter((p) => p.status === 'completed').length;
  const totalPhases = plan.phases.length;
  const percent = totalPhases > 0 ? Math.round((completedPhases / totalPhases) * 100) : 0;

  return <Progress percent={percent} size="small" strokeColor={getStatusColor(plan.status)} />;
}

export function PhaseProgressIndicator({ phases, currentPhaseId }: { phases: Phase[]; currentPhaseId?: string }) {
  const currentIndex = phases.findIndex((p) => p.id === currentPhaseId);

  return (
    <Space size="small">
      {phases.map((phase, index) => {
        const isCompleted = phase.status === 'completed';
        const isCurrent = phase.id === currentPhaseId || index === currentIndex;

        return (
          <React.Fragment key={phase.id}>
            {index > 0 && <span className="text-gray-300">/</span>}
            <span className={isCompleted ? 'text-green-500' : isCurrent ? 'text-blue-500 font-medium' : 'text-gray-400'}>
              {phase.name}
            </span>
          </React.Fragment>
        );
      })}
    </Space>
  );
}

export default ProgressDashboard;
