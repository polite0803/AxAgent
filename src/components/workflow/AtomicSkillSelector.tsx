import { PlusOutlined } from "@ant-design/icons";
import { Button, Input, Modal, Space, Table, Tag } from "antd";
import React, { useEffect, useState } from "react";
import { useAtomicSkillStore } from "../../stores/feature/atomicSkillStore";
import type { AtomicSkill } from "../../types";

interface AtomicSkillSelectorProps {
  visible: boolean;
  onSelect: (skill: AtomicSkill) => void;
  onClose: () => void;
  onCreateNew?: () => void;
}

const ENTRY_TYPE_COLORS: Record<string, string> = {
  builtin: "blue",
  mcp: "purple",
  local: "green",
  plugin: "orange",
};

export const AtomicSkillSelector: React.FC<AtomicSkillSelectorProps> = ({
  visible,
  onSelect,
  onClose,
  onCreateNew,
}) => {
  const { skills, loading, loadSkills } = useAtomicSkillStore();
  const [searchText, setSearchText] = useState("");

  useEffect(() => {
    if (visible) { loadSkills(); }
  }, [visible, loadSkills]);

  const filteredSkills = skills.filter((s) =>
    !searchText || s.name.toLowerCase().includes(searchText.toLowerCase())
    || s.description.toLowerCase().includes(searchText.toLowerCase())
  );

  return (
    <Modal
      title="选择原子Skill"
      open={visible}
      onCancel={onClose}
      footer={null}
      width={560}
    >
      <Space style={{ marginBottom: 12, width: "100%" }} direction="vertical">
        <Space>
          <Input
            placeholder="搜索..."
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 300 }}
            allowClear
          />
          {onCreateNew && <Button icon={<PlusOutlined />} onClick={onCreateNew}>新建</Button>}
        </Space>
      </Space>

      <Table
        dataSource={filteredSkills}
        columns={[
          { title: "名称", dataIndex: "name", key: "name" },
          { title: "描述", dataIndex: "description", key: "description", ellipsis: true },
          {
            title: "入口类型",
            dataIndex: "entry_type",
            key: "entry_type",
            width: 80,
            render: (t: string) => <Tag color={ENTRY_TYPE_COLORS[t] || "default"}>{t}</Tag>,
          },
          {
            title: "操作",
            key: "action",
            width: 70,
            render: (_: unknown, record: AtomicSkill) => (
              <Button
                size="small"
                type="link"
                onClick={() => {
                  onSelect(record);
                  onClose();
                }}
              >
                选择
              </Button>
            ),
          },
        ]}
        rowKey="id"
        loading={loading}
        size="small"
        pagination={{ pageSize: 10 }}
      />
    </Modal>
  );
};
