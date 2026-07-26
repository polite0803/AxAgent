// SPDX-License-Identifier: AGPL-3.0-only

/**
 * TokenPanel — Token 用量统计面板。
 *
 * 展示当前 fleet 的成员 token 用量（today / total），按今日用量降序。
 * 提供「重置今日用量」按钮，调用 store.resetDailyTokens。
 */

import { useOfficeStore } from "@/stores";
import type { FleetMember } from "@/types";
import { Button, Empty, Spin, Table, Tag, theme, Typography } from "antd";
import { RotateCcw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

function formatTokens(n: number) {
  if (n >= 1_000_000) { return `${(n / 1_000_000).toFixed(2)}M`; }
  if (n >= 1_000) { return `${(n / 1_000).toFixed(1)}K`; }
  return String(n);
}

export function TokenPanel({ fleetId }: { fleetId: string }) {
  const { t } = useTranslation();
  const { token: themeToken } = theme.useToken();
  const members = useOfficeStore((s) => s.membersByFleet[fleetId] ?? []);
  const loading = useOfficeStore((s) => s.loading);
  const resetDaily = useOfficeStore((s) => s.resetDailyTokens);
  const [resetting, setResetting] = useState(false);

  const sorted = [...members].sort((a, b) => b.todayTokens - a.todayTokens);
  const totalToday = sorted.reduce((s, m) => s + m.todayTokens, 0);
  const totalAll = sorted.reduce((s, m) => s + m.totalTokens, 0);

  const handleReset = async () => {
    setResetting(true);
    try {
      await resetDaily(fleetId);
    } finally {
      setResetting(false);
    }
  };

  if (loading && members.length === 0) {
    return (
      <div style={{ padding: 24, textAlign: "center" }}>
        <Spin size="small" />
      </div>
    );
  }

  if (members.length === 0) {
    return (
      <div style={{ padding: 24 }}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("office.token.empty")}
          styles={{ description: { fontSize: 12, color: themeToken.colorTextQuaternary } }}
        />
      </div>
    );
  }

  return (
    <div style={{ padding: 12, height: "100%", display: "flex", flexDirection: "column", gap: 8 }}>
      {/* 总览统计 */}
      <div style={{ display: "flex", gap: 8 }}>
        <div
          style={{
            flex: 1,
            padding: "10px 12px",
            background: themeToken.colorBgLayout,
            borderRadius: 6,
            border: `1px solid ${themeToken.colorBorderSecondary}`,
          }}
        >
          <div style={{ fontSize: 11, color: themeToken.colorTextTertiary }}>
            {t("office.token.todayTotal")}
          </div>
          <div style={{ fontSize: 18, fontWeight: 600, color: themeToken.colorPrimary }}>
            {formatTokens(totalToday)}
          </div>
        </div>
        <div
          style={{
            flex: 1,
            padding: "10px 12px",
            background: themeToken.colorBgLayout,
            borderRadius: 6,
            border: `1px solid ${themeToken.colorBorderSecondary}`,
          }}
        >
          <div style={{ fontSize: 11, color: themeToken.colorTextTertiary }}>
            {t("office.token.allTotal")}
          </div>
          <div style={{ fontSize: 18, fontWeight: 600, color: themeToken.colorText }}>
            {formatTokens(totalAll)}
          </div>
        </div>
      </div>

      {/* 操作按钮 */}
      <div style={{ display: "flex", justifyContent: "flex-end" }}>
        <Button
          size="small"
          icon={<RotateCcw size={12} />}
          loading={resetting}
          onClick={handleReset}
        >
          {t("office.token.resetDaily")}
        </Button>
      </div>

      {/* 成员列表 */}
      <div style={{ flex: 1, overflow: "auto" }}>
        <Table<FleetMember>
          size="small"
          dataSource={sorted}
          rowKey="id"
          pagination={false}
          columns={[
            {
              title: t("office.token.colMember"),
              dataIndex: "displayName",
              render: (v, r) => (
                <div>
                  <div style={{ fontWeight: 500, fontSize: 12 }}>{v}</div>
                  <div style={{ fontSize: 10, color: themeToken.colorTextQuaternary, fontFamily: "monospace" }}>
                    {r.agentSlug}
                  </div>
                </div>
              ),
            },
            {
              title: t("office.token.colStatus"),
              dataIndex: "status",
              width: 90,
              render: (s: FleetMember["status"]) => (
                <Tag
                  color={statusColor(s)}
                  style={{ fontSize: 10, margin: 0, padding: "0 6px" }}
                >
                  {t(`office.memberStatus.${s}`)}
                </Tag>
              ),
            },
            {
              title: t("office.token.colToday"),
              dataIndex: "todayTokens",
              width: 100,
              align: "right",
              render: (v: number) => (
                <Text style={{ fontWeight: 600, color: themeToken.colorPrimary }}>
                  {formatTokens(v)}
                </Text>
              ),
            },
            {
              title: t("office.token.colTotal"),
              dataIndex: "totalTokens",
              width: 100,
              align: "right",
              render: (v: number) => (
                <Text style={{ fontSize: 11 }}>
                  {formatTokens(v)}
                </Text>
              ),
            },
          ]}
        />
      </div>
    </div>
  );
}

function statusColor(status: FleetMember["status"]): string {
  switch (status) {
    case "idle":
      return "green";
    case "busy":
      return "blue";
    case "paused":
      return "orange";
    case "error":
      return "red";
    case "offline":
      return "default";
    default:
      return "default";
  }
}
