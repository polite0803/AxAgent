// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G2 模拟观察组合（Paper Trading Portfolio）主面板
 *
 * 功能：
 * - 顶部统计卡片（active 组合数 / 总市值 / 总盈亏 / 总收益率）
 * - 组合列表（Card 视图，含持仓展开 + 实时盈亏）
 * - 创建组合 / 添加持仓 / 平仓 / 关闭组合 操作
 *
 * 数据源：usePaperPortfolioStore
 */

import { usePaperPortfolioStore } from "@/stores/feature/paperPortfolioStore";
import type { PortfolioDetail, PositionWithPnl } from "@/types/paper-portfolio";
import {
  Button,
  Card,
  DatePicker,
  Empty,
  Form,
  Input,
  InputNumber,
  message,
  Modal,
  Select,
  Space,
  Statistic,
  Table,
  Tag,
  Typography,
} from "antd";
import dayjs from "dayjs";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

// ── 工具函数 ──

/** 格式化人民币金额 */
function formatCNY(v: number): string {
  return v.toLocaleString("zh-CN", { style: "currency", currency: "CNY" });
}

/** 盈亏颜色 */
function pnlColor(v: number | null | undefined): string {
  if (v == null) { return "inherit"; }
  return v > 0 ? "#cf1322" : v < 0 ? "#3f8600" : "inherit"; // A 股红涨绿跌
}

// ── 子组件：组合卡片 ──

interface PortfolioCardProps {
  detail: PortfolioDetail;
  onAddPosition: (portfolioId: string) => void;
  onClosePosition: (position: PositionWithPnl) => void;
  onClosePortfolio: (portfolioId: string) => void;
}

function PortfolioCard({
  detail,
  onAddPosition,
  onClosePosition,
  onClosePortfolio,
}: PortfolioCardProps) {
  const { t } = useTranslation();
  const s = detail.summary;

  const columns = [
    {
      title: t("paperPortfolio.symbol"),
      dataIndex: "symbol",
      width: 100,
    },
    {
      title: t("paperPortfolio.market"),
      dataIndex: "market",
      width: 70,
      render: (m: string) => <Tag>{m}</Tag>,
    },
    {
      title: t("paperPortfolio.entryPrice"),
      dataIndex: "entryPrice",
      width: 90,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("paperPortfolio.entryDate"),
      dataIndex: "entryDate",
      width: 110,
    },
    {
      title: t("paperPortfolio.quantity"),
      dataIndex: "quantity",
      width: 90,
      render: (v: number) => v.toLocaleString(),
    },
    {
      title: t("paperPortfolio.currentPrice"),
      dataIndex: "currentPrice",
      width: 100,
      render: (v: number | null) => (v == null ? "—" : v.toFixed(2)),
    },
    {
      title: t("paperPortfolio.unrealizedPnl"),
      dataIndex: "unrealizedPnl",
      width: 110,
      render: (v: number | null) => v == null ? "—" : <span style={{ color: pnlColor(v) }}>{formatCNY(v)}</span>,
    },
    {
      title: t("paperPortfolio.realizedPnl"),
      dataIndex: "realizedPnl",
      width: 110,
      render: (v: number | null) => v == null ? "—" : <span style={{ color: pnlColor(v) }}>{formatCNY(v)}</span>,
    },
    {
      title: t("paperPortfolio.status"),
      dataIndex: "status",
      width: 80,
      render: (s: string) =>
        s === "open"
          ? <Tag color="processing">{t("paperPortfolio.statusOpen")}</Tag>
          : <Tag color="default">{t("paperPortfolio.statusClosed")}</Tag>,
    },
    {
      title: t("paperPortfolio.actions"),
      width: 100,
      render: (_: unknown, record: PositionWithPnl) =>
        record.status === "open"
          ? (
            <Button size="small" onClick={() => onClosePosition(record)}>
              {t("paperPortfolio.closePositionBtn")}
            </Button>
          )
          : null,
    },
  ];

  return (
    <Card
      title={
        <Space>
          <Text strong>{detail.name}</Text>
          <Tag color={detail.status === "active" ? "green" : "default"}>{detail.status}</Tag>
          <Text type="secondary">· {detail.sourceEvent}</Text>
        </Space>
      }
      extra={
        <Space>
          <Button size="small" onClick={() => onAddPosition(detail.id)}>
            {t("paperPortfolio.addPositionBtn")}
          </Button>
          {detail.status === "active" && (
            <Button size="small" danger onClick={() => onClosePortfolio(detail.id)}>
              {t("paperPortfolio.closePortfolioBtn")}
            </Button>
          )}
        </Space>
      }
      styles={{ body: { padding: 16 } }}
    >
      {/* 汇总指标 */}
      <Space size="large" wrap style={{ marginBottom: 12 }}>
        <Statistic
          title={t("paperPortfolio.totalCost")}
          value={s.totalCost}
          precision={2}
          prefix="¥"
        />
        <Statistic
          title={t("paperPortfolio.totalMarketValue")}
          value={s.totalMarketValue}
          precision={2}
          prefix="¥"
        />
        <Statistic
          title={t("paperPortfolio.totalUnrealizedPnl")}
          value={s.totalUnrealizedPnl}
          precision={2}
          prefix="¥"
          styles={{ content: { color: pnlColor(s.totalUnrealizedPnl) } }}
        />
        <Statistic
          title={t("paperPortfolio.totalRealizedPnl")}
          value={s.totalRealizedPnl}
          precision={2}
          prefix="¥"
          styles={{ content: { color: pnlColor(s.totalRealizedPnl) } }}
        />
        <Statistic
          title={t("paperPortfolio.totalReturnPct")}
          value={s.totalReturnPct}
          precision={2}
          suffix="%"
          styles={{ content: { color: pnlColor(s.totalReturnPct) } }}
        />
        <Statistic
          title={t("paperPortfolio.positionCount")}
          value={`${s.openCount}/${s.positionCount}`}
        />
      </Space>

      {/* 持仓表 */}
      <Table
        size="small"
        rowKey="id"
        columns={columns}
        dataSource={detail.positions}
        pagination={false}
        locale={{
          emptyText: <Empty description={t("paperPortfolio.noPositions")} />,
        }}
      />
    </Card>
  );
}

// ── 子组件：创建组合 Modal ──

interface CreatePortfolioModalProps {
  open: boolean;
  onClose: () => void;
}

function CreatePortfolioModal({ open, onClose }: CreatePortfolioModalProps) {
  const { t } = useTranslation();
  const { createPortfolio, submitting } = usePaperPortfolioStore();
  const [form] = Form.useForm();

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      await createPortfolio({
        name: values.name,
        sourceEvent: values.sourceEvent,
        sourceNewsId: values.sourceNewsId ?? null,
        sourceScreenshotDiagnosisId: values.sourceScreenshotDiagnosisId ?? null,
      });
      message.success(t("paperPortfolio.createSuccess"));
      form.resetFields();
      onClose();
    } catch {
      // validateFields 会抛出验证错误，createPortfolio 抛出 IPC 错误，均忽略
    }
  };

  return (
    <Modal
      title={t("paperPortfolio.createTitle")}
      open={open}
      onOk={handleOk}
      onCancel={onClose}
      confirmLoading={submitting}
      destroyOnHidden
    >
      <Form form={form} layout="vertical">
        <Form.Item
          name="name"
          label={t("paperPortfolio.nameLabel")}
          rules={[{ required: true, message: t("paperPortfolio.nameRequired") }]}
        >
          <Input placeholder={t("paperPortfolio.namePlaceholder")} />
        </Form.Item>
        <Form.Item
          name="sourceEvent"
          label={t("paperPortfolio.sourceEventLabel")}
          rules={[{ required: true, message: t("paperPortfolio.sourceEventRequired") }]}
        >
          <Input.TextArea rows={2} placeholder={t("paperPortfolio.sourceEventPlaceholder")} />
        </Form.Item>
        <Form.Item name="sourceNewsId" label={t("paperPortfolio.sourceNewsIdLabel")}>
          <Input placeholder={t("paperPortfolio.sourceNewsIdPlaceholder")} />
        </Form.Item>
        <Form.Item
          name="sourceScreenshotDiagnosisId"
          label={t("paperPortfolio.sourceScreenshotDiagnosisIdLabel")}
        >
          <Input placeholder={t("paperPortfolio.sourceScreenshotDiagnosisIdPlaceholder")} />
        </Form.Item>
      </Form>
    </Modal>
  );
}

// ── 子组件：添加持仓 Modal ──

interface AddPositionModalProps {
  portfolioId: string | null;
  onClose: () => void;
}

function AddPositionModal({ portfolioId, onClose }: AddPositionModalProps) {
  const { t } = useTranslation();
  const { addPosition, submitting } = usePaperPortfolioStore();
  const [form] = Form.useForm();

  const handleOk = async () => {
    if (!portfolioId) { return; }
    try {
      const values = await form.validateFields();
      await addPosition({
        portfolioId,
        symbol: values.symbol,
        market: values.market ?? "A",
        entryPrice: values.entryPrice,
        entryDate: values.entryDate.format("YYYY-MM-DD"),
        quantity: values.quantity,
        note: values.note ?? null,
      });
      message.success(t("paperPortfolio.addPositionSuccess"));
      form.resetFields();
      onClose();
    } catch {
      // 忽略
    }
  };

  return (
    <Modal
      title={t("paperPortfolio.addPositionTitle")}
      open={!!portfolioId}
      onOk={handleOk}
      onCancel={onClose}
      confirmLoading={submitting}
      destroyOnHidden
    >
      <Form form={form} layout="vertical" initialValues={{ market: "A" }}>
        <Form.Item
          name="symbol"
          label={t("paperPortfolio.symbolLabel")}
          rules={[{ required: true, message: t("paperPortfolio.symbolRequired") }]}
        >
          <Input placeholder={t("paperPortfolio.symbolPlaceholder")} />
        </Form.Item>
        <Form.Item name="market" label={t("paperPortfolio.marketLabel")}>
          <Select
            options={[
              { value: "A", label: t("paperPortfolio.marketA") },
              { value: "US", label: t("paperPortfolio.marketUS") },
              { value: "HK", label: t("paperPortfolio.marketHK") },
              { value: "ETF", label: "ETF" },
            ]}
          />
        </Form.Item>
        <Form.Item
          name="entryPrice"
          label={t("paperPortfolio.entryPriceLabel")}
          rules={[{ required: true, message: t("paperPortfolio.entryPriceRequired") }]}
        >
          <InputNumber min={0} step={0.01} style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item
          name="entryDate"
          label={t("paperPortfolio.entryDateLabel")}
          rules={[{ required: true, message: t("paperPortfolio.entryDateRequired") }]}
        >
          <DatePicker style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item
          name="quantity"
          label={t("paperPortfolio.quantityLabel")}
          rules={[{ required: true, message: t("paperPortfolio.quantityRequired") }]}
        >
          <InputNumber min={0} step={100} style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item name="note" label={t("paperPortfolio.noteLabel")}>
          <Input placeholder={t("paperPortfolio.notePlaceholder")} />
        </Form.Item>
      </Form>
    </Modal>
  );
}

// ── 子组件：平仓 Modal ──

interface ClosePositionModalProps {
  position: PositionWithPnl | null;
  onClose: () => void;
}

function ClosePositionModal({ position, onClose }: ClosePositionModalProps) {
  const { t } = useTranslation();
  const { closePosition, submitting } = usePaperPortfolioStore();
  const [form] = Form.useForm();

  const handleOk = async () => {
    if (!position) { return; }
    try {
      const values = await form.validateFields();
      await closePosition({
        positionId: position.id,
        exitPrice: values.exitPrice,
        exitDate: values.exitDate.format("YYYY-MM-DD"),
      });
      message.success(t("paperPortfolio.closePositionSuccess"));
      form.resetFields();
      onClose();
    } catch {
      // 忽略
    }
  };

  return (
    <Modal
      title={t("paperPortfolio.closePositionTitle")}
      open={!!position}
      onOk={handleOk}
      onCancel={onClose}
      confirmLoading={submitting}
      destroyOnHidden
    >
      {position && (
        <Paragraph>
          {t("paperPortfolio.closePositionConfirm", {
            symbol: position.symbol,
            entryPrice: position.entryPrice.toFixed(2),
          })}
        </Paragraph>
      )}
      <Form form={form} layout="vertical" initialValues={{ exitDate: dayjs() }}>
        <Form.Item
          name="exitPrice"
          label={t("paperPortfolio.exitPriceLabel")}
          rules={[{ required: true, message: t("paperPortfolio.exitPriceRequired") }]}
        >
          <InputNumber min={0} step={0.01} style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item
          name="exitDate"
          label={t("paperPortfolio.exitDateLabel")}
          rules={[{ required: true, message: t("paperPortfolio.exitDateRequired") }]}
        >
          <DatePicker style={{ width: "100%" }} />
        </Form.Item>
      </Form>
    </Modal>
  );
}

// ── 主组件 ──

export function PaperPortfolioDashboard() {
  const { t } = useTranslation();
  const {
    activeDetails,
    loadingList,
    fetchActiveDetails,
    closePortfolio,
  } = usePaperPortfolioStore();

  const [createOpen, setCreateOpen] = useState(false);
  const [addPositionTo, setAddPositionTo] = useState<string | null>(null);
  const [closePositionTarget, setClosePositionTarget] = useState<PositionWithPnl | null>(null);

  useEffect(() => {
    fetchActiveDetails();
  }, [fetchActiveDetails]);

  // 顶部聚合指标
  const totalPortfolios = activeDetails.length;
  const totalMarketValue = activeDetails.reduce(
    (sum, p) => sum + p.summary.totalMarketValue,
    0,
  );
  const totalPnl = activeDetails.reduce(
    (sum, p) => sum + p.summary.totalUnrealizedPnl + p.summary.totalRealizedPnl,
    0,
  );
  const totalCost = activeDetails.reduce((sum, p) => sum + p.summary.totalCost, 0);
  const totalReturnPct = totalCost > 0 ? (totalPnl / totalCost) * 100 : 0;

  return (
    <div style={{ padding: 16 }}>
      {/* 顶部操作栏 */}
      <Space style={{ marginBottom: 16 }}>
        <Button type="primary" onClick={() => setCreateOpen(true)}>
          {t("paperPortfolio.createBtn")}
        </Button>
        <Button onClick={() => fetchActiveDetails()} loading={loadingList}>
          {t("paperPortfolio.refreshBtn")}
        </Button>
      </Space>

      {/* 顶部统计卡片 */}
      <Space size="large" wrap style={{ marginBottom: 16 }}>
        <Statistic title={t("paperPortfolio.activePortfolios")} value={totalPortfolios} />
        <Statistic
          title={t("paperPortfolio.totalMarketValueAll")}
          value={totalMarketValue}
          precision={2}
          prefix="¥"
        />
        <Statistic
          title={t("paperPortfolio.totalPnlAll")}
          value={totalPnl}
          precision={2}
          prefix="¥"
          styles={{ content: { color: pnlColor(totalPnl) } }}
        />
        <Statistic
          title={t("paperPortfolio.totalReturnPctAll")}
          value={totalReturnPct}
          precision={2}
          suffix="%"
          styles={{ content: { color: pnlColor(totalReturnPct) } }}
        />
      </Space>

      {/* 组合列表 */}
      {activeDetails.length === 0
        ? (
          <Empty description={t("paperPortfolio.noPortfolios")}>
            <Button type="primary" onClick={() => setCreateOpen(true)}>
              {t("paperPortfolio.createFirstPortfolio")}
            </Button>
          </Empty>
        )
        : (
          <Space orientation="vertical" size="large" style={{ width: "100%" }}>
            {activeDetails.map((d) => (
              <PortfolioCard
                key={d.id}
                detail={d}
                onAddPosition={(id) => setAddPositionTo(id)}
                onClosePosition={(pos) => setClosePositionTarget(pos)}
                onClosePortfolio={async (id) => {
                  await closePortfolio(id);
                  message.success(t("paperPortfolio.closePortfolioSuccess"));
                }}
              />
            ))}
          </Space>
        )}

      {/* Modals */}
      <CreatePortfolioModal open={createOpen} onClose={() => setCreateOpen(false)} />
      <AddPositionModal
        portfolioId={addPositionTo}
        onClose={() => setAddPositionTo(null)}
      />
      <ClosePositionModal
        position={closePositionTarget}
        onClose={() => setClosePositionTarget(null)}
      />
    </div>
  );
}
