import { Card, Tag, Typography } from "antd";
import { Clock, CheckCircle, XCircle } from "lucide-react";

const { Text, Paragraph } = Typography;

interface CronResultMessageProps {
  jobName: string;
  schedule: string;
  result: string;
  success: boolean;
  timestamp: number;
  platform?: string;
}

export function CronResultMessage({
  jobName,
  schedule,
  result,
  success,
  timestamp,
  platform,
}: CronResultMessageProps) {
  return (
    <Card
      size="small"
      style={{ maxWidth: 500 }}
      title={
        <div className="flex items-center gap-2">
          <Clock size={14} />
          <span>Cron: {jobName}</span>
          {success ? (
            <Tag color="success" icon={<CheckCircle size={12} />}>Success</Tag>
          ) : (
            <Tag color="error" icon={<XCircle size={12} />}>Failed</Tag>
          )}
        </div>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <div className="flex items-center justify-between">
          <Text type="secondary">Schedule</Text>
          <Text code>{schedule}</Text>
        </div>
        {platform && (
          <div className="flex items-center justify-between">
            <Text type="secondary">Platform</Text>
            <Tag>{platform}</Tag>
          </div>
        )}
        <div className="flex items-center justify-between">
          <Text type="secondary">Time</Text>
          <Text>{new Date(timestamp).toLocaleString()}</Text>
        </div>
        <Paragraph
          ellipsis={{ rows: 3, expandable: true }}
          style={{ marginTop: 8, marginBottom: 0 }}
        >
          {result}
        </Paragraph>
      </div>
    </Card>
  );
}
