// SPDX-License-Identifier: AGPL-3.0-only

import { DatasetManager } from "@/components/fine-tune/DatasetManager";
import { LoRAConfig } from "@/components/fine-tune/LoRAConfig";
import { TrainingJobList } from "@/components/fine-tune/TrainingJobList";
import { useFineTuneStore } from "@/stores/devtools/fineTuneStore";
import { Tabs, theme } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

export function FineTunePage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const fetchDatasets = useFineTuneStore((s) => s.fetchDatasets);
  const fetchTrainingJobs = useFineTuneStore((s) => s.fetchTrainingJobs);
  const fetchBaseModels = useFineTuneStore((s) => s.fetchBaseModels);
  const fetchLoRAAdapters = useFineTuneStore((s) => s.fetchLoRAAdapters);
  const fetchTrainingStats = useFineTuneStore((s) => s.fetchTrainingStats);
  const stats = useFineTuneStore((s) => s.stats);

  useEffect(() => {
    fetchDatasets();
    fetchTrainingJobs();
    fetchBaseModels();
    fetchLoRAAdapters();
    fetchTrainingStats();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const tabItems = [
    {
      key: "datasets",
      label: t("fineTune.tab.dataset"),
      children: <DatasetManager />,
    },
    {
      key: "jobs",
      label: `${t("fineTune.tab.training")}${stats ? ` (${stats.completed_jobs}/${stats.total_jobs})` : ""}`,
      children: <TrainingJobList />,
    },
    {
      key: "lora",
      label: t("fineTune.tab.loraConfig"),
      children: <LoRAConfig />,
    },
  ];

  return (
    <div className="dev-page">
      <h2 style={{ margin: "0 0 16px", fontSize: 20, fontWeight: 600 }}>
        {t("fineTune.title")}
      </h2>
      {stats && (
        <div
          style={{
            display: "flex",
            gap: 16,
            marginBottom: 16,
            padding: "8px 16px",
            borderRadius: 8,
            background: "var(--color-bg-tertiary)",
            fontSize: 13,
          }}
        >
          <span>
            {t("fineTune.stats.total")}: <b>{stats.total_jobs}</b>
          </span>
          <span style={{ color: token.colorSuccess }}>
            {t("fineTune.stats.completed")}: <b>{stats.completed_jobs}</b>
          </span>
          <span style={{ color: token.colorPrimary }}>
            {t("fineTune.stats.running")}: <b>{stats.running_jobs}</b>
          </span>
          <span style={{ color: token.colorError }}>
            {t("fineTune.stats.failed")}: <b>{stats.failed_jobs}</b>
          </span>
        </div>
      )}
      <Tabs items={tabItems} />
    </div>
  );
}
