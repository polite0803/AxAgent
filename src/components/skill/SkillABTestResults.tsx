// SPDX-License-Identifier: AGPL-3.0-only

import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { ABTestResult as ABTestResultType } from "@/stores/feature/evolutionStore";
import { Table, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface SkillABTestResultsProps {
  skillId: string;
}

export function SkillABTestResults({ skillId }: SkillABTestResultsProps) {
  const { t } = useTranslation();
  const getABTestResults = useEvolutionStore((s) => s.getABTestResults);
  const results: ABTestResultType[] = getABTestResults(skillId);

  if (results.length === 0) {
    return <Text type="secondary">{t("skill.abTest.noData")}</Text>;
  }

  const columns = [
    {
      title: t("skill.abTest.metric"),
      dataIndex: "metric",
      key: "metric",
      width: 160,
    },
    {
      title: t("skill.abTest.versionA"),
      dataIndex: "valueA",
      key: "valueA",
      width: 100,
      align: "right" as const,
    },
    {
      title: t("skill.abTest.versionB"),
      dataIndex: "valueB",
      key: "valueB",
      width: 100,
      align: "right" as const,
    },
    {
      title: t("skill.abTest.change"),
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
      title: t("skill.abTest.winner"),
      dataIndex: "winner",
      key: "winner",
      width: 80,
      render: (winner: string) => (
        <Tag color={winner === "A" ? "blue" : winner === "B" ? "green" : "default"}>
          {winner === "A"
            ? t("skill.abTest.versionA")
            : winner === "B"
            ? t("skill.abTest.versionB")
            : t("skill.abTest.tie")}
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
        rowKey="key"
        pagination={false}
        size="small"
        style={{ marginBottom: 12 }}
      />
      <Text type="secondary">
        {t("skill.abTest.conclusion", { winCountA, winCountB })}
        {winCountA > winCountB
          ? t("skill.abTest.recommendA")
          : winCountB > winCountA
          ? t("skill.abTest.recommendB")
          : t("skill.abTest.noDifference")}
      </Text>
    </div>
  );
}
