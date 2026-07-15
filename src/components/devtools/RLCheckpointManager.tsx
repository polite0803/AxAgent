// SPDX-License-Identifier: AGPL-3.0-only

import { useRlTrainingStore } from "@/stores/feature/rlTrainingStore";
import type { CheckpointInfo } from "@/stores/feature/rlTrainingStore";
import { Button, Input, Modal, Space, Table, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export function RLCheckpointManager() {
  const { t } = useTranslation();
  const checkpoints = useRlTrainingStore((s) => s.checkpoints);
  const saveCheckpoint = useRlTrainingStore((s) => s.saveCheckpoint);
  const loadCheckpoint = useRlTrainingStore((s) => s.loadCheckpoint);
  const deleteCheckpoint = useRlTrainingStore((s) => s.deleteCheckpoint);
  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [checkpointName, setCheckpointName] = useState("");
  const [loadingId, setLoadingId] = useState<string | null>(null);

  const handleSave = async () => {
    if (!checkpointName.trim()) { return; }
    await saveCheckpoint(checkpointName.trim());
    setCheckpointName("");
    setSaveModalOpen(false);
  };

  const handleLoad = async (id: string) => {
    setLoadingId(id);
    await loadCheckpoint(id);
    setLoadingId(null);
  };

  const columns = [
    { title: t("rl.checkpoints.name"), dataIndex: "name", key: "name" },
    { title: t("rl.checkpoints.step"), dataIndex: "step", key: "step", width: 80, align: "right" as const },
    {
      title: t("rl.checkpoints.loss"),
      dataIndex: "loss",
      key: "loss",
      width: 100,
      align: "right" as const,
      render: (v: number) => v.toFixed(4),
    },
    {
      title: t("rl.checkpoints.reward"),
      dataIndex: "reward",
      key: "reward",
      width: 100,
      align: "right" as const,
      render: (v: number) => v.toFixed(4),
    },
    {
      title: t("rl.checkpoints.time"),
      dataIndex: "timestamp",
      key: "timestamp",
      width: 160,
      render: (v: number) => new Date(v).toLocaleString(),
    },
    {
      title: t("rl.checkpoints.action"),
      key: "actions",
      width: 160,
      render: (_: unknown, record: CheckpointInfo) => (
        <Space>
          <Button
            size="small"
            type="link"
            loading={loadingId === record.id}
            onClick={() => handleLoad(record.id)}
          >
            {t("rl.checkpoints.load")}
          </Button>
          <Button size="small" type="link" danger onClick={() => deleteCheckpoint(record.id)}>
            {t("rl.checkpoints.delete")}
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 12, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <Text strong>{t("rl.checkpoints.title")}</Text>
        <Button type="primary" size="small" onClick={() => setSaveModalOpen(true)}>
          {t("rl.checkpoints.save")}
        </Button>
      </div>

      <Table
        dataSource={checkpoints.map((c) => ({ ...c, key: c.id }))}
        columns={columns}
        pagination={false}
        size="small"
        locale={{ emptyText: t("rl.checkpoints.empty") }}
      />

      <Modal
        title={t("rl.checkpoints.saveModal")}
        open={saveModalOpen}
        onCancel={() => setSaveModalOpen(false)}
        onOk={handleSave}
        okText={t("common.save")}
      >
        <Input
          placeholder={t("rl.checkpoints.namePlaceholder")}
          value={checkpointName}
          onChange={(e) => setCheckpointName(e.target.value)}
        />
      </Modal>
    </div>
  );
}
