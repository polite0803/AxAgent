import { type BuddyAttributes, useBuddyStore } from "@/stores/feature/buddyStore";
import { CloseOutlined, EyeInvisibleOutlined, EyeOutlined, RobotOutlined } from "@ant-design/icons";
import { Button, Card, Progress, Tag, Typography } from "antd";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BuddyMessageBubble } from "./BuddyMessage";

const { Text } = Typography;

const rarityColors: Record<string, string> = {
  common: "#8c8c8c",
  uncommon: "#52c41a",
  rare: "#1890ff",
  epic: "#722ed1",
  legendary: "#faad14",
};

const attrColors: Record<keyof BuddyAttributes, string> = {
  debugging: "#1890ff",
  patience: "#52c41a",
  chaos: "#fa541c",
  wisdom: "#722ed1",
  snark: "#eb2f96",
};

export function BuddyWidget() {
  const { t } = useTranslation();
  const activeBuddy = useBuddyStore((s) => s.activeBuddy);
  const showPanel = useBuddyStore((s) => s.showPanel);
  const messages = useBuddyStore((s) => s.messages);
  const visible = useBuddyStore((s) => s.visible);
  const savedPosition = useBuddyStore((s) => s.position);
  const summonBuddy = useBuddyStore((s) => s.summonBuddy);
  const dismissBuddy = useBuddyStore((s) => s.dismissBuddy);
  const togglePanel = useBuddyStore((s) => s.togglePanel);
  const setVisible = useBuddyStore((s) => s.setVisible);
  const setPosition = useBuddyStore((s) => s.setPosition);

  const rarityLabels = useMemo(
    () => ({
      common: t("buddy.rarity.common"),
      uncommon: t("buddy.rarity.uncommon"),
      rare: t("buddy.rarity.rare"),
      epic: t("buddy.rarity.epic"),
      legendary: t("buddy.rarity.legendary"),
    }),
    [t],
  );

  const attrLabels = useMemo<Record<keyof BuddyAttributes, string>>(
    () => ({
      debugging: t("buddy.attr.debugging"),
      patience: t("buddy.attr.patience"),
      chaos: t("buddy.attr.chaos"),
      wisdom: t("buddy.attr.wisdom"),
      snark: t("buddy.attr.snark"),
    }),
    [t],
  );

  // 拖动状态（按钮拖动）
  const [dragPos, setDragPos] = useState<{ x: number; y: number } | null>(
    savedPosition,
  );
  const dragging = useRef(false);
  const hasDragged = useRef(false);
  const dragStart = useRef({ x: 0, y: 0, posX: 0, posY: 0 });
  const currentDragPos = useRef<{ x: number; y: number } | null>(null);
  const widgetRef = useRef<HTMLDivElement>(null);

  // 面板拖动状态（独立于按钮）
  const [panelPos, setPanelPos] = useState<{ x: number; y: number } | null>(
    null,
  );
  const panelDragging = useRef(false);
  const panelHasDragged = useRef(false);
  const panelDragStart = useRef({ x: 0, y: 0, posX: 0, posY: 0 });
  const panelCurrentDragPos = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    setDragPos(savedPosition);
    currentDragPos.current = savedPosition;
  }, [savedPosition]);

  const lastMessage = useMemo(() => {
    if (messages.length === 0) {
      return null;
    }
    return messages[messages.length - 1];
  }, [messages]);

  // 拖动开始 (pointer 事件 + capture 阶段，在 Button 之前捕获)
  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    // 只在鼠标左键时拖动
    if (e.button !== 0) {
      return;
    }
    dragging.current = true;
    hasDragged.current = false;
    const currentPos = currentDragPos.current ?? {
      x: window.innerWidth - 76,
      y: window.innerHeight - 76,
    };
    currentDragPos.current = currentPos;
    dragStart.current = {
      x: e.clientX,
      y: e.clientY,
      posX: currentPos.x,
      posY: currentPos.y,
    };
    // 捕获指针，确保后续事件都发给我们
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }, []);

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragging.current) {
      return;
    }
    const dx = e.clientX - dragStart.current.x;
    const dy = e.clientY - dragStart.current.y;
    if (Math.abs(dx) >= 3 || Math.abs(dy) >= 3) {
      hasDragged.current = true;
    }
    if (!hasDragged.current) {
      return;
    }
    const newX = Math.max(
      0,
      Math.min(window.innerWidth - 60, dragStart.current.posX + dx),
    );
    const newY = Math.max(
      0,
      Math.min(window.innerHeight - 60, dragStart.current.posY + dy),
    );
    currentDragPos.current = { x: newX, y: newY };
    setDragPos({ x: newX, y: newY });
  }, []);

  const handlePointerUp = useCallback(() => {
    if (!dragging.current) {
      return;
    }
    dragging.current = false;
    if (hasDragged.current && currentDragPos.current) {
      setPosition(currentDragPos.current.x, currentDragPos.current.y);
    }
  }, [setPosition]);

  const wrapClick = useCallback((fn: () => void) => {
    return () => {
      if (!hasDragged.current) {
        fn();
      }
    };
  }, []);

  // ---- 面板标题栏拖动 ----
  const clampPanelPos = useCallback((x: number, y: number) => {
    const panelW = 300;
    const panelH = 300;
    return {
      x: Math.max(0, Math.min(window.innerWidth - panelW, x)),
      y: Math.max(0, Math.min(window.innerHeight - panelH, y)),
    };
  }, []);

  const handlePanelPointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (e.button !== 0) {
        return;
      }
      if (
        (e.target as HTMLElement).closest("button, input, select, textarea")
      ) {
        return;
      }
      e.stopPropagation();
      panelDragging.current = true;
      panelHasDragged.current = false;
      const currentPos = panelCurrentDragPos.current
        ?? panelPos ?? { x: 0, y: 0 };
      panelCurrentDragPos.current = currentPos;
      panelDragStart.current = {
        x: e.clientX,
        y: e.clientY,
        posX: currentPos.x,
        posY: currentPos.y,
      };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [panelPos],
  );

  const handlePanelPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!panelDragging.current) {
        return;
      }
      const dx = e.clientX - panelDragStart.current.x;
      const dy = e.clientY - panelDragStart.current.y;
      if (Math.abs(dx) >= 3 || Math.abs(dy) >= 3) {
        panelHasDragged.current = true;
      }
      if (!panelHasDragged.current) {
        return;
      }
      const rawX = panelDragStart.current.posX + dx;
      const rawY = panelDragStart.current.posY + dy;
      const clamped = clampPanelPos(rawX, rawY);
      panelCurrentDragPos.current = clamped;
      setPanelPos(clamped);
    },
    [clampPanelPos],
  );

  const handlePanelPointerUp = useCallback(() => {
    panelDragging.current = false;
  }, []);

  const panelPosition = (panelPos ?? panelDragging.current) ? panelCurrentDragPos.current : null;
  const panelPosStyle = panelPosition
    ? { left: panelPosition.x, top: panelPosition.y }
    : {};

  // 隐藏时显示微型恢复按钮
  if (!visible) {
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
          type="default"
          shape="circle"
          size="small"
          icon={<EyeOutlined />}
          onClick={() => setVisible(true)}
          title={t("buddy.showBuddy")}
          style={{
            width: 32,
            height: 32,
            opacity: 0.5,
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
          }}
        />
      </div>
    );
  }

  // 按钮定位：拖动时使用拖动位置，默认固定在右下角
  const positionStyle = dragPos
    ? { left: dragPos.x, top: dragPos.y, bottom: "auto", right: "auto" }
    : { bottom: 24, right: 24, left: "auto", top: "auto" };

  // 无 Buddy 时显示召唤按钮
  if (!activeBuddy) {
    return (
      <div
        ref={widgetRef}
        style={{
          position: "fixed",
          zIndex: 1000,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 4,
          cursor: "grab",
          touchAction: "none",
          userSelect: "none",
          ...positionStyle,
        }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <Button
          type="primary"
          shape="circle"
          size="large"
          icon={<RobotOutlined />}
          onClick={wrapClick(() => summonBuddy())}
          style={{
            width: 52,
            height: 52,
            boxShadow: "0 4px 14px rgba(0,0,0,0.15)",
          }}
          title={t("buddy.summonBuddy")}
        />
        <Button
          type="text"
          size="small"
          icon={<EyeInvisibleOutlined />}
          onClick={() => setVisible(false)}
          title={t("buddy.hideBuddy")}
          style={{ opacity: 0.3 }}
        />
      </div>
    );
  }

  const buddy = activeBuddy;
  const attrKeys: (keyof BuddyAttributes)[] = [
    "debugging",
    "patience",
    "chaos",
    "wisdom",
    "snark",
  ];

  return (
    <div
      ref={widgetRef}
      style={{
        position: "fixed",
        zIndex: 1000,
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-end",
        gap: 8,
        ...positionStyle,
      }}
    >
      {/* 展开的消息面板 */}
      {showPanel && (
        <Card
          size="small"
          title={
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                cursor: "grab",
                userSelect: "none",
              }}
              onPointerDown={handlePanelPointerDown}
              onPointerMove={handlePanelPointerMove}
              onPointerUp={handlePanelPointerUp}
            >
              <Text style={{ fontSize: 24 }}>{buddy.emoji}</Text>
              <div>
                <Text strong style={{ fontSize: 15 }}>
                  {buddy.name}
                </Text>
                <div>
                  <Tag
                    color={rarityColors[buddy.rarity]}
                    style={{ fontSize: 12, lineHeight: "18px", margin: 0 }}
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
            ...panelPosStyle,
          }}
          styles={{ body: { padding: "8px 16px 12px" } }}
        >
          {/* 属性条 */}
          <div style={{ marginBottom: 12 }}>
            <Text
              type="secondary"
              style={{ fontSize: 12, marginBottom: 6, display: "block" }}
            >
              {t("buddy.attributes")}
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
                <Text
                  style={{
                    fontSize: 12,
                    color: "#999",
                    width: 20,
                    textAlign: "right",
                  }}
                >
                  {buddy.attributes[key]}/10
                </Text>
              </div>
            ))}
          </div>

          {/* 经验条 */}
          <div style={{ marginBottom: 12 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("buddy.experience")}
            </Text>
            <Progress
              percent={Math.round((buddy.xp / (100 + buddy.level * 50)) * 100)}
              size="small"
              strokeColor="#faad14"
              format={() => `${buddy.xp} XP`}
              style={{ margin: 0 }}
            />
          </div>

          {/* 最近消息 */}
          {lastMessage && (
            <div>
              <Text
                type="secondary"
                style={{ fontSize: 12, marginBottom: 4, display: "block" }}
              >
                {t("buddy.recentMessages")}
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

      {/* 浮动按钮 + 隐藏按钮 */}
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 4,
          cursor: "grab",
          touchAction: "none",
          userSelect: "none",
        }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <Button
          type="primary"
          shape="circle"
          size="large"
          icon={<Text style={{ fontSize: 22, lineHeight: 1 }}>{buddy.emoji}</Text>}
          onClick={wrapClick(togglePanel)}
          style={{
            width: 52,
            height: 52,
            boxShadow: "0 4px 14px rgba(0,0,0,0.15)",
            position: "relative",
          }}
          title={t("buddy.togglePanel")}
        >
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
        <Button
          type="text"
          size="small"
          icon={<EyeInvisibleOutlined />}
          onClick={() => setVisible(false)}
          title={t("buddy.hideBuddy")}
          style={{ opacity: 0.3 }}
        />
      </div>

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
                {": "}
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
