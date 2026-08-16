// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G6 截图持仓诊断（Screenshot Diagnosis）主面板
 *
 * 功能：
 * - 顶部：上传截图区（拖拽 / 点击选择 / 粘贴）
 * - 中部：诊断列表（Card 视图，含缩略图 / 持仓表 / 风险指标 / 中文诊断说明）
 * - 单条诊断操作：查看详情 / 归档 / 一键转为模拟观察组合（联动 G2）
 *
 * 数据源：useScreenshotDiagnosisStore
 */

import { useProviderStore } from "@/stores/feature/providerStore";
import { useScreenshotDiagnosisStore } from "@/stores/feature/screenshotDiagnosisStore";
import type { ProviderConfig } from "@/types";
import type { RiskDiagnosis, ScreenshotDiagnosis, ScreenshotPosition } from "@/types/screenshot-diagnosis";
import { UploadOutlined } from "@ant-design/icons";
import {
  Alert,
  Button,
  Card,
  Col,
  Empty,
  Image,
  Input,
  message,
  Modal,
  Progress,
  Row,
  Segmented,
  Select,
  Space,
  Spin,
  Statistic,
  Table,
  Tag,
  Typography,
  Upload,
} from "antd";
import dayjs from "dayjs";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph, Title } = Typography;

// ── 工具函数 ──

/** 安全解析 JSON 字符串（positionsJson / diagnosisJson / recommendedActions） */
function safeParse<T>(s: string | null | undefined, fallback: T): T {
  if (!s) { return fallback; }
  try {
    return JSON.parse(s) as T;
  } catch {
    return fallback;
  }
}

/** 风险等级颜色 */
function levelColor(level: string): string {
  switch (level.toLowerCase()) {
    case "high": {
      return "#cf1322";
    }
    case "medium": {
      return "#fa8c16";
    }
    case "low": {
      return "#3f8600";
    }
    default: {
      return "inherit";
    }
  }
}

/** 格式化百分比 */
function formatPct(v: number): string {
  return `${v.toFixed(2)}%`;
}

/** 格式化人民币金额 */
function formatCNY(v: number): string {
  return v.toLocaleString("zh-CN", { style: "currency", currency: "CNY" });
}

// ── 子组件：上传区 ──

interface UploadSectionProps {
  onSubmit: (imageBase64: string, sourceApp: string, providerId: string, modelId: string) => void;
  submitting: boolean;
}

function UploadSection({ onSubmit, submitting }: UploadSectionProps) {
  const { t } = useTranslation();
  const { providers, fetchProviders } = useProviderStore();
  const [imageBase64, setImageBase64] = useState<string>("");
  const [sourceApp, setSourceApp] = useState<string>("");
  const [providerId, setProviderId] = useState<string>("");
  const [modelId, setModelId] = useState<string>("");

  // 启动时拉取供应商列表
  useEffect(() => {
    if (providers.length === 0) {
      fetchProviders();
    }
  }, [providers.length, fetchProviders]);

  // 默认选中第一个供应商
  useEffect(() => {
    if (!providerId && providers.length > 0) {
      setProviderId(providers[0].id);
    }
  }, [providers, providerId]);

  // 当前供应商的可选模型
  const currentProvider: ProviderConfig | undefined = providers.find(
    (p) => p.id === providerId,
  );
  const availableModels = currentProvider?.models ?? [];

  // 供应商变化时，默认选中第一个模型
  useEffect(() => {
    if (availableModels.length > 0 && !modelId) {
      setModelId(availableModels[0].modelId);
    }
  }, [availableModels, modelId]);

  const handleFile = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result === "string") {
        setImageBase64(result);
      }
    };
    reader.readAsDataURL(file);
    return false; // 阻止 antd 默认上传
  };

  const canSubmit = imageBase64 && providerId && modelId && !submitting;

  const handleSubmit = () => {
    if (!canSubmit) { return; }
    onSubmit(imageBase64, sourceApp, providerId, modelId);
    setImageBase64("");
  };

  return (
    <Card title={t("screenshotDiagnosis.uploadTitle")} style={{ marginBottom: 16 }}>
      <Space orientation="vertical" style={{ width: "100%" }} size="middle">
        <Upload
          accept="image/png,image/jpeg"
          showUploadList={false}
          beforeUpload={handleFile}
          maxCount={1}
        >
          <Button icon={<UploadOutlined />}>{t("screenshotDiagnosis.selectImage")}</Button>
        </Upload>

        {imageBase64 && (
          <div style={{ maxWidth: 300 }}>
            <Image src={imageBase64} alt="screenshot" />
          </div>
        )}

        <Row gutter={16}>
          <Col span={8}>
            <Text type="secondary">{t("screenshotDiagnosis.sourceApp")}</Text>
            <Input
              placeholder={t("screenshotDiagnosis.sourceAppPlaceholder")}
              value={sourceApp}
              onChange={(e) => setSourceApp(e.target.value)}
            />
          </Col>
          <Col span={8}>
            <Text type="secondary">{t("screenshotDiagnosis.provider")}</Text>
            <Select
              style={{ width: "100%" }}
              value={providerId}
              onChange={setProviderId}
              options={providers.map((p) => ({ label: p.name, value: p.id }))}
            />
          </Col>
          <Col span={8}>
            <Text type="secondary">{t("screenshotDiagnosis.model")}</Text>
            <Select
              style={{ width: "100%" }}
              value={modelId}
              onChange={setModelId}
              options={availableModels.map((m) => ({
                label: m.name,
                value: m.modelId,
              }))}
            />
          </Col>
        </Row>

        <Button type="primary" onClick={handleSubmit} disabled={!canSubmit} loading={submitting}>
          {t("screenshotDiagnosis.startDiagnosis")}
        </Button>
      </Space>
    </Card>
  );
}

// ── 子组件：风险指标展示 ──

interface RiskMetricsProps {
  diagnosis: RiskDiagnosis;
}

function RiskMetrics({ diagnosis }: RiskMetricsProps) {
  const { t } = useTranslation();
  const d = diagnosis;

  return (
    <div>
      <Title level={5}>{t("screenshotDiagnosis.riskMetrics")}</Title>
      <Row gutter={[16, 16]}>
        <Col span={8}>
          <Card size="small" title={t("screenshotDiagnosis.concentrationRisk")}>
            <Statistic
              value={d.concentrationRisk.top1Weight}
              precision={2}
              suffix="%"
              styles={{ content: { color: levelColor(d.concentrationRisk.level) } }}
            />
            <div style={{ marginTop: 8 }}>
              <Tag color={levelColor(d.concentrationRisk.level)}>
                {d.concentrationRisk.level.toUpperCase()}
              </Tag>
              <Text type="secondary" style={{ marginLeft: 8 }}>
                {d.concentrationRisk.top1Code}
              </Text>
            </div>
            <Paragraph style={{ marginTop: 8, fontSize: 12, marginBottom: 0 }}>
              {d.concentrationRisk.narrative}
            </Paragraph>
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" title={t("screenshotDiagnosis.coreConcentration")}>
            <Statistic
              value={d.coreConcentration.top3Weight}
              precision={2}
              suffix="%"
              styles={{ content: { color: levelColor(d.coreConcentration.level) } }}
            />
            <div style={{ marginTop: 8 }}>
              <Tag color={levelColor(d.coreConcentration.level)}>
                {d.coreConcentration.level.toUpperCase()}
              </Tag>
            </div>
            <Paragraph style={{ marginTop: 8, fontSize: 12, marginBottom: 0 }}>
              {d.coreConcentration.narrative}
            </Paragraph>
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" title={t("screenshotDiagnosis.defenseAndExposure")}>
            <Space orientation="vertical" style={{ width: "100%" }}>
              <div>
                <Text type="secondary">{t("screenshotDiagnosis.defenseRatio")}:</Text>
                <Text>{formatPct(d.defenseRatio)}</Text>
              </div>
              <div>
                <Text type="secondary">{t("screenshotDiagnosis.usExposure")}:</Text>
                <Text>{formatPct(d.usExposure)}</Text>
              </div>
              {d.repeatedPositions.length > 0 && (
                <div>
                  <Text type="secondary">{t("screenshotDiagnosis.repeatedPositions")}:</Text>
                  {d.repeatedPositions.map((c) => (
                    <Tag key={c} color="orange">
                      {c}
                    </Tag>
                  ))}
                </div>
              )}
            </Space>
          </Card>
        </Col>
      </Row>
      {d.weakExposure.codes.length > 0 && (
        <Alert
          style={{ marginTop: 12 }}
          type="warning"
          message={t("screenshotDiagnosis.weakExposure")}
          description={d.weakExposure.narrative}
          showIcon
        />
      )}
    </div>
  );
}

// ── 子组件：诊断卡片 ──

interface DiagnosisCardProps {
  diagnosis: ScreenshotDiagnosis;
  onArchive: (id: string) => void;
  onConvertToPortfolio: (diagnosis: ScreenshotDiagnosis) => void;
}

function DiagnosisCard({ diagnosis, onArchive, onConvertToPortfolio }: DiagnosisCardProps) {
  const { t } = useTranslation();
  const positions = safeParse<ScreenshotPosition[]>(diagnosis.positionsJson, []);
  const riskDiagnosis = safeParse<RiskDiagnosis>(diagnosis.diagnosisJson, {
    concentrationRisk: { top1Code: "", top1Weight: 0, level: "", narrative: "" },
    overlapPositions: [],
    defenseRatio: 0,
    usExposure: 0,
    weakExposure: { codes: [], totalWeight: 0, narrative: "" },
    repeatedPositions: [],
    coreConcentration: { top3Codes: [], top3Weight: 0, level: "", narrative: "" },
  });
  const recommendedActions = safeParse<string[]>(diagnosis.recommendedActions, []);

  const columns = [
    { title: t("screenshotDiagnosis.code"), dataIndex: "code", width: 100 },
    { title: t("screenshotDiagnosis.name"), dataIndex: "name", width: 120 },
    {
      title: t("screenshotDiagnosis.qty"),
      dataIndex: "qty",
      width: 100,
      render: (v: number) => v.toLocaleString(),
    },
    {
      title: t("screenshotDiagnosis.costPrice"),
      dataIndex: "costPrice",
      width: 90,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("screenshotDiagnosis.marketValue"),
      dataIndex: "marketValue",
      width: 110,
      render: (v: number) => formatCNY(v),
    },
    {
      title: t("screenshotDiagnosis.weight"),
      dataIndex: "weight",
      width: 80,
      render: (v: number) => <Progress percent={Math.min(v, 100)} size="small" format={() => formatPct(v)} />,
    },
  ];

  return (
    <Card
      title={
        <Space>
          <span>{dayjs(diagnosis.createdAt).format("YYYY-MM-DD HH:mm")}</span>
          {diagnosis.sourceApp && <Tag color="blue">{diagnosis.sourceApp}</Tag>}
          <Tag color={diagnosis.status === "active" ? "green" : "default"}>
            {diagnosis.status}
          </Tag>
        </Space>
      }
      extra={
        <Space>
          <Button size="small" onClick={() => onConvertToPortfolio(diagnosis)}>
            {t("screenshotDiagnosis.toPortfolio")}
          </Button>
          {diagnosis.status === "active" && (
            <Button size="small" danger onClick={() => onArchive(diagnosis.id)}>
              {t("screenshotDiagnosis.archive")}
            </Button>
          )}
        </Space>
      }
      style={{ marginBottom: 16 }}
    >
      <Row gutter={16}>
        {diagnosis.imageThumbnailBase64 && (
          <Col span={6}>
            <Image
              src={`data:image/png;base64,${diagnosis.imageThumbnailBase64}`}
              alt="thumbnail"
              style={{ maxWidth: "100%" }}
            />
          </Col>
        )}
        <Col span={diagnosis.imageThumbnailBase64 ? 18 : 24}>
          <Table
            columns={columns}
            dataSource={positions}
            rowKey="code"
            pagination={false}
            size="small"
          />
        </Col>
      </Row>

      <div style={{ marginTop: 16 }}>
        <RiskMetrics diagnosis={riskDiagnosis} />
      </div>

      {diagnosis.narrative && (
        <Alert
          style={{ marginTop: 12 }}
          type="info"
          message={t("screenshotDiagnosis.narrative")}
          description={diagnosis.narrative}
          showIcon
        />
      )}

      {recommendedActions.length > 0 && (
        <div style={{ marginTop: 12 }}>
          <Text strong>{t("screenshotDiagnosis.recommendedActions")}:</Text>
          <ul style={{ marginTop: 4, marginBottom: 0 }}>
            {recommendedActions.map((a, i) => <li key={i}>{a}</li>)}
          </ul>
        </div>
      )}
    </Card>
  );
}

// ── 主组件 ──

export function ScreenshotDiagnosisDashboard() {
  const { t } = useTranslation();
  const store = useScreenshotDiagnosisStore();
  const [filterStatus, setFilterStatus] = useState<string>("active");
  const [convertTarget, setConvertTarget] = useState<ScreenshotDiagnosis | null>(null);
  const [portfolioName, setPortfolioName] = useState("");
  const [portfolioSourceEvent, setPortfolioSourceEvent] = useState("");

  useEffect(() => {
    store.fetchRecent(50);
  }, []);

  const handleCreateFromImage = async (
    imageBase64: string,
    sourceApp: string,
    providerId: string,
    modelId: string,
  ) => {
    try {
      await store.createFromImage({ imageBase64, sourceApp, providerId, modelId });
      message.success(t("screenshotDiagnosis.createSuccess"));
      await store.fetchRecent(50);
    } catch (e) {
      message.error(`${t("screenshotDiagnosis.createFailed")}: ${e}`);
    }
  };

  const handleArchive = async (id: string) => {
    try {
      await store.archiveDiagnosis(id);
      message.success(t("screenshotDiagnosis.archiveSuccess"));
    } catch (e) {
      message.error(`${t("screenshotDiagnosis.archiveFailed")}: ${e}`);
    }
  };

  const handleConvertToPortfolio = (diagnosis: ScreenshotDiagnosis) => {
    setConvertTarget(diagnosis);
    setPortfolioName(
      `${t("screenshotDiagnosis.defaultPortfolioName")} ${
        dayjs(diagnosis.createdAt).format(
          "MM-DD",
        )
      }`,
    );
    setPortfolioSourceEvent(
      `${t("screenshotDiagnosis.portfolioSourceEvent")} ${
        dayjs(diagnosis.createdAt).format(
          "YYYY-MM-DD",
        )
      }`,
    );
  };

  const handleConfirmConvert = async () => {
    if (!convertTarget || !portfolioName || !portfolioSourceEvent) { return; }
    try {
      await store.convertToPaperPortfolio({
        diagnosisId: convertTarget.id,
        name: portfolioName,
        sourceEvent: portfolioSourceEvent,
      });
      message.success(t("screenshotDiagnosis.convertSuccess"));
      setConvertTarget(null);
    } catch (e) {
      message.error(`${t("screenshotDiagnosis.convertFailed")}: ${e}`);
    }
  };

  const filteredList = useMemo(() => {
    if (filterStatus === "all") { return store.recentDiagnoses; }
    return store.recentDiagnoses.filter((d) => d.status === filterStatus);
  }, [store.recentDiagnoses, filterStatus]);

  return (
    <div style={{ padding: 16 }}>
      <UploadSection
        onSubmit={handleCreateFromImage}
        submitting={store.submitting}
      />

      <Card
        title={t("screenshotDiagnosis.listTitle")}
        extra={
          <Segmented
            value={filterStatus}
            onChange={(v) => setFilterStatus(v as string)}
            options={[
              { label: t("screenshotDiagnosis.statusActive"), value: "active" },
              { label: t("screenshotDiagnosis.statusArchived"), value: "archived" },
              { label: t("screenshotDiagnosis.statusAll"), value: "all" },
            ]}
          />
        }
      >
        {store.loadingList
          ? (
            <div style={{ textAlign: "center", padding: 48 }}>
              <Spin size="large" />
            </div>
          )
          : filteredList.length === 0
          ? <Empty description={t("screenshotDiagnosis.empty")} />
          : (
            filteredList.map((d) => (
              <DiagnosisCard
                key={d.id}
                diagnosis={d}
                onArchive={handleArchive}
                onConvertToPortfolio={handleConvertToPortfolio}
              />
            ))
          )}
      </Card>

      <Modal
        title={t("screenshotDiagnosis.convertModalTitle")}
        open={!!convertTarget}
        onOk={handleConfirmConvert}
        onCancel={() => setConvertTarget(null)}
        confirmLoading={store.converting}
        okText={t("screenshotDiagnosis.confirmConvert")}
        cancelText={t("common.cancel")}
      >
        <Space orientation="vertical" style={{ width: "100%" }} size="middle">
          <div>
            <Text type="secondary">{t("screenshotDiagnosis.portfolioName")}:</Text>
            <Input
              value={portfolioName}
              onChange={(e) => setPortfolioName(e.target.value)}
              placeholder={t("screenshotDiagnosis.portfolioNamePlaceholder")}
            />
          </div>
          <div>
            <Text type="secondary">{t("screenshotDiagnosis.portfolioSourceEvent")}:</Text>
            <Input
              value={portfolioSourceEvent}
              onChange={(e) => setPortfolioSourceEvent(e.target.value)}
              placeholder={t("screenshotDiagnosis.portfolioSourceEventPlaceholder")}
            />
          </div>
        </Space>
      </Modal>
    </div>
  );
}
