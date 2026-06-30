// SPDX-License-Identifier: AGPL-3.0-only

import SkillABTestResults from "@/components/skill/SkillABTestResults";
import SkillVersionTimeline from "@/components/skill/SkillVersionTimeline";
import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { SkillVersion } from "@/stores/feature/evolutionStore";
import { Statistic, Typography } from "antd";
import { useMemo } from "react";

const { Text } = Typography;

interface SkillEvolutionViewerProps {
  skillId: string;
}

export default function SkillEvolutionViewer({ skillId }: SkillEvolutionViewerProps) {
  const getSkillEvolutionHistory = useEvolutionStore((s) => s.getSkillEvolutionHistory);
  const getABTestResults = useEvolutionStore((s) => s.getABTestResults);

  const versions: SkillVersion[] = useMemo(
    () => getSkillEvolutionHistory(skillId),
    [skillId, getSkillEvolutionHistory],
  );

  const abResults = useMemo(
    () => getABTestResults(skillId),
    [skillId, getABTestResults],
  );

  const currentVersion = versions.length > 0 ? versions[0].version : 0;
  const totalEvolutions = versions.length;
  const latestEvoTime = versions.length > 0 ? versions[0].timestamp : 0;

  return (
    <div>
      {/* Summary bar */}
      <div
        style={{
          display: "flex",
          gap: 24,
          marginBottom: 24,
          padding: "12px 16px",
          background: "var(--ant-color-bg-container, #fff)",
          borderRadius: 8,
          border: "1px solid var(--ant-color-border-secondary, #f0f0f0)",
        }}
      >
        <Statistic title="当前版本" value={`v${currentVersion}`} />
        <Statistic title="总进化次数" value={totalEvolutions} />
        <Statistic
          title="最近进化"
          value={latestEvoTime > 0 ? new Date(latestEvoTime).toLocaleDateString() : "无"}
        />
      </div>

      {/* Timeline */}
      {versions.length > 0
        ? <SkillVersionTimeline skillId={skillId} />
        : (
          <Text type="secondary" style={{ display: "block", marginBottom: 24 }}>
            该技能暂无进化历史
          </Text>
        )}

      {/* A/B Test Results */}
      {abResults.length > 0 && (
        <div style={{ marginTop: 24 }}>
          <Text strong style={{ display: "block", marginBottom: 12 }}>A/B 测试结果</Text>
          <SkillABTestResults skillId={skillId} />
        </div>
      )}
    </div>
  );
}
