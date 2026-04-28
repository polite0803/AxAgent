import { Badge, Card, Progress, Space, Tag, Typography } from "antd";
import {
  CheckCircle,
  Cpu,
  Loader2,
  ThumbsUp,
  ThumbsDown,
  Users,
  XCircle,
} from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface AgentVote {
  agent_id: string;
  agent_name: string;
  agent_role: string;
  vote: "approve" | "reject" | "abstain";
  reason?: string;
  confidence: number;
}

interface ConsensusResult {
  decision: "approved" | "rejected" | "pending" | "conflict";
  approve_count: number;
  reject_count: number;
  abstain_count: number;
  total_votes: number;
  threshold: number;
  strategy: ConsensusStrategy;
  votes: AgentVote[];
}

type ConsensusStrategy =
  | { type: "majority_vote" }
  | { type: "unanimous" }
  | { type: "leader_decides"; leader_role: string }
  | { type: "weighted_vote"; weights: Record<string, number> };

interface ConsensusViewProps {
  title?: string;
  description?: string;
  result: ConsensusResult;
  onViewDetails?: (agentId: string) => void;
}

const STRATEGY_LABELS: Record<string, string> = {
  majority_vote: "Majority Vote",
  unanimous: "Unanimous",
  leader_decides: "Leader Decides",
  weighted_vote: "Weighted Vote",
};

function ConsensusView({
  title,
  description,
  result,
  onViewDetails,
}: ConsensusViewProps) {
  const { t } = useTranslation();

  const approvalPercent = result.total_votes > 0
    ? Math.round((result.approve_count / result.total_votes) * 100)
    : 0;
  const rejectionPercent = result.total_votes > 0
    ? Math.round((result.reject_count / result.total_votes) * 100)
    : 0;

  const decisionColor = result.decision === "approved"
    ? "#52c41a"
    : result.decision === "rejected"
      ? "#ff4d4f"
      : result.decision === "conflict"
        ? "#faad14"
        : "#8c8c8c";

  const decisionIcon = result.decision === "approved"
    ? <CheckCircle size={20} />
    : result.decision === "rejected"
      ? <XCircle size={20} />
      : <Loader2 size={20} className="animate-spin" />;

  const sortedVotes = useMemo(() => {
    return [...result.votes].sort((a, b) => b.confidence - a.confidence);
  }, [result.votes]);

  return (
    <Card size="small" className="consensus-view">
      <div className="flex items-center justify-between mb-3">
        <Space>
          <Users size={16} className="text-purple-500" />
          <Title level={5} className="mb-0">
            {title || t("chat.multiAgent.consensus.title")}
          </Title>
        </Space>
        <Tag color={decisionColor} className="flex items-center gap-1">
          {decisionIcon}
          <span>{result.decision}</span>
        </Tag>
      </div>

      {description && (
        <Text type="secondary" className="block mb-3 text-sm">
          {description}
        </Text>
      )}

      <div className="mb-3">
        <Space size="small" className="mb-1">
          <Cpu size={12} className="text-gray-400" />
          <Text type="secondary" className="text-xs">
            {STRATEGY_LABELS[result.strategy.type] || result.strategy.type}
          </Text>
          <Text type="secondary" className="text-xs">
            {t("chat.multiAgent.consensus.threshold", { pct: Math.round(result.threshold * 100) })}
          </Text>
        </Space>
      </div>

      <div className="grid grid-cols-3 gap-2 mb-3">
        <Card size="small" className="bg-green-50 dark:bg-green-900/10 text-center">
          <Text className="text-lg font-bold text-green-600 block">{result.approve_count}</Text>
          <Space>
            <ThumbsUp size={12} className="text-green-500" />
            <Text type="secondary" className="text-xs">{t("chat.multiAgent.consensus.approve")}</Text>
          </Space>
        </Card>
        <Card size="small" className="bg-red-50 dark:bg-red-900/10 text-center">
          <Text className="text-lg font-bold text-red-600 block">{result.reject_count}</Text>
          <Space>
            <ThumbsDown size={12} className="text-red-500" />
            <Text type="secondary" className="text-xs">{t("chat.multiAgent.consensus.reject")}</Text>
          </Space>
        </Card>
        <Card size="small" className="bg-gray-50 dark:bg-gray-800/50 text-center">
          <Text className="text-lg font-bold text-gray-600 block">{result.abstain_count}</Text>
          <Space>
            <Users size={12} className="text-gray-400" />
            <Text type="secondary" className="text-xs">{t("chat.multiAgent.consensus.abstain")}</Text>
          </Space>
        </Card>
      </div>

      <div className="mb-3 space-y-1">
        <div className="flex items-center gap-2">
          <Text type="secondary" className="text-xs w-16">{t("chat.multiAgent.consensus.approve")}</Text>
          <Progress
            percent={approvalPercent}
            size="small"
            strokeColor="#52c41a"
            showInfo={false}
            className="flex-1"
          />
          <Text className="text-xs w-8 text-right">{approvalPercent}%</Text>
        </div>
        <div className="flex items-center gap-2">
          <Text type="secondary" className="text-xs w-16">{t("chat.multiAgent.consensus.reject")}</Text>
          <Progress
            percent={rejectionPercent}
            size="small"
            strokeColor="#ff4d4f"
            showInfo={false}
            className="flex-1"
          />
          <Text className="text-xs w-8 text-right">{rejectionPercent}%</Text>
        </div>
      </div>

      <div>
        <Text strong className="text-xs block mb-1">
          {t("chat.multiAgent.consensus.votes")} ({result.votes.length})
        </Text>
        <div className="space-y-1 max-h-48 overflow-auto">
          {sortedVotes.map((vote) => (
            <div
              key={vote.agent_id}
              className="flex items-center gap-2 px-2 py-1 rounded text-xs hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer"
              onClick={() => onViewDetails?.(vote.agent_id)}
            >
              {vote.vote === "approve"
                ? <ThumbsUp size={10} className="text-green-500 shrink-0" />
                : vote.vote === "reject"
                  ? <ThumbsDown size={10} className="text-red-500 shrink-0" />
                  : <Users size={10} className="text-gray-400 shrink-0" />}
              <span className="font-medium">{vote.agent_name}</span>
              <Badge
                color={vote.vote === "approve" ? "#52c41a" : vote.vote === "reject" ? "#ff4d4f" : "#8c8c8c"}
                text={<span className="text-xs text-gray-500">{vote.agent_role}</span>}
              />
              <span className="text-gray-400 ml-auto">
                {t("chat.multiAgent.consensus.confidence", { pct: Math.round(vote.confidence * 100) })}
              </span>
            </div>
          ))}
        </div>
      </div>

      {result.votes.length === 0 && (
        <div className="py-4 text-xs text-gray-400 text-center">
          {t("chat.multiAgent.consensus.noVotes")}
        </div>
      )}
    </Card>
  );
}

export default ConsensusView;
