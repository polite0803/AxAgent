// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { Button, Card, Col, Empty, Input, Row, Space, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { TALENT_CATEGORIES, TALENT_ROLES, type TalentRole } from "../utils/constants";

export function TalentMarketTab() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [importedIds, setImportedIds] = useState<Set<string>>(new Set());
  const [importing, setImporting] = useState<string | null>(null);

  const allRoles: TalentRole[] = TALENT_ROLES;

  useEffect(() => {
    invoke<Array<{ id: string }>>("list_agency_experts")
      .then((rows) => setImportedIds(new Set(rows.map((r) => r.id))))
      .catch(() => {});
  }, []);

  const handleHire = async (roleId: string) => {
    setImporting(roleId);
    try {
      await invoke("import_agency_experts", { request: { path: "agency-agents-src" } });
      const roleName = t(allRoles.find((r) => r.id === roleId)?.nameKey || "");
      const { message } = await import("antd");
      message.success(t("opc.talent.hireSuccess", { name: roleName }));
      setImportedIds((prev) => new Set(prev).add(roleId));
    } catch (e) {
      const { message } = await import("antd");
      message.error(t("opc.talent.hireFailed", { error: String(e) }));
    } finally {
      setImporting(null);
    }
  };

  const filtered = allRoles.filter((r) => {
    if (category && r.category !== category) { return false; }
    if (search && !t(r.nameKey).includes(search) && !t(r.descriptionKey).includes(search)) { return false; }
    return true;
  });

  const categories = [...new Set(allRoles.map((r) => r.category))].sort();

  return (
    <div>
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={8}>
          <Input.Search
            placeholder={t("opc.talent.searchPlaceholder")}
            allowClear
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </Col>
        <Col span={16}>
          <Space wrap>
            <Button size="small" type={category === null ? "primary" : "default"} onClick={() => setCategory(null)}>
              {t("opc.talent.all")}
            </Button>
            {categories.map((cat) => (
              <Button
                key={cat}
                size="small"
                type={category === cat ? "primary" : "default"}
                onClick={() => setCategory(cat)}
              >
                {TALENT_CATEGORIES[cat]?.icon} {TALENT_CATEGORIES[cat] ? t(TALENT_CATEGORIES[cat].labelKey) : cat}
              </Button>
            ))}
          </Space>
        </Col>
      </Row>
      <Row gutter={[12, 12]}>
        {filtered.length === 0
          ? (
            <Col span={24}>
              <Empty description={t("opc.talent.noMatch")} />
            </Col>
          )
          : (
            filtered.map((role) => {
              const isImported = importedIds.has(role.id);
              return (
                <Col span={6} key={role.id}>
                  <Card
                    size="small"
                    hoverable
                    style={{ height: "100%" }}
                    actions={[
                      isImported ? <Tag color="green">{t("opc.talent.onboarded")}</Tag> : (
                        <Button
                          type="primary"
                          size="small"
                          loading={importing === role.id}
                          onClick={() => handleHire(role.id)}
                        >
                          {t("opc.talent.hire")}
                        </Button>
                      ),
                    ]}
                  >
                    <Card.Meta
                      avatar={<div style={{ fontSize: 28 }}>{role.icon}</div>}
                      title={<span style={{ fontSize: 13 }}>{t(role.nameKey)}</span>}
                      description={
                        <div>
                          <Tag>
                            {TALENT_CATEGORIES[role.category]?.icon} {TALENT_CATEGORIES[role.category]
                              ? t(TALENT_CATEGORIES[role.category].labelKey)
                              : role.category}
                          </Tag>
                          <div style={{ fontSize: 12, color: "rgba(255,255,255,0.6)", marginTop: 4 }}>
                            {t(role.descriptionKey)}
                          </div>
                        </div>
                      }
                    />
                  </Card>
                </Col>
              );
            })
          )}
      </Row>
    </div>
  );
}
