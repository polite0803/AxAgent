// SPDX-License-Identifier: AGPL-3.0-only

/**
 * OfficeTab — 像素办公室 Tab 总入口。
 *
 * 布局：
 *   ┌──────────────────────────────────────────────────────┐
 *   │ 顶部工具栏：Fleet 选择器 + 创建按钮 + 状态标签      │
 *   ├────────────────────────────┬─────────────────────────┤
 *   │                            │ 右侧操作面板（4 个 Tab）│
 *   │   Phaser 像素办公室画布    │  - Chat (群聊智能路由)  │
 *   │   800×500                  │  - DM (直接 DM)         │
 *   │                            │  - Trajectory (轨迹)    │
 *   │                            │  - Token (用量统计)     │
 *   ├────────────────────────────┴─────────────────────────┤
 *   │ 底部成员列表（横向滚动 AgentCard）                  │
 *   └──────────────────────────────────────────────────────┘
 */

import { useOfficeStore } from "@/stores";
import type { Fleet, FleetMember } from "@/types";
import { App, Button, Dropdown, Empty, Input, Modal, Select, Spin, Tabs, Tag, theme, Tooltip, Typography } from "antd";
import { Building2, CirclePlus, MessageSquare, Send, TrendingUp, Users, Zap } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AgentCard } from "./AgentCard";
import { ChatPanel } from "./panels/ChatPanel";
import { DirectMessagePanel } from "./panels/DirectMessagePanel";
import { TokenPanel } from "./panels/TokenPanel";
import { TrajectoryPanel } from "./panels/TrajectoryPanel";
import { OfficeGame } from "./phaser/OfficeGame";
import { fleetMemberToSceneMember } from "./phaser/OfficeScene";
import { SCENE_TEMPLATES } from "./phaser/sceneTemplates";

const { Text } = Typography;

export function OfficeTab() {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const fleets = useOfficeStore((s) => s.fleets);
  const activeFleetId = useOfficeStore((s) => s.activeFleetId);
  const membersByFleet = useOfficeStore((s) => s.membersByFleet);
  const loading = useOfficeStore((s) => s.loading);
  const loadFleets = useOfficeStore((s) => s.loadFleets);
  const selectFleet = useOfficeStore((s) => s.selectFleet);
  const loadMembers = useOfficeStore((s) => s.loadMembers);
  const createFleet = useOfficeStore((s) => s.createFleet);
  const updateMemberStatus = useOfficeStore((s) => s.updateMemberStatus);

  const { message: messageApi } = App.useApp();

  const [rightTab, setRightTab] = useState<"chat" | "dm" | "trajectory" | "token">("chat");
  const [dmTarget, setDmTarget] = useState<FleetMember | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createTemplateSlug, setCreateTemplateSlug] = useState<string>(SCENE_TEMPLATES[0].slug);
  const [creating, setCreating] = useState(false);

  // 初次加载舰队列表
  useEffect(() => {
    void loadFleets();
  }, [loadFleets]);

  // 自动选中第一个 active 舰队
  useEffect(() => {
    if (!activeFleetId && fleets.length > 0) {
      selectFleet(fleets[0].id);
    }
  }, [fleets, activeFleetId, selectFleet]);

  // 选中舰队后加载成员
  useEffect(() => {
    if (activeFleetId) {
      void loadMembers(activeFleetId, true);
    }
  }, [activeFleetId, loadMembers]);

  const activeFleet: Fleet | undefined = fleets.find((f) => f.id === activeFleetId);
  const members = activeFleetId ? (membersByFleet[activeFleetId] ?? []) : [];

  // 转换为 SceneMember 给 Phaser 渲染
  const sceneMembers = useMemo(() => members.map(fleetMemberToSceneMember), [members]);

  // 切换 fleet 时清空 DM 目标
  useEffect(() => {
    setDmTarget(null);
    setRightTab("chat");
  }, [activeFleetId]);

  const handleAgentClick = (agentSlug: string, memberId: string) => {
    const m = members.find((x) => x.id === memberId);
    if (m) {
      setDmTarget(m);
      setRightTab("dm");
    }
    void agentSlug;
  };

  const handleCreateFleet = async () => {
    const name = createName.trim();
    if (!name) {
      messageApi.warning(t("office.createFleet.nameRequired"));
      return;
    }
    setCreating(true);
    try {
      const fleet = await createFleet({
        name,
        sceneTemplateSlug: createTemplateSlug,
      });
      if (fleet) {
        selectFleet(fleet.id);
        setCreateOpen(false);
        setCreateName("");
        setCreateTemplateSlug(SCENE_TEMPLATES[0].slug);
      } else {
        messageApi.error(t("office.createFleet.createFailed"));
      }
    } finally {
      setCreating(false);
    }
  };

  const renderCreateModal = () => (
    <Modal
      title={t("office.createFleet.button")}
      open={createOpen}
      onCancel={() => {
        setCreateOpen(false);
        setCreateName("");
        setCreateTemplateSlug(SCENE_TEMPLATES[0].slug);
      }}
      onOk={handleCreateFleet}
      okButtonProps={{ loading: creating }}
      destroyOnClose
      maskClosable={false}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <div>
          <div style={{ marginBottom: 6, fontSize: 12, color: token.colorTextSecondary }}>
            {t("office.createFleet.nameLabel")}
          </div>
          <Input
            autoFocus
            placeholder={t("office.createFleet.promptName")}
            value={createName}
            onChange={(e) => setCreateName(e.target.value)}
            onPressEnter={handleCreateFleet}
            maxLength={64}
          />
        </div>
        <div>
          <div style={{ marginBottom: 6, fontSize: 12, color: token.colorTextSecondary }}>
            {t("office.createFleet.templateLabel")}
          </div>
          <Select
            value={createTemplateSlug}
            onChange={setCreateTemplateSlug}
            options={SCENE_TEMPLATES.map((tpl) => ({
              value: tpl.slug,
              label: `${t(`office.scene.${tpl.displayNameKey}`)} · ${tpl.rooms.length} ${
                t("office.createFleet.roomsUnit")
              }`,
            }))}
            style={{ width: "100%" }}
          />
          <div style={{ marginTop: 4, fontSize: 11, color: token.colorTextQuaternary }}>
            {t(`office.scene.${
              SCENE_TEMPLATES.find((tpl) => tpl.slug === createTemplateSlug)?.displayNameKey ?? "default_office"
            }_desc`)}
          </div>
        </div>
      </div>
    </Modal>
  );

  // Fleet 下拉菜单项
  const fleetMenuItems = fleets.map((f) => ({
    key: f.id,
    label: (
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", minWidth: 200 }}>
        <span>{f.name}</span>
        <Tag
          color={f.status === "active" ? "green" : f.status === "paused" ? "orange" : "default"}
          style={{ fontSize: 10, margin: 0 }}
        >
          {t(`office.fleetStatus.${f.status}`)}
        </Tag>
      </div>
    ),
    onClick: () => selectFleet(f.id),
  }));

  if (loading && fleets.length === 0) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Spin />
      </div>
    );
  }

  if (fleets.length === 0) {
    return (
      <div style={{ padding: 48, height: "100%" }}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("office.emptyFleet")}
          styles={{ description: { fontSize: 13, color: token.colorTextQuaternary } }}
        >
          <Button type="primary" icon={<CirclePlus size={14} />} onClick={() => setCreateOpen(true)}>
            {t("office.createFleet.button")}
          </Button>
        </Empty>
        {renderCreateModal()}
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: 12 }}>
      {/* ── 顶部工具栏 ── */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
        <Building2 size={16} color={token.colorTextSecondary} />
        <Dropdown menu={{ items: fleetMenuItems }} trigger={["click"]}>
          <Button>
            <span style={{ fontWeight: 500 }}>
              {activeFleet?.name ?? t("office.selectFleet")}
            </span>
            {activeFleet && (
              <Tag
                color={activeFleet.status === "active" ? "green" : "orange"}
                style={{ marginLeft: 8, fontSize: 10 }}
              >
                {t(`office.fleetStatus.${activeFleet.status}`)}
              </Tag>
            )}
          </Button>
        </Dropdown>
        <Tooltip title={t("office.createFleet.button")}>
          <Button icon={<CirclePlus size={14} />} onClick={() => setCreateOpen(true)} />
        </Tooltip>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {t("office.memberCount", { count: members.length })}
        </Text>
      </div>

      {/* ── 主内容区：左 Phaser + 右操作面板 ── */}
      <div style={{ display: "flex", gap: 12, flex: 1, minHeight: 0 }}>
        {/* 左侧：Phaser 画布 */}
        <div
          style={{
            flex: "1 1 auto",
            minWidth: 0,
            background: token.colorBgContainer,
            borderRadius: 8,
            border: `1px solid ${token.colorBorderSecondary}`,
            padding: 8,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
          }}
        >
          {activeFleetId && (
            <OfficeGame
              sceneTemplateSlug={activeFleet?.sceneTemplateSlug}
              members={sceneMembers}
              onAgentClick={handleAgentClick}
              width={800}
              height={500}
            />
          )}
        </div>

        {/* 右侧：操作面板 Tabs */}
        <div
          style={{
            flex: "0 0 360px",
            background: token.colorBgContainer,
            borderRadius: 8,
            border: `1px solid ${token.colorBorderSecondary}`,
            display: "flex",
            flexDirection: "column",
            minHeight: 0,
          }}
        >
          <Tabs
            activeKey={rightTab}
            onChange={(k) => setRightTab(k as typeof rightTab)}
            size="small"
            style={{ padding: "0 8px", flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
            items={[
              {
                key: "chat",
                label: (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    <MessageSquare size={12} /> {t("office.tabs.chat")}
                  </span>
                ),
                children: activeFleetId
                  ? (
                    <div style={{ height: "100%", padding: "0 4px" }}>
                      <ChatPanel fleetId={activeFleetId} />
                    </div>
                  )
                  : null,
              },
              {
                key: "dm",
                label: (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    <Send size={12} /> {t("office.tabs.dm")}
                  </span>
                ),
                children: activeFleetId
                  ? (
                    <div style={{ height: "100%", padding: "0 4px" }}>
                      <DirectMessagePanel
                        fleetId={activeFleetId}
                        target={dmTarget}
                        onBack={() => setRightTab("chat")}
                      />
                    </div>
                  )
                  : null,
              },
              {
                key: "trajectory",
                label: (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    <TrendingUp size={12} /> {t("office.tabs.trajectory")}
                  </span>
                ),
                children: <TrajectoryPanel />,
              },
              {
                key: "token",
                label: (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    <Zap size={12} /> {t("office.tabs.token")}
                  </span>
                ),
                children: activeFleetId
                  ? <TokenPanel fleetId={activeFleetId} />
                  : null,
              },
            ]}
            // 让 tab 内容充满剩余空间
            tabBarStyle={{ marginBottom: 8 }}
          />
        </div>
      </div>

      {/* ── 底部成员列表 ── */}
      <div
        style={{
          background: token.colorBgContainer,
          borderRadius: 8,
          border: `1px solid ${token.colorBorderSecondary}`,
          padding: 8,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 8 }}>
          <Users size={12} color={token.colorTextSecondary} />
          <Text type="secondary" style={{ fontSize: 11, fontWeight: 500 }}>
            {t("office.membersTitle")}
          </Text>
        </div>
        {members.length === 0
          ? (
            <div style={{ textAlign: "center", color: token.colorTextQuaternary, fontSize: 12, padding: 16 }}>
              {t("office.noMembers")}
            </div>
          )
          : (
            <div style={{ display: "flex", gap: 8, overflowX: "auto", paddingBottom: 4 }}>
              {members.map((m) => (
                <div key={m.id} style={{ minWidth: 240, flex: "0 0 240px" }}>
                  <AgentCard
                    member={m}
                    highlighted={dmTarget?.id === m.id}
                    onClick={(member) => {
                      setDmTarget(member);
                      setRightTab("dm");
                      // 同时调用 store 更新状态（用于演示交互链路）
                      void updateMemberStatus(member.id, member.status);
                    }}
                  />
                </div>
              ))}
            </div>
          )}
      </div>
      {renderCreateModal()}
    </div>
  );
}
