import { Progress, Space, Typography } from 'antd';
import {
  ClockCircleOutlined,
  CheckCircleOutlined,
  SearchOutlined,
  StarOutlined,
  FileTextOutlined,
  LinkOutlined,
} from '@ant-design/icons';

const { Text } = Typography;

type ResearchPhase = 'planning' | 'searching' | 'extracting' | 'analyzing' | 'synthesizing' | 'reporting';

interface ResearchProgressProps {
  currentPhase: ResearchPhase;
  percentage: number;
  currentQuery?: string | null;
  showDetails?: boolean;
}

const phaseSteps: { key: ResearchPhase; label: string; icon: React.ReactNode }[] = [
  { key: 'planning', label: '规划', icon: <ClockCircleOutlined /> },
  { key: 'searching', label: '搜索', icon: <SearchOutlined /> },
  { key: 'extracting', label: '提取', icon: <LinkOutlined /> },
  { key: 'analyzing', label: '分析', icon: <StarOutlined /> },
  { key: 'synthesizing', label: '综合', icon: <CheckCircleOutlined /> },
  { key: 'reporting', label: '报告', icon: <FileTextOutlined /> },
];

export function ResearchProgress({ currentPhase, percentage, currentQuery, showDetails = true }: ResearchProgressProps) {
  const currentIndex = phaseSteps.findIndex((p) => p.key === currentPhase);

  return (
    <div className="research-progress">
      <div className="flex items-center justify-between mb-2">
        {phaseSteps.map((step, index) => {
          const isCompleted = index < currentIndex;
          const isCurrent = index === currentIndex;
          return (
            <div
              key={step.key}
              className={`flex flex-col items-center ${
                isCompleted ? 'text-green-500' : isCurrent ? 'text-blue-500' : 'text-gray-400'
              }`}
            >
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center ${
                  isCompleted ? 'bg-green-500 text-white' : isCurrent ? 'bg-blue-500 text-white' : 'bg-gray-200'
                }`}
              >
                {step.icon}
              </div>
              <Text className="text-xs mt-1">{step.label}</Text>
            </div>
          );
        })}
      </div>
      <Progress percent={percentage} showInfo={false} strokeColor="#1890ff" />

      {showDetails && (
        <div className="mt-2">
          <Text type="secondary" className="text-sm">
            当前阶段: {phaseSteps[currentIndex]?.label || '未知'}
            {currentQuery && ` - ${currentQuery}`}
          </Text>
        </div>
      )}
    </div>
  );
}

export function ResearchProgressMini({ percentage }: { percentage: number }) {
  return (
    <Progress
      percent={percentage}
      size="small"
      strokeColor="#1890ff"
      showInfo={false}
    />
  );
}

export function ResearchPhaseIndicator({ phase }: { phase: ResearchPhase }) {
  const phaseIndex = phaseSteps.findIndex((p) => p.key === phase);
  const completedPhases = phaseSteps.slice(0, phaseIndex);
  const remainingPhases = phaseSteps.slice(phaseIndex + 1);

  return (
    <Space size="small">
      {completedPhases.map((step) => (
        <CheckCircleOutlined key={step.key} className="text-green-500" />
      ))}
      <span className="text-blue-500 font-medium">{phaseSteps[phaseIndex]?.label}</span>
      {remainingPhases.map((step) => (
        <ClockCircleOutlined key={step.key} className="text-gray-400" />
      ))}
    </Space>
  );
}

export default ResearchProgress;
