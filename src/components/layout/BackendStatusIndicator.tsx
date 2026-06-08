import { initBackendStatusListeners, useBackendStatusStore } from "@/stores/shared/backendStatusStore";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

export function BackendStatusIndicator() {
  const { t } = useTranslation();
  const { tasks, clearCompleted } = useBackendStatusStore();

  useEffect(() => {
    initBackendStatusListeners();
  }, []);

  const active = tasks.filter((t) => t.status === "running");
  const recent = tasks.filter(
    (t) => t.status !== "running" && t.completedAt && Date.now() - t.completedAt < 10000,
  );

  if (active.length === 0 && recent.length === 0) {
    return null;
  }

  return (
    <div className="flex items-center gap-2 px-2 py-0.5 text-xs text-muted-foreground">
      {active.map((task) => (
        <span key={task.id} className="flex items-center gap-1">
          <span className="inline-block size-1.5 rounded-full bg-blue-400 animate-pulse" />
          {task.label}
          {task.progress != null && <span className="text-blue-400">{Math.round(task.progress * 100)}%</span>}
        </span>
      ))}
      {recent.map((task) => (
        <span key={task.id} className="flex items-center gap-1">
          <span
            className={`inline-block size-1.5 rounded-full ${task.status === "failed" ? "bg-red-400" : "bg-green-400"}`}
          />
          {task.label}
          <span className={task.status === "failed" ? "text-red-400" : "text-green-400"}>
            {task.status === "failed" ? t("backendStatus.failed") : t("backendStatus.done")}
          </span>
        </span>
      ))}
      {recent.length > 0 && (
        <button
          onClick={clearCompleted}
          className="hover:text-foreground transition-colors"
        >
          ✕
        </button>
      )}
    </div>
  );
}
