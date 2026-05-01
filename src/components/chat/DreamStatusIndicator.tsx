import { useDreamStore } from "@/stores";
import { CheckCircleOutlined } from "@ant-design/icons";
import { Brain, Moon } from "lucide-react";
import { useEffect, useState } from "react";
import "./DreamStatusIndicator.css";

/**
 * Dream 巩固状态指示器。
 *
 * 显示在 Agent 状态面板或标题栏中：
 * - 🌙 脉冲动画 = 后台 Dream 巩固运行中
 * - ✅ 短暂显示结果 = 巩固完成（记忆 N 条，模式 M 个）
 * - 隐藏 = 空闲
 */
export function DreamStatusIndicator() {
  const status = useDreamStore((s) => s.status);
  const lastResult = useDreamStore((s) => s.lastResult);
  const totalConsolidations = useDreamStore((s) => s.totalConsolidations);
  const totalMemories = useDreamStore((s) => s.totalMemoriesExtracted);
  const totalPatterns = useDreamStore((s) => s.totalPatternsDiscovered);

  const [showResult, setShowResult] = useState(false);

  // 当完成时显示结果快照，3s 后隐藏
  useEffect(() => {
    if (status === "completed" && lastResult) {
      setShowResult(true);
      const timer = setTimeout(() => setShowResult(false), 3500);
      return () => clearTimeout(timer);
    }
  }, [status, lastResult]);

  if (status === "idle" && !showResult && totalConsolidations === 0) {
    return null;
  }

  if (status === "running") {
    return (
      <div className="dream-indicator dream-indicator--running" title="Dream 巩固运行中...">
        <Moon size={14} className="dream-indicator__icon dream-indicator__icon--pulse" />
        <span className="dream-indicator__text">Dream 巩固中...</span>
      </div>
    );
  }

  if (showResult && lastResult) {
    const parts: string[] = [];
    if (lastResult.memoriesExtracted > 0) {
      parts.push(`${lastResult.memoriesExtracted} 条记忆`);
    }
    if (lastResult.patternsDiscovered > 0) {
      parts.push(`${lastResult.patternsDiscovered} 个模式`);
    }
    if (lastResult.error) {
      parts.push(`错误: ${lastResult.error}`);
    }
    const summary = parts.length > 0 ? parts.join("，") : "巩固完成";

    return (
      <div className="dream-indicator dream-indicator--completed" title={`Dream 完成 (${lastResult.durationSecs}s)`}>
        <CheckCircleOutlined className="dream-indicator__icon dream-indicator__icon--done" />
        <span className="dream-indicator__text">
          {summary}
          {lastResult.durationSecs > 0 && (
            <span className="dream-indicator__duration">({lastResult.durationSecs}s)</span>
          )}
        </span>
      </div>
    );
  }

  // 空闲但有过历史记录 — 显示累计统计
  if (totalConsolidations > 0) {
    return (
      <div className="dream-indicator dream-indicator--idle" title={`累计 ${totalConsolidations} 次巩固`}>
        <Brain size={13} className="dream-indicator__icon" />
        <span className="dream-indicator__text">
          {totalMemories} 记忆 · {totalPatterns} 模式
        </span>
      </div>
    );
  }

  return null;
}

export default DreamStatusIndicator;
