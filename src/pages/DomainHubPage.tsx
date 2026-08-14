// SPDX-License-Identifier: AGPL-3.0-only
// ! 能力域聚合入口页（DomainHub）
//
// 每个标准域（见 domainMeta.ts）的「域路径」路由渲染本页面，集中展示该域下
// 归入的内置导航项。域路径由 CAPABILITY_DOMAIN_META 定义（如 /finance、/automation）。

import { CAPABILITY_DOMAIN_META } from "@/lib/domainMeta";
import { navItemsByDomain } from "@/lib/navRegistry";
import { Button, Card, Col, Empty, Result, Row, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";

const { Title, Paragraph } = Typography;

/** 从当前路径解析对应当前标准域 */
function domainFromPath(pathname: string) {
  return CAPABILITY_DOMAIN_META.find(
    (d) => pathname === d.path || pathname.startsWith(`${d.path}/`),
  );
}

export function DomainHubPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();

  const domain = domainFromPath(location.pathname);
  if (!domain) {
    return (
      <Result
        status="404"
        title="404"
        subTitle={t("error.pageNotFound")}
        extra={
          <Button type="primary" onClick={() => navigate("/")}>
            {t("common.back")}
          </Button>
        }
      />
    );
  }

  const items = navItemsByDomain(domain.id);

  return (
    <div style={{ padding: 24, maxWidth: 1200, margin: "0 auto" }}>
      {/* 域头部 */}
      <div style={{ marginBottom: 32 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span
            style={{
              display: "inline-block",
              width: 14,
              height: 14,
              borderRadius: "50%",
              backgroundColor: domain.color,
            }}
          />
          <Title level={3} style={{ margin: 0 }}>
            {t(domain.labelKey)}
          </Title>
        </div>
        <Paragraph type="secondary" style={{ marginTop: 8 }}>
          {t("domain.description")}
        </Paragraph>
      </div>

      {items.length > 0
        ? (
          <Row gutter={[16, 16]}>
            {items.map((item) => (
              <Col key={item.key} xs={24} sm={12} md={8} lg={6}>
                <Card
                  hoverable
                  onClick={() => navigate(item.path)}
                  style={{
                    cursor: "pointer",
                    borderLeft: `4px solid ${domain.color}`,
                    transition: "all 0.2s",
                  }}
                  bodyStyle={{ padding: 16 }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    <div
                      style={{
                        width: 40,
                        height: 40,
                        borderRadius: 8,
                        backgroundColor: `${domain.color}15`,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        color: domain.color,
                        flexShrink: 0,
                      }}
                    >
                      {item.icon}
                    </div>
                    <Title level={5} style={{ margin: 0 }}>
                      {item.isPlugin ? item.labelKey : t(item.labelKey)}
                    </Title>
                  </div>
                </Card>
              </Col>
            ))}
          </Row>
        )
        : <Empty description={t("domain.empty")} />}
    </div>
  );
}
