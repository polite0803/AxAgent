// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { Button, Card, Col, message, Row, Space, Tag, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { MarketPack } from "../utils/constants";

const { Text } = Typography;

export function MarketPackTab() {
  const { t } = useTranslation();
  const [packs, setPacks] = useState<MarketPack[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<MarketPack[]>("opc_market_list");
      setPacks(data);
    } catch (e) {
      message.error(t("opc.common.loadFailed", { error: String(e) }));
      setPacks([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div>
      <Space style={{ marginBottom: 12 }}>
        <Button size="small" type="primary" onClick={refresh} loading={loading}>
          {t("opc.market.refresh")}
        </Button>
        <Text type="secondary">{t("opc.market.subtitle")}</Text>
      </Space>
      <Row gutter={[12, 12]}>
        {packs.map((p) => (
          <Col key={p.id} xs={24} sm={12} md={8}>
            <Card
              size="small"
              title={
                <Space>
                  <span>{p.icon}</span>
                  {p.name}
                  <Tag color={p.installed ? "green" : "blue"}>
                    {p.installed ? t("opc.market.installed") : t("opc.market.notInstalled")}
                  </Tag>
                </Space>
              }
            >
              <div style={{ fontSize: 12, color: "#888" }}>
                <div>ID: {p.id}</div>
                <div>{t("opc.market.version", { version: p.version })}</div>
                <div>
                  {t("opc.market.enabled", { value: p.enabled ? t("opc.market.yes") : t("opc.market.no") })}
                </div>
              </div>
              <Space style={{ marginTop: 8 }}>
                <Button
                  size="small"
                  type={p.installed ? "default" : "primary"}
                  disabled={p.installed}
                  onClick={async () => {
                    try {
                      await invoke("opc_import_industry_pack", { archivePath: p.path });
                      message.success(t("opc.market.installSuccess", { name: p.name }));
                      refresh();
                    } catch (e) {
                      message.error(t("opc.market.installFailed", { error: String(e) }));
                    }
                  }}
                >
                  {t("opc.market.install")}
                </Button>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>
    </div>
  );
}
