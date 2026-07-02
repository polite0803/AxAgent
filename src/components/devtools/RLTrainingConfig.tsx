// SPDX-License-Identifier: AGPL-3.0-only

import type { ConfigField } from "@/components/settings/EngineConfigForm";
import EngineConfigForm from "@/components/settings/EngineConfigForm";
import { useRlTrainingStore } from "@/stores/feature/rlTrainingStore";
import type { RLTrainingConfig } from "@/stores/feature/rlTrainingStore";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

export function RLTrainingConfig() {
  const { t } = useTranslation();
  const config = useRlTrainingStore((s) => s.config);
  const startTraining = useRlTrainingStore((s) => s.startTraining);

  const fields: ConfigField[] = useMemo(() => [
    {
      key: "algorithm",
      label: t("rl.config.algorithm"),
      type: "select",
      options: [
        { label: "PPO", value: "ppo" },
        { label: "GRPO", value: "grpo" },
        { label: "DPO", value: "dpo" },
        { label: "RLHF", value: "rlhf" },
      ],
    },
    { key: "learningRate", label: t("rl.config.learningRate"), type: "number", min: 1e-6, max: 1e-2, step: 1e-5 },
    { key: "batchSize", label: t("rl.config.batchSize"), type: "number", min: 1, max: 512, step: 1 },
    { key: "epochs", label: t("rl.config.epochs"), type: "number", min: 1, max: 1000, step: 1 },
    { key: "maxSteps", label: t("rl.config.maxSteps"), type: "number", min: 100, max: 100000, step: 100 },
  ], [t]);

  const handleSave = useCallback(
    (updatedConfig: Record<string, unknown>) => {
      // Validate and cast to RLTrainingConfig
      const validated: RLTrainingConfig = {
        algorithm: (updatedConfig.algorithm as RLTrainingConfig["algorithm"]) ?? "ppo",
        learningRate: Number(updatedConfig.learningRate) || 1e-5,
        batchSize: Number(updatedConfig.batchSize) || 64,
        epochs: Number(updatedConfig.epochs) || 10,
        maxSteps: Number(updatedConfig.maxSteps) || 10000,
      };
      startTraining(validated);
    },
    [startTraining],
  );

  return (
    <EngineConfigForm
      config={
        config as unknown as Record<
          string,
          unknown
        > /* SAFE: RLTrainingConfig has no index signature; EngineConfigForm requires Record<string, unknown> for dynamic field access */
      }
      fields={fields}
      onSave={handleSave}
    />
  );
}
