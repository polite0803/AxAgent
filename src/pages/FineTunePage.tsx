import { DatasetManager } from "@/components/fine-tune/DatasetManager";
import { LoRAConfig } from "@/components/fine-tune/LoRAConfig";
import { TrainingJobList } from "@/components/fine-tune/TrainingJobList";
import { useFineTuneStore } from "@/stores/devtools/fineTuneStore";
import { Tabs } from "antd";
import { useEffect } from "react";

export default function FineTunePage() {
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
  }, []);

  const tabItems = [
    {
      key: "datasets",
      label: "数据集",
      children: <DatasetManager />,
    },
    {
      key: "jobs",
      label: `训练任务${stats ? ` (${stats.completed_jobs}/${stats.total_jobs})` : ""}`,
      children: <TrainingJobList />,
    },
    {
      key: "lora",
      label: "LoRA 配置",
      children: <LoRAConfig />,
    },
  ];

  return (
    <div style={{ padding: "16px 24px", maxWidth: 1200, margin: "0 auto" }}>
      <h2 style={{ margin: "0 0 16px", fontSize: 20, fontWeight: 600 }}>模型微调</h2>
      {stats && (
        <div style={{
          display: "flex", gap: 16, marginBottom: 16,
          padding: "8px 16px", borderRadius: 8,
          background: "var(--color-bg-tertiary)",
          fontSize: 13,
        }}>
          <span>总任务: <b>{stats.total_jobs}</b></span>
          <span style={{ color: "#52c41a" }}>完成: <b>{stats.completed_jobs}</b></span>
          <span style={{ color: "#1890ff" }}>运行中: <b>{stats.running_jobs}</b></span>
          <span style={{ color: "#ff4d4f" }}>失败: <b>{stats.failed_jobs}</b></span>
        </div>
      )}
      <Tabs items={tabItems} />
    </div>
  );
}
