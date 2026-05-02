import { invoke } from "@/lib/invoke";
import {
  Button,
  Card,
  Empty,
  Form,
  Input,
  message,
  Modal,
  Popconfirm,
  Select,
  Spin,
  Switch,
  Table,
  Tag,
  Typography,
} from "antd";
import { Bell, BellOff, Copy, Plus, RefreshCw, Trash2, Webhook } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph, Title } = Typography;

interface WebhookSubscription {
  id: string;
  url: string;
  events: string[];
  secret?: string;
  enabled: boolean;
  created_at: string;
  last_triggered?: string;
  failure_count: number;
}

const EVENT_OPTIONS = [
  { value: "tool_complete", label: "Tool Complete" },
  { value: "tool_error", label: "Tool Error" },
  { value: "agent_start", label: "Agent Start" },
  { value: "agent_end", label: "Agent End" },
  { value: "agent_error", label: "Agent Error" },
  { value: "session_start", label: "Session Start" },
  { value: "session_end", label: "Session End" },
  { value: "message_received", label: "Message Received" },
  { value: "message_sent", label: "Message Sent" },
];

const EVENT_COLORS: Record<string, string> = {
  tool_complete: "green",
  tool_error: "red",
  agent_start: "blue",
  agent_end: "cyan",
  agent_error: "orange",
  session_start: "purple",
  session_end: "magenta",
  message_received: "geekblue",
  message_sent: "lime",
};

export default function WebhookSettings() {
  const { t } = useTranslation();
  const [subscriptions, setSubscriptions] = useState<WebhookSubscription[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);

  const loadSubscriptions = async () => {
    try {
      const result = await invoke<WebhookSubscription[]>(
        "webhook_list_subscriptions",
      );
      setSubscriptions(result);
    } catch (error) {
      message.error(`Failed to load subscriptions: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadSubscriptions();
  }, []);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await invoke("webhook_reload");
      await loadSubscriptions();
      message.success(t("settings.webhook.refreshSuccess"));
    } catch (error) {
      message.error(`Refresh failed: ${error}`);
    } finally {
      setRefreshing(false);
    }
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      await invoke("webhook_toggle_subscription", {
        subscriptionId: id,
        enabled,
      });
      setSubscriptions((prev) => prev.map((s) => (s.id === id ? { ...s, enabled } : s)));
      message.success(
        enabled
          ? t("settings.webhook.enabled")
          : t("settings.webhook.disabled"),
      );
    } catch (error) {
      message.error(`Toggle failed: ${error}`);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("webhook_delete_subscription", {
        subscriptionId: id,
      });
      setSubscriptions((prev) => prev.filter((s) => s.id !== id));
      message.success(t("settings.webhook.deleted"));
    } catch (error) {
      message.error(`Delete failed: ${error}`);
    }
  };

  const handleCreate = async (values: {
    url: string;
    events: string[];
    secret?: string;
  }) => {
    setSubmitting(true);
    try {
      const result = await invoke<WebhookSubscription>(
        "webhook_create_subscription",
        {
          url: values.url,
          events: values.events,
          secret: values.secret,
        },
      );
      setSubscriptions((prev) => [...prev, result]);
      setModalVisible(false);
      form.resetFields();
      message.success(t("settings.webhook.created"));
    } catch (error) {
      message.error(`Create failed: ${error}`);
    } finally {
      setSubmitting(false);
    }
  };

  const handleTest = async (id: string) => {
    try {
      await invoke("webhook_test_subscription", {
        subscriptionId: id,
      });
      message.success(t("settings.webhook.testSuccess"));
    } catch (error) {
      message.error(`Test failed: ${error}`);
    }
  };

  const columns = [
    {
      title: t("settings.webhook.url"),
      dataIndex: "url",
      key: "url",
      render: (url: string) => (
        <div className="flex items-center gap-2">
          <Webhook size={14} className="text-text-secondary" />
          <Text className="font-mono text-sm truncate max-w-75" title={url}>
            {url}
          </Text>
          <Copy
            size={14}
            className="cursor-pointer text-text-quaternary hover:text-text-secondary"
            onClick={() => {
              navigator.clipboard.writeText(url);
              message.success(t("settings.webhook.copied"));
            }}
          />
        </div>
      ),
    },
    {
      title: t("settings.webhook.events"),
      dataIndex: "events",
      key: "events",
      render: (events: string[]) => (
        <div className="flex flex-wrap gap-1">
          {events.map((event) => (
            <Tag
              key={event}
              color={EVENT_COLORS[event] || "default"}
              className="text-xs"
            >
              {event}
            </Tag>
          ))}
        </div>
      ),
    },
    {
      title: t("settings.webhook.status"),
      dataIndex: "enabled",
      key: "enabled",
      width: 100,
      render: (enabled: boolean, record: WebhookSubscription) => (
        <Switch
          checked={enabled}
          onChange={(checked) => handleToggle(record.id, checked)}
        />
      ),
    },
    {
      title: t("settings.webhook.lastTriggered"),
      dataIndex: "last_triggered",
      key: "last_triggered",
      width: 160,
      render: (ts: string | undefined) => ts ? new Date(ts).toLocaleString() : "-",
    },
    {
      title: t("settings.webhook.failures"),
      dataIndex: "failure_count",
      key: "failure_count",
      width: 80,
      render: (count: number) => count > 0 ? <Tag color="red">{count}</Tag> : <Tag color="green">0</Tag>,
    },
    {
      title: "",
      key: "actions",
      width: 150,
      render: (_: unknown, record: WebhookSubscription) => (
        <div className="flex gap-1">
          <Button
            type="text"
            size="small"
            onClick={() => handleTest(record.id)}
            title={t("settings.webhook.test")}
          >
            <Bell size={14} />
          </Button>
          <Popconfirm
            title={t("settings.webhook.deleteConfirm")}
            onConfirm={() => handleDelete(record.id)}
            okText={t("common.confirm")}
            cancelText={t("common.cancel")}
            okButtonProps={{ danger: true }}
          >
            <Button
              type="text"
              danger
              size="small"
              icon={<Trash2 size={14} />}
            />
          </Popconfirm>
        </div>
      ),
    },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48">
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div className="w-full">
      <div className="flex items-center justify-between mb-6">
        <div>
          <Title level={4}>{t("settings.webhook.title")}</Title>
          <Paragraph type="secondary">
            {t("settings.webhook.description")}
          </Paragraph>
        </div>
        <div className="flex gap-2">
          <Button
            icon={<RefreshCw size={16} className={refreshing ? "animate-spin" : ""} />}
            onClick={handleRefresh}
            loading={refreshing}
          >
            {t("settings.webhook.refresh")}
          </Button>
          <Button
            type="primary"
            icon={<Plus size={16} />}
            onClick={() => setModalVisible(true)}
          >
            {t("settings.webhook.addSubscription")}
          </Button>
        </div>
      </div>

      {subscriptions.length > 0
        ? (
          <Table
            dataSource={subscriptions}
            columns={columns}
            rowKey="id"
            pagination={false}
          />
        )
        : (
          <Card>
            <Empty
              image={<BellOff size={48} className="text-text-quaternary" />}
              description={
                <div>
                  <Paragraph>{t("settings.webhook.noSubscriptions")}</Paragraph>
                  <Button
                    type="primary"
                    icon={<Plus size={16} />}
                    onClick={() => setModalVisible(true)}
                  >
                    {t("settings.webhook.addFirst")}
                  </Button>
                </div>
              }
            />
          </Card>
        )}

      <Modal
        title={t("settings.webhook.createSubscription")}
        open={modalVisible}
        onCancel={() => {
          setModalVisible(false);
          form.resetFields();
        }}
        footer={null}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleCreate}
          className="mt-4"
        >
          <Form.Item
            name="url"
            label={t("settings.webhook.url")}
            rules={[
              { required: true, message: t("settings.webhook.urlRequired") },
              { type: "url", message: t("settings.webhook.urlInvalid") },
            ]}
          >
            <Input
              placeholder="https://example.com/webhook"
              prefix={<Webhook size={14} />}
            />
          </Form.Item>

          <Form.Item
            name="events"
            label={t("settings.webhook.events")}
            rules={[
              { required: true, message: t("settings.webhook.eventsRequired") },
            ]}
          >
            <Select
              mode="multiple"
              placeholder={t("settings.webhook.selectEvents")}
              options={EVENT_OPTIONS}
            />
          </Form.Item>

          <Form.Item name="secret" label={t("settings.webhook.secret")}>
            <Input.Password
              placeholder={t("settings.webhook.secretPlaceholder")}
            />
          </Form.Item>

          <Form.Item className="mb-0">
            <div className="flex justify-end gap-2">
              <Button
                onClick={() => {
                  setModalVisible(false);
                  form.resetFields();
                }}
              >
                {t("common.cancel")}
              </Button>
              <Button type="primary" htmlType="submit" loading={submitting}>
                {t("settings.webhook.create")}
              </Button>
            </div>
          </Form.Item>
        </Form>
      </Modal>

      <Card className="mt-6">
        <Title level={5}>{t("settings.webhook.eventReference")}</Title>
        <Paragraph type="secondary" className="mb-4">
          {t("settings.webhook.eventReferenceDescription")}
        </Paragraph>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <Text strong>{t("settings.webhook.agentEvents")}</Text>
            <div className="flex flex-wrap gap-1 mt-2">
              <Tag color="blue">agent_start</Tag>
              <Tag color="cyan">agent_end</Tag>
              <Tag color="orange">agent_error</Tag>
            </div>
          </div>
          <div>
            <Text strong>{t("settings.webhook.toolEvents")}</Text>
            <div className="flex flex-wrap gap-1 mt-2">
              <Tag color="green">tool_complete</Tag>
              <Tag color="red">tool_error</Tag>
            </div>
          </div>
          <div>
            <Text strong>{t("settings.webhook.sessionEvents")}</Text>
            <div className="flex flex-wrap gap-1 mt-2">
              <Tag color="purple">session_start</Tag>
              <Tag color="magenta">session_end</Tag>
            </div>
          </div>
          <div>
            <Text strong>{t("settings.webhook.messageEvents")}</Text>
            <div className="flex flex-wrap gap-1 mt-2">
              <Tag color="geekblue">message_received</Tag>
              <Tag color="lime">message_sent</Tag>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
