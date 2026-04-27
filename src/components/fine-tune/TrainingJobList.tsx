import { useEffect } from "react";
import { useFineTuneStore } from "@/stores/devtools/fineTuneStore";
import { Card, Table, Button, Space, Tag, Progress, Badge } from "antd";
import { PlayCircleOutlined, PauseCircleOutlined, DeleteOutlined } from "@ant-design/icons";

const { Column } = Table;

const getStatusColor = (status: string) => {
  switch (status) {
    case "Pending":
      return "default";
    case "Preparing":
      return "processing";
    case "Training":
      return "processing";
    case "Validating":
      return "processing";
    case "Completed":
      return "success";
    case "Failed":
      return "error";
    case "Cancelled":
      return "warning";
    default:
      return "default";
  }
};

export function TrainingJobList() {
  const {
    trainingJobs,
    stats,
    isLoading,
    fetchTrainingJobs,
    fetchTrainingStats,
    startTrainingJob,
    cancelTrainingJob,
    deleteTrainingJob,
  } = useFineTuneStore();

  useEffect(() => {
    fetchTrainingJobs();
    fetchTrainingStats();
  }, [fetchTrainingJobs, fetchTrainingStats]);

  const handleStartJob = async (id: string) => {
    await startTrainingJob(id);
  };

  const handleCancelJob = async (id: string) => {
    await cancelTrainingJob(id);
  };

  const handleDeleteJob = async (id: string) => {
    await deleteTrainingJob(id);
  };

  return (
    <div className="p-4">
      <Card title="Training Jobs">
        {stats && (
          <div className="grid grid-cols-4 gap-4 mb-4">
            <Card size="small">
              <div className="text-2xl font-bold">{stats.total_jobs}</div>
              <div className="text-gray-500">Total Jobs</div>
            </Card>
            <Card size="small">
              <div className="text-2xl font-bold">{stats.running_jobs}</div>
              <div className="text-gray-500">Running</div>
            </Card>
            <Card size="small">
              <div className="text-2xl font-bold">{stats.completed_jobs}</div>
              <div className="text-gray-500">Completed</div>
            </Card>
            <Card size="small">
              <div className="text-2xl font-bold">{stats.failed_jobs}</div>
              <div className="text-gray-500">Failed</div>
            </Card>
          </div>
        )}

        <Table
          dataSource={trainingJobs}
          rowKey="id"
          loading={isLoading}
          pagination={false}
        >
          <Column title="ID" dataIndex="id" key="id" width={200} ellipsis />
          <Column
            title="Status"
            dataIndex="status"
            key="status"
            render={(status: string) => (
              <Badge status={getStatusColor(status) as any} text={status} />
            )}
          />
          <Column title="Base Model" dataIndex="base_model" key="base_model" />
          <Column title="Dataset ID" dataIndex="dataset_id" key="dataset_id" />
          <Column
            title="Progress"
            key="progress"
            render={(_: unknown, record: { progress_percent: number }) => (
              <Progress percent={Math.round(record.progress_percent)} size="small" />
            )}
          />
          <Column
            title="Loss"
            dataIndex="current_loss"
            key="current_loss"
            render={(loss: number) => (loss > 0 ? loss.toFixed(4) : "-")}
          />
          <Column
            title="LoRA Output"
            dataIndex="output_lora"
            key="output_lora"
            render={(lora: string | null) => (lora ? <Tag color="green">Ready</Tag> : "-")}
          />
          <Column
            title="Action"
            key="action"
            render={(_: unknown, record: { id: string; status: string }) => {
              const isRunning =
                record.status === "Training" ||
                record.status === "Preparing" ||
                record.status === "Validating";

              return (
                <Space>
                  {!isRunning && record.status === "Pending" && (
                    <Button
                      size="small"
                      type="primary"
                      icon={<PlayCircleOutlined />}
                      onClick={() => handleStartJob(record.id)}
                    >
                      Start
                    </Button>
                  )}
                  {isRunning && (
                    <Button
                      size="small"
                      danger
                      icon={<PauseCircleOutlined />}
                      onClick={() => handleCancelJob(record.id)}
                    >
                      Cancel
                    </Button>
                  )}
                  {!isRunning && (
                    <Button
                      size="small"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={() => handleDeleteJob(record.id)}
                    />
                  )}
                </Space>
              );
            }}
          />
        </Table>
      </Card>
    </div>
  );
}
