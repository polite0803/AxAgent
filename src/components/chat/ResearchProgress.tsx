import {
  CheckCircleOutlined,
  ClockCircleOutlined,
  FileTextOutlined,
  LinkOutlined,
  SearchOutlined,
  StarOutlined,
} from "@ant-design/icons";
import { Progress, Space, Typography } from "antd";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

type ResearchPhase =
  | "planning"
  | "searching"
  | "extracting"
  | "analyzing"
  | "synthesizing"
  | "reporting";

interface ResearchProgressProps {
  currentPhase: ResearchPhase;
  percentage: number;
  currentQuery?: string | null;
  showDetails?: boolean;
}

const PHASE_KEYS: { key: ResearchPhase; labelKey: string; icon: React.ReactNode }[] = [
  { key: "planning", labelKey: "research.progress.phasePlanning", icon: <ClockCircleOutlined /> },
  { key: "searching", labelKey: "research.progress.phaseSearching", icon: <SearchOutlined /> },
  { key: "extracting", labelKey: "research.progress.phaseExtracting", icon: <LinkOutlined /> },
  { key: "analyzing", labelKey: "research.progress.phaseAnalyzing", icon: <StarOutlined /> },
  { key: "synthesizing", labelKey: "research.progress.phaseSynthesizing", icon: <CheckCircleOutlined /> },
  { key: "reporting", labelKey: "research.progress.phaseReporting", icon: <FileTextOutlined /> },
];

export function ResearchProgress({
  currentPhase,
  percentage,
  currentQuery,
  showDetails = true,
}: ResearchProgressProps) {
  const { t } = useTranslation();
  const currentIndex = PHASE_KEYS.findIndex((p) => p.key === currentPhase);

  return (
    <div className="research-progress">
      <div className="flex items-center justify-between mb-2">
        {PHASE_KEYS.map((step, index) => {
          const isCompleted = index < currentIndex;
          const isCurrent = index === currentIndex;
          return (
            <div
              key={step.key}
              className={`flex flex-col items-center ${
                isCompleted
                  ? "text-green-500"
                  : isCurrent
                  ? "text-blue-500"
                  : "text-zinc-400"
              }`}
            >
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center ${
                  isCompleted
                    ? "bg-green-500 text-white"
                    : isCurrent
                    ? "bg-blue-500 text-white"
                    : "bg-zinc-200"
                }`}
              >
                {step.icon}
              </div>
              <Text className="text-xs mt-1">{t(step.labelKey)}</Text>
            </div>
          );
        })}
      </div>
      <Progress percent={percentage} showInfo={false} strokeColor="#1890ff" />

      {showDetails && (
        <div className="mt-2">
          <Text type="secondary" className="text-sm">
            {t("research.progress.currentPhase", { phase: t(PHASE_KEYS[currentIndex]?.labelKey || "") })}
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
  const { t } = useTranslation();
  const phaseIndex = PHASE_KEYS.findIndex((p) => p.key === phase);
  const completedPhases = PHASE_KEYS.slice(0, phaseIndex);
  const remainingPhases = PHASE_KEYS.slice(phaseIndex + 1);

  return (
    <Space size="small">
      {completedPhases.map((step) => <CheckCircleOutlined key={step.key} className="text-green-500" />)}
      <span className="text-blue-500 font-medium">
        {t(PHASE_KEYS[phaseIndex]?.labelKey || "")}
      </span>
      {remainingPhases.map((step) => <ClockCircleOutlined key={step.key} className="text-zinc-400" />)}
    </Space>
  );
}
