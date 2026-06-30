// SPDX-License-Identifier: AGPL-3.0-only

import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { ABTestResult as ABTestResultType } from "@/stores/feature/evolutionStore";
import { Table, Tag, Typography } from "antd";

const { Text } = Typography;

interface SkillABTestResultsProps {
  skillId: string;
}

export default function SkillABTestResults({ skillId }: SkillABTestResultsProps) {
  const getABTestResults = useEvolutionStore((s) => s.getABTestResults);
  const results: ABTestResultType[] = getABTestResults(skillId);

  if (results.length === 0) {
    return <Text type="secondary">暂无 A/B 测试数据</Text>;
  }

  const columns = [
    {
      title: "指标",
      dataIndex: "metric",
      key: "metric",
      width: 160,
    },
    {
      title: "版本 A",
      dataIndex: "valueA",
      key: "valueA",
      width: 100,
      align: "right" as const,
    },
    {
      title: "版本 B",
      dataIndex: "valueB",
      key: "valueB",
      width: 100,
      align: "right" as const,
    },
    {
      title: "变化",
      dataIndex: "change",
      key: "change",
      width: 100,
      align: "right" as const,
      render: (change: number) => {
        const color = change > 0 ? "#52c41a" : change < 0 ? "#ff4d4f" : "#888";
        const arrow = change > 0 ? "↑" : change < 0 ? "↓" : "→";
        return <Text style={{ color }}>{arrow} {Math.abs(change).toFixed(1)}%</Text>;
      },
    },
    {
      title: "优胜",
      dataIndex: "winner",
      key: "winner",
      width: 80,
      render: (winner: string) => (
        <Tag color={winner === "A" ? "blue" : winner === "B" ? "green" : "default"}>
          {winner === "A" ? "版本 A" : winner === "B" ? "版本 B" : "平局"}
        </Tag>
      ),
    },
  ];

  const winCountA = results.filter((r) => r.winner === "A").length;
  const winCountB = results.filter((r) => r.winner === "B").length;

  return (
    <div>
      <Table
        dataSource={results.map((r, i) => ({ ...r, key: i }))}
        columns={columns}
        pagination={false}
        size="small"
        style={{ marginBottom: 12 }}
      />
      <Text type="secondary">
        结论：版本 A 胜出 {winCountA} 项，版本 B 胜出 {winCountB} 项。
        {winCountA > winCountB ? " 推荐采用版本 A。" : winCountB > winCountA ? " 推荐采用版本 B。" : " 两个版本无明显差异。"}
      </Text>
    </div>
  );
}
