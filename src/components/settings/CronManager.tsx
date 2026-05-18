import { App, Button, Input, Modal, Popconfirm, Switch, Table, Tag, Typography } from "antd";
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
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
  onAdd: (job: {
    name: string;
    schedule: string;
    prompt: string;
    platform?: string;
  }) => void;
  onDelete: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
}

export function CronManager({
  jobs,
  onAdd,
  onDelete,
  onToggle,
}: CronManagerProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [modalOpen, setModalOpen] = useState(false);
  const [name, setName] = useState("");
  const [schedule, setSchedule] = useState("");
  const [prompt, setPrompt] = useState("");
  const [platform, setPlatform] = useState("");

  const handleAdd = () => {
    if (!name.trim() || !schedule.trim() || !prompt.trim()) {
      message.error(t("settings.cron.validationRequired"));
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
    message.success(t("settings.cron.added"));
  };

  const columns = [
    {
      title: "Name",
      dataIndex: "name",
      key: "name",
      render: (name: string, record: CronJob) => (
        <div>
          <div className="font-medium">{name}</div>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {record.schedule}
          </Text>
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
      render: (lastRun: number | null) =>
        lastRun
          ? (
            new Date(lastRun).toLocaleString()
          )
          : <Text type="secondary">{t("cronManager.never")}</Text>,
    },
    {
      title: "Status",
      dataIndex: "enabled",
      key: "enabled",
      render: (enabled: boolean, record: CronJob) => (
        <Switch
          id="cron-manager-switch-45"
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
          okText={t("cronManager.delete")}
          cancelText="Cancel"
        >
          <Button type="text" danger size="small" icon={<Trash2 size={14} />} />
        </Popconfirm>
      ),
    },
  ];

  return (
    <div className="p-6">
      <SettingsGroup title={t("cronManager.title")}>
        <div style={{ marginBottom: 12 }}>
          <Button
            type="primary"
            icon={<Plus size={14} />}
            onClick={() => setModalOpen(true)}
          >
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
        title={t("cronManager.addJob")}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleAdd}
        okText={t("cronManager.addJob")}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div>
            <Text type="secondary">{t("cronManager.name")}</Text>
            <Input
              id="cron-manager-input-46"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t("settings.cron.namePlaceholder")}
            />
          </div>
          <div>
            <Text type="secondary">{t("settings.cron.schedule")}</Text>
            <Input
              id="cron-manager-input-47"
              value={schedule}
              onChange={(e) => setSchedule(e.target.value)}
              placeholder={t("settings.cron.schedulePlaceholder")}
            />
            <Text type="secondary" style={{ fontSize: 12 }}>
              Examples: "0 9 * * *" (daily 9am), "*/30 * * * *" (every 30 min)
            </Text>
          </div>
          <div>
            <Text type="secondary">{t("cronManager.prompt")}</Text>
            <Input.TextArea
              id="cron-manager-input-textarea-48"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder={t("settings.cron.promptPlaceholder")}
              rows={3}
            />
          </div>
          <div>
            <Text type="secondary">{t("settings.cron.platform")}</Text>
            <Input
              id="cron-manager-input-49"
              value={platform}
              onChange={(e) => setPlatform(e.target.value)}
              placeholder={t("settings.cron.platformPlaceholder")}
            />
          </div>
        </div>
      </Modal>
    </div>
  );
}
