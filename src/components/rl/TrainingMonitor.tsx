import { CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined } from "@ant-design/icons";
import { Badge, Card, Progress, Timeline, Typography } from "antd";

interface TrainingMonitorProps {
  policyId: string;
  status: "idle" | "training" | "completed" | "failed";
  episodesCompleted: number;
  totalEpisodes: number;
  avgReward: number;
}

export function TrainingMonitor({
  policyId,
  status,
  episodesCompleted,
  totalEpisodes,
  avgReward,
}: TrainingMonitorProps) {
  const progress = totalEpisodes > 0 ? (episodesCompleted / totalEpisodes) * 100 : 0;

  const getStatusBadge = () => {
    switch (status) {
      case "training":
        return <Badge status="processing" text="Training" />;
      case "completed":
        return <Badge status="success" text="Completed" />;
      case "failed":
        return <Badge status="error" text="Failed" />;
      default:
        return <Badge status="default" text="Idle" />;
    }
  };

  const getStatusIcon = () => {
    switch (status) {
      case "training":
        return <LoadingOutlined />;
      case "completed":
        return <CheckCircleOutlined />;
      case "failed":
        return <CloseCircleOutlined />;
      default:
        return null;
    }
  };

  return (
    <Card title="Training Progress">
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <Typography.Text type="secondary">Policy ID</Typography.Text>
          <Typography.Text code>{policyId}</Typography.Text>
        </div>

        <div className="flex items-center justify-between">
          <Typography.Text type="secondary">Status</Typography.Text>
          <div className="flex items-center gap-2">
            {getStatusIcon()}
            {getStatusBadge()}
          </div>
        </div>

        <div>
          <div className="flex justify-between mb-1">
            <Typography.Text type="secondary">Progress</Typography.Text>
            <Typography.Text>
              {episodesCompleted} / {totalEpisodes} episodes
            </Typography.Text>
          </div>
          <Progress percent={Math.round(progress)} status={status === "failed" ? "exception" : undefined} />
        </div>

        <div>
          <Typography.Text type="secondary">Average Reward</Typography.Text>
          <div className="text-2xl font-bold mt-1">{avgReward.toFixed(4)}</div>
        </div>

        {status === "training" && (
          <Timeline
            items={[
              { children: "Initializing training environment" },
              { children: "Collecting experiences..." },
              { children: "Updating policy gradients" },
            ]}
          />
        )}

        {status === "completed" && (
          <Timeline
            items={[
              { children: "Training completed", color: "green" },
              { children: `Final avg reward: ${avgReward.toFixed(4)}` },
              { children: "Policy updated successfully" },
            ]}
          />
        )}

        {status === "failed" && (
          <Timeline
            items={[
              { children: "Training failed", color: "red" },
              { children: "Check logs for details" },
            ]}
          />
        )}
      </div>
    </Card>
  );
}
