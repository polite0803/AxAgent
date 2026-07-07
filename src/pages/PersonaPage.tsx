// SPDX-License-Identifier: AGPL-3.0-only

import {
  personalityCreateBootstrap,
  personalityGet,
  personalityList,
  personalitySwitch,
  personalityUpdateIdentity,
  personalityUpdateUser,
} from "@/lib/invoke";
import type { Personality, PersonalityInfo } from "@/types";
import { Button, Card, Empty, Input, message, Modal, Space, Spin, Tabs, Tag, theme, Typography } from "antd";
import { Plus, Save, User } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { TextArea } = Input;
const { Title, Text, Paragraph } = Typography;

export function PersonaPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const [personas, setPersonas] = useState<PersonalityInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<Personality | null>(null);
  const [selectedLoading, setSelectedLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [saving, setSaving] = useState(false);

  // 新建表单
  const [newName, setNewName] = useState("");
  const [newSoul, setNewSoul] = useState("");
  const [newIdentity, setNewIdentity] = useState("");
  const [newUser, setNewUser] = useState("");

  const loadList = useCallback(async () => {
    setLoading(true);
    try {
      const list = await personalityList();
      setPersonas(list);
    } catch {
      // 浏览器模式静默
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadList();
  }, [loadList]);

  const loadSelected = useCallback(async (name: string) => {
    setSelectedLoading(true);
    try {
      const p = await personalityGet(name);
      setSelected(p);
    } catch {
      message.error(t("settings.persona.editFailed"));
    } finally {
      setSelectedLoading(false);
    }
  }, [t]);

  const handleSelect: (name: string) => void = useCallback((name) => {
    loadSelected(name);
  }, [loadSelected]);

  const handleActivate = useCallback(async (name: string) => {
    try {
      await personalitySwitch(name);
      message.success(t("settings.persona.switchSuccess", { name }));
      loadList();
    } catch {
      message.error(t("settings.persona.switchFailed"));
    }
  }, [t, loadList]);

  const handleCreate = async () => {
    if (!newName.trim()) { return; }
    setSaving(true);
    try {
      await personalityCreateBootstrap({
        name: newName.trim(),
        soul: newSoul,
        identity: newIdentity,
        user: newUser,
      });
      message.success(t("settings.persona.saved"));
      setCreating(false);
      setNewName("");
      setNewSoul("");
      setNewIdentity("");
      setNewUser("");
      loadList();
    } catch {
      message.error(t("settings.persona.createFailed"));
    } finally {
      setSaving(false);
    }
  };

  const handleSaveIdentity = useCallback(async () => {
    if (!selected) { return; }
    setSaving(true);
    try {
      await personalityUpdateIdentity(selected.name, selected.identity);
      message.success(t("settings.persona.saved"));
    } catch {
      message.error(t("settings.persona.saveFailed"));
    } finally {
      setSaving(false);
    }
  }, [selected, t]);

  const handleSaveUser = useCallback(async () => {
    if (!selected) { return; }
    setSaving(true);
    try {
      await personalityUpdateUser(selected.name, selected.user);
      message.success(t("settings.persona.saved"));
    } catch {
      message.error(t("settings.persona.saveFailed"));
    } finally {
      setSaving(false);
    }
  }, [selected, t]);

  const personaList = useMemo(
    () => (
      <div style={{ display: "flex", flexDirection: "column", gap: token.marginXS }}>
        {personas.map((p) => (
          <div
            key={p.name}
            onClick={() => handleSelect(p.name)}
            style={{
              padding: `${token.paddingSM}px ${token.padding}px`,
              borderRadius: token.borderRadius,
              cursor: "pointer",
              background: selected?.name === p.name
                ? token.colorFillSecondary
                : "transparent",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              transition: "background 0.2s",
            }}
          >
            <Space>
              <User size={16} />
              <div>
                <Text strong>{p.name}</Text>
                {p.description && (
                  <Text
                    type="secondary"
                    style={{ display: "block", fontSize: token.fontSizeSM }}
                  >
                    {p.description}
                  </Text>
                )}
              </div>
            </Space>
            <Space>
              {p.is_active && <Tag color="blue">{t("settings.persona.activated")}</Tag>}
              {!p.is_active && (
                <Button
                  size="small"
                  type="link"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleActivate(p.name);
                  }}
                >
                  {t("settings.persona.activate")}
                </Button>
              )}
            </Space>
          </div>
        ))}
        {personas.length === 0 && !loading && (
          <Empty
            description={t("settings.persona.noPersona")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        )}
      </div>
    ),
    [personas, selected, loading, token, t, handleActivate, handleSelect],
  );

  const editorPanel = useMemo(() => {
    if (selectedLoading) { return <Spin style={{ margin: "40px auto", display: "block" }} />; }
    if (!selected) {
      return (
        <Empty
          description={t("settings.persona.createFirst")}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      );
    }
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: token.margin }}>
        <div>
          <Title level={5} style={{ margin: 0 }}>
            {selected.name}
          </Title>
          {selected.description && (
            <Paragraph type="secondary" style={{ margin: 0 }}>
              {selected.description}
            </Paragraph>
          )}
        </div>
        <Tabs
          items={[
            {
              key: "soul",
              label: t("settings.persona.soul.label"),
              children: (
                <div>
                  <Text type="secondary">{t("settings.persona.soul.description")}</Text>
                  <Paragraph style={{ marginTop: token.marginSM, whiteSpace: "pre-wrap" }}>
                    {selected.content || <Text type="secondary">{t("settings.persona.emptyContent")}</Text>}
                  </Paragraph>
                </div>
              ),
            },
            {
              key: "identity",
              label: t("settings.persona.identity.label"),
              children: (
                <div>
                  <Text type="secondary">{t("settings.persona.identity.description")}</Text>
                  <TextArea
                    value={selected.identity}
                    onChange={(e) => setSelected({ ...selected, identity: e.target.value })}
                    rows={6}
                    placeholder={t("settings.persona.identity.placeholder")}
                    style={{ marginTop: token.marginSM }}
                  />
                  <Button
                    type="primary"
                    icon={<Save size={14} />}
                    onClick={handleSaveIdentity}
                    loading={saving}
                    style={{ marginTop: token.marginSM }}
                  >
                    {t("settings.persona.save")}
                  </Button>
                </div>
              ),
            },
            {
              key: "user",
              label: t("settings.persona.user.label"),
              children: (
                <div>
                  <Text type="secondary">{t("settings.persona.user.description")}</Text>
                  <TextArea
                    value={selected.user}
                    onChange={(e) => setSelected({ ...selected, user: e.target.value })}
                    rows={6}
                    placeholder={t("settings.persona.user.placeholder")}
                    style={{ marginTop: token.marginSM }}
                  />
                  <Button
                    type="primary"
                    icon={<Save size={14} />}
                    onClick={handleSaveUser}
                    loading={saving}
                    style={{ marginTop: token.marginSM }}
                  >
                    {t("settings.persona.save")}
                  </Button>
                </div>
              ),
            },
          ]}
        />
      </div>
    );
  }, [selected, selectedLoading, saving, token, t, handleSaveIdentity, handleSaveUser]);

  return (
    <div style={{ padding: token.paddingLG, maxWidth: 960, margin: "0 auto" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: token.marginLG,
        }}
      >
        <div>
          <Title level={3} style={{ margin: 0 }}>
            {t("settings.persona.title")}
          </Title>
          <Text type="secondary">{t("settings.persona.description")}</Text>
        </div>
        <Button type="primary" icon={<Plus size={16} />} onClick={() => setCreating(true)}>
          {t("settings.persona.create")}
        </Button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "280px 1fr", gap: token.marginLG }}>
        <Card size="small" title={t("settings.persona.personaList")} loading={loading}>
          {personaList}
        </Card>
        <Card size="small">
          {editorPanel}
        </Card>
      </div>

      {/* 新建人格 Modal */}
      <Modal
        title={t("settings.persona.create")}
        open={creating}
        onCancel={() => setCreating(false)}
        onOk={handleCreate}
        confirmLoading={saving}
        okText={t("settings.persona.create")}
        cancelText={t("settings.persona.cancel")}
        width={600}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: token.margin }}>
          <div>
            <Text strong>{t("settings.persona.nameLabel")}</Text>
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t("settings.persona.namePlaceholder")}
              style={{ marginTop: 4 }}
            />
          </div>
          <div>
            <Text strong>{t("settings.persona.soul.label")}</Text>
            <Text type="secondary" style={{ marginLeft: 8 }}>
              {t("settings.persona.soul.description")}
            </Text>
            <TextArea
              value={newSoul}
              onChange={(e) => setNewSoul(e.target.value)}
              rows={6}
              placeholder={t("settings.persona.soul.placeholder")}
              style={{ marginTop: 4 }}
            />
          </div>
          <div>
            <Text strong>{t("settings.persona.identity.label")}</Text>
            <Text type="secondary" style={{ marginLeft: 8 }}>
              {t("settings.persona.identity.description")}
            </Text>
            <TextArea
              value={newIdentity}
              onChange={(e) => setNewIdentity(e.target.value)}
              rows={4}
              placeholder={t("settings.persona.identity.placeholder")}
              style={{ marginTop: 4 }}
            />
          </div>
          <div>
            <Text strong>{t("settings.persona.user.label")}</Text>
            <Text type="secondary" style={{ marginLeft: 8 }}>
              {t("settings.persona.user.description")}
            </Text>
            <TextArea
              value={newUser}
              onChange={(e) => setNewUser(e.target.value)}
              rows={4}
              placeholder={t("settings.persona.user.placeholder")}
              style={{ marginTop: 4 }}
            />
          </div>
        </div>
      </Modal>
    </div>
  );
}
