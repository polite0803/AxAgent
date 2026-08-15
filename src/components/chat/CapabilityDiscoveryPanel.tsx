// SPDX-License-Identifier: AGPL-3.0-only

import { useCapabilityStore } from "@/stores";
import type { CapabilityPassportDto } from "@/types";
import { Button, Card, Collapse, Empty, Input, Skeleton, Space, Tag, theme, Typography } from "antd";
import { Database, RefreshCw, Search, ShieldCheck, Tag as TagIcon, Zap } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 能力护照 → 展示分类 key（agent 按 sub_category 细分为角色/专家） */
function passportKindKey(p: CapabilityPassportDto): string {
  return p.kind === "agent"
    ? p.sub_category === "agent_role" ? "agent_role" : "agent_profile"
    : p.kind;
}

const KIND_COLORS: Record<string, string> = {
  tool: "blue",
  workflow: "green",
  knowledge_base: "purple",
  skill: "orange",
  agent_role: "cyan",
  agent_profile: "geekblue",
};

/** 能力发现面板：展示能力索引统计、已注册护照，并支持输入查询触发能力发现 */
export function CapabilityDiscoveryPanel() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const passports = useCapabilityStore((s) => s.passports);
  const stats = useCapabilityStore((s) => s.stats);
  const discoveryResult = useCapabilityStore((s) => s.discoveryResult);
  const isLoading = useCapabilityStore((s) => s.isLoading);
  const isDiscovering = useCapabilityStore((s) => s.isDiscovering);
  const error = useCapabilityStore((s) => s.error);
  const listPassports = useCapabilityStore((s) => s.listPassports);
  const getStats = useCapabilityStore((s) => s.getStats);
  const discover = useCapabilityStore((s) => s.discover);

  const [query, setQuery] = useState("");

  useEffect(() => {
    void listPassports();
    void getStats();
  }, [listPassports, getStats]);

  const handleDiscover = async () => {
    if (!query.trim() || isDiscovering) {
      return;
    }
    await discover({ userInput: query.trim() });
  };

  const handleRefresh = () => {
    void listPassports();
    void getStats();
  };

  const kindCount = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const p of passports) {
      const key = passportKindKey(p);
      counts[key] = (counts[key] ?? 0) + 1;
    }
    return counts;
  }, [passports]);

  /** 分类 key → 友好名称（未命中翻译时回退为原始 key） */
  const kindLabel = (key: string): string => {
    const label = t(`capabilityPanel.kind.${key}`);
    return label === `capabilityPanel.kind.${key}` ? key : label;
  };

  return (
    <div style={{ padding: "12px 16px", display: "flex", flexDirection: "column", gap: 12 }}>
      {/* 索引统计 */}
      <Card
        size="small"
        styles={{ body: { padding: "12px 14px" } }}
        title={
          <Space size={6}>
            <Database size={13} />
            <Text style={{ fontSize: 13 }}>{t("capabilityPanel.statsTitle")}</Text>
          </Space>
        }
        extra={
          <Button
            type="text"
            size="small"
            icon={<RefreshCw size={13} />}
            onClick={handleRefresh}
          />
        }
      >
        {stats
          ? (
            <Space orientation="vertical" size={4} style={{ width: "100%" }}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("capabilityPanel.totalCapabilities")}
                </Text>
                <Text strong style={{ fontSize: 12 }}>{stats.total_capabilities}</Text>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("capabilityPanel.totalVectors")}
                </Text>
                <Text strong style={{ fontSize: 12 }}>{stats.total_vectors}</Text>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("capabilityPanel.positiveVectors")}
                </Text>
                <Text strong style={{ fontSize: 12 }}>{stats.positive_vectors}</Text>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("capabilityPanel.negativeVectors")}
                </Text>
                <Text strong style={{ fontSize: 12 }}>{stats.negative_vectors}</Text>
              </div>
            </Space>
          )
          : <Skeleton active paragraph={{ rows: 3 }} title={false} />}
      </Card>

      {/* 发现查询 */}
      <Card
        size="small"
        styles={{ body: { padding: "12px 14px" } }}
        title={
          <Space size={6}>
            <Search size={13} />
            <Text style={{ fontSize: 13 }}>{t("capabilityPanel.discoverTitle")}</Text>
          </Space>
        }
      >
        <Space orientation="vertical" size={8} style={{ width: "100%" }}>
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("capabilityPanel.discoverPlaceholder")}
            onPressEnter={handleDiscover}
            allowClear
          />
          <Button
            type="primary"
            size="small"
            block
            loading={isDiscovering}
            disabled={!query.trim()}
            onClick={handleDiscover}
            icon={<Zap size={13} />}
          >
            {t("capabilityPanel.discoverButton")}
          </Button>
        </Space>

        {error && (
          <Text type="danger" style={{ fontSize: 12, display: "block", marginTop: 8 }}>
            {error}
          </Text>
        )}

        {discoveryResult && (
          <div style={{ marginTop: 10 }}>
            <Text strong style={{ fontSize: 12.5, display: "block", marginBottom: 6 }}>
              {t("capabilityPanel.discoverResult")}
            </Text>
            {discoveryResult.primary_match
              ? (
                <div
                  style={{
                    padding: 8,
                    borderRadius: 6,
                    border: `1px solid ${token.colorPrimaryBorder}`,
                    backgroundColor: `${token.colorPrimaryBg}`,
                    marginBottom: 6,
                  }}
                >
                  <Text strong style={{ fontSize: 12.5, display: "block" }}>
                    {discoveryResult.primary_match.passport.name}
                  </Text>
                  <Text type="secondary" style={{ fontSize: 12, display: "block", marginTop: 2 }}>
                    {discoveryResult.primary_match.passport.capability_id}
                  </Text>
                  <Space size={4} style={{ marginTop: 4 }} wrap>
                    <Tag
                      color={KIND_COLORS[passportKindKey(discoveryResult.primary_match.passport)]}
                      style={{ fontSize: 11 }}
                    >
                      {kindLabel(passportKindKey(discoveryResult.primary_match.passport))}
                    </Tag>
                    <Tag style={{ fontSize: 11 }}>
                      {t("capabilityPanel.score")}: {Math.round(discoveryResult.primary_match.final_score * 100)}%
                    </Tag>
                  </Space>
                </div>
              )
              : discoveryResult.ambiguous
              ? (
                <Text type="warning" style={{ fontSize: 12, display: "block" }}>
                  {t("capabilityPanel.ambiguous")}
                </Text>
              )
              : (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t("capabilityPanel.noMatch")}
                    </Text>
                  }
                />
              )}
            {discoveryResult.alternatives.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 4 }}>
                {discoveryResult.alternatives.map((alt) => (
                  <div
                    key={alt.passport.capability_id}
                    style={{
                      fontSize: 12,
                      padding: "4px 8px",
                      borderRadius: 6,
                      backgroundColor: token.colorBgLayout,
                    }}
                  >
                    {alt.passport.name}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </Card>

      {/* 已注册护照 */}
      <Card
        size="small"
        styles={{ body: { padding: "12px 14px" } }}
        title={
          <Space size={6}>
            <ShieldCheck size={13} />
            <Text style={{ fontSize: 13 }}>
              {t("capabilityPanel.passportsTitle")} ({passports.length})
            </Text>
          </Space>
        }
      >
        {isLoading && !passports.length
          ? <Skeleton active paragraph={{ rows: 4 }} title={false} />
          : passports.length === 0
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("capabilityPanel.noPassports")}
                </Text>
              }
            />
          )
          : (
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {Object.entries(kindCount).map(([kind, count]) => (
                <div
                  key={kind}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: "4px 8px",
                    borderRadius: 6,
                    backgroundColor: token.colorBgLayout,
                  }}
                >
                  <Tag color={KIND_COLORS[kind] ?? "default"} style={{ marginInlineEnd: 0 }}>
                    {kindLabel(kind)}
                  </Tag>
                  <Text strong style={{ fontSize: 12 }}>{count}</Text>
                </div>
              ))}
            </div>
          )}

        {passports.length > 0 && (
          <Collapse
            ghost
            size="small"
            style={{ marginTop: 8 }}
            items={passports.slice(0, 20).map((p) => ({
              key: p.capability_id,
              label: (
                <Space size={6}>
                  <TagIcon size={11} style={{ color: token.colorTextTertiary }} />
                  <Text style={{ fontSize: 12.5 }}>{p.name}</Text>
                </Space>
              ),
              children: (
                <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
                  <Text type="secondary" style={{ fontSize: 11, wordBreak: "break-all" }}>
                    {p.capability_id}
                  </Text>
                  {p.description && (
                    <Text type="secondary" style={{ fontSize: 11.5, lineHeight: 1.5 }}>
                      {p.description}
                    </Text>
                  )}
                  <Space size={4} wrap style={{ marginTop: 2 }}>
                    <Tag color={KIND_COLORS[passportKindKey(p)] ?? "default"} style={{ fontSize: 11 }}>
                      {kindLabel(passportKindKey(p))}
                    </Tag>
                    {p.domain && <Tag style={{ fontSize: 11 }}>{p.domain}</Tag>}
                    <Tag color={p.enabled ? "success" : "default"} style={{ fontSize: 11 }}>
                      {p.enabled ? t("capabilityPanel.enabled") : t("capabilityPanel.disabled")}
                    </Tag>
                  </Space>
                </div>
              ),
            }))}
          />
        )}
      </Card>
    </div>
  );
}
