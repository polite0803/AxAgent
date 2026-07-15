// SPDX-License-Identifier: AGPL-3.0-only

import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { SkillVersion } from "@/stores/feature/evolutionStore";
import { Button, Modal, Tag, theme, Timeline, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface SkillVersionTimelineProps {
  skillId: string;
}

function MetricChange({ metrics }: { metrics: Record<string, { before: number; after: number }> }) {
  return (
    <div style={{ fontSize: 12, marginTop: 4 }}>
      {Object.entries(metrics).map(([key, { before, after }]) => {
        const diff = after - before;
        const pct = before !== 0 ? ((diff / before) * 100) : 0;
        const isGood = key.toLowerCase().includes("success") || key.toLowerCase().includes("rate")
          ? diff > 0
          : key.toLowerCase().includes("token") || key.toLowerCase().includes("error")
              || key.toLowerCase().includes("time")
          ? diff < 0
          : diff > 0;
        const arrow = diff > 0 ? "↑" : diff < 0 ? "↓" : "→";
        return (
          <Tag key={key} color={isGood ? "green" : "red"} style={{ marginBottom: 4 }}>
            {key}: {arrow}
            {Math.abs(pct).toFixed(0)}%
          </Tag>
        );
      })}
    </div>
  );
}

export function SkillVersionTimeline({ skillId }: SkillVersionTimelineProps) {
  const { t } = useTranslation();
  const getSkillEvolutionHistory = useEvolutionStore((s) => s.getSkillEvolutionHistory);
  const versions: SkillVersion[] = getSkillEvolutionHistory(skillId);
  const [diffModalOpen, setDiffModalOpen] = useState(false);
  const [selectedDiff, setSelectedDiff] = useState<{ old: string; new: string } | null>(null);
  const [rollbackConfirm, setRollbackConfirm] = useState<number | null>(null);
  const { token } = theme.useToken();

  if (versions.length === 0) {
    return <Text type="secondary">{t("skill.evolution.noHistory")}</Text>;
  }

  const items = versions.map((v) => ({
    key: v.version,
    color: v.version === versions.length ? "green" : "blue",
    children: (
      <div>
        <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
          <Text strong>v{v.version}</Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {new Date(v.timestamp).toLocaleString()}
          </Text>
        </div>
        <Paragraph style={{ margin: "4px 0", fontSize: 13 }}>{v.summary}</Paragraph>
        <MetricChange metrics={v.metrics} />
        <div style={{ marginTop: 8, display: "flex", gap: 8 }}>
          {v.promptDiff && (
            <Button
              size="small"
              type="link"
              onClick={() => {
                setSelectedDiff(v.promptDiff!);
                setDiffModalOpen(true);
              }}
            >
              {t("skill.evolution.viewDiff")}
            </Button>
          )}
          {v.version > 1 && (
            <Button
              size="small"
              danger
              type="link"
              onClick={() => setRollbackConfirm(v.version)}
            >
              {t("skill.evolution.rollback")}
            </Button>
          )}
        </div>
      </div>
    ),
  }));

  return (
    <div>
      <Timeline items={items} />

      <Modal
        title={t("skillTimeline.promptDiff")}
        open={diffModalOpen}
        onCancel={() => setDiffModalOpen(false)}
        footer={null}
        width={700}
      >
        {selectedDiff && (
          <div style={{ fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)", fontSize: 12 }}>
            <div
              style={{
                background: token.colorErrorBg ?? "#fff1f0",
                padding: 8,
                borderRadius: 4,
                marginBottom: 8,
              }}
            >
              <Text type="danger" strong>
                {t("skill.evolution.oldVersion", {
                  version: versions.findIndex((v) =>
                    v.promptDiff?.old === selectedDiff.old
                  ) + 1,
                })}
              </Text>
              <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0" }}>{selectedDiff.old}</pre>
            </div>
            <div
              style={{
                background: token.colorSuccessBg ?? "#f6ffed",
                padding: 8,
                borderRadius: 4,
              }}
            >
              <Text type="success" strong>
                {t("skill.evolution.newVersion", {
                  version: versions.findIndex((v) => v.promptDiff?.new === selectedDiff.new) + 1,
                })}
              </Text>
              <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0" }}>{selectedDiff.new}</pre>
            </div>
          </div>
        )}
      </Modal>

      <Modal
        title={t("skill.evolution.confirmRollback")}
        open={rollbackConfirm !== null}
        onCancel={() => setRollbackConfirm(null)}
        onOk={() => {
          setRollbackConfirm(null);
        }}
        okText={t("skill.evolution.confirmRollback")}
        okButtonProps={{ danger: true }}
      >
        <Paragraph>
          {t("skill.evolution.rollbackConfirmMessage", { version: rollbackConfirm })}
        </Paragraph>
      </Modal>
    </div>
  );
}
