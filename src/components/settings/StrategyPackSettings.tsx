import { invoke } from "@/lib/invoke";
import { DeleteOutlined, ReloadOutlined, SaveOutlined, SearchOutlined } from "@ant-design/icons";
import {
  App,
  Badge,
  Button,
  Card,
  Col,
  Empty,
  Input,
  Modal,
  Popconfirm,
  Row,
  Space,
  Statistic,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface StrategyPackInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  source: "builtin" | "user";
  strategyCount: number;
  enabledCount: number;
  enabled: boolean;
  minConfidence: number;
  maxPicks: number;
}

interface StrategyPackStrategyEntry {
  id: string;
  strategyId: string;
  style: string;
  period: string;
  enabled: boolean;
  weight: number;
  params: Record<string, unknown>;
  minConfidence?: number | null;
}

interface StrategyPackDetail extends StrategyPackInfo {
  spec: {
    name: string;
    description: string;
    version: string;
    author: string;
    minConfidence: number;
    maxPicks: number;
    strategies: StrategyPackStrategyEntry[];
  };
  templateVars: Record<string, unknown>;
}

const STYLE_COLORS: Record<string, string> = {
  trend: "blue",
  value: "green",
  capital: "orange",
  reversion: "red",
  watchlist: "purple",
  serenity: "cyan",
};

const PERIOD_KEYS = ["ultra_short", "short", "mid", "long"] as const;

export function StrategyPackSettings() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [packs, setPacks] = useState<StrategyPackInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [detail, setDetail] = useState<StrategyPackDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);
  const [yamlInput, setYamlInput] = useState("");
  const [yamlId, setYamlId] = useState("");

  const loadPacks = async () => {
    setLoading(true);
    try {
      const list = await invoke<StrategyPackInfo[]>("list_strategy_packs");
      if (Array.isArray(list)) { setPacks(list); }
    } catch (e) {
      message.error(t("stockAnalysis.settings.loadPackFailed", { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPacks();
  }, []);

  const filteredPacks = useMemo(() => {
    if (!search) { return packs; }
    const lower = search.toLowerCase();
    return packs.filter(
      (p) =>
        p.name.toLowerCase().includes(lower)
        || p.description.toLowerCase().includes(lower)
        || p.id.toLowerCase().includes(lower),
    );
  }, [packs, search]);

  const builtinCount = packs.filter((p) => p.source === "builtin").length;
  const userCount = packs.filter((p) => p.source === "user").length;

  const showDetail = async (record: StrategyPackInfo) => {
    setDetailLoading(true);
    try {
      const d = await invoke<StrategyPackDetail>("get_strategy_pack_detail", {
        id: record.id,
        source: record.source,
      });
      setDetail(d);
    } catch (e) {
      message.error(t("stockAnalysis.settings.loadDetailFailed", { error: String(e) }));
    } finally {
      setDetailLoading(false);
    }
  };

  const openEditor = (preset?: StrategyPackInfo) => {
    if (preset) {
      setYamlId(preset.source === "user" ? preset.id : `${preset.id}-copy`);
      // 加载完整 YAML 用于编辑
      invoke<StrategyPackDetail>("get_strategy_pack_detail", {
        id: preset.id,
        source: preset.source,
      })
        .then((d) => {
          // 用 spec 生成 YAML（简化：用 JSON 展示）
          setYamlInput(JSON.stringify(d.spec, null, 2));
        })
        .catch(() => {
          setYamlInput("");
        });
    } else {
      setYamlId("");
      setYamlInput(`name: "${t("settings.strategyPack.editorTemplate.name")}"
description: "${t("settings.strategyPack.editorTemplate.description")}"
version: "1.0.0"
author: "user"
minConfidence: 65
maxPicks: 8
strategies:
  - id: "trend_short"
    strategyId: "trend"
    style: "trend"
    period: "short"
    enabled: true
    weight: 1.0
    params:
      trend_ma_short_1: 5
      trend_ma_short_2: 10
      trend_ma_short_3: 20
`);
    }
    setEditorOpen(true);
  };

  const validateYaml = async () => {
    try {
      await invoke("validate_strategy_pack_yaml", { yaml: yamlInput });
      message.success(t("stockAnalysis.settings.validatePassed"));
    } catch (e) {
      message.error(t("stockAnalysis.settings.validateFailed", { error: String(e) }));
    }
  };

  const saveYaml = async () => {
    if (!yamlId.trim()) {
      message.warning(t("stockAnalysis.settings.packIdRequired"));
      return;
    }
    try {
      await invoke("save_user_strategy_pack", { id: yamlId, yaml: yamlInput });
      message.success(t("stockAnalysis.settings.saveSuccess"));
      setEditorOpen(false);
      loadPacks();
    } catch (e) {
      message.error(t("stockAnalysis.settings.saveFailed", { error: String(e) }));
    }
  };

  const deleteUserPack = async (id: string) => {
    try {
      await invoke("delete_user_strategy_pack", { id });
      message.success(t("stockAnalysis.settings.deleteSuccess"));
      loadPacks();
    } catch (e) {
      message.error(t("stockAnalysis.settings.deleteFailed", { error: String(e) }));
    }
  };

  const columns = [
    {
      title: t("settings.strategyPack.column.name"),
      dataIndex: "name",
      key: "name",
      render: (text: string, record: StrategyPackInfo) => (
        <Space orientation="vertical" size={0}>
          <Text strong>{text}</Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {record.id} · v{record.version}
          </Text>
        </Space>
      ),
    },
    {
      title: t("settings.strategyPack.column.description"),
      dataIndex: "description",
      key: "description",
      ellipsis: true,
    },
    {
      title: t("settings.strategyPack.column.source"),
      dataIndex: "source",
      key: "source",
      width: 90,
      render: (source: string) =>
        source === "builtin"
          ? <Tag color="blue">{t("settings.strategyPack.source.builtin")}</Tag>
          : <Tag color="orange">{t("settings.strategyPack.source.user")}</Tag>,
    },
    {
      title: t("settings.strategyPack.column.strategies"),
      key: "strategies",
      width: 100,
      render: (_: unknown, record: StrategyPackInfo) => (
        <Text>
          {record.enabledCount}/{record.strategyCount}
        </Text>
      ),
    },
    {
      title: t("settings.strategyPack.column.minConfidence"),
      dataIndex: "minConfidence",
      key: "minConfidence",
      width: 110,
      render: (v: number) => <Tag color={v >= 80 ? "red" : v >= 70 ? "orange" : "default"}>{v}%</Tag>,
    },
    {
      title: t("settings.strategyPack.column.maxPicks"),
      dataIndex: "maxPicks",
      key: "maxPicks",
      width: 90,
    },
    {
      title: t("settings.strategyPack.column.actions"),
      key: "actions",
      width: 180,
      render: (_: unknown, record: StrategyPackInfo) => (
        <Space size="small">
          <Button size="small" onClick={() => showDetail(record)}>
            {t("settings.strategyPack.action.detail")}
          </Button>
          <Button size="small" onClick={() => openEditor(record)}>
            {t("settings.strategyPack.action.copy")}
          </Button>
          {record.source === "user" && (
            <Popconfirm
              title={t("settings.strategyPack.confirmDelete")}
              onConfirm={() => deleteUserPack(record.id)}
            >
              <Button size="small" danger icon={<DeleteOutlined />} />
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      <Row gutter={16}>
        <Col span={8}>
          <Card>
            <Statistic
              title={t("settings.strategyPack.stat.total")}
              value={packs.length}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic
              title={t("settings.strategyPack.stat.builtin")}
              value={builtinCount}
              styles={{ content: { color: "#1677ff" } }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic
              title={t("settings.strategyPack.stat.user")}
              value={userCount}
              styles={{ content: { color: "#fa8c16" } }}
            />
          </Card>
        </Col>
      </Row>

      <Card
        title={t("settings.strategyPack.title")}
        extra={
          <Space>
            <Input
              placeholder={t("settings.strategyPack.searchPlaceholder")}
              prefix={<SearchOutlined />}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              allowClear
              style={{ width: 240 }}
            />
            <Button icon={<ReloadOutlined />} onClick={loadPacks} loading={loading}>
              {t("settings.strategyPack.action.refresh")}
            </Button>
            <Button type="primary" icon={<SaveOutlined />} onClick={() => openEditor()}>
              {t("settings.strategyPack.action.new")}
            </Button>
          </Space>
        }
      >
        <Table
          dataSource={filteredPacks}
          columns={columns}
          rowKey={(r) => `${r.source}-${r.id}`}
          loading={loading}
          pagination={{ pageSize: 10, showSizeChanger: true }}
          locale={{
            emptyText: <Empty description={t("settings.strategyPack.empty")} />,
          }}
        />
      </Card>

      <Modal
        title={detail?.name ?? t("settings.strategyPack.detailTitle")}
        open={!!detail}
        onCancel={() => setDetail(null)}
        footer={[
          <Button key="close" onClick={() => setDetail(null)}>
            {t("settings.strategyPack.action.close")}
          </Button>,
        ]}
        width={800}
        loading={detailLoading}
      >
        {detail && (
          <div className="space-y-4">
            <Paragraph type="secondary">{detail.description}</Paragraph>
            <Row gutter={16}>
              <Col span={6}>
                <Statistic
                  title={t("stockAnalysis.settings.statId")}
                  value={detail.id}
                  styles={{ content: { fontSize: 14 } }}
                />
              </Col>
              <Col span={6}>
                <Statistic title={t("stockAnalysis.settings.statVersion")} value={detail.version} />
              </Col>
              <Col span={6}>
                <Statistic title={t("stockAnalysis.settings.statAuthor")} value={detail.author} />
              </Col>
              <Col span={6}>
                <Statistic title={t("stockAnalysis.settings.statSource")} value={detail.source} />
              </Col>
            </Row>
            <Table
              dataSource={detail.spec.strategies}
              rowKey="id"
              pagination={false}
              size="small"
              columns={[
                {
                  title: "ID",
                  dataIndex: "id",
                  key: "id",
                },
                {
                  title: t("settings.strategyPack.column.style"),
                  dataIndex: "style",
                  key: "style",
                  render: (s: string) => <Tag color={STYLE_COLORS[s] ?? "default"}>{s}</Tag>,
                },
                {
                  title: t("settings.strategyPack.column.period"),
                  dataIndex: "period",
                  key: "period",
                  render: (p: string) =>
                    PERIOD_KEYS.includes(p as (typeof PERIOD_KEYS)[number])
                      ? t(`settings.strategyPack.period.${p}`)
                      : p,
                },
                {
                  title: t("settings.strategyPack.column.enabled"),
                  dataIndex: "enabled",
                  key: "enabled",
                  render: (e: boolean) =>
                    e
                      ? <Badge status="success" text={t("settings.strategyPack.yes")} />
                      : <Badge status="default" text={t("settings.strategyPack.no")} />,
                },
                {
                  title: t("settings.strategyPack.column.weight"),
                  dataIndex: "weight",
                  key: "weight",
                  render: (w: number) => <Text>{w.toFixed(2)}</Text>,
                },
              ]}
            />
          </div>
        )}
      </Modal>

      <Modal
        title={t("settings.strategyPack.editorTitle")}
        open={editorOpen}
        onCancel={() => setEditorOpen(false)}
        footer={[
          <Button key="cancel" onClick={() => setEditorOpen(false)}>
            {t("settings.strategyPack.action.cancel")}
          </Button>,
          <Button key="validate" onClick={validateYaml}>
            {t("settings.strategyPack.action.validate")}
          </Button>,
          <Button key="save" type="primary" icon={<SaveOutlined />} onClick={saveYaml}>
            {t("settings.strategyPack.action.save")}
          </Button>,
        ]}
        width={800}
      >
        <div className="space-y-3">
          <div>
            <Text type="secondary">{t("settings.strategyPack.editorIdLabel")}</Text>
            <Input
              value={yamlId}
              onChange={(e) => setYamlId(e.target.value)}
              placeholder={t("stockAnalysis.settings.packPlaceholder")}
              style={{ marginTop: 4 }}
            />
          </div>
          <div>
            <Text type="secondary">{t("settings.strategyPack.editorYamlLabel")}</Text>
            <Tooltip title={t("settings.strategyPack.editorYamlTip")}>
              <Input.TextArea
                value={yamlInput}
                onChange={(e) => setYamlInput(e.target.value)}
                rows={20}
                style={{
                  fontFamily: "monospace",
                  fontSize: 12,
                  marginTop: 4,
                }}
                placeholder={t("settings.strategyPack.editorYamlPlaceholder")}
              />
            </Tooltip>
          </div>
        </div>
      </Modal>
    </div>
  );
}
