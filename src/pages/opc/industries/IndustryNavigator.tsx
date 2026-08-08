// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业导航页面 - 集中展示所有行业入口
 */

import i18n from "@/i18n";
import {
  ApiOutlined,
  AuditOutlined,
  BankOutlined,
  BookOutlined,
  BugOutlined,
  CodeSandboxOutlined,
  CrownOutlined,
  DollarCircleOutlined,
  EditOutlined,
  ExperimentOutlined,
  GlobalOutlined,
  LineChartOutlined,
  RocketOutlined,
  SafetyCertificateOutlined,
  ShopOutlined,
  SolutionOutlined,
  TagsOutlined,
  TrophyOutlined,
  VideoCameraOutlined,
} from "@ant-design/icons";
import { Card, Col, Empty, Row, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Title, Text, Paragraph } = Typography;

/** 行业配置元数据 */
export interface IndustryMeta {
  id: string;
  icon: React.ReactNode;
  color: string;
  category: "tech" | "business" | "creative" | "finance" | "security";
  workflowCount: number;
  actionCount: number;
}

/** 行业元数据注册表 */
export const INDUSTRY_META: IndustryMeta[] = [
  {
    id: "ai-research",
    icon: <ExperimentOutlined />,
    color: "#1677ff",
    category: "tech",
    workflowCount: 2,
    actionCount: 4,
  },
  {
    id: "software-dev",
    icon: <CodeSandboxOutlined />,
    color: "#722ed1",
    category: "tech",
    workflowCount: 13,
    actionCount: 7,
  },
  {
    id: "finance-invest",
    icon: <DollarCircleOutlined />,
    color: "#52c41a",
    category: "finance",
    workflowCount: 2,
    actionCount: 3,
  },
  {
    id: "sales-growth",
    icon: <LineChartOutlined />,
    color: "#fa541c",
    category: "business",
    workflowCount: 7,
    actionCount: 5,
  },
  {
    id: "content-media",
    icon: <VideoCameraOutlined />,
    color: "#eb2f96",
    category: "creative",
    workflowCount: 6,
    actionCount: 4,
  },
  {
    id: "industry-consulting",
    icon: <CrownOutlined />,
    color: "#faad14",
    category: "business",
    workflowCount: 11,
    actionCount: 6,
  },
  {
    id: "accounting",
    icon: <AuditOutlined />,
    color: "#13c2c2",
    category: "finance",
    workflowCount: 3,
    actionCount: 3,
  },
  {
    id: "ecommerce",
    icon: <ShopOutlined />,
    color: "#f5222d",
    category: "business",
    workflowCount: 9,
    actionCount: 8,
  },
  {
    id: "education",
    icon: <BookOutlined />,
    color: "#2f54eb",
    category: "creative",
    workflowCount: 3,
    actionCount: 4,
  },
  {
    id: "design",
    icon: <EditOutlined />,
    color: "#b37feb",
    category: "creative",
    workflowCount: 4,
    actionCount: 0,
  },
  {
    id: "project-management",
    icon: <RocketOutlined />,
    color: "#fa8c16",
    category: "business",
    workflowCount: 5,
    actionCount: 0,
  },
  {
    id: "security",
    icon: <SafetyCertificateOutlined />,
    color: "#cf1322",
    category: "security",
    workflowCount: 3,
    actionCount: 0,
  },
  {
    id: "geospatial",
    icon: <GlobalOutlined />,
    color: "#389e0d",
    category: "tech",
    workflowCount: 6,
    actionCount: 0,
  },
  {
    id: "game-dev",
    icon: <BugOutlined />,
    color: "#722ed1",
    category: "creative",
    workflowCount: 3,
    actionCount: 0,
  },
];

/** 分类配置 */
const CATEGORY_CONFIG: Record<
  IndustryMeta["category"],
  { label: string; icon: React.ReactNode; color: string }
> = {
  tech: { label: i18n.t("opc.industry.categories.tech"), icon: <ApiOutlined />, color: "#1677ff" },
  business: { label: i18n.t("opc.industry.categories.business"), icon: <TrophyOutlined />, color: "#fa541c" },
  creative: { label: i18n.t("opc.industry.categories.creative"), icon: <TagsOutlined />, color: "#eb2f96" },
  finance: { label: i18n.t("opc.industry.categories.finance"), icon: <BankOutlined />, color: "#52c41a" },
  security: { label: i18n.t("opc.industry.categories.security"), icon: <SolutionOutlined />, color: "#cf1322" },
};

/** 行业卡片 */
function IndustryCard({ meta }: { meta: IndustryMeta }) {
  const navigate = useNavigate();
  const { t } = useTranslation();

  const handleClick = () => {
    navigate(`/opc/industry/${meta.id}`);
  };

  return (
    <Card
      hoverable
      onClick={handleClick}
      style={{
        cursor: "pointer",
        borderLeft: `4px solid ${meta.color}`,
        transition: "all 0.2s",
      }}
      bodyStyle={{ padding: 16 }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          marginBottom: 12,
        }}
      >
        <div
          style={{
            width: 48,
            height: 48,
            borderRadius: 8,
            background: `${meta.color}15`,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 24,
            color: meta.color,
          }}
        >
          {meta.icon}
        </div>
        <div>
          <Title level={5} style={{ margin: 0 }}>
            {t(`opc.industries.${meta.id.replace(/-/g, "_")}`)}
          </Title>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t(`opc.industries.${meta.id.replace(/-/g, "_")}_desc`)}
          </Text>
        </div>
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <Tag color="blue" style={{ fontSize: 12 }}>
          {t("opc.industry.nav.workflows", { count: meta.workflowCount })}
        </Tag>
        {meta.actionCount > 0 && (
          <Tag color="green" style={{ fontSize: 12 }}>
            {t("opc.industry.nav.actions", { count: meta.actionCount })}
          </Tag>
        )}
      </div>
    </Card>
  );
}

/** 行业导航页面 */
export function IndustryNavigatorPage() {
  const { t } = useTranslation();

  const groupedIndustries = INDUSTRY_META.reduce(
    (acc, meta) => {
      if (!acc[meta.category]) {
        acc[meta.category] = [];
      }
      acc[meta.category].push(meta);
      return acc;
    },
    {} as Record<IndustryMeta["category"], IndustryMeta[]>,
  );

  return (
    <div style={{ padding: 24, maxWidth: 1200, margin: "0 auto" }}>
      {/* 头部 */}
      <div style={{ marginBottom: 32 }}>
        <Title level={3}>{t("opc.industry.nav.title")}</Title>
        <Paragraph type="secondary">{t("opc.industry.nav.description")}</Paragraph>
      </div>

      {/* 统计概览 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 32 }}>
        <Col xs={12} sm={6}>
          <Card size="small" style={{ textAlign: "center" }}>
            <Title level={2} style={{ margin: 0, color: "#1677ff" }}>
              {INDUSTRY_META.length}
            </Title>
            <Text type="secondary">{t("opc.industry.nav.totalIndustries")}</Text>
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card size="small" style={{ textAlign: "center" }}>
            <Title level={2} style={{ margin: 0, color: "#52c41a" }}>
              {INDUSTRY_META.reduce((sum, m) => sum + m.workflowCount, 0)}
            </Title>
            <Text type="secondary">{t("opc.industry.nav.totalWorkflows")}</Text>
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card size="small" style={{ textAlign: "center" }}>
            <Title level={2} style={{ margin: 0, color: "#fa541c" }}>
              {INDUSTRY_META.reduce((sum, m) => sum + m.actionCount, 0)}
            </Title>
            <Text type="secondary">{t("opc.industry.nav.totalActions")}</Text>
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card size="small" style={{ textAlign: "center" }}>
            <Title level={2} style={{ margin: 0, color: "#722ed1" }}>
              {Object.keys(groupedIndustries).length}
            </Title>
            <Text type="secondary">{t("opc.industry.nav.totalCategories")}</Text>
          </Card>
        </Col>
      </Row>

      {/* 分类展示 */}
      {Object.entries(groupedIndustries).map(([category, industries]) => {
        const config = CATEGORY_CONFIG[category as IndustryMeta["category"]];
        return (
          <div key={category} style={{ marginBottom: 32 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                marginBottom: 16,
                paddingBottom: 8,
                borderBottom: `2px solid ${config.color}`,
              }}
            >
              <span style={{ color: config.color, fontSize: 20 }}>{config.icon}</span>
              <Title level={4} style={{ margin: 0 }}>
                {config.label}
              </Title>
              <Tag color={config.color}>{industries.length}</Tag>
            </div>

            {industries.length > 0
              ? (
                <Row gutter={[16, 16]}>
                  {industries.map((meta) => (
                    <Col key={meta.id} xs={24} sm={12} md={8} lg={6}>
                      <IndustryCard meta={meta} />
                    </Col>
                  ))}
                </Row>
              )
              : <Empty description={t("opc.industry.nav.noIndustries")} />}
          </div>
        );
      })}
    </div>
  );
}
