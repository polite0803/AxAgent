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
    { title: "名称", dataIndex: "name", key: "name" },
    { title: "步数", dataIndex: "step", key: "step", width: 80, align: "right" as const },
    {
      title: "损失",
      dataIndex: "loss",
      key: "loss",
      width: 100,
      align: "right" as const,
      render: (v: number) => v.toFixed(4),
    },
    {
      title: "奖励",
      dataIndex: "reward",
      key: "reward",
      width: 100,
      align: "right" as const,
      render: (v: number) => v.toFixed(4),
    },
    {
      title: "时间",
      dataIndex: "timestamp",
      key: "timestamp",
      width: 160,
      render: (v: number) => new Date(v).toLocaleString(),
    },
    {
      title: "操作",
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
            加载
          </Button>
          <Button size="small" type="link" danger>
            删除
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div style={{ marginBottom: 12, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <Text strong>{t("rl.checkpoints.title", "检查点列表")}</Text>
        <Button type="primary" size="small" onClick={() => setSaveModalOpen(true)}>
          {t("rl.checkpoints.save", "保存检查点")}
        </Button>
      </div>

      <Table
        dataSource={checkpoints.map((c) => ({ ...c, key: c.id }))}
        columns={columns}
        pagination={false}
        size="small"
        locale={{ emptyText: t("rl.checkpoints.empty", "暂无检查点") }}
      />

      <Modal
        title={t("rl.checkpoints.saveModal", "保存检查点")}
        open={saveModalOpen}
        onCancel={() => setSaveModalOpen(false)}
        onOk={handleSave}
        okText={t("common.save", "保存")}
      >
        <Input
          placeholder={t("rl.checkpoints.namePlaceholder", "输入检查点名称...")}
          value={checkpointName}
          onChange={(e) => setCheckpointName(e.target.value)}
        />
      </Modal>
    </div>
  );
}
