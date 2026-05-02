import { App, Button, Input, Modal, Popconfirm, Switch, Table, Tag, Typography } from "antd";
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

interface CronJob {
  id: string;
  name: string;
  schedule: string;
  prompt: string;
  platform: string | null;
  enabled_toolsets: string[] | null;
  enabled: boolean;
  last_run_at: number | null;
  next_run_at: number | null;
}

interface CronManagerProps {
  jobs: CronJob[];
  onAdd: (job: { name: string; schedule: string; prompt: string; platform?: string }) => void;
  onDelete: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
}

export function CronManager({ jobs, onAdd, onDelete, onToggle }: CronManagerProps) {
  const { message } = App.useApp();
  const [modalOpen, setModalOpen] = useState(false);
  const [name, setName] = useState("");
  const [schedule, setSchedule] = useState("");
  const [prompt, setPrompt] = useState("");
  const [platform, setPlatform] = useState("");

  const handleAdd = () => {
    if (!name.trim() || !schedule.trim() || !prompt.trim()) {
      message.error("Name, schedule, and prompt are required");
      return;
    }
    onAdd({
      name: name.trim(),
      schedule: schedule.trim(),
      prompt: prompt.trim(),
      platform: platform.trim() || undefined,
    });
    setName("");
    setSchedule("");
    setPrompt("");
    setPlatform("");
    setModalOpen(false);
    message.success("Cron job added");
  };

  const columns = [
    {
      title: "Name",
      dataIndex: "name",
      key: "name",
      render: (name: string, record: CronJob) => (
        <div>
          <div className="font-medium">{name}</div>
          <Text type="secondary" style={{ fontSize: 12 }}>{record.schedule}</Text>
        </div>
      ),
    },
    {
      title: "Prompt",
      dataIndex: "prompt",
      key: "prompt",
      ellipsis: true,
      width: 300,
    },
    {
      title: "Platform",
      dataIndex: "platform",
      key: "platform",
      render: (p: string | null) => p ? <Tag>{p}</Tag> : <Tag color="default">default</Tag>,
    },
    {
      title: "Last Run",
      dataIndex: "last_run_at",
      key: "last_run_at",
      render: (t: number | null) => t ? new Date(t).toLocaleString() : <Text type="secondary">Never</Text>,
    },
    {
      title: "Status",
      dataIndex: "enabled",
      key: "enabled",
      render: (enabled: boolean, record: CronJob) => (
        <Switch
          checked={enabled}
          onChange={(v) => onToggle(record.id, v)}
          size="small"
        />
      ),
    },
    {
      title: "",
      key: "actions",
      width: 60,
      render: (_: unknown, record: CronJob) => (
        <Popconfirm
          title="Delete this cron job?"
          onConfirm={() => onDelete(record.id)}
          okText="Delete"
          cancelText="Cancel"
        >
          <Button type="text" danger size="small" icon={<Trash2 size={14} />} />
        </Popconfirm>
      ),
    },
  ];

  return (
    <div className="p-6">
      <SettingsGroup title="Cron Jobs">
        <div style={{ marginBottom: 12 }}>
          <Button type="primary" icon={<Plus size={14} />} onClick={() => setModalOpen(true)}>
            Add Cron Job
          </Button>
        </div>
        <Table
          dataSource={jobs}
          columns={columns}
          rowKey="id"
          size="small"
          pagination={false}
          locale={{ emptyText: "No cron jobs configured" }}
        />
      </SettingsGroup>

      <Modal
        title="Add Cron Job"
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleAdd}
        okText="Add Job"
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div>
            <Text type="secondary">Name</Text>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Daily summary"
            />
          </div>
          <div>
            <Text type="secondary">Schedule (cron expression)</Text>
            <Input
              value={schedule}
              onChange={(e) => setSchedule(e.target.value)}
              placeholder="0 9 * * *"
            />
            <Text type="secondary" style={{ fontSize: 12 }}>
              Examples: "0 9 * * *" (daily 9am), "*/30 * * * *" (every 30 min)
            </Text>
          </div>
          <div>
            <Text type="secondary">Prompt</Text>
            <Input.TextArea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Generate a daily summary of my inbox"
              rows={3}
            />
          </div>
          <div>
            <Text type="secondary">Platform (optional)</Text>
            <Input
              value={platform}
              onChange={(e) => setPlatform(e.target.value)}
              placeholder="telegram / discord / web"
            />
          </div>
        </div>
      </Modal>
    </div>
  );
}
