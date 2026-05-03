import { type BuddyAttributes, useBuddyStore } from "@/stores/feature/buddyStore";
import { CloseOutlined, RobotOutlined } from "@ant-design/icons";
import { Button, Card, Progress, Tag, Typography } from "antd";
import { useMemo } from "react";
import { BuddyMessageBubble } from "./BuddyMessage";

const { Text } = Typography;

// 稀有度 → 颜色映射
const rarityColors: Record<string, string> = {
  common: "#8c8c8c",
  uncommon: "#52c41a",
  rare: "#1890ff",
  epic: "#722ed1",
  legendary: "#faad14",
};

const rarityLabels: Record<string, string> = {
  common: "普通",
  uncommon: "罕见",
  rare: "稀有",
  epic: "史诗",
  legendary: "传说",
};

// 属性中文名映射
const attrLabels: Record<keyof BuddyAttributes, string> = {
  debugging: "调试",
  patience: "耐心",
  chaos: "混乱",
  wisdom: "智慧",
  snark: "毒舌",
};

// 属性进度条颜色
const attrColors: Record<keyof BuddyAttributes, string> = {
  debugging: "#1890ff",
  patience: "#52c41a",
  chaos: "#fa541c",
  wisdom: "#722ed1",
  snark: "#eb2f96",
};

export function BuddyWidget() {
  const activeBuddy = useBuddyStore((s) => s.activeBuddy);
  const showPanel = useBuddyStore((s) => s.showPanel);
  const messages = useBuddyStore((s) => s.messages);
  const summonBuddy = useBuddyStore((s) => s.summonBuddy);
  const dismissBuddy = useBuddyStore((s) => s.dismissBuddy);
  const togglePanel = useBuddyStore((s) => s.togglePanel);

  // 最近一条消息
  const lastMessage = useMemo(() => {
    if (messages.length === 0) { return null; }
    return messages[messages.length - 1];
  }, [messages]);

  // 无 Buddy 时显示召唤按钮
  if (!activeBuddy) {
    return (
      <div
        style={{
          position: "fixed",
          bottom: 24,
          right: 24,
          zIndex: 1000,
        }}
      >
        <Button
          type="primary"
          shape="circle"
          size="large"
          icon={<RobotOutlined />}
          onClick={() => summonBuddy()}
          style={{
            width: 52,
            height: 52,
            boxShadow: "0 4px 14px rgba(0,0,0,0.15)",
          }}
        />
      </div>
    );
  }

  const buddy = activeBuddy;
  const attrKeys: (keyof BuddyAttributes)[] = ["debugging", "patience", "chaos", "wisdom", "snark"];

  return (
    <div
      style={{
        position: "fixed",
        bottom: 24,
        right: 24,
        zIndex: 1000,
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-end",
        gap: 8,
      }}
    >
      {/* 展开的消息面板 */}
      {showPanel && (
        <Card
          size="small"
          title={
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Text style={{ fontSize: 24 }}>{buddy.emoji}</Text>
              <div>
                <Text strong style={{ fontSize: 15 }}>
                  {buddy.name}
                </Text>
                <div>
                  <Tag
                    color={rarityColors[buddy.rarity]}
                    style={{ fontSize: 11, lineHeight: "18px", margin: 0 }}
                  >
                    {rarityLabels[buddy.rarity]}
                  </Tag>
                  <Text style={{ fontSize: 12, color: "#999", marginLeft: 6 }}>
                    Lv.{buddy.level}
                  </Text>
                </div>
              </div>
            </div>
          }
          extra={
            <Button
              type="text"
              size="small"
              icon={<CloseOutlined />}
              onClick={dismissBuddy}
            />
          }
          style={{
            width: 300,
            boxShadow: "0 6px 20px rgba(0,0,0,0.12)",
            borderRadius: 12,
          }}
          styles={{ body: { padding: "8px 16px 12px" } }}
        >
          {/* 属性条 */}
          <div style={{ marginBottom: 12 }}>
            <Text type="secondary" style={{ fontSize: 12, marginBottom: 6, display: "block" }}>
              属性
            </Text>
            {attrKeys.map((key) => (
              <div
                key={key}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  marginBottom: 6,
                }}
              >
                <Text style={{ fontSize: 12, width: 32, flexShrink: 0 }}>
                  {attrLabels[key]}
                </Text>
                <Progress
                  percent={buddy.attributes[key] * 10}
                  size="small"
                  strokeColor={attrColors[key]}
                  showInfo={false}
                  style={{ flex: 1, margin: 0 }}
                />
                <Text style={{ fontSize: 11, color: "#999", width: 20, textAlign: "right" }}>
                  {buddy.attributes[key]}/10
                </Text>
              </div>
            ))}
          </div>

          {/* 经验条 */}
          <div style={{ marginBottom: 12 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              经验值
            </Text>
            <Progress
              percent={Math.round(
                (buddy.xp / (100 + buddy.level * 50)) * 100,
              )}
              size="small"
              strokeColor="#faad14"
              format={() => `${buddy.xp} XP`}
              style={{ margin: 0 }}
            />
          </div>

          {/* 最近消息 */}
          {lastMessage && (
            <div>
              <Text type="secondary" style={{ fontSize: 12, marginBottom: 4, display: "block" }}>
                最近发言
              </Text>
              <BuddyMessageBubble
                message={lastMessage}
                buddyEmoji={buddy.emoji}
                buddyName={buddy.name}
              />
            </div>
          )}
        </Card>
      )}

      {/* 浮动按钮：展开/折叠 */}
      <Button
        type="primary"
        shape="circle"
        size="large"
        icon={<Text style={{ fontSize: 22, lineHeight: 1 }}>{buddy.emoji}</Text>}
        onClick={togglePanel}
        style={{
          width: 52,
          height: 52,
          boxShadow: "0 4px 14px rgba(0,0,0,0.15)",
          position: "relative",
        }}
      >
        {/* 等级角标 */}
        <span
          style={{
            position: "absolute",
            top: -4,
            right: -4,
            background: "#faad14",
            color: "#fff",
            fontSize: 10,
            fontWeight: 700,
            width: 20,
            height: 20,
            borderRadius: "50%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: "2px solid #fff",
          }}
        >
          {buddy.level}
        </span>
      </Button>

      {/* 折叠时的简短提示 */}
      {!showPanel && (
        <Card
          size="small"
          styles={{ body: { padding: "6px 12px" } }}
          style={{
            boxShadow: "0 2px 8px rgba(0,0,0,0.08)",
            borderRadius: 10,
            marginBottom: -8,
          }}
        >
          <Text style={{ fontSize: 13 }}>
            {buddy.emoji} {buddy.name} Lv.{buddy.level}
            {lastMessage && (
              <>
                {" "}
                —{" "}
                <Text style={{ fontSize: 12, color: "#999" }}>
                  "{lastMessage.text.slice(0, 20)}
                  {lastMessage.text.length > 20 ? "..." : ""}"
                </Text>
              </>
            )}
          </Text>
        </Card>
      )}
    </div>
  );
}
