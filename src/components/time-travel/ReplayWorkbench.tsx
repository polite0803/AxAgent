import { AsOfDatePicker } from "@/components/time-travel/AsOfDatePicker";
import { ReplayBadge, ReplayWatermark } from "@/components/time-travel/ReplayBadge";
import { ReplaySweep } from "@/components/time-travel/ReplaySweep";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { Alert, Button, Card, Empty, Space, Typography } from "antd";
import { History, RotateCcw, Zap } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Title, Paragraph } = Typography;

/**
 * ReplayWorkbench — 时间旅行回放工作台
 *
 * 设计目标（HCI §10.1）：
 *   - 强制重选 as-of（进入本页时不沿用旧 asOfDate）
 *   - 给用户一个明确的"我现在用过去眼光看市场"的操作空间
 *   - 在工作台内可跳转分析/荐股面板（带 as-of 注入）
 *
 * 行为：
 *   - 页面加载 → 渲染 AsOfDatePicker，等待用户选日期
 *   - 用户选定 → enterReplayWorkbench(date) 覆盖当前 as-of
 *   - 选定后展示 3 个动作：分析单股 / 看荐股 / 看回测
 *   - "切回 Live" 按钮 → 调用 enterLive(requireConfirm=false)
 *     （PageHeader 的 ModeSwitch 仍会弹二次确认 Modal）
 */
export function ReplayWorkbench() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  const mode = useTimeAnchorStore((s) => s.mode);
  const enterReplayWorkbench = useTimeAnchorStore((s) => s.enterReplayWorkbench);
  const enterLive = useTimeAnchorStore((s) => s.enterLive);
  const [chosen, setChosen] = useState<string | null>(asOfDate);

  const isLocked = mode === "replay" && asOfDate !== null;

  const handlePick = (date: string) => {
    enterReplayWorkbench(date);
    setChosen(date);
  };

  const handleReset = () => {
    setChosen(null);
  };

  const handleSwitchToLive = () => {
    // 注意：ModeSwitch 的二次确认 Modal 仍会在切到 live 时弹出
    enterLive(false);
  };

  return (
    <div style={{ padding: "24px 16px" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16 }}>
        <History size={20} />
        <Title level={3} style={{ margin: 0 }}>
          {t("replayWorkbench.title")}
        </Title>
        {isLocked && <ReplayBadge />}
      </div>

      <Paragraph type="secondary" style={{ marginBottom: 16 }}>
        {t("replayWorkbench.description")}
      </Paragraph>

      <Card
        title={
          <span>
            <span style={{ marginRight: 8 }}>1️⃣</span>
            {t("replayWorkbench.step1.title")}
          </span>
        }
        size="small"
        style={{ marginBottom: 16 }}
      >
        {chosen === null
          ? (
            <AsOfDatePicker
              onPick={handlePick}
              onCancel={() => navigate("/backtest")}
            />
          )
          : (
            <Space size="middle" wrap>
              <span style={{ fontSize: 13 }}>
                {t("replayWorkbench.step1.currentlyPicked", { date: chosen })}
              </span>
              <Button
                size="small"
                icon={<RotateCcw size={12} />}
                onClick={handleReset}
                data-testid="replay-reset-btn"
              >
                {t("replayWorkbench.step1.reselect")}
              </Button>
            </Space>
          )}
      </Card>

      {chosen && (
        <Card
          title={
            <span>
              <span style={{ marginRight: 8 }}>2️⃣</span>
              {t("replayWorkbench.step2.title")}
            </span>
          }
          size="small"
          style={{ marginBottom: 16, position: "relative" }}
        >
          <div style={{ position: "relative" }}>
            <Paragraph type="secondary" style={{ marginBottom: 12 }}>
              {t("replayWorkbench.step2.description", { date: chosen })}
            </Paragraph>
            <Space wrap>
              <Button
                type="primary"
                onClick={() => navigate("/stock-analysis")}
                data-testid="goto-stock-analysis"
              >
                {t("replayWorkbench.step2.analyze")}
              </Button>
              <Button
                onClick={() => navigate("/backtest")}
                data-testid="goto-backtest"
              >
                {t("replayWorkbench.step2.backtest")}
              </Button>
              <Button
                onClick={() => navigate("/watchlist")}
                data-testid="goto-watchlist"
              >
                {t("replayWorkbench.step2.watchlist")}
              </Button>
            </Space>
            <ReplayWatermark />
          </div>
        </Card>
      )}

      <Alert
        type="info"
        showIcon
        message={t("replayWorkbench.notice")}
        style={{ marginBottom: 16 }}
      />

      <ReplaySweep />

      {isLocked && (
        <Card size="small" style={{ borderColor: "var(--sa-red, #ef4444)" }}>
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("replayWorkbench.exitHint")}
          />
          <div style={{ textAlign: "center", marginTop: 12 }}>
            <Button
              danger
              icon={<Zap size={12} />}
              onClick={handleSwitchToLive}
              data-testid="replay-exit-btn"
            >
              {t("replayWorkbench.exitBtn")}
            </Button>
          </div>
        </Card>
      )}
    </div>
  );
}
