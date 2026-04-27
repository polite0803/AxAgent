import { PlusOutlined, ReloadOutlined, SearchOutlined } from "@ant-design/icons";
import { Button, Empty, Input, Select, Space, Switch, Table, Tag, Typography } from "antd";
import React, { useEffect, useState } from "react";
import { useAtomicSkillStore } from "../../stores/feature/atomicSkillStore";
import type { AtomicSkill } from "../../types";

const { Text } = Typography;

interface AtomicSkillListProps {
  onEdit?: (skill: AtomicSkill) => void;
  onCreate?: () => void;
}

const ENTRY_TYPE_COLORS: Record<string, string> = {
  builtin: "blue",
  mcp: "purple",
  local: "green",
  plugin: "orange",
};

const ENTRY_TYPE_LABELS: Record<string, string> = {
  builtin: "内置",
  mcp: "MCP",
  local: "本地",
  plugin: "插件",
};

export const AtomicSkillList: React.FC<AtomicSkillListProps> = ({ onEdit, onCreate }) => {
  const { skills, loading, loadSkills, toggleSkill, filter, setFilter } = useAtomicSkillStore();
  const [searchText, setSearchText] = useState("");

  useEffect(() => {
    loadSkills();
  }, [loadSkills]);

  const filteredSkills = skills.filter((s) =>
    !searchText || s.name.toLowerCase().includes(searchText.toLowerCase())
    || s.description.toLowerCase().includes(searchText.toLowerCase())
  );

  const columns = [
    {
      title: "名称",
      dataIndex: "name",
      key: "name",
      render: (name: string, record: AtomicSkill) => <a onClick={() => onEdit?.(record)}>{name}</a>,
    },
    {
      title: "描述",
      dataIndex: "description",
      key: "description",
      ellipsis: true,
      render: (desc: string) => <Text type="secondary">{desc}</Text>,
    },
    {
      title: "分类",
      dataIndex: "category",
      key: "category",
      width: 100,
      render: (category: string) => <Tag>{category}</Tag>,
    },
    {
      title: "入口类型",
      dataIndex: "entry_type",
      key: "entry_type",
      width: 90,
      render: (entryType: string) => (
        <Tag color={ENTRY_TYPE_COLORS[entryType] || "default"}>
          {ENTRY_TYPE_LABELS[entryType] || entryType}
        </Tag>
      ),
    },
    {
      title: "来源",
      dataIndex: "source",
      key: "source",
      width: 100,
      render: (source: string) => (
        <Tag color={source === "auto-generated" ? "volcano" : "cyan"}>
          {source === "auto-generated" ? "自动生成" : "原子"}
        </Tag>
      ),
    },
    {
      title: "启用",
      dataIndex: "enabled",
      key: "enabled",
      width: 70,
      render: (enabled: boolean, record: AtomicSkill) => (
        <Switch
          size="small"
          checked={enabled}
          onChange={(checked) => toggleSkill(record.id, checked)}
        />
      ),
    },
  ];

  return (
    <div style={{ padding: "0 16px" }}>
      <Space style={{ marginBottom: 16 }} wrap>
        <Input
          placeholder="搜索原子Skill..."
          prefix={<SearchOutlined />}
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          style={{ width: 240 }}
          allowClear
        />
        <Select
          placeholder="分类"
          allowClear
          style={{ width: 120 }}
          value={filter.category}
          onChange={(v) => {
            setFilter({ ...filter, category: v });
            loadSkills({ ...filter, category: v });
          }}
          options={[
            { label: "通用", value: "general" },
            { label: "分解", value: "decomposed" },
            { label: "未分解", value: "undecomposed" },
          ]}
        />
        <Select
          placeholder="来源"
          allowClear
          style={{ width: 120 }}
          value={filter.source}
          onChange={(v) => {
            setFilter({ ...filter, source: v });
            loadSkills({ ...filter, source: v });
          }}
          options={[
            { label: "原子", value: "atomic" },
            { label: "自动生成", value: "auto-generated" },
          ]}
        />
        <Button icon={<ReloadOutlined />} onClick={() => loadSkills()}>刷新</Button>
        {onCreate && <Button type="primary" icon={<PlusOutlined />} onClick={onCreate}>新建</Button>}
      </Space>

      {filteredSkills.length === 0 && !loading ? <Empty description="暂无原子Skill" /> : (
        <Table
          dataSource={filteredSkills}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={{ pageSize: 20, showSizeChanger: false }}
        />
      )}
    </div>
  );
};
