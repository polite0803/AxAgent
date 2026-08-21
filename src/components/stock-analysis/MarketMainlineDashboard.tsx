// i18n-exempt: 业务逻辑判断字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * G4 市场主线 Dashboard
 *
 * 功能：
 * - 展示最近 N 日市场主线（按日期 / 强度 / 大类过滤）
 * - 每条主线卡片：主题 / 叙述 / 代表性标的 / 强度评分 / 持续性
 * - 创建 / 归档主线（手动触发）
 * - 工作流自动触发：每日 18:00 由 daily-market-events 模板自动写入
 *
 * 数据来自后端 Tauri 命令（commands/market_mainline.rs）：
 * - market_mainline_list_recent / list_by_date / list_by_status / list_by_category
 * - market_mainline_create / update / archive
 * - market_mainline_batch_upsert
 */

import { useMarketMainlineStore } from "@/stores";
import {
  type MarketMainline,
  parseEvidence,
  parseRepresentativeSymbols,
  type Persistence,
  type ThemeCategory,
} from "@/types";
import {
  Button,
  Card,
  Col,
  DatePicker,
  Empty,
  Input,
  message,
  Modal,
  Row,
  Select,
  Space,
  Statistic,
  Tag,
  Typography,
} from "antd";
import dayjs, { type Dayjs } from "dayjs";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Paragraph, Text } = Typography;

// 持续性 Tag 颜色映射
const PERSISTENCE_COLOR: Record<string, string> = {
  emerging: "green",
  "1m": "gold",
  "1w": "blue",
  "1d": "default",
  fading: "orange",
};

// 主题大类颜色
const CATEGORY_COLOR: Record<string, string> = {
  科技: "geekblue",
  消费: "magenta",
  周期: "orange",
  金融: "cyan",
  医药: "red",
  政策: "purple",
  其他: "default",
};

function strengthColor(score: number): string {
  if (score >= 80) { return "#f5222d"; }
  if (score >= 60) { return "#fa8c16"; }
  if (score >= 40) { return "#faad14"; }
  return "#8c8c8c";
}

function MainlineCard({
  mainline,
  onArchive,
}: {
  mainline: MarketMainline;
  onArchive: (id: string) => void;
}) {
  const { t } = useTranslation();
  const symbols = parseRepresentativeSymbols(mainline.representativeSymbols);
  const evidence = parseEvidence(mainline.evidenceJson);

  return (
    <Card
      data-testid={`mainline-card-${mainline.id}`}
      size="small"
      title={
        <Space>
          <Tag color={CATEGORY_COLOR[mainline.themeCategory] ?? "default"}>
            {mainline.themeCategory}
          </Tag>
          <Text strong>{mainline.theme}</Text>
          <Tag color={PERSISTENCE_COLOR[mainline.persistence] ?? "default"}>
            {mainline.persistence}
          </Tag>
          {mainline.status !== "active" && <Tag color="default">{mainline.status}</Tag>}
        </Space>
      }
      extra={
        <Button
          size="small"
          type="text"
          danger
          onClick={() => onArchive(mainline.id)}
          disabled={mainline.status === "archived"}
        >
          {t("marketMainline.archive")}
        </Button>
      }
    >
      <Paragraph style={{ marginBottom: 8 }}>{mainline.narrative}</Paragraph>

      <Row gutter={16}>
        <Col span={8}>
          <Statistic
            title={t("marketMainline.strength")}
            value={mainline.strengthScore}
            styles={{ content: { color: strengthColor(mainline.strengthScore), fontSize: 18 } }}
            suffix="/100"
          />
        </Col>
        <Col span={16}>
          <div style={{ marginBottom: 4 }}>
            <Text type="secondary">{t("marketMainline.representativeSymbols")}：</Text>
            <Space wrap size={[4, 4]} style={{ marginTop: 4 }}>
              {symbols.length === 0 ? <Text type="secondary">—</Text> : (
                symbols.map((s) => <Tag key={s}>{s}</Tag>)
              )}
            </Space>
          </div>
          <div>
            <Text type="secondary">{t("marketMainline.evidence")}：</Text>
            <Text code style={{ fontSize: 12 }}>
              {Object.keys(evidence).length === 0
                ? "—"
                : Object.entries(evidence)
                  .map(([k, v]) => `${k}=${typeof v === "object" ? JSON.stringify(v) : v}`)
                  .join(" / ")}
            </Text>
          </div>
        </Col>
      </Row>
    </Card>
  );
}

export function MarketMainlineDashboard() {
  const { t } = useTranslation();
  const {
    recentMainlines,
    dateMainlines,
    loadingRecent,
    loadingDate,
    submitting,
    error,
    fetchRecentMainlines,
    fetchMainlinesByDate,
    createMainline,
    archiveMainline,
    clearError,
  } = useMarketMainlineStore();

  const [selectedDate, setSelectedDate] = useState<Dayjs | null>(null);
  const [statusFilter, setStatusFilter] = useState<string>("active");
  const [categoryFilter, setCategoryFilter] = useState<string>("");
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [createForm, setCreateForm] = useState({
    mainlineDate: dayjs().format("YYYY-MM-DD"),
    theme: "",
    themeCategory: "其他" as ThemeCategory,
    narrative: "",
    representativeSymbols: "",
    strengthScore: 50,
    persistence: "1d" as Persistence,
  });

  // 启动时加载最近 7 天主线
  useEffect(() => {
    fetchRecentMainlines(7);
  }, [fetchRecentMainlines]);

  // 错误统一 toast
  useEffect(() => {
    if (error) {
      message.error(error);
      clearError();
    }
  }, [error, clearError]);

  // 按状态 / 大类过滤最近主线
  const filteredMainlines = useMemo(() => {
    return recentMainlines.filter((m) => {
      if (statusFilter && m.status !== statusFilter) { return false; }
      if (categoryFilter && m.themeCategory !== categoryFilter) { return false; }
      return true;
    });
  }, [recentMainlines, statusFilter, categoryFilter]);

  const handleSelectDate = (date: Dayjs | null) => {
    setSelectedDate(date);
    if (date) {
      fetchMainlinesByDate(date.format("YYYY-MM-DD"));
    }
  };

  const handleArchive = async (id: string) => {
    try {
      await archiveMainline(id);
      message.success(t("marketMainline.archiveSuccess"));
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleCreate = async () => {
    if (!createForm.theme.trim() || !createForm.narrative.trim()) {
      message.warning(t("marketMainline.createRequired"));
      return;
    }
    try {
      await createMainline({
        mainlineDate: createForm.mainlineDate,
        theme: createForm.theme.trim(),
        themeCategory: createForm.themeCategory,
        narrative: createForm.narrative.trim(),
        representativeSymbols: createForm.representativeSymbols
          .split(/[,，\s]+/)
          .map((s) => s.trim())
          .filter(Boolean),
        strengthScore: createForm.strengthScore,
        persistence: createForm.persistence,
        evidence: {},
        sourceWorkflowExecutionId: null,
      });
      message.success(t("marketMainline.createSuccess"));
      setCreateModalOpen(false);
      setCreateForm({
        mainlineDate: dayjs().format("YYYY-MM-DD"),
        theme: "",
        themeCategory: "其他",
        narrative: "",
        representativeSymbols: "",
        strengthScore: 50,
        persistence: "1d",
      });
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div style={{ padding: 16 }}>
      <Title level={3}>{t("marketMainline.title")}</Title>
      <Paragraph type="secondary">
        {t("marketMainline.subtitle")}
      </Paragraph>

      <Space style={{ marginBottom: 16 }} wrap>
        <Button
          type="primary"
          onClick={() => setCreateModalOpen(true)}
          loading={submitting}
        >
          {t("marketMainline.create")}
        </Button>
        <Button onClick={() => fetchRecentMainlines(7)} loading={loadingRecent}>
          {t("marketMainline.refresh")}
        </Button>
        <Select
          style={{ width: 140 }}
          placeholder={t("marketMainline.statusFilter")}
          value={statusFilter}
          onChange={setStatusFilter}
          options={[
            { label: "active", value: "active" },
            { label: "fading", value: "fading" },
            { label: "archived", value: "archived" },
          ]}
        />
        <Select
          style={{ width: 140 }}
          placeholder={t("marketMainline.categoryFilter")}
          value={categoryFilter}
          onChange={setCategoryFilter}
          allowClear
          options={[
            { label: "科技", value: "科技" },
            { label: "消费", value: "消费" },
            { label: "周期", value: "周期" },
            { label: "金融", value: "金融" },
            { label: "医药", value: "医药" },
            { label: "政策", value: "政策" },
            { label: "其他", value: "其他" },
          ]}
        />
        <DatePicker
          value={selectedDate}
          onChange={handleSelectDate}
          placeholder={t("marketMainline.selectDate")}
        />
      </Space>

      {selectedDate
        ? (
          <div data-testid="mainlines-by-date">
            <Title level={5}>
              {t("marketMainline.dateTitle")}：{selectedDate.format("YYYY-MM-DD")}
            </Title>
            {loadingDate
              ? <Card loading />
              : dateMainlines.length === 0
              ? <Empty description={t("marketMainline.noDataForDate")} />
              : (
                <Row gutter={[12, 12]}>
                  {dateMainlines.map((m) => (
                    <Col key={m.id} xs={24} md={12} xl={8}>
                      <MainlineCard mainline={m} onArchive={handleArchive} />
                    </Col>
                  ))}
                </Row>
              )}
          </div>
        )
        : (
          <div data-testid="mainlines-recent">
            {loadingRecent
              ? <Card loading />
              : filteredMainlines.length === 0
              ? <Empty description={t("marketMainline.noData")} />
              : (
                <Row gutter={[12, 12]}>
                  {filteredMainlines.map((m) => (
                    <Col key={m.id} xs={24} md={12} xl={8}>
                      <MainlineCard mainline={m} onArchive={handleArchive} />
                    </Col>
                  ))}
                </Row>
              )}
          </div>
        )}

      <Modal
        open={createModalOpen}
        title={t("marketMainline.createTitle")}
        onCancel={() => setCreateModalOpen(false)}
        onOk={handleCreate}
        confirmLoading={submitting}
        width={600}
      >
        <Space orientation="vertical" style={{ width: "100%" }} size="middle">
          <Row gutter={8}>
            <Col span={12}>
              <Text>{t("marketMainline.form.date")}</Text>
              <Input
                value={createForm.mainlineDate}
                onChange={(e) => setCreateForm({ ...createForm, mainlineDate: e.target.value })}
                placeholder="YYYY-MM-DD"
              />
            </Col>
            <Col span={12}>
              <Text>{t("marketMainline.form.category")}</Text>
              <Select
                style={{ width: "100%" }}
                value={createForm.themeCategory}
                onChange={(v: ThemeCategory) => setCreateForm({ ...createForm, themeCategory: v })}
                options={[
                  { label: "科技", value: "科技" },
                  { label: "消费", value: "消费" },
                  { label: "周期", value: "周期" },
                  { label: "金融", value: "金融" },
                  { label: "医药", value: "医药" },
                  { label: "政策", value: "政策" },
                  { label: "其他", value: "其他" },
                ]}
              />
            </Col>
          </Row>
          <div>
            <Text>{t("marketMainline.form.theme")}</Text>
            <Input
              value={createForm.theme}
              onChange={(e) => setCreateForm({ ...createForm, theme: e.target.value })}
              placeholder={t("marketMainline.form.themePlaceholder")}
            />
          </div>
          <div>
            <Text>{t("marketMainline.form.narrative")}</Text>
            <Input.TextArea
              rows={3}
              value={createForm.narrative}
              onChange={(e) => setCreateForm({ ...createForm, narrative: e.target.value })}
              placeholder={t("marketMainline.form.narrativePlaceholder")}
            />
          </div>
          <Row gutter={8}>
            <Col span={12}>
              <Text>{t("marketMainline.form.symbols")}</Text>
              <Input
                value={createForm.representativeSymbols}
                onChange={(e) => setCreateForm({ ...createForm, representativeSymbols: e.target.value })}
                placeholder="600519, 000858, 002230"
              />
            </Col>
            <Col span={6}>
              <Text>{t("marketMainline.form.score")}</Text>
              <Input
                type="number"
                min={0}
                max={100}
                value={createForm.strengthScore}
                onChange={(e) =>
                  setCreateForm({
                    ...createForm,
                    strengthScore: Number(e.target.value) || 0,
                  })}
              />
            </Col>
            <Col span={6}>
              <Text>{t("marketMainline.form.persistence")}</Text>
              <Select
                style={{ width: "100%" }}
                value={createForm.persistence}
                onChange={(v: Persistence) => setCreateForm({ ...createForm, persistence: v })}
                options={[
                  { label: "1d", value: "1d" },
                  { label: "1w", value: "1w" },
                  { label: "1m", value: "1m" },
                  { label: "fading", value: "fading" },
                  { label: "emerging", value: "emerging" },
                ]}
              />
            </Col>
          </Row>
        </Space>
      </Modal>
    </div>
  );
}
