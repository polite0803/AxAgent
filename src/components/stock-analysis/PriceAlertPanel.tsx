import { invoke, listen } from "@/lib/invoke";
import type { UnlistenFn } from "@/lib/invoke";
import { BellOutlined, DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { App, Badge, Button, Card, Form, Input, InputNumber, Select, Table, Tabs, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface PriceAlert {
  id: string;
  stockCode: string;
  stockName: string;
  condition: string;
  targetPrice: number;
  isTriggered: boolean;
  triggeredAt: number | null;
  createdAt: number;
}

export function PriceAlertPanel() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [alerts, setAlerts] = useState<PriceAlert[]>([]);
  const [history, setHistory] = useState<PriceAlert[]>([]);
  const [adding, setAdding] = useState(false);
  const [form] = Form.useForm();
  const [activeTab, setActiveTab] = useState("active");

  const loadAlerts = async () => {
    try {
      const list = await invoke<PriceAlert[]>("list_price_alerts");
      if (Array.isArray(list)) {
        setAlerts(list.filter((a) => !a.isTriggered));
        setHistory(list.filter((a) => a.isTriggered));
      }
    } catch { /* 后端未运行 */ }
  };

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      return invoke<PriceAlert[]>("list_price_alerts");
    })
      .then((list) => {
        if (cancelled || !list) { return; }
        if (Array.isArray(list)) {
          setAlerts(list.filter((a) => !a.isTriggered));
          setHistory(list.filter((a) => a.isTriggered));
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;
    listen<PriceAlert>("price-alert-triggered", () => {
      if (cancelled) { return; }
      message.info(t("stockAnalysis.alert.triggered"));
      loadAlerts();
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch(() => {/* 可选 */});
    return () => {
      if (unlisten) { unlisten(); }
      cancelled = true;
    };
  }, [t, message]);

  const addAlert = async (values: Record<string, unknown>) => {
    try {
      await invoke("create_price_alert", {
        stockCode: values.stockCode,
        stockName: values.stockName,
        condition: values.condition,
        targetPrice: values.targetPrice,
      });
      form.resetFields();
      setAdding(false);
      loadAlerts();
    } catch { /* 静默 */ }
  };

  const removeAlert = async (id: string) => {
    try {
      await invoke("delete_price_alert", { id });
      loadAlerts();
    } catch { /* 静默 */ }
  };

  const columns = [
    {
      title: t("stockAnalysis.alert.code"),
      dataIndex: "stockCode",
      width: 60,
      render: (v: string) => <Tag className="text-xs m-0">{v}</Tag>,
    },
    { title: t("stockAnalysis.alert.name"), dataIndex: "stockName", ellipsis: true as const },
    {
      title: t("stockAnalysis.alert.condition"),
      dataIndex: "condition",
      width: 50,
      render: (v: string) => (
        <Tag color={v === "above" ? "red" : "green"} className="text-xs m-0">
          {v === "above" ? "≥" : v === "below" ? "≤" : v}
        </Tag>
      ),
    },
    {
      title: t("stockAnalysis.alert.price"),
      dataIndex: "targetPrice",
      width: 60,
      align: "right" as const,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: "",
      key: "action",
      width: 30,
      render: (_: unknown, record: PriceAlert) => (
        <Button
          size="small"
          type="text"
          danger
          icon={<DeleteOutlined />}
          onClick={() => removeAlert(record.id)}
        />
      ),
    },
  ];

  const historyColumns = [
    ...columns.slice(0, 4),
    {
      title: "触发时间",
      dataIndex: "triggeredAt",
      width: 80,
      render: (v: number | null) => v ? new Date(v).toLocaleDateString() : "—",
    },
    columns[4],
  ];

  return (
    <Card
      size="small"
      styles={{ body: { padding: "8px 8px" } }}
      title={
        <span>
          <BellOutlined /> {t("stockAnalysis.alert.title")}
        </span>
      }
      extra={
        <div className="flex gap-1 items-center">
          <Badge
            status={alerts.length > 0 ? "processing" : "default"}
            text={alerts.length > 0 ? `${alerts.length} 活跃` : "无"}
          />
          <Button size="small" icon={<PlusOutlined />} onClick={() => setAdding(!adding)} />
        </div>
      }
    >
      {adding && (
        <Form form={form} size="small" onFinish={addAlert} className="mb-2">
          <div className="flex gap-1 mb-1 flex-wrap">
            <Form.Item name="stockCode" rules={[{ required: true }]} noStyle>
              <Input placeholder={t("stockAnalysis.alert.code")} style={{ width: 72 }} size="small" />
            </Form.Item>
            <Form.Item name="stockName" rules={[{ required: true }]} noStyle>
              <Input placeholder={t("stockAnalysis.alert.name")} style={{ width: 72 }} size="small" />
            </Form.Item>
            <Form.Item name="condition" initialValue="above" noStyle>
              <Select style={{ width: 80 }} size="small">
                <Select.Option value="above">≥ 价格突破</Select.Option>
                <Select.Option value="below">≤ 价格跌破</Select.Option>
                <Select.Option value="change_up">↗ 涨幅超</Select.Option>
                <Select.Option value="change_down">↘ 跌幅超</Select.Option>
                <Select.Option value="volume_spike">📊 放量</Select.Option>
              </Select>
            </Form.Item>
          </div>
          <div className="flex gap-1">
            <Form.Item name="targetPrice" rules={[{ required: true }]} noStyle>
              <InputNumber
                size="small"
                placeholder={t("stockAnalysis.alert.price")}
                style={{ width: 100 }}
                min={0}
                step={0.01}
              />
            </Form.Item>
            <Form.Item noStyle>
              <Button type="primary" htmlType="submit" size="small">{t("common.confirm")}</Button>
            </Form.Item>
          </div>
        </Form>
      )}
      <Tabs
        size="small"
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: "active",
            label: `活跃 (${alerts.length})`,
            children: (
              <Table
                dataSource={alerts}
                columns={columns}
                rowKey="id"
                size="small"
                pagination={false}
                locale={{ emptyText: t("stockAnalysis.alert.empty") }}
              />
            ),
          },
          {
            key: "history",
            label: `已触发 (${history.length})`,
            children: (
              <Table
                dataSource={history.slice(0, 20)}
                columns={historyColumns}
                rowKey="id"
                size="small"
                pagination={false}
                locale={{ emptyText: "无触发记录" }}
              />
            ),
          },
        ]}
      />
    </Card>
  );
}
